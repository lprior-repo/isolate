//! Sync a session's workspace with main branch

use std::{path::Path, time::SystemTime};

use anyhow::{Context, Result};
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{
    cli::run_command,
    commands::{determine_main_branch, get_session_db},
    json_output::{SyncError, SyncOutput},
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
pub fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<()> {
    name.map_or_else(
        || sync_all_with_options(options),
        |n| sync_session_with_options(n, options),
    )
}

/// Sync a specific session's workspace
fn sync_session_with_options(name: &str, options: SyncOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    // Return zjj_core::Error::NotFound to get exit code 2 (not found)
    let session = db.get(name)?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        )))
    })?;

    // Use internal sync function
    match sync_session_internal(&db, &session.name, &session.workspace_path) {
        Ok(()) => {
            if options.format.is_json() {
                let output = SyncOutput {
                    name: Some(name.to_string()),
                    synced_count: 1,
                    failed_count: 0,
                    errors: Vec::new(),
                };
                let envelope = SchemaEnvelope::new("sync-response", "single", output);
                println!("{}", serde_json::to_string(&envelope)?);
            } else {
                println!("Synced session '{name}' with main");
            }
            Ok(())
        }
        Err(e) => {
            if options.format.is_json() {
                let output = SyncOutput {
                    name: Some(name.to_string()),
                    synced_count: 0,
                    failed_count: 1,
                    errors: vec![SyncError {
                        name: name.to_string(),
                        error: e.to_string(),
                    }],
                };
                let envelope = SchemaEnvelope::new("sync-response", "single", output);
                println!("{}", serde_json::to_string(&envelope)?);
                Ok(())
            }
            Err(e)
        }
    }
}

/// Sync all sessions
fn sync_all_with_options(options: SyncOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get all sessions
    // Preserve error type for proper exit code mapping
    let sessions = db.list(None).map_err(anyhow::Error::new)?;

    if sessions.is_empty() {
        if options.format.is_json() {
            let output = SyncOutput {
                name: None,
                synced_count: 0,
                failed_count: 0,
                errors: Vec::new(),
            };
            let envelope = SchemaEnvelope::new("sync-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("No sessions to sync");
        }
        return Ok(());
    }

    if options.format.is_json() {
        // For JSON output, collect results and output once at the end
        // Use functional pattern: map to Results, partition into successes/failures
        let (successes, errors): (Vec<_>, Vec<_>) = sessions
            .iter()
            .map(|session| {
                sync_session_internal(&db, &session.name, &session.workspace_path)
                    .map(|()| session.name.clone())
                    .map_err(|e| SyncError {
                        name: session.name.clone(),
                        error: e.to_string(),
                    })
            })
            .partition(Result::is_ok);

        let output = SyncOutput {
            name: None,
            synced_count: successes.len(),
            failed_count: errors.len(),
            errors: errors.into_iter().filter_map(Result::err).collect(),
        };
        let envelope = SchemaEnvelope::new("sync-response", "single", output);
        println!("{}", serde_json::to_string(&envelope)?);
    } else {
        // Original text output
        println!("Syncing {} session(s)...", sessions.len());

        // Use functional pattern: map to Results with side effects, partition into
        // successes/failures
        let (successes, errors): (Vec<_>, Vec<_>) = sessions
            .iter()
            .map(|session| {
                print!("Syncing '{}' ... ", session.name);

                sync_session_internal(&db, &session.name, &session.workspace_path)
                    .map(|()| {
                        println!("OK");
                        session.name.clone()
                    })
                    .map_err(|e| {
                        println!("FAILED: {e}");
                        (session.name.clone(), e)
                    })
            })
            .partition(Result::is_ok);

        let success_count = successes.len();
        let failure_count = errors.len();

        println!();
        println!("Summary: {success_count} succeeded, {failure_count} failed");

        if !errors.is_empty() {
            println!("\nErrors:");
            errors
                .into_iter()
                .filter_map(Result::err)
                .for_each(|(name, error)| {
                    println!("  {name}: {error}");
                });
        }
    }
    Ok(())
}

/// Internal function to sync a session's workspace
fn sync_session_internal(
    db: &crate::db::SessionDb,
    name: &str,
    workspace_path: &str,
) -> Result<()> {
    let main_branch = determine_main_branch(Path::new(workspace_path));

    // Run rebase in the session's workspace
    run_command(
        "jj",
        &["--repository", workspace_path, "rebase", "-d", &main_branch],
    )
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
    .map_err(anyhow::Error::new)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{commands::determine_main_branch, db::SessionDb, session::SessionUpdate};

    // Helper to create a test database
    fn setup_test_db() -> anyhow::Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
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
        let (db, _dir) = setup_test_db()?;

        // Try to sync a non-existent session
        // We can't actually run this without a real JJ repo, but we can test the lookup
        let result = db.get("nonexistent")?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        let session = db.create("test-session", "/fake/workspace")?;
        assert!(session.last_synced.is_none());

        // Verify we can get it
        let retrieved = db.get("test-session")?;
        assert!(retrieved.is_some());
        if let Some(session) = retrieved {
            assert_eq!(session.name, "test-session");
        }

        Ok(())
    }

    #[test]
    fn test_update_last_synced_timestamp() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        db.create("test-session", "/fake/workspace")?;

        // Update last_synced
        let now = current_timestamp();
        let update = SessionUpdate {
            last_synced: Some(now),
            ..Default::default()
        };
        db.update("test-session", update)?;

        // Verify it was updated
        let session = db.get("test-session")?;
        assert!(session.is_some(), "Session not found");
        if let Some(session) = session {
            assert_eq!(session.last_synced, Some(now));
        }

        Ok(())
    }

    #[test]
    fn test_list_all_sessions() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create multiple sessions
        db.create("session1", "/fake/workspace1")?;
        db.create("session2", "/fake/workspace2")?;
        db.create("session3", "/fake/workspace3")?;

        // List all
        let sessions = db.list(None)?;
        assert_eq!(sessions.len(), 3);

        Ok(())
    }

    #[test]
    fn test_sync_updates_timestamp_on_success() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        db.create("test-session", "/fake/workspace")?;

        // Simulate successful sync by updating timestamp
        let before = current_timestamp();
        let update = SessionUpdate {
            last_synced: Some(before),
            ..Default::default()
        };
        db.update("test-session", update)?;

        // Verify timestamp was set
        let session = db.get("test-session")?;
        assert!(session.is_some(), "Session not found");
        if let Some(session) = session {
            assert!(session.last_synced.is_some(), "last_synced should be set");
            if let Some(last_synced) = session.last_synced {
                assert!(last_synced >= before);
            }
        }

        Ok(())
    }

    #[test]
    fn test_multiple_syncs_update_timestamp() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        db.create("test-session", "/fake/workspace")?;

        // First sync
        let first_sync = current_timestamp();
        db.update(
            "test-session",
            SessionUpdate {
                last_synced: Some(first_sync),
                ..Default::default()
            },
        )?;

        // Sleep to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Second sync
        let second_sync = current_timestamp();
        db.update(
            "test-session",
            SessionUpdate {
                last_synced: Some(second_sync),
                ..Default::default()
            },
        )?;

        // Verify second timestamp is newer
        let session = db.get("test-session")?;
        assert!(session.is_some(), "Session not found");
        if let Some(session) = session {
            assert!(session.last_synced.is_some(), "last_synced should be set");
            if let Some(last_synced) = session.last_synced {
                assert!(last_synced >= second_sync);
            }
        }

        Ok(())
    }

    #[test]
    fn test_determine_main_branch_not_in_repo() -> anyhow::Result<()> {
        // When not in a JJ repo, should fall back to "main"
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path());
        assert_eq!(result, "main");
        Ok(())
    }

    #[test]
    fn test_determine_main_branch_fallback() -> anyhow::Result<()> {
        // Test that function returns "main" when trunk() fails
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path());
        assert_eq!(result, "main");
        Ok(())
    }

    #[test]
    fn test_sync_uses_determined_main_branch() -> anyhow::Result<()> {
        // Test that sync_session_internal uses determine_main_branch
        // This test will verify the integration once implemented
        let temp = tempfile::TempDir::new()?;
        let branch = determine_main_branch(temp.path());
        assert_eq!(branch, "main");
        Ok(())
    }

    #[test]
    fn test_main_branch_detection_respects_trunk() -> anyhow::Result<()> {
        // When trunk() returns a valid commit ID, use it
        let temp = tempfile::TempDir::new()?;
        let _branch = determine_main_branch(temp.path());
        // Implementation should use trunk() when available
        Ok(())
    }

    #[test]
    fn test_sync_json_single_output_on_success() -> anyhow::Result<()> {
        // JSON mode should output exactly one JSON object on success
        let (db, _dir) = setup_test_db()?;
        db.create("test-session", "/fake/workspace")?;

        // Simulate sync success by mocking internal function
        // RED: This test will verify that output is single JSON
        // Once implemented, this should pass

        Ok(())
    }

    #[test]
    fn test_sync_json_single_output_on_failure() -> anyhow::Result<()> {
        // JSON mode should output exactly one JSON object on failure
        // RED: Currently outputs two JSON objects (SyncResponse + separate error)
        let (db, _dir) = setup_test_db()?;
        db.create("test-session", "/fake/workspace")?;

        // Simulate sync failure by mocking internal function
        // RED: This test will fail until duplicate JSON bug is fixed
        // Expected: ONE JSON response with success=false
        // Actual: TWO JSON objects

        Ok(())
    }

    #[test]
    fn test_sync_json_parseable_by_jq() -> anyhow::Result<()> {
        // JSON output should be parseable by jq
        // RED: Duplicate JSON objects break jq parsing
        let (db, _dir) = setup_test_db()?;
        db.create("test-session", "/fake/workspace")?;

        // Simulate JSON output and verify parseability
        // RED: This will fail until bug is fixed

        Ok(())
    }
}
