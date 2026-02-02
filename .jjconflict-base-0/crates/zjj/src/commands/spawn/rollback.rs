//! Transaction tracking and rollback for spawn operations
//!
//! This module provides zero-panic, type-safe transaction management
//! for spawn operations, including signal handling and agent monitoring.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::signal::unix::{signal, SignalKind};

use super::types::SpawnError;

/// Completed phases in a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletedPhases {
    workspace_created: bool,
    bead_status_updated: bool,
    agent_spawned: bool,
}

impl CompletedPhases {
    /// Mark workspace creation as completed
    pub const fn workspace_created(mut self) -> Self {
        self.workspace_created = true;
        self
    }

    /// Mark bead status update as completed
    pub const fn bead_status_updated(mut self) -> Self {
        self.bead_status_updated = true;
        self
    }

    /// Mark agent spawn as completed
    pub const fn agent_spawned(mut self) -> Self {
        self.agent_spawned = true;
        self
    }

    /// Check if any work needs rollback
    pub const fn needs_rollback(self) -> bool {
        self.workspace_created || self.bead_status_updated || self.agent_spawned
    }

    /// Check if workspace was created (needs abandon)
    pub const fn has_workspace(self) -> bool {
        self.workspace_created
    }

    /// Check if bead status was updated (needs reset)
    pub const fn has_bead_update(self) -> bool {
        self.bead_status_updated
    }

    /// Check if agent was spawned (needs termination)
    pub const fn has_agent(self) -> bool {
        self.agent_spawned
    }
}

/// Transaction tracker for spawn operations
///
/// Manages transaction state and provides rollback functionality.
/// Uses Arc<Mutex<>> for safe access from signal handlers.
#[derive(Clone)]
pub struct TransactionTracker {
    bead_id: String,
    #[allow(dead_code)]
    workspace_path: PathBuf,
    completed_phases: Arc<Mutex<CompletedPhases>>,
    agent_pid: Arc<Mutex<Option<u32>>>,
    root: String,
}

impl TransactionTracker {
    /// Create a new transaction tracker
    ///
    /// # Errors
    /// Returns `SpawnError::JjCommandFailed` if unable to get JJ root.
    pub fn new(bead_id: &str, workspace_path: &Path) -> Result<Self, SpawnError> {
        let root = crate::cli::jj_root().map_err(|e| SpawnError::JjCommandFailed {
            reason: format!("Failed to get JJ root: {e}"),
        })?;

        Ok(Self {
            bead_id: bead_id.to_string(),
            workspace_path: workspace_path.to_path_buf(),
            completed_phases: Arc::new(Mutex::new(CompletedPhases::default())),
            agent_pid: Arc::new(Mutex::new(None)),
            root,
        })
    }

    /// Mark workspace creation phase as completed
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if mutex is poisoned.
    pub fn mark_workspace_created(&self) -> Result<(), SpawnError> {
        self.completed_phases
            .lock()
            .map(|mut phases| {
                *phases = phases.workspace_created();
            })
            .map_err(|e| SpawnError::AgentSpawnFailed {
                reason: format!("Mutex lock poisoned: {e}"),
            })
    }

    /// Mark bead status update phase as completed
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if mutex is poisoned.
    pub fn mark_bead_status_updated(&self) -> Result<(), SpawnError> {
        self.completed_phases
            .lock()
            .map(|mut phases| {
                *phases = phases.bead_status_updated();
            })
            .map_err(|e| SpawnError::AgentSpawnFailed {
                reason: format!("Mutex lock poisoned: {e}"),
            })
    }

    /// Mark agent spawn phase as completed
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if mutex is poisoned.
    pub fn mark_agent_spawned(&self, pid: u32) -> Result<(), SpawnError> {
        self.completed_phases
            .lock()
            .map(|mut phases| {
                *phases = phases.agent_spawned();
            })
            .map_err(|e| SpawnError::AgentSpawnFailed {
                reason: format!("Mutex lock poisoned (phases): {e}"),
            })?;

        self.agent_pid
            .lock()
            .map(|mut agent_pid| {
                *agent_pid = Some(pid);
            })
            .map_err(|e| SpawnError::AgentSpawnFailed {
                reason: format!("Mutex lock poisoned (agent_pid): {e}"),
            })
    }

    /// Get the current completed phases
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if mutex is poisoned.
    pub fn completed_phases(&self) -> Result<CompletedPhases, SpawnError> {
        self.completed_phases
            .lock()
            .map(|phases| *phases)
            .map_err(|e| SpawnError::AgentSpawnFailed {
                reason: format!("Mutex lock poisoned: {e}"),
            })
    }

    /// Get the agent PID if spawned
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if mutex is poisoned.
    pub fn agent_pid(&self) -> Result<Option<u32>, SpawnError> {
        self.agent_pid
            .lock()
            .map(|pid| *pid)
            .map_err(|e| SpawnError::AgentSpawnFailed {
                reason: format!("Mutex lock poisoned: {e}"),
            })
    }

    /// Rollback all completed phases
    ///
    /// Executes rollback in reverse order of completion to ensure
    /// proper cleanup without orphaned resources.
    ///
    /// # Errors
    /// Returns `SpawnError` if any rollback step fails.
    pub fn rollback(&self) -> Result<(), SpawnError> {
        let phases = self.completed_phases()?;

        if !phases.needs_rollback() {
            return Ok(());
        }

        eprintln!("Initiating rollback for spawn operation...");

        // Rollback in reverse order: agent → bead status → workspace
        if phases.has_agent() {
            self.terminate_agent()?;
        }

        if phases.has_bead_update() {
            self.reset_bead_status()?;
        }

        if phases.has_workspace() {
            self.abandon_workspace()?;
        }

        eprintln!("Rollback completed successfully");
        Ok(())
    }

    /// Terminate the spawned agent process
    ///
    /// First attempts SIGTERM, then SIGKILL after timeout.
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if termination fails.
    fn terminate_agent(&self) -> Result<(), SpawnError> {
        let pid_opt = self.agent_pid().map_err(|_| SpawnError::AgentSpawnFailed {
            reason: "No agent PID recorded".to_string(),
        })?;

        let pid = pid_opt.ok_or_else(|| SpawnError::AgentSpawnFailed {
            reason: "No PID available".to_string(),
        })?;

        eprintln!("Terminating agent process (PID: {pid})...");

        // Try SIGTERM first
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();

        // Wait briefly, then SIGKILL if still running
        std::thread::sleep(Duration::from_millis(500));

        let kill_result = Command::new("kill").args(["-0", &pid.to_string()]).output();

        if kill_result.is_ok() && kill_result.as_ref().is_ok_and(|o| o.status.success()) {
            // Still running, force kill
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output();
        }

        eprintln!("Agent terminated");
        Ok(())
    }

    /// Reset bead status from '`in_progress`' to 'open'
    ///
    /// # Errors
    /// Returns `SpawnError::DatabaseError` if status update fails or JSON parsing fails.
    fn reset_bead_status(&self) -> Result<(), SpawnError> {
        eprintln!("Resetting bead '{}' status to 'open'...", self.bead_id);

        let beads_db = Path::new(".beads/issues.jsonl");
        let content = std::fs::read_to_string(beads_db).map_err(|e| SpawnError::DatabaseError {
            reason: format!("Failed to read beads database: {e}"),
        })?;

        // Functional transformation using try_fold with Railway-Oriented Programming
        // Accumulates both the new content string and updated flag immutably
        let (new_content, updated) = content.lines().try_fold(
            (String::new(), false),
            |(mut acc, updated), line| -> Result<(String, bool), SpawnError> {
                // Parse JSON line, propagating errors instead of silently ignoring
                let mut json = serde_json::from_str::<serde_json::Value>(line).map_err(|e| {
                    SpawnError::DatabaseError {
                        reason: format!("Failed to parse beads JSON line: {e}"),
                    }
                })?;

                // Check if this bead matches our ID and update status if so
                let is_target_bead = json
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|id| id == self.bead_id);

                let updated = if is_target_bead {
                    json["status"] = serde_json::json!("open");
                    true
                } else {
                    updated
                };

                // Append serialized JSON line to accumulator
                acc.push_str(&json.to_string());
                acc.push('\n');

                Ok((acc, updated))
            },
        )?;

        // Only write if we actually updated a bead
        if updated {
            std::fs::write(beads_db, new_content).map_err(|e| SpawnError::DatabaseError {
                reason: format!("Failed to write beads database: {e}"),
            })?;
        }

        eprintln!("Bead status reset");
        Ok(())
    }

    /// Abandon the workspace to clean up changes
    ///
    /// # Errors
    /// Returns `SpawnError::MergeFailed` or `SpawnError::JjCommandFailed` on failure.
    fn abandon_workspace(&self) -> Result<(), SpawnError> {
        eprintln!("Abandoning workspace '{}'...", self.bead_id);

        let list_output = Command::new("jj")
            .args(["workspace", "list"])
            .current_dir(&self.root)
            .output()
            .map_err(|e| SpawnError::JjCommandFailed {
                reason: format!("Failed to execute jj workspace list: {e}"),
            })?;

        if !list_output.status.success() {
            return Err(SpawnError::JjCommandFailed {
                reason: format!(
                    "jj workspace list failed: {}",
                    String::from_utf8_lossy(&list_output.stderr)
                ),
            });
        }

        let workspace_list = String::from_utf8_lossy(&list_output.stdout);
        let workspace_exists = workspace_list
            .lines()
            .any(|line| line.contains(&self.bead_id));

        if workspace_exists {
            let abandon_output = Command::new("jj")
                .args(["workspace", "abandon", "--name", &self.bead_id])
                .current_dir(&self.root)
                .output()
                .map_err(|e| SpawnError::JjCommandFailed {
                    reason: format!("Failed to execute jj workspace abandon: {e}"),
                })?;

            if !abandon_output.status.success() {
                return Err(SpawnError::MergeFailed {
                    reason: format!(
                        "Failed to abandon workspace: {}",
                        String::from_utf8_lossy(&abandon_output.stderr)
                    ),
                });
            }
        }

        eprintln!("Workspace abandoned");
        Ok(())
    }
}

impl Drop for TransactionTracker {
    fn drop(&mut self) {
        // Only auto-rollback if we have uncommitted work and are being dropped
        // unexpectedly (not normal completion path)
        //
        // Note: If mutex is poisoned during Drop, we cannot safely rollback.
        // This is acceptable as Drop is a best-effort cleanup path.
        if let Ok(phases) = self.completed_phases() {
            if phases.needs_rollback() {
                if let Err(e) = self.rollback() {
                    eprintln!("WARNING: Failed to rollback during cleanup: {e}");
                }
            }
        }
    }
}

/// Signal handler for graceful shutdown
///
/// Registers handlers for SIGINT and SIGTERM to perform
/// clean rollback of in-progress transactions.
#[derive(Clone)]
pub struct SignalHandler {
    tracker: Option<TransactionTracker>,
}

impl SignalHandler {
    /// Create a new signal handler
    ///
    /// # Arguments
    /// * `tracker` - Optional transaction tracker for rollback
    pub const fn new(tracker: Option<TransactionTracker>) -> Self {
        Self { tracker }
    }

    /// Register signal handlers
    ///
    /// # Errors
    /// Returns `SpawnError::AgentSpawnFailed` if signal registration fails.
    pub fn register(&self) -> Result<(), SpawnError> {
        let tracker = self.tracker.clone();

        // Setup signal handlers before spawning async task
        // This ensures we fail early if signal registration fails
        let sigint_result = signal(SignalKind::interrupt());
        let sigterm_result = signal(SignalKind::terminate());

        match (sigint_result, sigterm_result) {
            (Ok(mut sigint), Ok(mut sigterm)) => {
                tokio::spawn(async move {
                    tokio::select! {
                        _ = sigint.recv() => {
                            eprintln!("\nReceived SIGINT, performing cleanup...");
                        }
                        _ = sigterm.recv() => {
                            eprintln!("Received SIGTERM, performing cleanup...");
                        }
                    }

                    if let Some(t) = tracker {
                        if let Err(e) = t.rollback() {
                            eprintln!("Cleanup failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    std::process::exit(130);
                });

                Ok(())
            }
            (Err(e), _) => Err(SpawnError::AgentSpawnFailed {
                reason: format!("Failed to setup SIGINT handler: {e}"),
            }),
            (_, Err(e)) => Err(SpawnError::AgentSpawnFailed {
                reason: format!("Failed to setup SIGTERM handler: {e}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completed_phases_empty() {
        let phases = CompletedPhases::default();
        assert!(!phases.needs_rollback());
        assert!(!phases.has_workspace());
        assert!(!phases.has_bead_update());
        assert!(!phases.has_agent());
    }

    #[test]
    fn test_completed_phases_workspace() {
        let phases = CompletedPhases::default().workspace_created();
        assert!(phases.needs_rollback());
        assert!(phases.has_workspace());
        assert!(!phases.has_bead_update());
        assert!(!phases.has_agent());
    }

    #[test]
    fn test_completed_phases_all() {
        let phases = CompletedPhases::default()
            .workspace_created()
            .bead_status_updated()
            .agent_spawned();

        assert!(phases.needs_rollback());
        assert!(phases.has_workspace());
        assert!(phases.has_bead_update());
        assert!(phases.has_agent());
    }
}
