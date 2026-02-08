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
    #[allow(dead_code)]
    workspace_path: PathBuf,
    heartbeat_path: PathBuf,
    #[allow(dead_code)]
    interval: Duration,
    #[allow(dead_code)]
    timeout: Duration,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace directory
    /// * `interval` - Heartbeat update interval
    /// * `timeout` - Timeout before agent is considered dead
    #[must_use]
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
    #[must_use]
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
    pub async fn initialize(&self) -> Result<(), SpawnError> {
        let heartbeat_dir =
            self.heartbeat_path
                .parent()
                .ok_or_else(|| SpawnError::WorkspaceCreationFailed {
                    reason: "Invalid heartbeat path".to_string(),
                })?;

        tokio::fs::create_dir_all(heartbeat_dir)
            .await
            .map_err(|e| SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to create heartbeat directory: {e}"),
            })?;

        self.update().await?;

        Ok(())
    }

    /// Update heartbeat timestamp
    ///
    /// Writes the current Unix timestamp to the heartbeat file.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if write fails.
    pub async fn update(&self) -> Result<(), SpawnError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
            SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to get current time: {e}"),
            }
        })?;

        let timestamp = now.as_secs().to_string();

        tokio::fs::write(&self.heartbeat_path, timestamp)
            .await
            .map_err(|e| SpawnError::WorkspaceCreationFailed {
                reason: format!("Failed to write heartbeat: {e}"),
            })?;

        Ok(())
    }

    /// Check if heartbeat is within timeout
    ///
    /// Returns `true` if heartbeat is recent, `false` if timeout exceeded.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if unable to read heartbeat.
    #[allow(dead_code)]
    pub async fn is_alive(&self) -> Result<bool, SpawnError> {
        match tokio::fs::try_exists(&self.heartbeat_path).await {
            Ok(true) => {
                let content = tokio::fs::read_to_string(&self.heartbeat_path)
                    .await
                    .map_err(|e| SpawnError::WorkspaceCreationFailed {
                        reason: format!("Failed to read heartbeat: {e}"),
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
            _ => Ok(false),
        }
    }

    /// Get the time elapsed since last heartbeat
    ///
    /// Returns the elapsed time in seconds.
    ///
    /// # Errors
    /// Returns `SpawnError::WorkspaceCreationFailed` if unable to read heartbeat.
    #[allow(dead_code)]
    pub async fn elapsed(&self) -> Result<u64, SpawnError> {
        match tokio::fs::try_exists(&self.heartbeat_path).await {
            Ok(true) => {
                let content = tokio::fs::read_to_string(&self.heartbeat_path)
                    .await
                    .map_err(|e| SpawnError::WorkspaceCreationFailed {
                        reason: format!("Failed to read heartbeat: {e}"),
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
            _ => Ok(u64::MAX),
        }
    }

    /// Clean up heartbeat file
    ///
    /// Removes the heartbeat file.
    ///
    /// # Errors
    /// Returns `SpawnError::CleanupFailed` if cleanup fails.
    pub async fn cleanup(&self) -> Result<(), SpawnError> {
        if matches!(tokio::fs::try_exists(&self.heartbeat_path).await, Ok(true)) {
            tokio::fs::remove_file(&self.heartbeat_path)
                .await
                .map_err(|e| SpawnError::CleanupFailed {
                    reason: format!("Failed to remove heartbeat file: {e}"),
                })?;
        }
        Ok(())
    }
}

// NOTE: Drop cannot be async, so we'll have to rely on manual cleanup or a background task if
// needed. But for now, we'll keep manual cleanup in spawn/mod.rs.

/// Write heartbeat update instruction for agents
///
/// This function writes instructions to the workspace telling agents
/// how to update their heartbeat.
///
/// # Errors
/// Returns `SpawnError::WorkspaceCreationFailed` if write fails.
pub async fn write_heartbeat_instructions(workspace_path: &Path) -> Result<(), SpawnError> {
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

    tokio::fs::create_dir_all(heartbeat_dir)
        .await
        .map_err(|e| SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to create heartbeat directory: {e}"),
        })?;

    tokio::fs::write(&instructions_path, instructions)
        .await
        .map_err(|e| SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to write heartbeat instructions: {e}"),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_heartbeat_monitor_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let workspace = temp.path();

        let monitor = HeartbeatMonitor::with_defaults(workspace);

        // Initialize should create heartbeat file
        monitor.initialize().await?;
        assert!(monitor.heartbeat_path.exists());

        // Should be alive after initialization
        assert!(monitor.is_alive().await?);

        // Update heartbeat
        monitor.update().await?;
        assert!(monitor.is_alive().await?);

        // Elapsed should be small
        let elapsed = monitor.elapsed().await?;
        assert!(elapsed < 5);
        Ok(())
    }

    #[tokio::test]
    async fn test_heartbeat_monitor_timeout() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let workspace = temp.path();

        let monitor =
            HeartbeatMonitor::new(workspace, Duration::from_secs(1), Duration::from_secs(2));

        monitor.initialize().await?;

        // Should be alive initially
        assert!(monitor.is_alive().await?);

        // Write old timestamp to simulate timeout
        let old_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() - 10;

        tokio::fs::write(&monitor.heartbeat_path, old_timestamp.to_string()).await?;

        // Should not be alive after timeout
        assert!(!monitor.is_alive().await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_write_heartbeat_instructions() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let workspace = temp.path();

        write_heartbeat_instructions(workspace).await?;

        let instructions_path = workspace.join(".zjj/HEARTBEAT.md");
        assert!(instructions_path.exists());

        let content = tokio::fs::read_to_string(&instructions_path).await?;
        assert!(content.contains("Heartbeat Instructions"));
        assert!(content.contains(HEARTBEAT_FILE));
        Ok(())
    }
}
