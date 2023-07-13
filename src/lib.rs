use std::fs::File;
use std::io::BufReader;

use gerber_parser::gerber_doc::GerberDoc;
use gerber_parser::parser::parse_gerber;
use gerber_types::{Aperture, Command, Coordinates, GCode, InterpolationMode};
use gerber_types::{CoordinateOffset, FunctionCode};

use svg;
// use svg::node::element::Path;
use svg::node::element::{path, Circle, Path, Rectangle};

mod point;
use crate::point::Point;

pub struct Gerber2SVG {
    //Gerber fields
    draw_state: InterpolationMode,
    position: Point,
    selected_aperture: Option<Aperture>,

    // SVG fields
    svg_document: svg::Document,
    current_path_data: path::Data,
}

impl Gerber2SVG {
    pub fn from_file(filename: &str) -> Result<Self, std::io::Error> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        let gerber_doc: GerberDoc = parse_gerber(reader);

        Ok(Self::from_gerber_doc(&&gerber_doc))
    }

    pub fn from_gerber_doc(gerber_doc: &GerberDoc) -> Self {
        let mut s = Self {
            draw_state: InterpolationMode::Linear,
            position: Point::new(0.0, 0.0),
            selected_aperture: None,
            svg_document: svg::Document::new().set("viewbox", (0, 0, 80, 80)),
            current_path_data: path::Data::new(),
        };

        s.build(&gerber_doc);

        return s;
    }

    pub fn save_svg(&self, filename: &str) -> std::io::Result<()> {
        svg::save(filename, &self.svg_document)
    }

    pub fn to_string(&self) -> String {
        self.svg_document.to_string()
    }

    fn build(&mut self, gerber_doc: &GerberDoc) -> () {
        log::debug!("Start building...");
        for c in &gerber_doc.commands {
            match c {
                gerber_types::Command::FunctionCode(f) => {
                    match f {
                        FunctionCode::DCode(d) => match d {
                            gerber_types::DCode::Operation(o) => match o {
                                gerber_types::Operation::Interpolate(coord, offset) => {
                                    if self.draw_state == InterpolationMode::Linear {
                                        self.add_draw_segment(coord);
                                    } else {
                                        self.add_arc_segment(coord, offset.as_ref().expect(format!("No offset coord with 'Circular' state\r\n{:#?}", c).as_str()))
                                    }
                                    self.move_position(coord);
                                }
                                gerber_types::Operation::Move(m) => {
                                    log::debug!("Move to {:?}, create path.", &m);
                                    self.create_path_from_data();
                                    self.move_position(m);
                                }
                                gerber_types::Operation::Flash(f) => {
                                    self.create_path_from_data();
                                    self.place_aperture(f);
                                    self.move_position(f);
                                }
                            },
                            gerber_types::DCode::SelectAperture(i) => {
                                self.create_path_from_data();
                                self.selected_aperture = Some(
                                    gerber_doc
                                        .apertures
                                        .get(&i)
                                        .expect(format!("Unknown aperture id '{}'", i).as_str())
                                        .clone(),
                                )
                            }
                        },
                        FunctionCode::GCode(g) => match g {
                            GCode::InterpolationMode(im) => self.draw_state = *im,
                            GCode::Comment(c) => log::info!("[COMMENT] \"{}\"", c),
                            _ => log::error!("Unsupported GCode:\r\n{:#?}", g),
                        },
                        FunctionCode::MCode(_) => (),
                    }
                }
                Command::ExtendedCode(_) => (),
            };
        }
    }

    fn place_aperture(&mut self, coord: &Coordinates) -> () {
        let target = Self::coordinate_to_float(coord);
        let target = (
            target.0.unwrap_or(self.position.x),
            target.1.unwrap_or(self.position.y),
        );

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
                let circle = Circle::new()
                    .set("cx", target.0)
                    .set("cy", target.1)
                    .set("r", c.diameter / 2.0)
                    .set("fill", "white");
                doc = doc.add(circle);
            }
            Aperture::Rectangle(r) => {
                let rect = Rectangle::new()
                    .set("x", target.0 - (r.x / 2.0) as f32)
                    .set("y", target.1 - (r.y / 2.0) as f32)
                    .set("width", r.x)
                    .set("height", r.y)
                    .set("fill", "white");
                doc = doc.add(rect);
            }
            Aperture::Obround(o) => log::error!("Unsupported Obround aperture:\r\n{:#?}", o),
            Aperture::Polygon(p) => log::error!("Unsupported Polygon aperture:\r\n{:#?}", p),
            Aperture::Other(o) => log::error!("Unsupported Other aperture:\r\n{:#?}", o),
        }

        self.svg_document = doc;
    }

    fn add_draw_segment(&mut self, coord: &Coordinates) -> () {
        let target = Self::coordinate_to_float(coord);
        let target = (
            target.0.unwrap_or(self.position.x),
            target.1.unwrap_or(self.position.y),
        );
        let mut path = std::mem::take(&mut self.current_path_data);

        log::debug!("Draw segment from {:?} to {:?}", self.position, &target);

        if path.is_empty() {
            path = path.move_to((self.position.x, self.position.y));
        }

        self.current_path_data = path.line_to((target.0, target.1));
    }

    fn add_arc_segment(&mut self, coord: &Coordinates, offset: &CoordinateOffset) -> () {
        log::debug!(
            "Draw arc from {:?} to {:?} with offset {:?}",
            self.position,
            Self::coordinate_to_float(coord),
            Self::coordinate_offset_to_float(offset)
        );
        log::warn!("Arc are not supported ! Skip.",);
    }

    fn move_position(&mut self, coord: &Coordinates) -> () {
        let pos = Self::coordinate_to_float(coord);

        self.position.x = pos.0.unwrap_or(self.position.x);
        self.position.y = pos.1.unwrap_or(self.position.y);
    }

    fn create_path_from_data(&mut self) {
        if self.current_path_data.is_empty() {
            return;
        }

        let stroke = match self
            .selected_aperture
            .as_ref()
            .expect("No selected aperture for storke")
        {
            Aperture::Circle(c) => c.diameter as f32,
            _ => {
                log::warn!(
                    "Unsupported stroke aperture other than Circle.\r\n{:#?}",
                    self.selected_aperture
                );
                0_f32
            }
        };

        let data = std::mem::replace(&mut self.current_path_data, path::Data::new());
        let svg = std::mem::replace(&mut self.svg_document, svg::Document::new());

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", "white")
            .set("stroke-width", stroke)
            .set("d", data);

        self.svg_document = svg.add(path);
    }

    fn coordinate_to_float(coord: &Coordinates) -> (Option<f32>, Option<f32>) {
        let mut result: (Option<f32>, Option<f32>) = (None, None);

        if coord.x.is_some() {
            result.0 = Some(
                coord
                    .x
                    .unwrap()
                    .gerber(&coord.format)
                    .unwrap()
                    .parse::<f32>()
                    .unwrap()
                    / 10_f32.powi(coord.format.decimal as i32),
            );
        }

        if coord.y.is_some() {
            result.1 = Some(
                coord
                    .y
                    .unwrap()
                    .gerber(&coord.format)
                    .unwrap()
                    .parse::<f32>()
                    .unwrap()
                    / 10_f32.powi(coord.format.decimal as i32),
            )
        }

        return result;
    }

    fn coordinate_offset_to_float(coord: &CoordinateOffset) -> (Option<f32>, Option<f32>) {
        let mut result: (Option<f32>, Option<f32>) = (None, None);

        if coord.x.is_some() {
            result.0 = Some(
                coord
                    .x
                    .unwrap()
                    .gerber(&coord.format)
                    .unwrap()
                    .parse::<f32>()
                    .unwrap()
                    / 10_f32.powi(coord.format.decimal as i32),
            );
        }

        if coord.y.is_some() {
            result.1 = Some(
                coord
                    .y
                    .unwrap()
                    .gerber(&coord.format)
                    .unwrap()
                    .parse::<f32>()
                    .unwrap()
                    / 10_f32.powi(coord.format.decimal as i32),
            )
        }

        return result;
    }
}
