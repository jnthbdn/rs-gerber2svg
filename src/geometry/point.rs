use gerber_parser::gerber_types::{CoordinateFormat, CoordinateNumber, Coordinates};

use crate::error::{ConversionError, Gerber2SvgError};

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        return Point { x, y };
    }

    pub fn from_coordinates(coord: Coordinates, default: &Point) -> Self {
        Point {
            x: coord.x.map(|x| x.into()).unwrap_or(default.x),
            y: coord.y.map(|x| x.into()).unwrap_or(default.y),
        }
    }

    pub fn from_option_coordinates(coord: Option<Coordinates>, default: &Point) -> Self {
        match coord {
            Some(c) => Self::from_coordinates(c, default),
            None => default.clone(),
        }
    }

    pub fn into_coordinates(
        &self,
        format: CoordinateFormat,
    ) -> Result<Coordinates, Gerber2SvgError> {
        Ok(Coordinates {
            x: Some(
                CoordinateNumber::try_from(self.x)
                    .map_err(|x| ConversionError::PointToCoordError(x.to_string()))?,
            ),
            y: Some(
                CoordinateNumber::try_from(self.y)
                    .map_err(|x| ConversionError::PointToCoordError(x.to_string()))?,
            ),
            format: format,
        })
    }
}
