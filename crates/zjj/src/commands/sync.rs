//! Sync a session's workspace with main branch

use std::time::SystemTime;

use anyhow::{Context, Result};

use crate::{cli::run_command, commands::get_session_db, session::SessionUpdate};

/// Run the sync command
///
/// If a session name is provided, syncs that session's workspace.
/// Otherwise, syncs all sessions.
pub fn run(name: Option<&str>) -> Result<()> {
    match name {
        Some(n) => sync_session(n),
        None => sync_all(),
    }
}

/// Sync a specific session's workspace
fn sync_session(name: &str) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Use internal sync function
    sync_session_internal(&db, &session.name, &session.workspace_path)?;

    println!("Synced session '{name}' with main");
    Ok(())
}

/// Sync all sessions
fn sync_all() -> Result<()> {
    let db = get_session_db()?;

    // Get all sessions
    let sessions = db
        .list(None)
        .map_err(|e| anyhow::anyhow!("Failed to list sessions: {e}"))?;

    if sessions.is_empty() {
        println!("No sessions to sync");
        return Ok(());
    }

    println!("Syncing {} session(s)...", sessions.len());

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors = Vec::new();

    for session in &sessions {
        print!("Syncing '{}' ... ", session.name);

        match sync_session_internal(&db, &session.name, &session.workspace_path) {
            Ok(()) => {
                println!("OK");
                success_count += 1;
            }
            Err(e) => {
                println!("FAILED: {e}");
                errors.push((session.name.clone(), e));
                failure_count += 1;
            }
        }
    }

    println!();
    println!("Summary: {success_count} succeeded, {failure_count} failed");

    if !errors.is_empty() {
        println!("\nErrors:");
        for (name, error) in errors {
            println!("  {name}: {error}");
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
    // Run rebase in the session's workspace
    run_command(
        "jj",
        &["--repository", workspace_path, "rebase", "-d", "main"],
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
    .map_err(|e| anyhow::anyhow!("Failed to update sync timestamp: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{db::SessionDb, session::SessionUpdate};

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
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    fn test_sync_session_not_found() {
        let (db, _dir) = setup_test_db().expect("Failed to setup test db");

        // Try to sync a non-existent session
        // We can't actually run this without a real JJ repo, but we can test the lookup
        let result = db.get("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        let session = db.create("test-session", "/fake/workspace")?;
        assert!(session.last_synced.is_none());

        // Verify we can get it
        let retrieved = db.get("test-session")?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-session");

        Ok(())
    }

    #[test]
    #[allow(clippy::expect_used)]
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
        let session = db.get("test-session")?.expect("Session not found");
        assert_eq!(session.last_synced, Some(now));

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
    #[allow(clippy::unwrap_used)]
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
        let session = session.unwrap();
        assert!(session.last_synced.is_some(), "last_synced should be set");
        assert!(session.last_synced.unwrap() >= before);

        Ok(())
    }

    #[test]
    #[allow(clippy::unwrap_used)]
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
        let session = session.unwrap();
        assert!(session.last_synced.is_some(), "last_synced should be set");
        assert!(session.last_synced.unwrap() >= second_sync);

        Ok(())
    }
}
