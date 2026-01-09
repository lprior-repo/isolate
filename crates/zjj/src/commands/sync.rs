//! Sync a session's workspace with main branch

use anyhow::{Context, Result};

use crate::{
    cli::{jj_root, run_command},
    commands::get_session_db,
};

/// Run the sync command
///
/// If a session name is provided, syncs that session's workspace.
/// Otherwise, syncs the current workspace (if in a JJ repo).
pub fn run(name: Option<&str>) -> Result<()> {
    name.map_or_else(sync_current, sync_session)
}

/// Sync a specific session's workspace
fn sync_session(name: &str) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Run rebase in the session's workspace
    // jj --repository <path> rebase -d main
    run_command(
        "jj",
        &[
            "--repository",
            &session.workspace_path,
            "rebase",
            "-d",
            "main",
        ],
    )
    .context("Failed to sync workspace with main")?;

    println!("Synced session '{name}' with main");
    Ok(())
}

/// Sync the current workspace
fn sync_current() -> Result<()> {
    // Verify we're in a JJ repository
    let root = jj_root()?;

    // Run rebase on current workspace
    run_command("jj", &["rebase", "-d", "main"]).context("Failed to sync workspace with main")?;

    println!("Synced current workspace at {root}");
    Ok(())
}
