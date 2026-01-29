//! JSON output module for zjj commands
//!
//! This module provides structured JSON output types and error handling
//! for all zjj CLI commands. It is organized into two submodules:
//!
//! - [`serializers`]: Command output structures (InitOutput, AddOutput, etc.)
//! - [`error`]: Error conversion and JSON error formatting

use anyhow::Error;

pub mod error;
pub mod serializers;

// Re-export commonly used types for convenience
pub use error::{output_json_error_and_exit, SyncError};
pub use serializers::{
    AddOutput, DiffOutput, DiffStat, FileDiffStat, FocusOutput, InitOutput, RemoveOutput,
    SyncOutput,
};

/// Output a JSON success response to stdout
///
/// This is a convenience function for serializing any serializable type
/// to pretty-printed JSON and outputting it to stdout.
pub fn output_json_success<T: serde::Serialize>(data: &T) -> Result<(), Error> {
    let json_str = serde_json::to_string_pretty(data)
        .map_err(|e| Error::msg(format!("Failed to serialize JSON: {e}")))?;
    println!("{json_str}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_json_success_basic() {
        let data = serde_json::json!({
            "message": "success",
            "value": 42
        });

        let result = output_json_success(&data);
        assert!(result.is_ok(), "output_json_success should succeed");
    }

    #[test]
    fn test_output_json_success_with_struct() {
        let output = AddOutput {
            name: "test".to_string(),
            workspace_path: "/path/to/workspace".to_string(),
            zellij_tab: "zjj:test".to_string(),
            status: "active".to_string(),
        };

        let result = output_json_success(&output);
        assert!(
            result.is_ok(),
            "output_json_success should succeed with AddOutput"
        );
    }

    #[test]
    fn test_output_json_success_complex_nested() {
        let data = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": "deep value"
                },
                "array": [1, 2, 3]
            }
        });

        let result = output_json_success(&data);
        assert!(
            result.is_ok(),
            "output_json_success should handle nested structures"
        );
    }
}
