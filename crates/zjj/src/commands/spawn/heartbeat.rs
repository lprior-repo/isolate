//! Heartbeat monitoring for spawned agents
//!
//! This module provides heartbeat tracking for spawned agents
//! to detect and handle agent failures.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::types::SpawnError;

/// Path to heartbeat file
const HEARTBEAT_FILE: &str = ".zjj/heartbeat";

/// Default heartbeat interval in seconds
const DEFAULT_HEARTBEAT_INTERVAL: u64 = 30;

/// Default heartbeat timeout in seconds
const DEFAULT_HEARTBEAT_TIMEOUT: u64 = 120;

/// Heartbeat monitor for tracking agent liveness
pub struct HeartbeatMonitor {
    workspace_path: PathBuf,
    heartbeat_path: PathBuf,
    interval: Duration,
    timeout: Duration,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace directory
    /// * `interval` - Heartbeat update interval
    /// * `timeout` - Timeout before agent is considered dead
    pub fn new(workspace_path: &Path, interval: Duration, timeout: Duration) -> Self {
        let heartbeat_path = workspace_path.join(HEARTBEAT_FILE);

        Self {
            workspace_path: workspace_path.to_path_buf(),
            heartbeat_path,
            interval,
            timeout,
        }
    }

    /// Create with default intervals
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace directory
    pub fn with_defaults(workspace_path: &Path) -> Self {
        Self::new(
            workspace_path,
            Duration::from_secs(DEFAULT_HEARTBEAT_INTERVAL),
            Duration::from_secs(DEFAULT_HEARTBEAT_TIMEOUT),
        )
    }

    /// Initialize heartbeat file
    ///
    /// Creates the heartbeat file directory and initializes the heartbeat.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if initialization fails.
    pub fn initialize(&self) -> Result<(), SpawnError> {
        let heartbeat_dir =
            self.heartbeat_path
                .parent()
                .ok_or_else(|| SpawnError::WorkspaceCreationFailed {
                    reason: "Invalid heartbeat path".to_string(),
                })?;

        fs::create_dir_all(heartbeat_dir).map_err(|e| SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to create heartbeat directory: {e}"),
        })?;

        self.update()?;

        Ok(())
    }

    /// Update heartbeat timestamp
    ///
    /// Writes the current Unix timestamp to the heartbeat file.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if write fails.
    pub fn update(&self) -> Result<(), SpawnError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to get current time: {e}"),
            }
        })?;

        let timestamp = now.as_secs().to_string();

        fs::write(&self.heartbeat_path, timestamp).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to write heartbeat: {e}"),
            }
        })?;

        Ok(())
    }

    /// Check if heartbeat is within timeout
    ///
    /// Returns `true` if heartbeat is recent, `false` if timeout exceeded.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if unable to read heartbeat.
    pub fn is_alive(&self) -> Result<bool, SpawnError> {
        if !self.heartbeat_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&self.heartbeat_path).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to read heartbeat: {e}"),
            }
        })?;

        let timestamp: u64 =
            content
                .trim()
                .parse()
                .map_err(|e| SpawnError::WorkspaceCreationFailed {
                    reason: format!("Invalid heartbeat timestamp: {e}"),
                })?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to get current time: {e}"),
            }
        })?;

        let elapsed = now.as_secs().saturating_sub(timestamp);

        Ok(elapsed < self.timeout.as_secs())
    }

    /// Get the time elapsed since last heartbeat
    ///
    /// Returns the elapsed time in seconds.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if unable to read heartbeat.
    pub fn elapsed(&self) -> Result<u64, SpawnError> {
        if !self.heartbeat_path.exists() {
            return Ok(u64::MAX);
        }

        let content = fs::read_to_string(&self.heartbeat_path).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to read heartbeat: {e}"),
            }
        })?;

        let timestamp: u64 =
            content
                .trim()
                .parse()
                .map_err(|e| SpawnError::WorkspaceCreationFailed {
                    reason: format!("Invalid heartbeat timestamp: {e}"),
                })?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to get current time: {e}"),
            }
        })?;

        Ok(now.as_secs().saturating_sub(timestamp))
    }

    /// Clean up heartbeat file
    ///
    /// Removes the heartbeat file.
    ///
    /// # Errors
    /// Returns `SpawnError::CleanupFailed` if cleanup fails.
    pub fn cleanup(&self) -> Result<(), SpawnError> {
        if self.heartbeat_path.exists() {
            fs::remove_file(&self.heartbeat_path).map_err(|e| SpawnError::CleanupFailed {
                reason: format!("Failed to remove heartbeat file: {e}"),
            })?;
        }
        Ok(())
    }
}

impl Drop for HeartbeatMonitor {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Write heartbeat update instruction for agents
///
/// This function writes instructions to the workspace telling agents
/// how to update their heartbeat.
///
/// # Errors
/// Returns `SpawnError::WorkspaceCreationFailed` if write fails.
pub fn write_heartbeat_instructions(workspace_path: &Path) -> Result<(), SpawnError> {
    let instructions = format!(
        r"# Heartbeat Instructions

Your agent should update the heartbeat file regularly to show it's alive.

## Heartbeat File Location
`{HEARTBEAT_FILE}`

## Update Interval
Every {DEFAULT_HEARTBEAT_INTERVAL} seconds

## How to Update
```bash
echo $(date +%s) > {HEARTBEAT_FILE}
```

## Example (for long-running agents)
```bash
# In your agent loop, call:
update_heartbeat() {{
    echo $(date +%s) > {HEARTBEAT_FILE}
}}

# Call every {DEFAULT_HEARTBEAT_INTERVAL} seconds
while true; do
    update_heartbeat
    # ... do your work ...
    sleep {DEFAULT_HEARTBEAT_INTERVAL}
done
```
"
    );

    let instructions_path = workspace_path.join(".zjj/HEARTBEAT.md");

    let heartbeat_dir =
        instructions_path
            .parent()
            .ok_or_else(|| SpawnError::WorkspaceCreationFailed {
                reason: "Invalid heartbeat instructions path".to_string(),
            })?;

    fs::create_dir_all(heartbeat_dir).map_err(|e| SpawnError::WorkspaceCreationFailed {
        reason: format!("Failed to create heartbeat directory: {e}"),
    })?;

    fs::write(&instructions_path, instructions).map_err(|e| {
        SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to write heartbeat instructions: {e}"),
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_heartbeat_monitor_lifecycle() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path();

        let monitor = HeartbeatMonitor::with_defaults(workspace);

        // Initialize should create heartbeat file
        monitor
            .initialize()
            .expect("Failed to initialize heartbeat");
        assert!(monitor.heartbeat_path.exists());

        // Should be alive after initialization
        assert!(monitor.is_alive().expect("Failed to check alive"));

        // Update heartbeat
        monitor.update().expect("Failed to update heartbeat");
        assert!(monitor.is_alive().expect("Failed to check alive"));

        // Elapsed should be small
        let elapsed = monitor.elapsed().expect("Failed to get elapsed");
        assert!(elapsed < 5);
    }

    #[test]
    fn test_heartbeat_monitor_timeout() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path();

        let monitor =
            HeartbeatMonitor::new(workspace, Duration::from_secs(1), Duration::from_secs(2));

        monitor
            .initialize()
            .expect("Failed to initialize heartbeat");

        // Should be alive initially
        assert!(monitor.is_alive().expect("Failed to check alive"));

        // Write old timestamp to simulate timeout
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get time")
            .as_secs()
            - 10;

        fs::write(&monitor.heartbeat_path, old_timestamp.to_string())
            .expect("Failed to write old timestamp");

        // Should not be alive after timeout
        assert!(!monitor.is_alive().expect("Failed to check alive"));
    }

    #[test]
    fn test_write_heartbeat_instructions() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path();

        write_heartbeat_instructions(workspace).expect("Failed to write instructions");

        let instructions_path = workspace.join(".zjj/HEARTBEAT.md");
        assert!(instructions_path.exists());

        let content = fs::read_to_string(&instructions_path).expect("Failed to read instructions");
        assert!(content.contains("Heartbeat Instructions"));
        assert!(content.contains(HEARTBEAT_FILE));
    }
}
