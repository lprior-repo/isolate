//! Command dispatch logic for routing CLI commands to their handlers
//!
//! This module delegates to command routers which organize commands into logical groups:
//! - Session commands: session management (add, remove, focus, list, status, add-batch)
//! - Utility commands: supporting functionality (backup, restore, completions, query)
//! - Introspection commands: metadata and diagnostics (context, introspect, dashboard, doctor)

use anyhow::Result;

use crate::commands::{agent, diff, routers, sync};

/// Handle session management commands
///
/// Delegates to the session router for command handling.
/// Routes: add, add-batch, list, remove, focus, status
pub async fn handle_session_cmd(cmd: &str, sub_m: &clap::ArgMatches) -> Result<()> {
    routers::session::handle_session_cmd(cmd, sub_m).await
}

/// Handle utility commands
///
/// Delegates to the utility router for command handling.
/// Routes: backup, restore, verify-backup, completions, query
pub async fn handle_utility_cmd(cmd: &str, sub_m: &clap::ArgMatches) -> Result<()> {
    routers::utility::handle_utility_cmd(cmd, sub_m).await
}

/// Handle sync command
///
/// Syncs a session's workspace with main branch using rebase.
/// This is separated from session commands as it has unique options.
pub async fn handle_sync_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    sync::run_with_options(
        sub_m.get_one::<String>("name").map(String::as_str),
        sync::SyncOptions {
            json: sub_m.get_flag("json"),
            dry_run: sub_m.get_flag("dry-run"),
        },
    )
    .await
}

/// Handle diff command
///
/// Shows diff between a session and the main branch.
/// This is separated from session commands as it has unique options.
pub async fn handle_diff_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    diff::run_with_options(
        name,
        diff::DiffOptions {
            stat: sub_m.get_flag("stat"),
            json: sub_m.get_flag("json"),
        },
    )
    .await
}

/// Handle agent command
///
/// Tracks and queries AI agents working in sessions.
pub async fn handle_agent_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", list_m)) => {
            let session = list_m.get_one::<String>("session").map(String::as_str);
            let json = list_m.get_flag("json");
            agent::run_list(session, json).await
        }
        _ => Err(anyhow::anyhow!(
            "Unknown agent subcommand. Use 'zjj agent list' to list agents."
        )),
    }
}

/// Handle introspection commands
///
/// Delegates to the introspection router for command handling.
/// Routes: context/ctx, introspect, dashboard/dash, doctor/check
pub async fn handle_introspection_cmd(cmd: &str, sub_m: &clap::ArgMatches) -> Result<()> {
    routers::introspection::handle_introspection_cmd(cmd, sub_m).await
}
