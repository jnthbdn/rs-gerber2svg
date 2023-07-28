use std::fs::File;
use std::io::BufReader;

use gerber_parser::gerber_doc::GerberDoc;
use gerber_parser::parser::parse_gerber;
use gerber_types::{Aperture, Command, Coordinates, GCode, InterpolationMode};
use gerber_types::{CoordinateOffset, FunctionCode};

use svg;
use svg::node::element::{path, Circle, Path, Rectangle};

mod point;
use crate::point::Point;


pub struct Gerber2SVG {
    gerber_doc: GerberDoc,
    scale: f32,

    draw_state: InterpolationMode,
    position: Point,
    selected_aperture: Option<Aperture>,

    svg_document: svg::Document,
    current_path_data: path::Data,

    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl Gerber2SVG {
    /// Create instance from a Gerber file 
    /// * filename: `&str` path to the gerber file
    pub fn from_file(filename: &str) -> Result<Self, std::io::Error> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        let gerber_doc: GerberDoc = parse_gerber(reader);

        Ok(Self::from_gerber_doc(gerber_doc))
    }

    /// Create Instance form GerberDoc struct
    /// * gerber_doc: `GerberDoc` struct
    pub fn from_gerber_doc(gerber_doc: GerberDoc) -> Self {
        let s = Self {
            gerber_doc: gerber_doc,
            scale: 1.0,
            draw_state: InterpolationMode::Linear,
            position: Point::new(0.0, 0.0),
            selected_aperture: None,
            svg_document: svg::Document::new(),//.set("viewbox", (0, 0, 80, 80)),
            current_path_data: path::Data::new(),
            min_x: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            min_y: f32::INFINITY,
            max_y: f32::NEG_INFINITY,
        };

        return s;
    }

    
    /// Set the scale of the path and aperture. (Must be called **before** the build function)
    /// * scale : `f32` the scale value (> 0.0)
    pub fn set_scale(mut self, scale: f32) -> Self {
        
        if scale > 0.0 {
            self.scale = scale;
        }
        else{
            log::warn!("Scale value need to be greater than 0.0. Skip scale setting");
        }

        return self;
    }

    /// Save the gerber as SVG file
    /// * filename: `&str` path to save the SVG file
    /// * crop: `bool` trim unused space
    pub fn save_svg(&mut self, filename: &str, crop: bool) -> std::io::Result<()> {
        self.set_bbox(crop);
        svg::save(filename, &self.svg_document)
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
        for c in &self.gerber_doc.commands.clone() {
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
                                    self.gerber_doc
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

        return self;
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
                let radius = (c.diameter / 2.0) * self.scale as f64;
                let circle = Circle::new()
                    .set("cx", target.0)
                    .set("cy", target.1)
                    .set("r", radius)
                    .set("fill", "white");
                doc = doc.add(circle);
                self.check_bbox(target.0, target.1, radius as f32, radius as f32);
            }
            Aperture::Rectangle(r) => {

                let width = r.x * self.scale as f64;
                let height = r.y * self.scale as f64;
                let x = target.0 - (width / 2.0) as f32;
                let y = target.1 - (width / 2.0) as f32;

                let rect = Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", width)
                    .set("height", height)
                    .set("fill", "white");
                doc = doc.add(rect);
                self.check_bbox(target.0, target.1, (width / 2.0) as f32, (height / 2.0) as f32);
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

        let stroke = self.get_path_stroke();
        self.check_bbox(target.0, target.1, stroke / 2.0, stroke / 2.0);
    }

    fn add_arc_segment(&mut self, coord: &Coordinates, offset: &CoordinateOffset) -> () {
        log::debug!(
            "Draw arc from {:?} to {:?} with offset {:?}",
            self.position,
            Self::coordinate_to_float(coord),
            Self::coordinate_offset_to_float(offset)
        );
        log::warn!("Arc are not supported ! Skip.",);
        //TODO : self.check_bbox(...);
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

        let mut stroke = self.get_path_stroke(); // * (self.scale * 2.0);

        if self.scale > 1.0 {
            stroke *= 2.0;
        }
        else if self.scale < 1.0 {
            stroke /= 2.0;
        }

        let data = std::mem::replace(&mut self.current_path_data, path::Data::new());
        let svg = std::mem::replace(&mut self.svg_document, svg::Document::new());

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", "white")
            .set("stroke-width", stroke)
            .set("d", data);

        self.svg_document = svg.add(path);
    }

    fn get_path_stroke(&self) -> f32 {
        return match self
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

    fn check_bbox(&mut self, pos_x: f32, pos_y: f32, stroke_x: f32, stroke_y: f32){
        self.min_x = f32::min(pos_x - stroke_x, self.min_x);
        self.max_x = f32::max(pos_x + stroke_x, self.max_x);
        self.min_y = f32::min(pos_y - stroke_y, self.min_y);
        self.max_y = f32::max(pos_y + stroke_y, self.max_y);
    }

    fn set_bbox(&mut self, crop: bool){
        let mut doc = std::mem::replace(&mut self.svg_document, svg::Document::new());

        if crop{
            log::debug!("Crop enable");
            doc = doc.set("viewbox", (self.min_x, self.min_y, self.max_x - self.min_x, self.max_y - self.min_y));
        }
        else{
            log::debug!("Crop disable");
            doc = doc.set("viewbox", (0, 0, self.max_x, self.max_y));
        }

        self.svg_document = doc;
    }
}
