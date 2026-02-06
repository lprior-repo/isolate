use anyhow::{Context, Result};
use zjj_core::jj;

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
pub(super) fn atomic_create_session(
    name: &str,
    workspace_path: &std::path::Path,
    db: &SessionDb,
    bead_metadata: Option<serde_json::Value>,
) -> Result<()> {
    let workspace_path_str = workspace_path.display().to_string();

    // STEP 1: Create DB record with 'creating' status FIRST
    // This makes the creation attempt detectable by doctor
    let db_result = db.create_blocking(name, &workspace_path_str);

    let _session = match db_result {
        Ok(s) => s,
        Err(db_error) => {
            // DB creation failed - no cleanup needed (nothing created yet)
            return Err(db_error).context("Failed to create session record");
        }
    };

    // Update session with bead metadata if provided
    if let Some(metadata) = bead_metadata {
        db.update_blocking(
            name,
            SessionUpdate {
                metadata: Some(metadata),
                ..Default::default()
            },
        )
        .context("Failed to update session metadata")?;
    }

    // STEP 2: Create JJ workspace (can be interrupted by SIGKILL)
    // If this fails or is interrupted, rollback will clean up
    let workspace_result = create_jj_workspace(name, workspace_path);

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
pub(super) async fn rollback_partial_state(name: &str, workspace_path: &std::path::Path) {
    let workspace_dir = workspace_path;

    // Only attempt cleanup if path exists (handle missing gracefully)
    if workspace_dir.exists() {
        match tokio::fs::remove_dir_all(workspace_dir).await {
            Ok(()) => {
                tracing::info!("Rolled back partial workspace for session '{}'", name);
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
    } else {
        // Workspace doesn't exist - nothing to clean
        tracing::info!("No workspace to rollback for session '{}'", name);
    }

    // DB record intentionally left in 'creating' state for doctor detection
}

/// Create a JJ workspace for the session
fn create_jj_workspace(name: &str, workspace_path: &std::path::Path) -> Result<()> {
    // Use the JJ workspace manager from core
    // Preserve the zjj_core::Error to maintain exit code semantics
    jj::workspace_create(name, workspace_path).map_err(anyhow::Error::new)?;

    Ok(())
}
