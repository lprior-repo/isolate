//! Output and presentation layer for add command
//!
//! This module handles all output formatting, including JSON serialization
//! and human-readable messages. It separates presentation concerns from
//! business logic.

use anyhow::Result;
use zjj_core::json::ErrorDetail;

use crate::json_output::AddOutput;

/// Output success message in JSON or human-readable format
///
/// # Errors
/// Returns error if JSON serialization fails
pub fn output_success(name: &str, workspace_path: &str, json: bool) -> Result<()> {
    if json {
        let output = AddOutput {
            success: true,
            session_name: name.to_string(),
            workspace_path: workspace_path.to_string(),
            zellij_tab: format!("jjz:{name}"),
            status: "active".to_string(),
            error: None,
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("✓ Session '{name}' created successfully");
    }
    Ok(())
}

/// Determine the error code based on the error message
fn determine_error_code(error_msg: &str) -> &'static str {
    let msg_lower = error_msg.to_lowercase();
    if msg_lower.contains("already exists") || msg_lower.contains("duplicate") {
        "DUPLICATE_SESSION"
    } else if msg_lower.contains("validation")
        || msg_lower.contains("invalid")
        || msg_lower.contains("must start with")
        || msg_lower.contains("cannot start with")
    {
        "VALIDATION_ERROR"
    } else if msg_lower.contains("database") {
        "DATABASE_ERROR"
    } else if msg_lower.contains("workspace") {
        "WORKSPACE_ERROR"
    } else {
        "ADD_FAILED"
    }
}

/// Output JSON error and exit
///
/// This function does not return - it prints the error as JSON to stdout and exits with code 1
/// In JSON mode, all output must be JSON on stdout (not stderr) for AI agent compatibility.
pub fn output_json_error(name: &str, error: &anyhow::Error) -> ! {
    let error_msg = error.to_string();
    let error_code = determine_error_code(&error_msg);

    let error_detail = ErrorDetail {
        code: error_code.to_string(),
        message: error_msg,
        details: None,
        suggestion: None,
    };

    let output = AddOutput {
        success: false,
        session_name: name.to_string(),
        workspace_path: String::new(),
        zellij_tab: String::new(),
        status: "failed".to_string(),
        error: Some(error_detail),
    };
    // Only output JSON to stdout, no stderr output in JSON mode
    if let Ok(json) = serde_json::to_string(&output) {
        println!("{json}");
    } else {
        // If JSON serialization fails, output a minimal error JSON
        let fallback = r#"{"success":false,"error":"Failed to serialize error response"}"#;
        println!("{fallback}");
    }
    std::process::exit(1);
}
