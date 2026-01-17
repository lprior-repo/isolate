//! Utility commands router
//!
//! Routes utility commands (backup, restore, verify-backup, completions, query)
//! These commands don't directly manage sessions but provide supporting functionality
//! like backups, shell completions, and system queries.

use anyhow::Result;

use crate::commands::{backup, completions, query};

/// Handle utility commands
///
/// Routes utility commands to their appropriate handlers.
/// Utility commands provide supporting functionality for session management.
///
/// # Errors
///
/// Returns an error if the command execution fails
pub async fn handle_utility_cmd(cmd: &str, sub_m: &clap::ArgMatches) -> Result<()> {
    match cmd {
        "backup" => handle_backup_cmd(sub_m).await,
        "restore" => handle_restore_cmd(sub_m).await,
        "verify-backup" => handle_verify_backup_cmd(sub_m).await,
        "completions" => handle_completions_cmd(sub_m).await,
        "query" => handle_query_cmd(sub_m).await,
        _ => Err(anyhow::anyhow!("Unknown utility command: {cmd}")),
    }
}

/// Handle the 'backup' command
async fn handle_backup_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    backup::run_backup(
        sub_m.get_one::<String>("path").map(String::as_str),
        sub_m.get_flag("json"),
    )
    .await
}

/// Handle the 'restore' command
async fn handle_restore_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let path = sub_m
        .get_one::<String>("path")
        .ok_or_else(|| anyhow::anyhow!("Backup path is required"))?;
    backup::run_restore(path, sub_m.get_flag("force"), sub_m.get_flag("json")).await
}

/// Handle the 'verify-backup' command
async fn handle_verify_backup_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let path = sub_m
        .get_one::<String>("path")
        .ok_or_else(|| anyhow::anyhow!("Backup path is required"))?;
    backup::run_verify_backup(path, sub_m.get_flag("json")).await
}

/// Handle the 'completions' command
async fn handle_completions_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let shell = sub_m
        .get_one::<String>("shell")
        .ok_or_else(|| anyhow::anyhow!("Shell is required"))?;
    completions::run(shell, sub_m.get_flag("instructions")).await
}

/// Handle the 'query' command
async fn handle_query_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let query_type = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
    query::run(
        query_type,
        sub_m.get_one::<String>("args").map(String::as_str),
    )
    .await
}
