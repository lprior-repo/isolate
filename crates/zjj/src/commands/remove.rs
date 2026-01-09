//! Remove a session and its workspace

use std::fs;

use anyhow::{Context, Result};

use crate::{
    cli::{is_inside_zellij, run_command},
    commands::get_session_db,
};

/// Run the remove command
pub fn run(name: &str) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Remove JJ workspace (this removes the workspace from JJ's tracking)
    let workspace_result = run_command("jj", &["workspace", "forget", name]);
    if let Err(e) = workspace_result {
        tracing::warn!("Failed to forget JJ workspace: {e}");
    }

    // Remove the workspace directory
    if fs::metadata(&session.workspace_path).is_ok() {
        fs::remove_dir_all(&session.workspace_path)
            .context("Failed to remove workspace directory")?;
    }

    // Close Zellij tab if inside Zellij
    if is_inside_zellij() {
        // Try to close the tab - ignore errors if tab doesn't exist
        let _ = close_zellij_tab(&session.zellij_tab);
    }

    // Remove from database
    db.delete(name)?;

    println!("Removed session '{name}'");

    Ok(())
}

/// Close a Zellij tab by name
fn close_zellij_tab(tab_name: &str) -> Result<()> {
    // First, go to the tab
    run_command("zellij", &["action", "go-to-tab-name", tab_name])?;
    // Then close it
    run_command("zellij", &["action", "close-tab"])?;
    Ok(())
}
