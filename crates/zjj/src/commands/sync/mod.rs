//! Sync command modules
//!
//! This module orchestrates syncing session workspaces with the main branch.
//! It delegates specific concerns to focused submodules.

mod branch_detection;
mod dry_run;
mod formatters;
mod operations;
mod rebase;
mod repo_diagnostics;
mod validation;

use anyhow::Result;
pub use dry_run::{output_all_sessions_dry_run, output_single_session_dry_run};
pub use validation::validate_session_status;

use crate::commands::get_session_db;

/// Options for the sync command
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOptions {
    /// Output as JSON
    pub json: bool,
    /// Show what would be done without executing
    pub dry_run: bool,
    /// Minimal output for pipes
    pub silent: bool,
}

/// Run the sync command with options
///
/// If a session name is provided, syncs that session's workspace.
/// Otherwise, syncs all sessions.
pub async fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<()> {
    match name {
        Some(n) => sync_session_with_options(n, options).await,
        None => sync_all_with_options(options).await,
    }
}

/// Sync a specific session's workspace
async fn sync_session_with_options(name: &str, options: SyncOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Get the session
    let session = db
        .get(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Validate session status is appropriate for sync
    validate_session_status(&session.status, name)?;

    // DRY-RUN: Show what would be done without executing
    if options.dry_run {
        return output_single_session_dry_run(name, &session, options);
    }

    // Use internal sync function
    match operations::sync_session_internal(&db, &session.name, &session.workspace_path).await {
        Ok(stats) => {
            formatters::output_sync_success(name, &stats, options)?;
            Ok(())
        }
        Err(e) => {
            formatters::output_sync_failure(name, e, options)?;
            std::process::exit(2); // Exit code 2: System error (sync/rebase failure)
        }
    }
}

/// Sync all sessions
async fn sync_all_with_options(options: SyncOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Get all sessions
    let sessions = db
        .list(None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list sessions: {e}"))?;

    // Handle empty session list
    if sessions.is_empty() {
        formatters::output_no_sessions(options)?;
        return Ok(());
    }

    // DRY-RUN: Show what would be done for all sessions
    if options.dry_run {
        return output_all_sessions_dry_run(&sessions, options);
    }

    // Execute sync for all sessions
    let results = operations::sync_all_sessions(&db, &sessions).await;

    // Output results based on format
    formatters::output_all_sessions_results(&results, options);
    Ok(())
}
