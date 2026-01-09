//! Custom error types with comprehensive error information.

use std::fmt;

/// The result type for ZJJ operations.
pub type ZJJResult<T> = std::result::Result<T, Error>;

/// Errors that can occur in ZJJ operations.
#[derive(Debug, Clone)]
pub enum Error {
    /// Invalid configuration.
    InvalidConfig(String),

    /// IO error description.
    IoError(String),

    /// Parsing error.
    ParseError(String),

    /// Validation error.
    ValidationError(String),

    /// Resource not found.
    NotFound(String),

    /// Unknown error.
    Unknown(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_config() {
        let err = Error::InvalidConfig("test error".into());
        assert_eq!(err.to_string(), "Invalid configuration: test error");
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::IoError(_)));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::InvalidConfig("test".into());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidConfig"));
    }
}
