//! CLI output formatting functions
//!
//! This module provides output formatting for help and structured output.
//! For error formatting, see `cli::error`.

use std::process;

use super::help_json;

/// Output machine-readable help in JSON format
///
/// Prints the complete CLI documentation as JSON to stdout.
/// Includes all commands, validation rules, workflows, exit codes, and prerequisites.
/// This provides AI agents with complete understanding of the CLI surface area.
pub fn output_help_json() {
    let docs = help_json::generate_cli_documentation();

    if let Ok(json_str) = serde_json::to_string_pretty(&docs) {
        println!("{json_str}");
    } else {
        eprintln!("Error: Failed to serialize help output");
        process::exit(2);
    }
}
