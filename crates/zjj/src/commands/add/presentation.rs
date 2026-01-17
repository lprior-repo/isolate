//! Output and presentation layer for add command
//!
//! This module handles all output formatting, including JSON serialization
//! and human-readable messages. It separates presentation concerns from
//! business logic.

use anyhow::Result;

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
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("✓ Session '{name}' created successfully");
    }
    Ok(())
}

/// Output JSON error and exit
///
/// This function does not return - it prints the error and exits with code 1
pub fn output_json_error(name: &str, error: &anyhow::Error) -> ! {
    let output = AddOutput {
        success: false,
        session_name: name.to_string(),
        workspace_path: String::new(),
        zellij_tab: String::new(),
        status: "failed".to_string(),
    };
    eprintln!("Error: {error}");
    if let Ok(json) = serde_json::to_string(&output) {
        println!("{json}");
    }
    std::process::exit(1);
}
