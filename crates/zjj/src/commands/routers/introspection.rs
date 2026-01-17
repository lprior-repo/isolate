//! Introspection commands router
//!
//! Routes introspection commands (context, introspect, dashboard, doctor)
//! These commands provide metadata about the system, command structures, and diagnostic information.

use anyhow::Result;

use crate::commands::{context, dashboard, doctor, introspect};

/// Handle introspection commands
///
/// Routes introspection-related commands to their appropriate handlers.
/// Introspection commands provide metadata, diagnostics, and system information.
///
/// # Errors
///
/// Returns an error if the command execution fails
pub async fn handle_introspection_cmd(cmd: &str, sub_m: &clap::ArgMatches) -> Result<()> {
    match cmd {
        "context" | "ctx" => handle_context_cmd(sub_m).await,
        "introspect" => handle_introspect_cmd(sub_m).await,
        "dashboard" | "dash" => handle_dashboard_cmd(sub_m).await,
        "doctor" | "check" => handle_doctor_cmd(sub_m).await,
        _ => Err(anyhow::anyhow!("Unknown introspection command: {cmd}")),
    }
}

/// Handle the 'context' command (alias: 'ctx')
async fn handle_context_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    context::run(sub_m.get_flag("json")).await
}

/// Handle the 'introspect' command
async fn handle_introspect_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    match sub_m.get_one::<String>("command").map(String::as_str) {
        Some(cmd) => introspect::run_command_introspect(cmd, json).await,
        None => introspect::run(json).await,
    }
}

/// Handle the 'dashboard' command (alias: 'dash')
async fn handle_dashboard_cmd(_sub_m: &clap::ArgMatches) -> Result<()> {
    dashboard::run().await
}

/// Handle the 'doctor' command (alias: 'check')
async fn handle_doctor_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    doctor::run(sub_m.get_flag("json"), sub_m.get_flag("fix")).await
}
