use std::fmt;

#[derive(Debug)]
pub enum ForecasterError {
    ParseError(String),
    InvalidTimePeriod(String),
    FlatbufferError(String),
}

impl fmt::Display for ForecasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForecasterError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ForecasterError::InvalidTimePeriod(msg) => write!(f, "Invalid time period: {}", msg),
            ForecasterError::FlatbufferError(msg) => write!(f, "Flatbuffer error: {}", msg),
        }
    }
}

impl std::error::Error for ForecasterError {}

impl From<std::num::ParseIntError> for ForecasterError {
    fn from(err: std::num::ParseIntError) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}

impl From<std::str::Utf8Error> for ForecasterError {
    fn from(err: std::str::Utf8Error) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}

impl From<quick_xml::Error> for ForecasterError {
    fn from(err: quick_xml::Error) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}

impl From<quick_xml::events::attributes::AttrError> for ForecasterError {
    fn from(err: quick_xml::events::attributes::AttrError) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}

impl From<base64::DecodeError> for ForecasterError {
    fn from(err: base64::DecodeError) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}

impl From<chrono::ParseError> for ForecasterError {
    fn from(err: chrono::ParseError) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}

impl From<String> for ForecasterError {
    fn from(err: String) -> Self {
        ForecasterError::ParseError(err)
    }
}

impl From<&str> for ForecasterError {
    fn from(err: &str) -> Self {
        ForecasterError::ParseError(err.to_string())
    }
}
