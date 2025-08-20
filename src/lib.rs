use std::fs::File;
use std::io::BufReader;

use gerber_parser::gerber_types::{self, Aperture, Command, GCode, InterpolationMode, Unit};
use gerber_parser::gerber_types::{CoordinateOffset, FunctionCode};
use gerber_parser::{parse, GerberDoc};

use log::warn;
use svg;
use svg::node::element::{path, Circle, Path, Rectangle};

mod geometry;
use geometry::point::Point;

pub mod error;
use crate::error::{ExportError, Gerber2SvgError, ImportError};

const SVG_COLOR_ELEMENT: &str = "black";

#[derive(Debug)]
pub struct Gerber2SVG {
    gerber_doc: GerberDoc,
    unit: Unit,
    scale: f32,

    draw_state: InterpolationMode,
    position: Point,
    selected_aperture: Option<Aperture>,

    svg_document: svg::Document,
    current_path_data: path::Data,

    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Gerber2SVG {
    /// Create instance from a Gerber file
    /// * filename: `&str` path to the gerber file
    pub fn from_file(filename: &str) -> Result<Self, ImportError> {
        let file = File::open(filename).map_err(ImportError::IOError)?;
        let reader = BufReader::new(file);

        match parse(reader) {
            Ok(doc) => Self::from_gerber_doc(doc),
            Err((doc, error)) => {
                if doc.apertures.is_empty() && doc.commands.is_empty() {
                    Err(ImportError::ParseError(error.to_string()))
                } else {
                    Err(ImportError::NonFatalError(
                        Self::from_gerber_doc(doc)?,
                        error.to_string(),
                    ))
                }
            }
        }
    }

    /// Create Instance form GerberDoc struct
    /// * gerber_doc: `GerberDoc` struct
    pub fn from_gerber_doc(gerber_doc: GerberDoc) -> Result<Self, ImportError> {
        if gerber_doc.format_specification.is_none() {
            Err(ImportError::MissingCoordinatesFormat)
        } else if gerber_doc.units.is_none() {
            Err(ImportError::MissingUnit)
        } else {
            let unit = gerber_doc.units.clone().unwrap();
            Ok(Self {
                gerber_doc: gerber_doc,
                unit,
                scale: 1.0,
                draw_state: InterpolationMode::Linear,
                position: Point::new(0.0, 0.0),
                selected_aperture: None,
                svg_document: svg::Document::new(),
                current_path_data: path::Data::new(),
                min_x: f64::INFINITY,
                max_x: f64::NEG_INFINITY,
                min_y: f64::INFINITY,
                max_y: f64::NEG_INFINITY,
            })
        }
    }

    /// Set the scale of the path and aperture. (Must be called **before** the build function)
    /// * scale : `f32` the scale value (> 0.0)
    pub fn set_scale(mut self, scale: f32) -> Self {
        if scale > 0.0 {
            self.scale = scale;
        } else {
            log::warn!("Scale value need to be greater than 0.0. Skip scale setting");
        }

        return self;
    }

    /// Save the gerber as SVG file
    /// * filename: `&str` path to save the SVG file
    /// * crop: `bool` trim unused space
    pub fn save_svg(&mut self, filename: &str, crop: bool) -> Result<(), Gerber2SvgError> {
        self.set_bbox(crop);
        svg::save(filename, &self.svg_document).map_err(|x| ExportError::IOError(x).into())
    }

    /// Get SVG as String
    /// * crop: `bool` trim unused space
    pub fn to_string(&mut self, crop: bool) -> String {
        self.set_bbox(crop);
        self.svg_document.to_string()
    }

    /// Build the SVG
    pub fn build(mut self) -> Self {
        log::debug!("Start building...");
        for i in 0..self.gerber_doc.commands.len() {
            if self.gerber_doc.commands[i].is_err() {
                continue;
            }

            let command = self.gerber_doc.commands[i].as_ref().cloned().unwrap();

            match command {
                gerber_types::Command::FunctionCode(f) => match f {
                    FunctionCode::DCode(d) => match d {
                        gerber_types::DCode::Operation(o) => match o {
                            gerber_types::Operation::Interpolate(coord, offset) => {
                                if coord.is_none() {
                                    warn!("D01 (Interpolate) operation without coordinates is not allowed. Operation skipped.");
                                    continue;
                                }

                                let target =
                                    Point::from_coordinates(coord.clone().unwrap(), &self.position);

                                if self.draw_state == InterpolationMode::Linear {
                                    self.add_draw_segment(&target);
                                } else {
                                    if offset.is_none() {
                                        warn!(
                                            "Offset is required in Counter/Clockwise Circular mode"
                                        );
                                        continue;
                                    }
                                    self.add_arc_segment(&target, offset.as_ref().unwrap())
                                }

                                self.move_position(&target);
                            }
                            gerber_types::Operation::Move(m) => {
                                if m.is_none() {
                                    warn!("D02 (Move) operation without coordinates is not allowed. Operation skipped.");
                                    continue;
                                }
                                let target =
                                    Point::from_coordinates(m.clone().unwrap(), &self.position);
                                log::debug!("Move to {:?}, create path.", target);
                                self.create_path_from_data();
                                self.move_position(&target);
                            }
                            gerber_types::Operation::Flash(f) => {
                                let pts = Point::from_option_coordinates(f.clone(), &self.position);
                                self.create_path_from_data();
                                self.place_aperture(&pts);
                                self.move_position(&pts);
                            }
                        },
                        gerber_types::DCode::SelectAperture(i) => {
                            self.create_path_from_data();
                            self.selected_aperture = Some(
                                self.gerber_doc
                                    .apertures
                                    .get(&i)
                                    .expect(format!("Unknown aperture id '{}'", i).as_str())
                                    .clone(),
                            )
                        }
                    },
                    FunctionCode::GCode(g) => match g {
                        GCode::InterpolationMode(im) => self.draw_state = im,
                        GCode::Comment(c) => log::info!("[COMMENT] \"{:?}\"", c),
                        _ => log::error!("Unsupported GCode:\r\n{:#?}", g),
                    },
                    FunctionCode::MCode(_) => (),
                },
                Command::ExtendedCode(_) => (),
            };
        }

        return self;
    }

    fn place_aperture(&mut self, target: &Point) -> () {
        // let target = Self::coordinate_to_float(coord);
        // let target = (
        //     target.0.unwrap_or(self.position.x),
        //     target.1.unwrap_or(self.position.y),
        // );

        let mut doc = std::mem::replace(&mut self.svg_document, svg::Document::new());

        log::debug!(
            "Place aperture {:?} to {:?}",
            self.selected_aperture,
            &target
        );

        match self
            .selected_aperture
            .as_ref()
            .expect("No aperture selected")
        {
            Aperture::Circle(c) => {
                let radius = (c.diameter / 2.0) * self.scale as f64;
                let circle = Circle::new()
                    .set("cx", self.with_unit(target.x))
                    .set("cy", self.with_unit(target.y))
                    .set("r", radius)
                    .set("fill", SVG_COLOR_ELEMENT);
                doc = doc.add(circle);
                self.check_bbox(target.x, target.y, radius, radius);
            }
            Aperture::Rectangle(r) => {
                let width = r.x * self.scale as f64;
                let height = r.y * self.scale as f64;
                let x = target.x - width / 2.0;
                let y = target.y - height / 2.0;

                let rect = Rectangle::new()
                    .set("x", self.with_unit(x))
                    .set("y", self.with_unit(y))
                    .set("width", self.with_unit(width))
                    .set("height", self.with_unit(height))
                    .set("fill", SVG_COLOR_ELEMENT);
                doc = doc.add(rect);
                self.check_bbox(target.x, target.y, width / 2.0, height / 2.0);
            }
            Aperture::Obround(o) => log::error!("Unsupported Obround aperture:\r\n{o:#?}"),
            Aperture::Polygon(p) => log::error!("Unsupported Polygon aperture:\r\n{p:#?}"),
            Aperture::Macro(macro_str, macro_decimals) => {
                log::error!("Unsupported Macro aperture:\r\n{macro_str} -- {macro_decimals:#?}")
            }
        }

        self.svg_document = doc;
    }

    fn add_draw_segment(&mut self, target: &Point) -> () {
        let mut path = std::mem::take(&mut self.current_path_data);

        log::debug!("Draw segment from {:?} to {:?}", self.position, &target);

        if path.is_empty() {
            path = path.move_to((self.position.x, self.position.y));
        }

        self.current_path_data = path.line_to((target.x, target.y));

        let stroke = self.get_path_stroke();
        self.check_bbox(target.x, target.y, stroke / 2.0, stroke / 2.0);
    }

    fn add_arc_segment(&mut self, _target: &Point, _offset: &CoordinateOffset) -> () {
        log::warn!("Arc are not supported ! Skip.",);
        //TODO : self.check_bbox(...);
    }

    fn move_position(&mut self, coord: &Point) -> () {
        self.position = coord.clone();
    }

    fn create_path_from_data(&mut self) {
        if self.current_path_data.is_empty() {
            return;
        }

        let mut stroke = self.get_path_stroke(); // * (self.scale * 2.0);

        if self.scale > 1.0 {
            stroke *= 2.0;
        } else if self.scale < 1.0 {
            stroke /= 2.0;
        }

        let data = std::mem::replace(&mut self.current_path_data, path::Data::new());
        let svg = std::mem::replace(&mut self.svg_document, svg::Document::new());

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", SVG_COLOR_ELEMENT)
            .set("stroke-width", self.with_unit(stroke))
            .set("d", data);

        self.svg_document = svg.add(path);
    }

    fn get_path_stroke(&self) -> f64 {
        return match self
            .selected_aperture
            .as_ref()
            .expect("No selected aperture for storke")
        {
            Aperture::Circle(c) => c.diameter,
            _ => {
                log::warn!(
                    "Unsupported stroke aperture other than Circle.\r\n{:#?}",
                    self.selected_aperture
                );
                0.0
            }
        };
    }

    // fn coordinate_to_float(coord: &Coordinates) -> (Option<f32>, Option<f32>) {
    //     let mut result: (Option<f32>, Option<f32>) = (None, None);

    //     if coord.x.is_some() {
    //         result.0 = Some(
    //             coord
    //                 .x
    //                 .unwrap()
    //                 .gerber(&coord.format)
    //                 .unwrap()
    //                 .parse::<f32>()
    //                 .unwrap()
    //                 / 10_f32.powi(coord.format.decimal as i32),
    //         );
    //     }

    //     if coord.y.is_some() {
    //         result.1 = Some(
    //             coord
    //                 .y
    //                 .unwrap()
    //                 .gerber(&coord.format)
    //                 .unwrap()
    //                 .parse::<f32>()
    //                 .unwrap()
    //                 / 10_f32.powi(coord.format.decimal as i32),
    //         )
    //     }

    //     return result;
    // }

    // fn coordinate_offset_to_float(coord: &CoordinateOffset) -> (Option<f32>, Option<f32>) {
    //     let mut result: (Option<f32>, Option<f32>) = (None, None);

    //     if coord.x.is_some() {
    //         result.0 = Some(
    //             coord
    //                 .x
    //                 .unwrap()
    //                 .gerber(&coord.format)
    //                 .unwrap()
    //                 .parse::<f32>()
    //                 .unwrap()
    //                 / 10_f32.powi(coord.format.decimal as i32),
    //         );
    //     }

    //     if coord.y.is_some() {
    //         result.1 = Some(
    //             coord
    //                 .y
    //                 .unwrap()
    //                 .gerber(&coord.format)
    //                 .unwrap()
    //                 .parse::<f32>()
    //                 .unwrap()
    //                 / 10_f32.powi(coord.format.decimal as i32),
    //         )
    //     }

    //     return result;
    // }

    fn check_bbox(&mut self, pos_x: f64, pos_y: f64, stroke_x: f64, stroke_y: f64) {
        self.min_x = f64::min(pos_x - stroke_x, self.min_x);
        self.max_x = f64::max(pos_x + stroke_x, self.max_x);
        self.min_y = f64::min(pos_y - stroke_y, self.min_y);
        self.max_y = f64::max(pos_y + stroke_y, self.max_y);
    }

    fn set_bbox(&mut self, crop: bool) {
        let mut doc = std::mem::replace(&mut self.svg_document, svg::Document::new());

        if crop {
            log::info!("Crop enable");
            doc = doc
                // .set(
                //     "viewbox",
                //     (
                //         format!("{}{}", self.min_x, unit),
                //         format!("{}{}", self.min_y, unit),
                //         format!("{}{}", self.max_x - self.min_x, unit),
                //         format!("{}{}", self.max_y - self.min_y, unit),
                //     ),
                // )
                .set("width", self.with_unit(self.max_x - self.min_x))
                .set("height", self.with_unit(self.max_y - self.min_y));
        } else {
            log::debug!("Crop disable");
            doc = doc
                // .set(
                //     "viewbox",
                //     (
                //         0,
                //         0,
                //         format!("{}{}", self.max_x, unit),
                //         format!("{}{}", self.max_y, unit),
                //     ),
                // )
                .set("width", self.with_unit(self.max_x))
                .set("height", self.with_unit(self.max_y));
        }

        self.svg_document = doc;
    }

    fn with_unit(&self, val: f64) -> String {
        format!(
            "{}{}",
            val,
            match self.unit {
                Unit::Inches => "in",
                Unit::Millimeters => "mm",
            }
        )
    }
}
