//! Error formatting utilities for CLI output
//!
//! Provides error formatting functions for both human-readable and JSON output modes.
//! Follows functional Rust patterns with zero unwraps and zero panics.

use crate::json_output;

/// Format an error for user display (no stack traces)
///
/// Extracts the root cause message and includes context from the error chain
/// if it adds value. Uses functional pattern matching instead of imperative logic.
///
/// # Arguments
/// * `err` - The error to format
///
/// # Returns
/// A formatted error message suitable for display to users
#[must_use]
pub fn format_error(err: &anyhow::Error) -> String {
    // Get the root cause message
    let mut msg = err.to_string();

    // If the error chain has more context, include it
    if let Some(source) = err.source() {
        let source_msg = source.to_string();
        // Only add source if it's different and adds value
        if !msg.contains(&source_msg) && !source_msg.is_empty() {
            msg = format!("{msg}\nCause: {source_msg}");
        }
    }

    msg
}

/// Extract appropriate exit code from an error
///
/// Tries to downcast to `zjj_core::Error` to get semantic exit codes.
/// Falls back to exit code 2 (system error) for unknown errors.
///
/// # Exit Codes
/// * 0 - Success (not returned here)
/// * 1 - User error (invalid input, validation failure, bad configuration)
/// * 2 - System error (IO failure, external command error, hook failure)
/// * 3 - Not found (session not found, resource missing, JJ not installed)
/// * 4 - Invalid state (database corruption, unhealthy system)
///
/// # Arguments
/// * `err` - The error to extract exit code from
///
/// # Returns
/// An appropriate exit code based on the error type
#[must_use]
pub fn get_exit_code(err: &anyhow::Error) -> i32 {
    // Try to downcast to zjj_core::Error
    if let Some(core_err) = err.downcast_ref::<zjj_core::Error>() {
        return core_err.exit_code();
    }

    // Check if it's a wrapped IO error (not found)
    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
        return if io_err.kind() == std::io::ErrorKind::NotFound {
            3 // Not found
        } else {
            2 // System error
        };
    }

    // Default to system error for anyhow errors
    2
}

/// Output error in JSON format
///
/// Serializes the error to JSON and outputs it to stdout.
/// Uses a functional approach with Result-based error handling.
///
/// # Arguments
/// * `code` - Error code string (e.g., "ERROR", "`RUNTIME_ERROR`")
/// * `message` - Human-readable error message
/// * `suggestion` - Optional suggestion for how to fix the error
///
/// # Panics
/// Never panics. Falls back to a simple JSON string if serialization fails.
pub fn output_json_error(code: &str, message: &str, suggestion: Option<String>) {
    let error_output = json_output::ErrorOutput {
        success: false,
        error: json_output::ErrorDetail {
            code: code.to_string(),
            message: message.to_string(),
            details: None, // No structured details for generic CLI errors
            suggestion,
        },
    };

    // Attempt to serialize to JSON, with fallback
    #[allow(clippy::single_match_else)]
    match serde_json::to_string(&error_output) {
        Ok(json_str) => println!("{json_str}"),
        Err(_) => {
            // Fallback if JSON serialization fails
            // Escape quotes in message and code for JSON safety
            let escaped_code = code.replace('"', "\\\"");
            let escaped_message = message.replace('"', "\\\"");
            eprintln!(
                "{{\"success\":false,\"error\":{{\"code\":\"{escaped_code}\",\"message\":\"{escaped_message}\"}}}}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_error_simple() {
        let err = anyhow::anyhow!("Simple error");
        let formatted = format_error(&err);
        assert_eq!(formatted, "Simple error");
    }

    #[test]
    fn test_format_error_with_context() {
        let err = anyhow::anyhow!("Root cause").context("Additional context");
        let formatted = format_error(&err);
        assert!(formatted.contains("Additional context"));
    }

    #[test]
    fn test_get_exit_code_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let err = anyhow::Error::new(io_err);
        assert_eq!(get_exit_code(&err), 3);
    }

    #[test]
    fn test_get_exit_code_generic_error() {
        let err = anyhow::anyhow!("Generic error");
        assert_eq!(get_exit_code(&err), 2);
    }

    #[test]
    fn test_output_json_error_no_panic() {
        // This test ensures that output_json_error never panics
        output_json_error("TEST_ERROR", "Test message", None);
        output_json_error(
            "TEST_ERROR",
            "Test with suggestion",
            Some("Try this".to_string()),
        );
    }
}
