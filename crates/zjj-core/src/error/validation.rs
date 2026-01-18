//! Validation error types for configuration and input validation.
//!
//! These errors represent user input or configuration problems that can be
//! corrected by the user.

use std::fmt;

/// Validation errors represent incorrect user input or configuration.
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Invalid configuration provided
    InvalidConfig(String),
    /// Parse error when reading configuration or data
    ParseError(String),
    /// Generic validation error
    ValidationError(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::ValidationError(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl ValidationError {
    /// Get exit code for validation errors (always 1).
    pub const fn exit_code(&self) -> i32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_config_display() {
        let err = ValidationError::InvalidConfig("test error".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: test error");
    }

    #[test]
    fn test_parse_error_display() {
        let err = ValidationError::ParseError("invalid json".to_string());
        assert_eq!(err.to_string(), "Parse error: invalid json");
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::ValidationError("invalid input".to_string());
        assert_eq!(err.to_string(), "Validation error: invalid input");
    }

    #[test]
    fn test_validation_exit_code() {
        assert_eq!(
            ValidationError::InvalidConfig("test".to_string()).exit_code(),
            1
        );
        assert_eq!(
            ValidationError::ParseError("test".to_string()).exit_code(),
            1
        );
        assert_eq!(
            ValidationError::ValidationError("test".to_string()).exit_code(),
            1
        );
    }
}
