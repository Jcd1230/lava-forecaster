use std::fmt;

/// Standardized error type for all forecaster operations.
///
/// Variants are organized by failure domain:
/// - **Parse errors**: Malformed input (XML, JSON, DSL, date strings)
/// - **Date arithmetic errors**: Invalid or out-of-range date calculations
/// - **Evaluation errors**: Failures during dose evaluation or forecasting
/// - **Serialization errors**: Response encoding failures (FlatBuffers, JSON)
#[derive(Debug)]
pub enum ForecasterError {
    /// Failed to parse input data (XML, JSON, or other wire formats).
    ParseError(String),

    /// A time period string could not be interpreted.
    InvalidTimePeriod(String),

    /// A date arithmetic operation produced an invalid or out-of-range date.
    DateArithmeticError {
        operation: &'static str,
        detail: String,
    },

    /// FlatBuffers serialization/deserialization failure.
    FlatbufferError(String),

    /// A test DSL file could not be parsed.
    TestDslError { line: usize, detail: String },
}

impl fmt::Display for ForecasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForecasterError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ForecasterError::InvalidTimePeriod(msg) => write!(f, "Invalid time period: {}", msg),
            ForecasterError::DateArithmeticError { operation, detail } => {
                write!(f, "Date arithmetic error in {}: {}", operation, detail)
            }
            ForecasterError::FlatbufferError(msg) => write!(f, "Flatbuffer error: {}", msg),
            ForecasterError::TestDslError { line, detail } => {
                write!(f, "Test DSL error at line {}: {}", line, detail)
            }
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
