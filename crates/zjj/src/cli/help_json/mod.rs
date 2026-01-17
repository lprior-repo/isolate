//! Machine-readable help output for AI agents
//!
//! This module provides structured JSON help output that AI agents
//! can parse to understand the entire CLI surface area.

mod commands;
mod metadata;
mod types;

pub use types::*;

/// Generate complete CLI documentation
pub fn generate_cli_documentation() -> CliDocumentation {
    CliDocumentation {
        version: env!("CARGO_PKG_VERSION").to_string(),
        tool: metadata::generate_tool_metadata(),
        commands: commands::generate_command_docs(),
        categories: metadata::generate_categories(),
        workflows: metadata::generate_workflows(),
        exit_codes: metadata::generate_exit_codes(),
        prerequisites: metadata::generate_prerequisites(),
    }
}
