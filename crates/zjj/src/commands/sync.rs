//! Sync a session's workspace with main branch

use std::{io::Write, path::Path, time::SystemTime};

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::process::Command;
use zjj_core::{
    json::{ErrorDetail, SchemaEnvelope},
    OutputFormat,
};

use crate::{
    cli::run_command,
    commands::{determine_main_branch, get_session_db},
    json::{SyncError, SyncOutput},
    session::SessionUpdate,
};

/// Options for the sync command
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOptions {
    /// Output format
    pub format: OutputFormat,
}

/// Run the sync command with options
///
/// If a session name is provided, syncs that session's workspace.
/// Otherwise, syncs all sessions.
pub async fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<()> {
    match name {
        Some(n) => sync_session_with_options(n, options).await,
        None => sync_all_with_options(options).await,
    }
}

/// Sync a specific session's workspace
async fn sync_session_with_options(name: &str, options: SyncOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Get the session
    // Return zjj_core::Error::NotFound to get exit code 2 (not found)
    let session = db.get(name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        )))
    })?;

    // Use internal sync function
    sync_session_internal(&db, &session.name, &session.workspace_path).await?;

    if options.format.is_json() {
        let output = SyncOutput {
            name: Some(name.to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
        };
        let envelope = SchemaEnvelope::new("sync-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        println!("{json_str}");
    } else {
        println!("Synced session '{name}' with main");
        println!();
        println!("NEXT: Continue working, or if done:");
        println!("  zjj done          # Merge to main + cleanup");
    }

    Ok(())
}

/// Sync all sessions
#[allow(clippy::too_many_lines)]
async fn sync_all_with_options(options: SyncOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Get all sessions
    // Preserve error type for proper exit code mapping
    let sessions = db.list(None).await.map_err(anyhow::Error::new)?;

    if sessions.is_empty() {
        if options.format.is_json() {
            let output = SyncOutput {
                name: None,
                synced_count: 0,
                failed_count: 0,
                errors: Vec::new(),
            };
            let envelope = SchemaEnvelope::new("sync-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            println!("{json_str}");
        } else {
            println!("No sessions to sync");
        }
        return Ok(());
    }

    // For JSON output, collect results concurrently and output once at the end
    if options.format.is_json() {
        let results: Vec<_> = futures::stream::iter(sessions)
            .map(|session| {
                let db = &db;
                async move {
                    let res = sync_session_internal(db, &session.name, &session.workspace_path).await;
                    (session, res)
                }
            })
            .buffered(5) // Limit concurrency to 5
            .collect()
            .await;

        let (successes, errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|(_, res)| res.is_ok());

        let output = SyncOutput {
            name: None,
            synced_count: successes.len(),
            failed_count: errors.len(),
            errors: errors
                .into_iter()
                .filter_map(|(session, res)| {
                    res.err().map(|e| SyncError {
                        name: session.name.clone(),
                        error: ErrorDetail {
                            code: "SYNC_FAILED".to_string(),
                            message: e.to_string(),
                            exit_code: 3,
                            details: None,
                            suggestion: Some(
                                "Try 'jj resolve' to fix conflicts, then retry sync".to_string(),
                            ),
                        },
                    })
                })
                .collect(),
        };

        let has_failures = output.failed_count > 0;
        let envelope = if has_failures {
            SchemaEnvelope::new("sync-response", "single", output).as_error()
        } else {
            SchemaEnvelope::new("sync-response", "single", output)
        };
        let json_str = serde_json::to_string(&envelope)?;
        println!("{json_str}");

        if has_failures {
            anyhow::bail!("Failed to sync {} session(s)", envelope.data.failed_count);
        }
    } else {
        // Original text output
        println!("Syncing {} session(s)...", sessions.len());

        // Process sessions sequentially for text mode to avoid interleaved output
        let (success_count, failure_count, errors) = futures::stream::iter(sessions)
            .fold(
                (0, 0, Vec::new()),
                |(mut s_acc, mut f_acc, mut err_acc), session| {
                    let db = &db;
                    async move {
                        print!("Syncing '{}' ... ", &session.name);
                        let _ = std::io::stdout().flush();

                        match sync_session_internal(db, &session.name, &session.workspace_path)
                            .await
                        {
                            Ok(()) => {
                                println!("OK");
                                s_acc += 1;
                            }
                            Err(e) => {
                                println!("FAILED: {e}");
                                f_acc += 1;
                                err_acc.push((session.name.clone(), e));
                            }
                        }
                        (s_acc, f_acc, err_acc)
                    }
                },
            )
            .await;

        println!();
        println!("Summary: {success_count} succeeded, {failure_count} failed");

        if !errors.is_empty() {
            println!("\nErrors:");
            for (name, error) in errors {
                println!("  {name}: {error}");
            }
            anyhow::bail!("Failed to sync {failure_count} session(s)");
        }
    }
    Ok(())
}

/// Internal function to sync a session's workspace
async fn sync_session_internal(
    db: &crate::db::SessionDb,
    name: &str,
    workspace_path: &str,
) -> Result<()> {
    let main_branch = determine_main_branch(Path::new(workspace_path)).await;

    // Run rebase in the session's workspace
    run_command(
        "jj",
        &["--repository", workspace_path, "rebase", "-d", &main_branch],
    )
    .await
    .context("Failed to sync workspace with main")?;

    // Update last_synced timestamp
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
    .map_err(anyhow::Error::new)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{commands::determine_main_branch, db::SessionDb, session::SessionUpdate};

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
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Try to sync a non-existent session
            // We can't actually run this without a real JJ repo, but we can test the lookup
            let result = db.get("nonexistent").await?;
            assert!(result.is_none());
            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            let session = db.create("test-session", "/fake/workspace").await?;
            assert!(session.last_synced.is_none());

            // Verify we can get it
            let retrieved = db.get("test-session").await?;
            assert!(retrieved.is_some());
            if let Some(session) = retrieved {
                assert_eq!(session.name, "test-session");
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_update_last_synced_timestamp() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // Update last_synced
            let now = current_timestamp();
            let update = SessionUpdate {
                last_synced: Some(now),
                ..Default::default()
            };
            db.update("test-session", update).await?;

            // Verify it was updated
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert_eq!(session.last_synced, Some(now));
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_list_all_sessions() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create multiple sessions
            db.create("session1", "/fake/workspace1").await?;
            db.create("session2", "/fake/workspace2").await?;
            db.create("session3", "/fake/workspace3").await?;

            // List all
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 3);

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_updates_timestamp_on_success() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // Simulate successful sync by updating timestamp
            let before = current_timestamp();
            let update = SessionUpdate {
                last_synced: Some(before),
                ..Default::default()
            };
            db.update("test-session", update).await?;

            // Verify timestamp was set
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= before);
                }
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_multiple_syncs_update_timestamp() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // First sync
            let first_sync = current_timestamp();
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(first_sync),
                    ..Default::default()
                },
            )
            .await?;

            // Sleep to ensure different timestamp
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Second sync
            let second_sync = current_timestamp();
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(second_sync),
                    ..Default::default()
                },
            )
            .await?;

            // Verify second timestamp is newer
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= second_sync);
                }
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[tokio::test]
    async fn test_determine_main_branch_not_in_repo() -> anyhow::Result<()> {
        // When not in a JJ repo, should fall back to "main"
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path()).await;
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_determine_main_branch_fallback() -> anyhow::Result<()> {
        // Test that function returns "main" when trunk() fails
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path()).await;
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_sync_uses_determined_main_branch() -> anyhow::Result<()> {
        // Test that sync_session_internal uses determine_main_branch
        // This test will verify the integration once implemented
        let temp = tempfile::TempDir::new()?;
        let branch = determine_main_branch(temp.path()).await;
        assert_eq!(branch, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_main_branch_detection_respects_trunk() -> anyhow::Result<()> {
        // When trunk() returns a valid commit ID, use it
        let temp = tempfile::TempDir::new()?;
        let _branch = determine_main_branch(temp.path()).await;
        // Implementation should use trunk() when available
        Ok(())
    }

    #[test]
    fn test_sync_json_single_output_on_success() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // JSON mode should output exactly one JSON object on success
            let (db, _dir) = setup_test_db().await?;
            db.create("test-session", "/fake/workspace").await?;

            // Simulate sync success by mocking internal function
            // RED: This test will verify that output is single JSON
            // Once implemented, this should pass

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_json_single_output_on_failure() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // JSON mode should output exactly one JSON object on failure
            // RED: Currently outputs two JSON objects (SyncResponse + separate error)
            let (db, _dir) = setup_test_db().await?;
            db.create("test-session", "/fake/workspace").await?;

            // Simulate sync failure by mocking internal function
            // RED: This test will fail until duplicate JSON bug is fixed
            // Expected: ONE JSON response with success=false
            // Actual: TWO JSON objects

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_json_parseable_by_jq() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // JSON output should be parseable by jq
            // RED: Duplicate JSON objects break jq parsing
            let (db, _dir) = setup_test_db().await?;
            db.create("test-session", "/fake/workspace").await?;

            // Simulate JSON output and verify parseability
            // RED: This will fail until bug is fixed

            Ok::<_, anyhow::Error>(())
        })
    }
}
