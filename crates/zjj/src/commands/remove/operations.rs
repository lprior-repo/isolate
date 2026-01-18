//! Core removal operations for cleanup

use std::{fs, path::Path};

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
        Err(e) => {
            let path = &session.workspace_path;
            Err(e).with_context(|| {
                format!(
                    "Failed to remove workspace directory: {path}\n\
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
                     • Check directory permissions: ls -la {path}\n\
                     • Manually remove with: rm -rf {path}"
                )
            })
        }
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

/// Remove session layout file from the layouts directory
///
/// Layout files are stored at `{workspace_dir}/layouts/{session_name}.kdl`.
/// This function computes the path from the session's `workspace_path` and removes the file.
///
/// # Railway Pattern
/// - Returns `Ok(Some(RemoveOperation))` if file was removed
/// - Returns `Ok(None)` if file didn't exist (not an error)
/// - Returns `Err` only on actual filesystem errors (permissions, etc.)
pub fn remove_layout_file(session: &Session) -> Result<Option<RemoveOperation>> {
    // Derive layout path from workspace path: {workspace_dir}/{session_name} -> {workspace_dir}/layouts/{session_name}.kdl
    let workspace_path = Path::new(&session.workspace_path);
    let layout_path = workspace_path
        .parent()
        .map(|parent| parent.join("layouts").join(format!("{}.kdl", session.name)));

    // Use Option combinators to handle the path derivation
    let Some(layout_file) = layout_path else {
        // Couldn't derive parent directory - no layout to remove
        return Ok(None);
    };

    // Remove layout file if it exists
    match fs::remove_file(&layout_file) {
        Ok(()) => Ok(Some(RemoveOperation {
            action: "removed_layout_file".to_string(),
            path: Some(layout_file.display().to_string()),
            id: None,
            tab: None,
        })),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File already gone - that's fine, not an error
            Ok(None)
        }
        Err(e) => {
            let display = layout_file.display();
            Err(e).with_context(|| {
                format!(
                    "Failed to remove layout file: {display}\n\
                     This is non-critical - the session was removed but the layout file remains."
                )
            })
        }
    }
}
