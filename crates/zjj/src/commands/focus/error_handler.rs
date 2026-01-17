//! Error handling for focus command
//!
//! Handles JSON error output and exit code mapping with zero-panic, zero-unwrap design.

use std::process;

use crate::json_output::{ErrorDetail, ErrorOutput};

/// Error types for focus command with corresponding exit codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusError {
    /// Validation error (exit code 1)
    Validation,
    /// System/TTY error (exit code 2)
    System,
    /// Session/database not found (exit code 3)
    NotFound,
}

impl FocusError {
    /// Get the exit code for this error type
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Validation => 1,
            Self::System => 2,
            Self::NotFound => 3,
        }
    }

    /// Get the error code string for JSON output
    pub fn code_string(&self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION_ERROR",
            Self::System => "SYSTEM_ERROR",
            Self::NotFound => "SESSION_NOT_FOUND",
        }
    }
}

/// Output error as JSON and exit with specified exit code
///
/// This function never returns (marked with `-> !`).
/// Uses fallback JSON serialization if the primary method fails.
///
/// # Arguments
/// * `error_type` - Type of error (determines exit code)
/// * `error_msg` - Human-readable error message
/// * `suggestion` - Optional suggestion for user (None for system errors)
pub fn output_error_json_and_exit(
    error_type: FocusError,
    error_msg: &str,
    suggestion: Option<String>,
) -> ! {
    let exit_code = error_type.exit_code();

    let output = ErrorOutput {
        success: false,
        error: ErrorDetail {
            code: error_type.code_string().to_string(),
            message: error_msg.to_string(),
            details: None,
            suggestion: suggestion.clone(),
        },
    };

    // Try to serialize to JSON; use fallback if it fails
    match serde_json::to_string(&output) {
        Ok(json) => println!("{json}"),
        Err(_) => {
            // Fallback: construct JSON manually with escaped strings
            let sugg = suggestion
                .map(|s| format!(",\"suggestion\":\"{}\"", s.replace('"', "\\\"")))
                .unwrap_or_default();
            println!(
                r#"{{"success":false,"error":{{"code":"{}","message":"{msg}"{sugg}}}}}"#,
                error_type.code_string(),
                msg = error_msg.replace('"', "\\\"")
            );
        }
    }

    process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_exit_codes() {
        assert_eq!(FocusError::Validation.exit_code(), 1);
        assert_eq!(FocusError::System.exit_code(), 2);
        assert_eq!(FocusError::NotFound.exit_code(), 3);
    }

    #[test]
    fn test_error_code_strings() {
        assert_eq!(FocusError::Validation.code_string(), "VALIDATION_ERROR");
        assert_eq!(FocusError::System.code_string(), "SYSTEM_ERROR");
        assert_eq!(FocusError::NotFound.code_string(), "SESSION_NOT_FOUND");
    }
}
