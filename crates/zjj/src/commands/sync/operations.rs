//! Sync operation implementations
//!
//! Core sync logic for rebasing sessions and managing session state.

use anyhow::{Context, Result};

use super::branch_detection::detect_main_branch;
use super::rebase::RebaseStats;
use crate::session::{Session, SessionUpdate};

/// Result of syncing a session
#[derive(Debug)]
pub struct SessionSyncResult {
    pub name: String,
    pub result: Result<RebaseStats>,
}

/// Internal function to sync a session's workspace
pub async fn sync_session_internal(
    db: &crate::database::SessionDb,
    name: &str,
    workspace_path: &str,
) -> Result<RebaseStats> {
    // Validate workspace exists before attempting sync
    validate_workspace_exists(workspace_path, name)?;

    // Load config to get main branch setting
    let config = zjj_core::config::load_config().context("Failed to load configuration")?;

    // Determine target branch: use config if set, otherwise auto-detect
    let target_branch = determine_target_branch_internal(&config, workspace_path)?;

    // Run rebase in the session's workspace
    let stats = super::rebase::execute_rebase(workspace_path, &target_branch)?;

    // Update last_synced timestamp
    update_sync_timestamp(db, name).await?;

    Ok(stats)
}

/// Validate that a workspace exists and is a directory
fn validate_workspace_exists(workspace_path: &str, name: &str) -> Result<()> {
    let workspace_pathbuf = std::path::Path::new(workspace_path);

    if !workspace_pathbuf.exists() {
        anyhow::bail!(
            "Workspace directory not found: {workspace_path}\n\
             \n\
             The workspace may have been deleted manually.\n\
             \n\
             Suggestions:\n\
             • Run 'jjz doctor' to detect and fix orphaned sessions\n\
             • Remove the session: jjz remove {name} --force\n\
             • Recreate the session: jjz add {name}"
        );
    }

    if !workspace_pathbuf.is_dir() {
        anyhow::bail!(
            "Workspace path is not a directory: {workspace_path}\n\
             \n\
             Expected a directory but found a file.\n\
             This indicates database corruption or manual file system changes."
        );
    }

    Ok(())
}

/// Determine the target branch for syncing
fn determine_target_branch_internal(
    config: &zjj_core::config::Config,
    workspace_path: &str,
) -> Result<String> {
    match &config.main_branch {
        Some(branch) if !branch.trim().is_empty() => Ok(branch.clone()),
        Some(_) => {
            // This should be caught by validation, but double-check
            anyhow::bail!(
                "main_branch cannot be empty in config - either unset it or provide a branch name"
            )
        }
        None => detect_main_branch(workspace_path)
            .context("Failed to detect main branch. Set 'main_branch' in .zjj/config.toml"),
    }
}

/// Update the `last_synced` timestamp for a session
async fn update_sync_timestamp(db: &crate::database::SessionDb, name: &str) -> Result<()> {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("System time error")?
        .as_secs();

    db.update(
        name,
        SessionUpdate {
            last_synced: Some(now),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to update sync timestamp: {e}"))
}

/// Sync all sessions and collect results (sequential to avoid workspace conflicts)
pub async fn sync_all_sessions(
    db: &crate::database::SessionDb,
    sessions: &[Session],
) -> Vec<SessionSyncResult> {
    // Sequential async iteration required for workspace safety
    // Cannot parallelize: workspace conflicts, Cannot use functional patterns: async/await with ?
    // early return This is an acceptable iterative pattern per functional-rust-generator
    // guidelines
    let mut results = Vec::with_capacity(sessions.len());

    for session in sessions {
        let result = sync_session_internal(db, &session.name, &session.workspace_path).await;
        results.push(SessionSyncResult {
            name: session.name.clone(),
            result,
        });
    }

    results
}

/// Aggregate sync results into counts and errors
pub fn aggregate_results(
    results: &[SessionSyncResult],
) -> (usize, usize, Vec<crate::json_output::SyncError>) {
    // Functional approach: partition results into successes and failures
    let (successes, failures): (Vec<_>, Vec<_>) =
        results.iter().partition(|result| result.result.is_ok());

    let success_count = successes.len();
    let failure_count = failures.len();

    // Map failures to SyncError using functional iterator chain
    let errors = failures
        .into_iter()
        .filter_map(|result| {
            result
                .result
                .as_ref()
                .err()
                .map(|e| crate::json_output::SyncError {
                    session_name: result.name.clone(),
                    error: e.to_string(),
                })
        })
        .collect();

    (success_count, failure_count, errors)
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{database::SessionDb, session::SessionUpdate};

    // Helper to create a test database
    async fn setup_test_db() -> anyhow::Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    // Helper to get current unix timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    #[test]
    fn test_sync_session_not_found() -> anyhow::Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            let result = db.get("nonexistent").await?;
            assert!(result.is_none());
            Ok(())
        })
    }

    #[test]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            let session = db.create("test-session", "/fake/workspace").await?;
            assert!(session.last_synced.is_none());
            let retrieved = db.get("test-session").await?;
            assert!(retrieved.is_some());
            if let Some(session) = retrieved {
                assert_eq!(session.name, "test-session");
            }
            Ok(())
        })
    }

    #[test]
    fn test_update_last_synced_timestamp() -> anyhow::Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            db.create("test-session", "/fake/workspace").await?;
            let now = current_timestamp();
            let update = SessionUpdate {
                last_synced: Some(now),
                ..Default::default()
            };
            db.update("test-session", update).await?;
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert_eq!(session.last_synced, Some(now));
            }
            Ok(())
        })
    }

    #[test]
    fn test_list_all_sessions() -> anyhow::Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            db.create("session1", "/fake/workspace1").await?;
            db.create("session2", "/fake/workspace2").await?;
            db.create("session3", "/fake/workspace3").await?;
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 3);
            Ok(())
        })
    }

    #[test]
    fn test_sync_updates_timestamp_on_success() -> anyhow::Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            db.create("test-session", "/fake/workspace").await?;
            let before = current_timestamp();
            let update = SessionUpdate {
                last_synced: Some(before),
                ..Default::default()
            };
            db.update("test-session", update).await?;
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= before);
                }
            }
            Ok(())
        })
    }

    #[test]
    fn test_multiple_syncs_update_timestamp() -> anyhow::Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            db.create("test-session", "/fake/workspace").await?;
            let first_sync = current_timestamp();
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(first_sync),
                    ..Default::default()
                },
            )
            .await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let second_sync = current_timestamp();
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(second_sync),
                    ..Default::default()
                },
            )
            .await?;
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= second_sync);
                }
            }
            Ok(())
        })
    }
}
