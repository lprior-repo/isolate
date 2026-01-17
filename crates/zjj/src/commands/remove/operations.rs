//! Core removal operations for cleanup

use std::fs;

use anyhow::{Context, Result};

use crate::{
    cli::{is_inside_zellij, run_command},
    json_output::RemoveOperation,
    session::Session,
};

/// Remove workspace directory with proper error handling
///
/// This function attempts to remove the workspace directory.
/// If the directory is already gone, that's considered success.
/// If there's a real error (permissions, locked files, etc.), it fails with context.
pub fn remove_workspace_directory(session: &Session) -> Result<RemoveOperation> {
    match fs::remove_dir_all(&session.workspace_path) {
        Ok(()) => Ok(RemoveOperation {
            action: "removed_workspace".to_string(),
            path: Some(session.workspace_path.clone()),
            id: None,
            tab: None,
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Directory already gone - that's fine
            Ok(RemoveOperation {
                action: "workspace_already_gone".to_string(),
                path: Some(session.workspace_path.clone()),
                id: None,
                tab: None,
            })
        }
        Err(e) => Err(e).with_context(|| {
            format!(
                "Failed to remove workspace directory: {}\n\
                 \n\
                 Possible causes:\n\
                 • Directory is in use by another process\n\
                 • Insufficient permissions\n\
                 • Files are locked\n\
                 \n\
                 The database entry and JJ workspace have NOT been removed.\n\
                 You can retry after fixing the issue.\n\
                 \n\
                 Try:\n\
                 • Close all programs using files in this directory\n\
                 • Check directory permissions: ls -la {}\n\
                 • Manually remove with: rm -rf {}",
                session.workspace_path, session.workspace_path, session.workspace_path
            )
        }),
    }
}

/// Forget JJ workspace registration
pub fn forget_jj_workspace(name: &str) -> Result<RemoveOperation> {
    run_command("jj", &["workspace", "forget", name])
        .context("Failed to forget JJ workspace")
        .map(|_| RemoveOperation {
            action: "forgot_jj_workspace".to_string(),
            path: None,
            id: None,
            tab: None,
        })
}

/// Delete session from database
pub async fn delete_database_entry(
    db: &crate::database::SessionDb,
    name: &str,
    session_id: Option<i64>,
) -> Result<RemoveOperation> {
    db.delete(name)
        .await
        .map(|_| RemoveOperation {
            action: "deleted_db_entry".to_string(),
            path: None,
            id: session_id,
            tab: None,
        })
        .context("Failed to delete session from database")
}

/// Close Zellij tab if inside Zellij
pub fn close_zellij_tab_if_present(tab_name: &str) -> Option<RemoveOperation> {
    if !is_inside_zellij() {
        return None;
    }

    close_zellij_tab(tab_name).ok().map(|()| RemoveOperation {
        action: "closed_zellij_tab".to_string(),
        path: None,
        id: None,
        tab: Some(tab_name.to_string()),
    })
}

/// Close a Zellij tab by name
fn close_zellij_tab(tab_name: &str) -> Result<()> {
    // First, go to the tab
    run_command("zellij", &["action", "go-to-tab-name", tab_name])?;
    // Then close it
    run_command("zellij", &["action", "close-tab"])?;
    Ok(())
}
