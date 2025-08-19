use thiserror::Error;

use crate::Gerber2SVG;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("IO Error occrured: {0}")]
    IOError(std::io::Error),

    #[error("Fatal parsing error occurred (no partial document available): {0}.")]
    ParseError(String),

    #[error("A non-fatal parsing error occurred: {1}. A partial document is available, but the final output may be degraded.")]
    NonFatalError(Gerber2SVG, String),

    #[error("No coordinate format specified in file. Please ensure that the `%FS...*%` command is present.")]
    MissingCoordinatesFormat,
}

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("IO Error occrured: {0}")]
    IOError(std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Unable to convert Point to Coordinates: {0}.")]
    PointToCoordError(String),
}

#[derive(Error, Debug)]
pub enum Gerber2SvgError {
    #[error("[Import Error] {0}")]
    ImportError(ImportError),

    #[error("[Export Error] {0}")]
    ExportError(ExportError),

    #[error("[Conversion Error] {0}")]
    ConversionError(ConversionError),
}

macro_rules! impl_from_error {
    ($class:ident) => {
        impl From<$class> for Gerber2SvgError {
            fn from(val: $class) -> Self {
                Gerber2SvgError::$class(val)
            }
        }
    };
}

impl_from_error!(ImportError);
impl_from_error!(ExportError);
impl_from_error!(ConversionError);
