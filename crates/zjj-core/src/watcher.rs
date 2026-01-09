//! File watching for beads database changes
//!
//! Monitors `.beads/beads.db` in all workspace directories and emits
//! events when changes are detected. Events are debounced to prevent
//! excessive updates during bulk changes.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//!
//! use zjj_core::{
//!     config::WatchConfig,
//!     watcher::{FileWatcher, WatchEvent},
//! };
//!
//! # async fn example() -> zjj_core::Result<()> {
//! let config = WatchConfig {
//!     enabled: true,
//!     debounce_ms: 100,
//!     paths: vec![".beads/beads.db".to_string()],
//! };
//!
//! let workspaces = vec![PathBuf::from("/path/to/workspace")];
//! let mut rx = FileWatcher::watch_workspaces(&config, workspaces)?;
//!
//! while let Some(event) = rx.recv().await {
//!     match event {
//!         WatchEvent::BeadsChanged { workspace_path } => {
//!             // Update UI
//!             println!("Beads changed in {:?}", workspace_path);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use notify::RecursiveMode;
#[cfg(test)]
use notify_debouncer_mini::DebouncedEventKind;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use tokio::sync::mpsc;

use crate::{config::WatchConfig, Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Events emitted by the file watcher
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// Beads database changed in a workspace
    BeadsChanged { workspace_path: PathBuf },
}

/// Beads status for a workspace
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadsStatus {
    /// No beads database found
    NoBeads,
    /// Beads database with issue counts
    Counts {
        open: u32,
        in_progress: u32,
        blocked: u32,
        closed: u32,
    },
}

/// File watcher for beads database changes
pub struct FileWatcher;

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

impl FileWatcher {
    /// Watch beads databases in multiple workspaces
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Watcher is disabled in config
    /// - Debounce duration is invalid
    /// - Unable to watch any of the workspace paths
    /// - Unable to create event channel
    pub fn watch_workspaces(
        config: &WatchConfig,
        workspaces: Vec<PathBuf>,
    ) -> Result<mpsc::Receiver<WatchEvent>> {
        if !config.enabled {
            return Err(Error::InvalidConfig("File watcher is disabled".to_string()));
        }

        // Validate debounce_ms is in acceptable range (10-5000ms)
        if config.debounce_ms < 10 || config.debounce_ms > 5000 {
            return Err(Error::InvalidConfig(format!(
                "debounce_ms must be between 10 and 5000, got {}",
                config.debounce_ms
            )));
        }

        let (tx, rx) = mpsc::channel(100);

        // Create debouncer with the event handler
        let mut debouncer = new_debouncer(
            Duration::from_millis(u64::from(config.debounce_ms)),
            move |res: notify_debouncer_mini::DebounceEventResult| {
                if let Ok(events) = res {
                    for event in events {
                        if let Some(workspace_path) = extract_workspace_path(&event) {
                            let _ = tx.blocking_send(WatchEvent::BeadsChanged { workspace_path });
                        }
                    }
                }
            },
        )
        .map_err(|e| Error::IoError(format!("Failed to create file watcher: {e}")))?;

        // Watch each workspace's beads database
        for workspace in workspaces {
            let beads_db = workspace.join(".beads/beads.db");
            if beads_db.exists() {
                debouncer
                    .watcher()
                    .watch(&beads_db, RecursiveMode::NonRecursive)
                    .map_err(|e| {
                        Error::IoError(format!("Failed to watch {}: {e}", beads_db.display()))
                    })?;
            }
        }

        // Keep debouncer alive by moving it into a background task
        tokio::spawn(async move {
            // Hold onto the debouncer to keep watching
            let _debouncer = debouncer;
            // Wait indefinitely
            tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
        });

        Ok(rx)
    }
}

/// Query beads status for a workspace
///
/// # Errors
///
/// Returns error if:
/// - Unable to open database
/// - Database query fails
/// - Database schema is invalid
pub fn query_beads_status(workspace_path: &Path) -> Result<BeadsStatus> {
    let beads_db = workspace_path.join(".beads/beads.db");

    if !beads_db.exists() {
        return Ok(BeadsStatus::NoBeads);
    }

    let conn = rusqlite::Connection::open(&beads_db)
        .map_err(|e| Error::DatabaseError(format!("Failed to open beads database: {e}")))?;

    // Query for each status count
    let open = query_count(&conn, "open")?;
    let in_progress = query_count(&conn, "in_progress")?;
    let blocked = query_count(&conn, "blocked")?;
    let closed = query_count(&conn, "closed")?;

    Ok(BeadsStatus::Counts {
        open,
        in_progress,
        blocked,
        closed,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Extract workspace path from a debounced event
fn extract_workspace_path(event: &DebouncedEvent) -> Option<PathBuf> {
    event
        .path
        .parent() // .beads
        .and_then(|p| p.parent()) // workspace root
        .map(std::path::Path::to_path_buf)
}

/// Query count of issues with a specific status
fn query_count(conn: &rusqlite::Connection, status: &str) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM issues WHERE status = ?1",
        [status],
        |row| row.get::<_, u32>(0),
    )
    .map_err(|e| Error::DatabaseError(format!("Failed to query {status} count: {e}")))
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 1: FileWatcher with disabled config returns error
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_watcher_disabled() {
        let config = WatchConfig {
            enabled: false,
            debounce_ms: 100,
            paths: vec![".beads/beads.db".to_string()],
        };

        let result = FileWatcher::watch_workspaces(&config, vec![]);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, Error::InvalidConfig(_)));
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 2: Invalid debounce_ms too low
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_watcher_invalid_debounce_too_low() {
        let config = WatchConfig {
            enabled: true,
            debounce_ms: 5,
            paths: vec![".beads/beads.db".to_string()],
        };

        let result = FileWatcher::watch_workspaces(&config, vec![]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::InvalidConfig(_)));
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 3: Invalid debounce_ms too high
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_watcher_invalid_debounce_too_high() {
        let config = WatchConfig {
            enabled: true,
            debounce_ms: 10000,
            paths: vec![".beads/beads.db".to_string()],
        };

        let result = FileWatcher::watch_workspaces(&config, vec![]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::InvalidConfig(_)));
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 5: Query beads status - no beads
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_query_beads_status_no_beads() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(_) => return,
        };
        let result = query_beads_status(temp_dir.path());

        assert!(result.is_ok());
        if let Ok(status) = result {
            assert_eq!(status, BeadsStatus::NoBeads);
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 6: Query beads status - with valid database
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_query_beads_status_with_database() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(_) => return,
        };
        let beads_dir = temp_dir.path().join(".beads");
        if fs::create_dir(&beads_dir).is_err() {
            return;
        }

        let db_path = beads_dir.join("beads.db");
        let conn = match rusqlite::Connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Create schema
        if conn
            .execute(
                "CREATE TABLE issues (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL
            )",
                [],
            )
            .is_err()
        {
            return;
        }

        // Insert test data
        let _ = conn.execute("INSERT INTO issues (id, status) VALUES ('1', 'open')", []);
        let _ = conn.execute(
            "INSERT INTO issues (id, status) VALUES ('2', 'in_progress')",
            [],
        );
        let _ = conn.execute(
            "INSERT INTO issues (id, status) VALUES ('3', 'in_progress')",
            [],
        );
        let _ = conn.execute(
            "INSERT INTO issues (id, status) VALUES ('4', 'blocked')",
            [],
        );
        let _ = conn.execute("INSERT INTO issues (id, status) VALUES ('5', 'closed')", []);
        let _ = conn.execute("INSERT INTO issues (id, status) VALUES ('6', 'closed')", []);
        let _ = conn.execute("INSERT INTO issues (id, status) VALUES ('7', 'closed')", []);

        drop(conn);

        let result = query_beads_status(temp_dir.path());
        assert!(result.is_ok());

        if let Ok(status) = result {
            match status {
                BeadsStatus::Counts {
                    open,
                    in_progress,
                    blocked,
                    closed,
                } => {
                    assert_eq!(open, 1);
                    assert_eq!(in_progress, 2);
                    assert_eq!(blocked, 1);
                    assert_eq!(closed, 3);
                }
                BeadsStatus::NoBeads => {
                    assert!(false, "Expected Counts, got NoBeads");
                }
            }
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 7: Extract workspace path from event
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_extract_workspace_path() {
        let event = DebouncedEvent {
            path: PathBuf::from("/workspace/.beads/beads.db"),
            kind: DebouncedEventKind::Any,
        };

        let result = extract_workspace_path(&event);
        assert!(result.is_some());
        if let Some(path) = result {
            assert_eq!(path, PathBuf::from("/workspace"));
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 8: Watch event equality
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_watch_event_equality() {
        let event1 = WatchEvent::BeadsChanged {
            workspace_path: PathBuf::from("/workspace"),
        };
        let event2 = WatchEvent::BeadsChanged {
            workspace_path: PathBuf::from("/workspace"),
        };
        let event3 = WatchEvent::BeadsChanged {
            workspace_path: PathBuf::from("/other"),
        };

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 9: BeadsStatus equality
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_beads_status_equality() {
        let status1 = BeadsStatus::Counts {
            open: 1,
            in_progress: 2,
            blocked: 0,
            closed: 3,
        };
        let status2 = BeadsStatus::Counts {
            open: 1,
            in_progress: 2,
            blocked: 0,
            closed: 3,
        };
        let status3 = BeadsStatus::NoBeads;

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }
}
