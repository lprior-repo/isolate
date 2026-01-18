//! Error builder and conversion utilities for JSON error responses.
//!
//! Provides fluent builders for constructing rich error responses and
//! automatic conversion from internal error types to JSON error format.

use super::types::{ErrorCode, ErrorDetail, JsonError};
use serde_json::json;

impl JsonError {
    /// Create a new JSON error with just a code and message.
    ///
    /// # Functional Pattern
    /// Immutable constructor that returns the error in a buildable state.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                details: None,
                suggestion: None,
            },
        }
    }

    /// Add structured details to the error using immutable update.
    ///
    /// # Functional Pattern
    /// Returns a new instance with updated details, preserving immutability
    /// and enabling method chaining.
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        Self {
            success: self.success,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                details: Some(details),
                suggestion: self.error.suggestion,
            },
        }
    }

    /// Add a resolution suggestion to the error using immutable update.
    ///
    /// # Functional Pattern
    /// Returns a new instance with updated suggestion, preserving immutability
    /// and enabling method chaining.
    #[must_use]
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        Self {
            success: self.success,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                details: self.error.details,
                suggestion: Some(suggestion.into()),
            },
        }
    }

    /// Convert to pretty-printed JSON string.
    ///
    /// # Functional Pattern
    /// Returns a Result type for error propagation using the `?` operator.
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::parse_error(format!("Failed to serialize error: {e}")))
    }
}

impl From<ErrorCode> for JsonError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code, "An error occurred")
    }
}

impl From<&crate::Error> for JsonError {
    #[allow(clippy::too_many_lines)]
    fn from(err: &crate::Error) -> Self {
        use crate::{
            error::execution::ExecutionError, error::system::SystemError,
            error::validation::ValidationError, Error,
        };

        let (code, message, suggestion) = match err {
            Error::Validation(v_err) => {
                // Map validation errors
                let display_msg = v_err.to_string();
                if display_msg.contains("Invalid configuration:") {
                    (
                        ErrorCode::ConfigParseError,
                        display_msg,
                        Some("Check your configuration file for errors".to_string()),
                    )
                } else if display_msg.contains("Parse error:") {
                    (ErrorCode::ConfigParseError, display_msg, None)
                } else {
                    (ErrorCode::InvalidArgument, display_msg, None)
                }
            }
            Error::System(s_err) => {
                // Map system errors
                let display_msg = s_err.to_string();
                if display_msg.contains("IO error:") {
                    (ErrorCode::Unknown, display_msg, None)
                } else if display_msg.contains("Hook") {
                    (
                        ErrorCode::HookFailed,
                        display_msg,
                        Some(
                            "Check your hook configuration and ensure the command is correct"
                                .to_string(),
                        ),
                    )
                } else if display_msg.contains("JJ") || display_msg.contains("jj") {
                    if display_msg.contains("not installed") || display_msg.contains("not in PATH")
                    {
                        (
                            ErrorCode::JjNotInstalled,
                            display_msg,
                            Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
                        )
                    } else {
                        (ErrorCode::JjCommandFailed, display_msg, None)
                    }
                } else if display_msg.contains("Zellij") {
                    (ErrorCode::ZellijCommandFailed, display_msg, None)
                } else {
                    (ErrorCode::Unknown, display_msg, None)
                }
            }
            Error::Execution(e_err) => {
                // Map execution errors
                let display_msg = e_err.to_string();
                if display_msg.contains("Not found:") {
                    (
                        ErrorCode::SessionNotFound,
                        display_msg,
                        Some("Use 'zjj list' to see available sessions".to_string()),
                    )
                } else if display_msg.contains("Database error:") {
                    (
                        ErrorCode::StateDbCorrupted,
                        display_msg,
                        Some("Try running 'zjj doctor --fix' to repair the database".to_string()),
                    )
                } else if display_msg.contains("No commits") {
                    (
                        ErrorCode::InvalidState,
                        display_msg,
                        Some(
                            "Create an initial commit: jj commit -m \"Initial commit\"".to_string(),
                        ),
                    )
                } else if display_msg.contains("bookmark") {
                    (
                        ErrorCode::InvalidState,
                        display_msg,
                        Some(
                            "Create the missing bookmark with: jj bookmark create <name>"
                                .to_string(),
                        ),
                    )
                } else {
                    (ErrorCode::Unknown, display_msg, None)
                }
            }
            Error::Unknown(msg) => (ErrorCode::Unknown, format!("Unknown error: {msg}"), None),
        };

        // Functional immutable update: apply suggestion if present
        match suggestion {
            Some(sugg) => Self::new(code, message).with_suggestion(sugg),
            None => Self::new(code, message),
        }
    }
}

impl From<crate::Error> for JsonError {
    fn from(err: crate::Error) -> Self {
        Self::from(&err)
    }
}

/// Helper to create error details with available sessions as context.
///
/// # Functional Pattern
/// Pure function that combines error building with contextual data enrichment.
pub fn error_with_available_sessions(
    code: ErrorCode,
    message: impl Into<String>,
    session_name: impl Into<String>,
    available: &[String],
) -> JsonError {
    let details = json!({
        "session_name": session_name.into(),
        "available_sessions": available,
    });

    JsonError::new(code, message)
        .with_details(details)
        .with_suggestion("Use 'zjj list' to see available sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error_new() {
        let err = JsonError::new("TEST_ERROR", "Test error message");
        assert_eq!(err.error.code, "TEST_ERROR");
        assert_eq!(err.error.message, "Test error message");
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_with_details() {
        let details = json!({"key": "value"});
        let err = JsonError::new("TEST_ERROR", "Test").with_details(details.clone());

        assert!(err.error.details.is_some());
        assert_eq!(err.error.details, Some(details));
    }

    #[test]
    fn test_json_error_with_suggestion() {
        let err = JsonError::new("TEST_ERROR", "Test").with_suggestion("Try this instead");

        assert_eq!(err.error.suggestion, Some("Try this instead".to_string()));
    }

    #[test]
    fn test_json_error_chain_methods() {
        let details = json!({"key": "value"});
        let err = JsonError::new("TEST_ERROR", "Test")
            .with_details(details)
            .with_suggestion("Try this");

        assert!(err.error.details.is_some());
        assert_eq!(err.error.suggestion, Some("Try this".to_string()));
    }

    #[test]
    fn test_error_with_available_sessions() {
        let available = vec!["session1".to_string(), "session2".to_string()];
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "Session 'foo' not found",
            "foo",
            &available,
        );

        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert!(err.error.details.is_some());
        assert!(err.error.suggestion.is_some());
    }

    #[test]
    fn test_json_error_serialization() -> crate::Result<()> {
        let err = JsonError::new("TEST_ERROR", "Test message");
        let json = err.to_json()?;

        assert!(json.contains("\"code\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));

        Ok(())
    }
}
