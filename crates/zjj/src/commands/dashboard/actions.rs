//! Session actions
//!
//! Handles session-level operations like focusing,
//! adding, and removing sessions.

use anyhow::{Context, Result};

use crate::session::Session;

/// Focus a session by switching to its Zellij tab
///
/// # Errors
/// Returns error if the zellij command fails or the tab doesn't exist
pub async fn focus_session(session: &Session) -> Result<()> {
    let tab_name = session.zellij_tab.clone();

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("zellij")
            .args(["action", "go-to-tab-name", &tab_name])
            .output()
    })
    .await
    .context("Failed to join zellij command task")?
    .context("Failed to execute zellij command")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to focus session: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Add a new session
///
/// # Errors
/// Returns error if the add command fails
pub async fn add_session(name: &str) -> Result<()> {
    crate::commands::add::run(name).await
}

/// Remove a session
///
/// # Errors
/// Returns error if the remove command fails
pub async fn remove_session(name: &str) -> Result<()> {
    crate::commands::remove::run(name).await
}
