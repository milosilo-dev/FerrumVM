use std::fmt;

use sdl2::IntegerOrSdlError;
use sdl2::render::TextureValueError;

#[derive(Debug)]
pub enum FerrumSDLContextError {
    SdlIntError(IntegerOrSdlError),
    SdlGeneralError(String),
    TextureError(TextureValueError),
    WindowInitError(String),
}

impl fmt::Display for FerrumSDLContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SdlIntError(e) => write!(f, "Sdl Int Error: {}", e),
            Self::SdlGeneralError(e) => write!(f, "Sdl Error: {}", e),
            Self::TextureError(e) => write!(f, "Texture Error: {}", e),
            Self::WindowInitError(e) => write!(f, "Could not create window: {}", e),
        }
    }
}

impl From<String> for FerrumSDLContextError {
    fn from(e: String) -> Self {
        Self::SdlGeneralError(e)
    }
}

impl From<IntegerOrSdlError> for FerrumSDLContextError {
    fn from(e: IntegerOrSdlError) -> Self {
        Self::SdlIntError(e)
    }
}

impl From<TextureValueError> for FerrumSDLContextError {
    fn from(e: TextureValueError) -> Self {
        Self::TextureError(e)
    }
}
