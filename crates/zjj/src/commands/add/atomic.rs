use anyhow::{Context, Result};

use crate::{db::SessionDb, session::SessionUpdate};

/// Create session atomically to prevent partial state between DB and workspace.
///
/// This implements atomicity by:
/// 1. Creating DB record with 'creating' status FIRST
/// 2. Creating JJ workspace SECOND (interruptible by SIGKILL)
/// 3. On failure: cleaning workspace and deleting DB record
///
/// # Atomicity Guarantee
///
/// If SIGKILL occurs during step 2:
/// - DB record exists in 'creating' state until cleanup completes
/// - Partial workspace may exist (cleaned by `rollback_partial_state`)
/// - No durable partial state remains after cleanup path
///
/// # Errors
///
/// Returns error on any failure and triggers cleanup of partial state.
pub(super) async fn atomic_create_session(
    name: &str,
    workspace_path: &std::path::Path,
    db: &SessionDb,
    bead_metadata: Option<serde_json::Value>,
    create_command_id: Option<&str>,
) -> Result<()> {
    let workspace_path_str = workspace_path.display().to_string();

    // STEP 1: Create DB record with 'creating' status FIRST
    let db_result = match create_command_id {
        Some(command_id) => {
            db.create_with_command_id(name, &workspace_path_str, Some(command_id))
                .await
        }
        None => db.create(name, &workspace_path_str).await,
    };

    let _session = match db_result {
        Ok(s) => s,
        Err(db_error) => {
            // DB creation failed - no cleanup needed (nothing created yet)
            return Err(db_error).context("Failed to create session record");
        }
    };

    // Update session with bead metadata if provided
    if let Some(metadata) = bead_metadata {
        let metadata_result = db
            .update(
                name,
                SessionUpdate {
                    metadata: Some(metadata),
                    ..Default::default()
                },
            )
            .await;

        if let Err(metadata_error) = metadata_result {
            rollback_database_state(name, db, create_command_id).await?;
            return Err(metadata_error).context("Failed to update session metadata");
        }
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
            // Workspace creation failed or was interrupted. Roll back both sides.
            rollback_partial_state(name, workspace_path).await?;
            rollback_database_state(name, db, create_command_id).await?;
            Err(workspace_error).context("Failed to create workspace, rolled back")
        }
    }
}

async fn rollback_database_state(
    name: &str,
    db: &SessionDb,
    create_command_id: Option<&str>,
) -> Result<()> {
    db.delete(name)
        .await
        .map(|_| ())
        .context("Failed to remove partial session record")?;

    if let Some(command_id) = create_command_id {
        db.unmark_command_processed(command_id)
            .await
            .context("Failed to clear command idempotency marker")?;
    }

    Ok(())
}

/// Rollback partial state after failed or interrupted session creation
///
/// This cleans up filesystem state for failed session creation.
///
/// # Rollback Strategy
///
/// 1. Remove workspace directory if it exists (partial state cleanup)
/// 2. Database rollback happens separately in `rollback_database_state`
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
pub(super) async fn rollback_partial_state(
    name: &str,
    workspace_path: &std::path::Path,
) -> Result<()> {
    let workspace_dir = workspace_path;

    // Remove workspace directory directly without checking existence first
    // This prevents TOCTOU - if it doesn't exist, we get an error we ignore
    match tokio::fs::remove_dir_all(workspace_dir).await {
        Ok(()) => {
            tracing::info!("Rolled back partial workspace for session '{name}'");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Workspace doesn't exist - nothing to clean
            tracing::info!(
                "No workspace to rollback for session '{}' (already gone)",
                name
            );
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotADirectory => {
            match tokio::fs::remove_file(workspace_dir).await {
                Ok(()) => {
                    tracing::info!("Rolled back partial workspace file for session '{}'", name);
                    Ok(())
                }
                Err(file_err) if file_err.kind() == std::io::ErrorKind::NotFound => {
                    tracing::info!(
                        "No workspace file to rollback for session '{}' (already gone)",
                        name
                    );
                    Ok(())
                }
                Err(cleanup_err) => Err(anyhow::anyhow!(
                    "Failed to remove partial workspace file '{}': {}",
                    workspace_path.display(),
                    cleanup_err
                )),
            }
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
            Err(anyhow::anyhow!(
                "Failed to rollback partial workspace '{}': {}",
                workspace_path.display(),
                cleanup_err
            ))
        }
    }
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
