//! File watching for beads database changes
//!
//! Monitors `.beads/beads.db` in all workspace directories and emits
//! events when changes are detected. Events are debounced to prevent
//! excessive updates during bulk changes.

use std::{path::PathBuf, time::Duration};

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
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
            return Err(Error::invalid_config("File watcher is disabled"));
        }

        // Validate debounce_ms is in acceptable range (10-5000ms)
        if config.debounce_ms < 10 || config.debounce_ms > 5000 {
            return Err(Error::invalid_config(format!(
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
                    // Functional pipeline to process events
                    events
                        .into_iter()
                        .filter_map(|event| super::callbacks::extract_workspace_path(&event))
                        .for_each(|workspace_path| {
                            let _ = tx.blocking_send(WatchEvent::BeadsChanged { workspace_path });
                        });
                }
            },
        )
        .map_err(|e| Error::io_error(format!("Failed to create file watcher: {e}")))?;

        // Watch each workspace's beads database using functional pipeline
        workspaces
            .into_iter()
            .map(|workspace| workspace.join(".beads/beads.db"))
            .filter(|beads_db| beads_db.exists())
            .try_for_each(|beads_db| {
                debouncer
                    .watcher()
                    .watch(&beads_db, RecursiveMode::NonRecursive)
                    .map_err(|e| {
                        Error::io_error(format!("Failed to watch {}: {e}", beads_db.display()))
                    })
            })?;

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

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
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
            assert!(matches!(e, Error::Validation(_)));
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
            assert!(matches!(err, Error::Validation(_)));
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
            assert!(matches!(err, Error::Validation(_)));
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 4: Watch event equality
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
}
