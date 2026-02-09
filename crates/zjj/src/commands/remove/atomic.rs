//! Atomic session removal with zero unwraps/panics
//!
//! This module implements atomic cleanup of sessions and workspaces to prevent
//! orphaned resources. All operations use Railway-Oriented Programming with
//! proper error handling and recovery.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::Path;

use anyhow::Result;
use thiserror::Error;

use crate::{
    cli::{is_inside_zellij, run_command},
    db::SessionDb,
    session::Session,
};

/// Errors that can occur during session removal
#[derive(Debug, Error)]
pub enum RemoveError {
    /// Workspace path invalid or inaccessible
    #[error("Workspace inaccessible: {path} - {reason}")]
    WorkspaceInaccessible { path: String, reason: String },

    /// Workspace directory removal failed (session preserved)
    #[error("Failed to remove workspace at {path}: {source}")]
    WorkspaceRemovalFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Database deletion failed (workspace already deleted)
    #[error("Failed to delete session '{name}' from database: {source}")]
    DatabaseDeletionFailed {
        name: String,
        #[source]
        source: zjj_core::Error,
    },

    /// Zellij tab closure failed (non-critical)
    #[error("Failed to close Zellij tab '{tab}': {source}")]
    ZellijTabCloseFailed {
        tab: String,
        #[source]
        source: anyhow::Error,
    },
}

/// Result of successful removal operation
#[derive(Debug, Clone)]
pub struct RemoveResult {
    /// Whether removal was successful
    pub removed: bool,
}

/// Remove a session and its workspace atomically
///
/// This function implements atomic cleanup to prevent orphaned resources.
/// It follows a phased approach with proper error handling:
///
/// # Phases
/// 1. **Validation**: Verify session exists and workspace is accessible
/// 2. **Zellij Cleanup**: Close Zellij tab (non-critical, warnings only)
/// 3. **JJ Forget**: Remove from JJ tracking (non-critical, warnings only)
/// 4. **Workspace Removal**: Delete workspace directory (critical, marks failed on error)
/// 5. **Database Deletion**: Remove session record (critical, but workspace already deleted)
///
/// # Error Handling
/// - Zellij/JJ failures: Log warning, continue (local cleanup more important)
/// - Workspace removal failures: Mark session as "removal_failed" in database
/// - Database deletion failures: Log critical error (workspace deleted, manual cleanup needed)
///
/// # Errors
///
/// Returns `RemoveError` if:
/// - Session not found
/// - Workspace inaccessible
/// - Workspace removal fails (marks failed in DB first)
/// - Database deletion fails (after workspace deleted)
pub async fn cleanup_session_atomically(
    db: &SessionDb,
    session: &Session,
    jj_forget: bool,
) -> Result<RemoveResult, RemoveError> {
    // Phase 1: Validate workspace path exists
    let workspace_path = Path::new(&session.workspace_path);
    if !workspace_path.exists() {
        return Err(RemoveError::WorkspaceInaccessible {
            path: session.workspace_path.clone(),
            reason: "Workspace directory does not exist".to_string(),
        });
    }

    // Phase 2: Close Zellij tab (non-critical)
    // Skip tab operations when not running inside a Zellij session to avoid
    // blocking on `zellij action` calls in non-interactive contexts (tests/CI).
    if is_inside_zellij() {
        if let Err(e) = close_zellij_tab(&session.zellij_tab).await {
            tracing::warn!("Failed to close Zellij tab '{}': {}", session.zellij_tab, e);
            // Continue - tab closure is UI cleanup, not data integrity
        }
    }

    // Phase 3: Forget from JJ (non-critical)
    if jj_forget {
        if let Err(e) = run_command("jj", &["workspace", "forget", &session.name]).await {
            tracing::warn!(
                "JJ workspace forget failed for '{}': {}. Workspace will be deleted locally but JJ may still track it.",
                session.name,
                e
            );
            // Continue - local cleanup more important than JJ state
        }
    }

    // Phase 4: Remove workspace directory (critical)
    if let Err(e) = tokio::fs::remove_dir_all(workspace_path).await {
        let error_msg = format!("Failed to remove workspace: {e}");
        let _ = db.mark_removal_failed(&session.name, &error_msg).await;

        return Err(RemoveError::WorkspaceRemovalFailed {
            path: session.workspace_path.clone(),
            source: e,
        });
    }

    // Phase 5: Delete from database (critical, but workspace already deleted)
    db.delete(&session.name)
        .await
        .map_err(|e| RemoveError::DatabaseDeletionFailed {
            name: session.name.clone(),
            source: e,
        })?;

    Ok(RemoveResult { removed: true })
}

/// Close a Zellij tab by name
///
/// This is a non-critical operation - failures are logged but don't prevent removal.
async fn close_zellij_tab(tab_name: &str) -> Result<(), RemoveError> {
    // First, go to the tab
    run_command("zellij", &["action", "go-to-tab-name", tab_name])
        .await
        .map_err(|e| RemoveError::ZellijTabCloseFailed {
            tab: tab_name.to_string(),
            source: e,
        })?;

    // Then close it
    run_command("zellij", &["action", "close-tab"])
        .await
        .map_err(|e| RemoveError::ZellijTabCloseFailed {
            tab: tab_name.to_string(),
            source: e,
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use tokio::fs;

    use super::*;

    #[tokio::test]
    async fn test_cleanup_session_deletes_workspace_and_record() -> Result<()> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;

        // Create workspace directory
        let workspace = dir.path().join("workspaces").join("test-session");
        fs::create_dir_all(&workspace).await?;

        // Create session
        db.create(
            "test-session",
            workspace
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?,
        )
        .await?;

        // Get session
        let session = db
            .get("test-session")
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Run cleanup
        let result = cleanup_session_atomically(&db, &session, false).await?;

        // Verify workspace deleted
        assert!(!workspace.exists(), "Workspace should be deleted");

        // Verify session deleted
        let session_opt = db.get("test-session").await?;
        assert!(session_opt.is_none(), "Session should be deleted");

        assert!(result.removed);
        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_when_workspace_inaccessible_returns_error() -> Result<()> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;

        // Create session with non-existent workspace path
        db.create("test-session", "/nonexistent/path").await?;

        // Get session
        let session = db
            .get("test-session")
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Run cleanup - should fail with WorkspaceInaccessible
        let result = cleanup_session_atomically(&db, &session, false).await;

        assert!(result.is_err());
        match result {
            Err(RemoveError::WorkspaceInaccessible { .. }) => Ok(()),
            _ => Err(anyhow::anyhow!("Expected WorkspaceInaccessible error")),
        }
    }

    #[tokio::test]
    async fn test_find_orphaned_sessions_detects_missing_workspaces() -> Result<()> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;

        // Create session and workspace
        let workspace = dir.path().join("workspaces").join("orphan-test");
        fs::create_dir_all(&workspace).await?;
        db.create(
            "orphan-test",
            workspace
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?,
        )
        .await?;

        // Delete workspace manually (simulate external deletion)
        fs::remove_dir_all(&workspace).await?;

        // Find orphans
        let orphans = db.find_orphaned_sessions().await?;

        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0], "orphan-test");

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_orphaned_sessions_removes_type1_orphans() -> Result<()> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;

        // Create session and workspace
        let workspace = dir.path().join("workspaces").join("cleanup-test");
        fs::create_dir_all(&workspace).await?;
        db.create(
            "cleanup-test",
            workspace
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?,
        )
        .await?;

        // Delete workspace manually
        fs::remove_dir_all(&workspace).await?;

        // Cleanup orphans
        let cleaned_count = db.cleanup_orphaned_sessions().await?;

        assert_eq!(cleaned_count, 1);

        // Verify session deleted
        let session_opt = db.get("cleanup-test").await?;
        assert!(session_opt.is_none());

        Ok(())
    }
}
