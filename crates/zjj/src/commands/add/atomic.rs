use anyhow::{Context, Result};
use futures::StreamExt;

use crate::{
    db::{AddOperationRecord, SessionDb},
    session::SessionUpdate,
};

const JOURNAL_PENDING_EXTERNAL: &str = "pending_external";
const JOURNAL_COMPENSATING: &str = "compensating";
const JOURNAL_DONE: &str = "done";
const JOURNAL_FAILED_COMPENSATION: &str = "failed_compensation";

fn add_operation_id(name: &str, command_id: Option<&str>) -> String {
    command_id.map_or_else(|| format!("add:{name}"), |id| format!("add:{name}:{id}"))
}

#[derive(Clone, Copy)]
enum AddAtomicState {
    Start,
    DbRecordCreated,
    MetadataUpdated,
    WorkspaceCreateStarted,
    WorkspaceCreateFailed,
    WorkspaceCreateSucceeded,
    WorkspaceRollbackStarted,
    WorkspaceRollbackFailed,
    WorkspaceRollbackSucceeded,
    DatabaseRollbackStarted,
    DatabaseRollbackFailed,
    DatabaseRollbackSucceeded,
}

impl AddAtomicState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::DbRecordCreated => "db_record_created",
            Self::MetadataUpdated => "metadata_updated",
            Self::WorkspaceCreateStarted => "workspace_create_started",
            Self::WorkspaceCreateFailed => "workspace_create_failed",
            Self::WorkspaceCreateSucceeded => "workspace_create_succeeded",
            Self::WorkspaceRollbackStarted => "workspace_rollback_started",
            Self::WorkspaceRollbackFailed => "workspace_rollback_failed",
            Self::WorkspaceRollbackSucceeded => "workspace_rollback_succeeded",
            Self::DatabaseRollbackStarted => "database_rollback_started",
            Self::DatabaseRollbackFailed => "database_rollback_failed",
            Self::DatabaseRollbackSucceeded => "database_rollback_succeeded",
        }
    }
}

fn log_add_state(name: &str, state: AddAtomicState, recoverable: bool, detail: &str) {
    if recoverable {
        tracing::warn!(
            session = %name,
            state = state.as_str(),
            recoverable,
            detail,
            "add.atomic"
        );
    } else {
        tracing::info!(
            session = %name,
            state = state.as_str(),
            recoverable,
            detail,
            "add.atomic"
        );
    }
}

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
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub(super) async fn atomic_create_session(
    name: &str,
    workspace_path: &std::path::Path,
    repo_root: &std::path::Path,
    db: &SessionDb,
    bead_metadata: Option<serde_json::Value>,
    create_command_id: Option<&str>,
) -> Result<()> {
    let workspace_path_str = workspace_path.display().to_string();
    let operation_id = add_operation_id(name, create_command_id);
    log_add_state(
        name,
        AddAtomicState::Start,
        false,
        "starting atomic add sequence",
    );

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
            log_add_state(
                name,
                AddAtomicState::DbRecordCreated,
                true,
                "db create failed before workspace creation",
            );
            return Err(db_error).context("Failed to create session record");
        }
    };
    log_add_state(
        name,
        AddAtomicState::DbRecordCreated,
        false,
        "db record created in 'creating' state",
    );

    db.upsert_add_operation_journal(
        &operation_id,
        name,
        &workspace_path_str,
        create_command_id,
        JOURNAL_PENDING_EXTERNAL,
        None,
    )
    .await?;

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
            log_add_state(
                name,
                AddAtomicState::MetadataUpdated,
                true,
                "metadata update failed, starting database rollback",
            );
            db.upsert_add_operation_journal(
                &operation_id,
                name,
                &workspace_path_str,
                create_command_id,
                JOURNAL_COMPENSATING,
                Some(&metadata_error.to_string()),
            )
            .await?;
            rollback_database_state(name, db, create_command_id).await?;
            db.upsert_add_operation_journal(
                &operation_id,
                name,
                &workspace_path_str,
                create_command_id,
                JOURNAL_DONE,
                None,
            )
            .await?;
            return Err(metadata_error).context("Failed to update session metadata");
        }

        log_add_state(
            name,
            AddAtomicState::MetadataUpdated,
            false,
            "session metadata updated",
        );
    }

    // STEP 2: Create JJ workspace (can be interrupted by SIGKILL)
    // If this fails or is interrupted, rollback will clean up
    log_add_state(
        name,
        AddAtomicState::WorkspaceCreateStarted,
        false,
        "creating jj workspace",
    );
    let workspace_result = match create_jj_workspace(name, workspace_path, repo_root).await {
        Ok(()) => {
            let exists = tokio::fs::try_exists(workspace_path)
                .await
                .map_err(|error| {
                    anyhow::anyhow!(
                        "workspace creation reported success but path check failed for '{}': {error}",
                        workspace_path.display()
                    )
                });

            match exists {
                Ok(true) => Ok(()),
                Ok(false) => Err(anyhow::anyhow!(
                    "workspace creation reported success but directory '{}' is missing",
                    workspace_path.display()
                )),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    };

    match workspace_result {
        Ok(()) => {
            log_add_state(
                name,
                AddAtomicState::WorkspaceCreateSucceeded,
                false,
                "workspace created, ready for hooks",
            );
            db.upsert_add_operation_journal(
                &operation_id,
                name,
                &workspace_path_str,
                create_command_id,
                JOURNAL_DONE,
                None,
            )
            .await?;
            // Workspace created successfully
            // DB record already in 'creating' state, will transition to 'active'
            // after post_create hooks
            Ok(())
        }
        Err(workspace_error) => {
            log_add_state(
                name,
                AddAtomicState::WorkspaceCreateFailed,
                true,
                "workspace creation failed, starting rollback",
            );
            db.upsert_add_operation_journal(
                &operation_id,
                name,
                &workspace_path_str,
                create_command_id,
                JOURNAL_COMPENSATING,
                Some(&workspace_error.to_string()),
            )
            .await?;
            // Workspace creation failed or was interrupted. Roll back both sides.
            let workspace_cleanup = rollback_partial_state(name, workspace_path).await;
            let database_cleanup = rollback_database_state(name, db, create_command_id).await;

            match (workspace_cleanup, database_cleanup) {
                (Ok(()), Ok(())) => {
                    db.upsert_add_operation_journal(
                        &operation_id,
                        name,
                        &workspace_path_str,
                        create_command_id,
                        JOURNAL_DONE,
                        None,
                    )
                    .await?;
                    log_add_state(
                        name,
                        AddAtomicState::WorkspaceRollbackSucceeded,
                        false,
                        "workspace and database rollback completed",
                    );
                    Err(workspace_error).context(format!(
                        "Failed to create workspace, rolled back. Recovery: run 'zjj remove {name} --force' if stale artifacts remain"
                    ))
                }
                (Err(workspace_rollback_error), Ok(())) => {
                    db.upsert_add_operation_journal(
                        &operation_id,
                        name,
                        &workspace_path_str,
                        create_command_id,
                        JOURNAL_FAILED_COMPENSATION,
                        Some(&workspace_rollback_error.to_string()),
                    )
                    .await?;
                    log_add_state(
                        name,
                        AddAtomicState::WorkspaceRollbackFailed,
                        true,
                        "workspace rollback failed; manual cleanup may be required",
                    );
                    Err(workspace_error).context(format!(
                        "Failed to create workspace and workspace rollback failed: {workspace_rollback_error}. Recovery: run 'zjj remove {name} --force'"
                    ))
                }
                (Ok(()), Err(database_rollback_error)) => {
                    db.upsert_add_operation_journal(
                        &operation_id,
                        name,
                        &workspace_path_str,
                        create_command_id,
                        JOURNAL_FAILED_COMPENSATION,
                        Some(&database_rollback_error.to_string()),
                    )
                    .await?;
                    log_add_state(
                        name,
                        AddAtomicState::DatabaseRollbackFailed,
                        true,
                        "database rollback failed; command marker may need cleanup",
                    );
                    Err(workspace_error).context(format!(
                        "Failed to create workspace and database rollback failed: {database_rollback_error}. Recovery: run 'zjj remove {name} --force'"
                    ))
                }
                (Err(workspace_rollback_error), Err(database_rollback_error)) => {
                    db.upsert_add_operation_journal(
                        &operation_id,
                        name,
                        &workspace_path_str,
                        create_command_id,
                        JOURNAL_FAILED_COMPENSATION,
                        Some(&format!(
                            "workspace rollback: {workspace_rollback_error}; database rollback: {database_rollback_error}"
                        )),
                    )
                    .await?;
                    log_add_state(
                        name,
                        AddAtomicState::WorkspaceRollbackFailed,
                        true,
                        "workspace and database rollback failed; manual recovery required",
                    );
                    Err(workspace_error).context(format!(
                        "Failed to create workspace, workspace rollback failed: {workspace_rollback_error}, database rollback failed: {database_rollback_error}. Recovery: run 'zjj remove {name} --force'"
                    ))
                }
            }
        }
    }
}

pub(super) async fn replay_add_operation_journal(db: &SessionDb) -> Result<usize> {
    let operations = db.list_incomplete_add_operations().await?;

    let recovered = futures::stream::iter(operations)
        .fold(0usize, |recovered, operation| async move {
            match replay_single_add_operation(db, operation).await {
                Ok(true) => recovered + 1,
                Ok(false) => recovered,
                Err(error) => {
                    tracing::warn!("Failed to replay add operation journal entry: {error}");
                    recovered
                }
            }
        })
        .await;

    Ok(recovered)
}

async fn replay_single_add_operation(
    db: &SessionDb,
    operation: AddOperationRecord,
) -> Result<bool> {
    if operation.state == JOURNAL_DONE {
        return Ok(false);
    }

    let workspace_path = std::path::Path::new(&operation.workspace_path);
    let command_id = operation.command_id.as_deref();

    db.upsert_add_operation_journal(
        &operation.operation_id,
        &operation.session_name,
        &operation.workspace_path,
        command_id,
        JOURNAL_COMPENSATING,
        operation.last_error.as_deref(),
    )
    .await?;

    let workspace_cleanup = rollback_partial_state(&operation.session_name, workspace_path).await;
    let database_cleanup = rollback_database_state(&operation.session_name, db, command_id).await;

    match (workspace_cleanup, database_cleanup) {
        (Ok(()), Ok(())) => {
            db.upsert_add_operation_journal(
                &operation.operation_id,
                &operation.session_name,
                &operation.workspace_path,
                command_id,
                JOURNAL_DONE,
                None,
            )
            .await?;
            Ok(true)
        }
        (workspace_result, database_result) => {
            let error_message = format!(
                "workspace={:?}; database={:?}",
                workspace_result.err(),
                database_result.err()
            );

            db.upsert_add_operation_journal(
                &operation.operation_id,
                &operation.session_name,
                &operation.workspace_path,
                command_id,
                JOURNAL_FAILED_COMPENSATION,
                Some(&error_message),
            )
            .await?;

            Err(anyhow::anyhow!(
                "failed to reconcile add operation {}: {}",
                operation.operation_id,
                error_message
            ))
        }
    }
}

async fn rollback_database_state(
    name: &str,
    db: &SessionDb,
    create_command_id: Option<&str>,
) -> Result<()> {
    log_add_state(
        name,
        AddAtomicState::DatabaseRollbackStarted,
        false,
        "removing partial session record",
    );

    match db.delete(name).await {
        Ok(true) => {
            log_add_state(
                name,
                AddAtomicState::DatabaseRollbackSucceeded,
                false,
                "partial session record removed",
            );
        }
        Ok(false) => {
            log_add_state(
                name,
                AddAtomicState::DatabaseRollbackSucceeded,
                false,
                "partial session record already absent",
            );
        }
        Err(db_error) => {
            log_add_state(
                name,
                AddAtomicState::DatabaseRollbackFailed,
                true,
                "failed to remove partial session record",
            );
            return Err(db_error).context("Failed to remove partial session record");
        }
    }

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
    log_add_state(
        name,
        AddAtomicState::WorkspaceRollbackStarted,
        false,
        "starting workspace rollback",
    );

    let forget_result = zjj_core::jj::workspace_forget(name).await;
    if let Err(forget_error) = forget_result {
        tracing::warn!(
            "Failed to forget workspace '{}' during rollback: {}",
            name,
            forget_error
        );
    }

    let workspace_dir = workspace_path;

    // Remove workspace directory directly without checking existence first
    // This prevents TOCTOU - if it doesn't exist, we get an error we ignore
    match tokio::fs::remove_dir_all(workspace_dir).await {
        Ok(()) => {
            tracing::info!("Rolled back partial workspace for session '{name}'");
            log_add_state(
                name,
                AddAtomicState::WorkspaceRollbackSucceeded,
                false,
                "workspace directory removed",
            );
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Workspace doesn't exist - nothing to clean
            tracing::info!(
                "No workspace to rollback for session '{}' (already gone)",
                name
            );
            log_add_state(
                name,
                AddAtomicState::WorkspaceRollbackSucceeded,
                false,
                "workspace already missing",
            );
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotADirectory => {
            match tokio::fs::remove_file(workspace_dir).await {
                Ok(()) => {
                    tracing::info!("Rolled back partial workspace file for session '{}'", name);
                    log_add_state(
                        name,
                        AddAtomicState::WorkspaceRollbackSucceeded,
                        false,
                        "workspace file removed",
                    );
                    Ok(())
                }
                Err(file_err) if file_err.kind() == std::io::ErrorKind::NotFound => {
                    tracing::info!(
                        "No workspace file to rollback for session '{}' (already gone)",
                        name
                    );
                    log_add_state(
                        name,
                        AddAtomicState::WorkspaceRollbackSucceeded,
                        false,
                        "workspace file already missing",
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
            tracing::warn!(
                "Failed to rollback workspace for session '{}': {}",
                name,
                cleanup_err
            );
            log_add_state(
                name,
                AddAtomicState::WorkspaceRollbackFailed,
                true,
                "workspace rollback failed; session remains recoverable",
            );
            Err(anyhow::anyhow!(
                "Failed to rollback partial workspace '{}': {}. Recovery: run 'zjj remove {} --force'",
                workspace_path.display(),
                cleanup_err,
                name
            ))
        }
    }
}

/// Create a JJ workspace for the session with operation graph synchronization
///
/// This uses the synchronized workspace creation to prevent operation graph
/// corruption when multiple workspaces are created concurrently.
async fn create_jj_workspace(
    name: &str,
    workspace_path: &std::path::Path,
    repo_root: &std::path::Path,
) -> Result<()> {
    // Use the synchronized workspace creation to prevent operation graph corruption
    // This ensures:
    // 1. Workspace creations are serialized (prevents concurrent modification)
    // 2. All workspaces are based on the same repository operation
    // 3. Operation graph consistency is verified after creation
    // CRITICAL-004 fix: Pass repo_root explicitly to support sibling workspace directories
    zjj_core::jj_operation_sync::create_workspace_synced(name, workspace_path, repo_root)
        .await
        .map_err(anyhow::Error::new)?;

    Ok(())
}
