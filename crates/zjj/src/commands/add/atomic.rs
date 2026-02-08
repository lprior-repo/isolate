use anyhow::{Context, Result};

use crate::{db::SessionDb, session::SessionUpdate};

/// Create session atomically to prevent partial state on SIGKILL
///
/// This implements atomicity by:
/// 1. Creating DB record with 'creating' status FIRST (detectable)
/// 2. Creating JJ workspace SECOND (interruptible by SIGKILL)
/// 3. On failure: cleaning workspace, leaving DB in 'creating' state for doctor
///
/// # Atomicity Guarantee
///
/// If SIGKILL occurs during step 2:
/// - DB record exists in 'creating' state (detectable by doctor)
/// - Partial workspace may exist (cleaned by `rollback_partial_state`)
/// - No partial state that prevents recovery
///
/// # Errors
///
/// Returns error on any failure, leaving DB in 'creating' state
/// and triggering cleanup of partial workspace state.
pub(super) async fn atomic_create_session(
    name: &str,
    workspace_path: &std::path::Path,
    db: &SessionDb,
    bead_metadata: Option<serde_json::Value>,
) -> Result<()> {
    let workspace_path_str = workspace_path.display().to_string();

    // STEP 1: Create DB record with 'creating' status FIRST
    // This makes the creation attempt detectable by doctor
    let db_result = db.create(name, &workspace_path_str).await;

    let _session = match db_result {
        Ok(s) => s,
        Err(db_error) => {
            // DB creation failed - no cleanup needed (nothing created yet)
            return Err(db_error).context("Failed to create session record");
        }
    };

    // Update session with bead metadata if provided
    if let Some(metadata) = bead_metadata {
        db.update(
            name,
            SessionUpdate {
                metadata: Some(metadata),
                ..Default::default()
            },
        )
        .await
        .context("Failed to update session metadata")?;
    }

    // STEP 2: Create JJ workspace (can be interrupted by SIGKILL)
    // If this fails or is interrupted, rollback will clean up
    let workspace_result = create_jj_workspace(name, workspace_path).await;

    match workspace_result {
        Ok(()) => {
            // Workspace created successfully
            // DB record already in 'creating' state, will transition to 'active'
            // after post_create hooks
            Ok(())
        }
        Err(workspace_error) => {
            // Workspace creation failed or was interrupted
            // Rollback: clean workspace, leave DB in 'creating' state
            rollback_partial_state(name, workspace_path).await;
            Err(workspace_error).context("Failed to create workspace, rolled back")
        }
    }
}

/// Rollback partial state after failed or interrupted session creation
///
/// This cleans up filesystem state while leaving the DB record
/// in 'creating' state for detection by doctor.
///
/// # Rollback Strategy
///
/// 1. Remove workspace directory if it exists (partial state cleanup)
/// 2. DO NOT remove DB record (leave for doctor detection)
/// 3. Handle missing paths gracefully (no panic on cleanup failure)
///
/// # Atomicity Contract
///
/// This function NEVER panics - all cleanup failures are logged
/// and handled gracefully. Partial state is always detectable.
///
/// # TOCTOU Prevention
///
/// Uses `remove_dir_all` directly without checking if path exists first.
/// If the directory doesn't exist, the error is safely ignored.
pub(super) async fn rollback_partial_state(name: &str, workspace_path: &std::path::Path) {
    let workspace_dir = workspace_path;

    // Remove workspace directory directly without checking existence first
    // This prevents TOCTOU - if it doesn't exist, we get an error we ignore
    match tokio::fs::remove_dir_all(workspace_dir).await {
        Ok(()) => {
            tracing::info!("Rolled back partial workspace for session '{name}'");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Workspace doesn't exist - nothing to clean
            tracing::info!(
                "No workspace to rollback for session '{}' (already gone)",
                name
            );
        }
        Err(cleanup_err) => {
            // Cleanup failed but we still leave DB in 'creating' state
            // Doctor will detect the stale session
            eprintln!(
                "Warning: failed to cleanup partial workspace '{}': {}",
                workspace_path.display(),
                cleanup_err
            );
            tracing::warn!(
                "Failed to rollback workspace for session '{}': {}",
                name,
                cleanup_err
            );
        }
    }

    // DB record intentionally left in 'creating' state for doctor detection
}

/// Create a JJ workspace for the session with operation graph synchronization
///
/// This uses the synchronized workspace creation to prevent operation graph
/// corruption when multiple workspaces are created concurrently.
async fn create_jj_workspace(name: &str, workspace_path: &std::path::Path) -> Result<()> {
    // Use the synchronized workspace creation to prevent operation graph corruption
    // This ensures:
    // 1. Workspace creations are serialized (prevents concurrent modification)
    // 2. All workspaces are based on the same repository operation
    // 3. Operation graph consistency is verified after creation
    zjj_core::jj_operation_sync::create_workspace_synced(name, workspace_path)
        .await
        .map_err(anyhow::Error::new)?;

    Ok(())
}
