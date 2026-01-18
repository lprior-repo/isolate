//! Introspect command - discover jjz capabilities
//!
//! This command enables AI agents to understand available commands,
//! system state, and dependencies. The module is organized into:
//!
//! - `specs`: Command specifications (builders and routing)
//! - `command_specs`: Command spec aggregation interface
//! - `dependencies`: Dependency checking and version detection
//! - `system_state`: System state gathering
//! - `output`: Human-readable and JSON output formatting

mod command_specs;
mod dependencies;
mod output;
pub mod specs;
mod system_state;

use anyhow::Result;
use zjj_core::introspection::IntrospectOutput;
use zjj_core::json::{SchemaEnvelope, SchemaType};

use crate::commands::introspect::{
    command_specs::get_command_spec,
    dependencies::check_dependencies,
    output::{print_command_spec, print_full_output},
    system_state::get_system_state,
};

/// Run the introspect command - show all capabilities
///
/// # Arguments
/// * `json` - Whether to output JSON format instead of human-readable
///
/// # Errors
/// Returns error if JSON serialization fails
pub async fn run(json: bool) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mut output = IntrospectOutput::new(version);

    output.dependencies = check_dependencies();
    output.system_state = get_system_state().await;

    if json {
        let envelope = SchemaEnvelope::new(SchemaType::Introspect, &output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_full_output(&output);
    }

    Ok(())
}

/// Run command-specific introspection
///
/// # Arguments
/// * `command` - Name of the command to introspect
/// * `json` - Whether to output JSON format instead of human-readable
///
/// # Errors
/// Returns error if command is unknown or JSON serialization fails
pub async fn run_command_introspect(command: &str, json: bool) -> Result<()> {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    let introspection = get_command_spec(command).map_err(anyhow::Error::msg)?;

    if json {
        let envelope = SchemaEnvelope::new(SchemaType::CommandSpec, &introspection);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_command_spec(&introspection);
    }

    Ok(())
}
