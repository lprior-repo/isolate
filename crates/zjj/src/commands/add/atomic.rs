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
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub(super) async fn atomic_create_session(
    name: &str,
    workspace_path: &std::path::Path,
    repo_root: &std::path::Path,
    db: &SessionDb,
    bead_metadata: Option<serde_json::Value>,
    create_command_id: Option<&str>,
    allow_existing: bool,
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

    // STEP 2: Create JJ workspace
    log_add_state(
        name,
        AddAtomicState::WorkspaceCreateStarted,
        false,
        "creating jj workspace",
    );

    // ADVERSARIAL FIX: Improved check for existing paths.
    // If allow_existing is false, we error out BEFORE calling JJ to maintain
    // consistency and avoid surprising JJ behavior.
    // We use the specific error message expected by behavioral tests.
    let workspace_result = if !allow_existing
        && tokio::fs::try_exists(workspace_path).await.unwrap_or(false)
    {
        Err(anyhow::anyhow!(
            "Failed to create workspace: path already exists at {}",
            workspace_path.display()
        )
        .context("Recovery: run 'zjj doctor --fix' or manually remove the directory"))
    } else {
        create_jj_workspace(name, workspace_path, repo_root, allow_existing).await.and_then(|()| {
            if std::fs::metadata(workspace_path).map(|m| m.is_dir()).unwrap_or(false) {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "workspace creation reported success but directory '{}' is missing or not a directory",
                    workspace_path.display()
                ))
            }
        })
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
            Ok(())
        }
        Err(ref workspace_error) => {
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
            // Roll back both sides.
            let workspace_cleanup = rollback_partial_state(name, workspace_path).await;
            let database_cleanup = rollback_database_state(name, db, create_command_id).await;

            if matches!((&workspace_cleanup, &database_cleanup), (Ok(()), Ok(()))) {
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
                Err(anyhow::anyhow!(workspace_error.to_string())).context(format!(
                    "Failed to create workspace, rolled back. Recovery: run 'zjj remove {name} --force' if stale artifacts remain"
                ))
            } else {
                db.upsert_add_operation_journal(
                    &operation_id,
                    name,
                    &workspace_path_str,
                    create_command_id,
                    JOURNAL_FAILED_COMPENSATION,
                    Some(&format!("rollback failure: {workspace_error}")),
                )
                .await?;
                Err(anyhow::anyhow!(workspace_error.to_string())).context(format!(
                    "Failed to create workspace and rollback failed. Recovery: run 'zjj remove {name} --force'"
                ))
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

    if matches!((&workspace_cleanup, &database_cleanup), (Ok(()), Ok(()))) {
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
    } else {
        db.upsert_add_operation_journal(
            &operation.operation_id,
            &operation.session_name,
            &operation.workspace_path,
            command_id,
            JOURNAL_FAILED_COMPENSATION,
            Some("replay failure"),
        )
        .await?;
        Err(anyhow::anyhow!(
            "failed to reconcile add operation {}",
            operation.operation_id
        ))
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
        Ok(_) => {
            log_add_state(
                name,
                AddAtomicState::DatabaseRollbackSucceeded,
                false,
                "partial session record removed",
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

    let _ = zjj_core::jj::workspace_forget(name).await;

    // ADVERSARIAL FIX: Improved robust cleanup during rollback.
    // Try removing as directory first, fallback to file if it fails.
    let res = if workspace_path.is_dir() {
        tokio::fs::remove_dir_all(workspace_path).await
    } else if workspace_path.is_file() {
        tokio::fs::remove_file(workspace_path).await
    } else {
        // Path might not exist or be a special file
        tokio::fs::remove_dir_all(workspace_path).await
    };

    match res {
        Ok(()) => {
            log_add_state(
                name,
                AddAtomicState::WorkspaceRollbackSucceeded,
                false,
                "workspace path removed",
            );
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log_add_state(
                name,
                AddAtomicState::WorkspaceRollbackSucceeded,
                false,
                "workspace already missing",
            );
            Ok(())
        }
        Err(cleanup_err) => {
            log_add_state(
                name,
                AddAtomicState::WorkspaceRollbackFailed,
                true,
                "workspace rollback failed",
            );
            Err(cleanup_err.into())
        }
    }
}

async fn create_jj_workspace(
    name: &str,
    workspace_path: &std::path::Path,
    repo_root: &std::path::Path,
    allow_existing: bool,
) -> Result<()> {
    zjj_core::jj_operation_sync::create_workspace_synced_ext(
        name,
        workspace_path,
        repo_root,
        allow_existing,
    )
    .await
    .map_err(anyhow::Error::new)?;
    Ok(())
}
