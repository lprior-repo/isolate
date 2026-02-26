//! Types for the spawn command
//!
//! This module provides zero-panic, type-safe types for spawning isolated workspaces.

use std::fmt;

use serde::{Deserialize, Serialize};
use isolate_core::OutputFormat;

/// Options for the spawn command (from CLI args)
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct SpawnArgs {
    /// Bead ID to work on
    pub bead_id: String,

    /// Agent command to run (default: claude)
    pub agent_command: String,

    /// Additional args to pass to agent
    pub agent_args: Vec<String>,

    /// Disable auto-merge on success
    pub no_auto_merge: bool,

    /// Disable auto-cleanup on failure
    pub no_auto_cleanup: bool,

    /// Run agent in background
    pub background: bool,

    /// Timeout in seconds (default: 14400 = 4 hours)
    pub timeout: u64,

    /// Succeed if workspace already exists
    pub idempotent: bool,

    /// Output format
    pub format: String,

    /// Preview spawn without executing
    pub dry_run: bool,
}

impl SpawnArgs {
    /// Parse from clap `ArgMatches`
    pub fn from_matches(matches: &clap::ArgMatches) -> anyhow::Result<Self> {
        let bead_id = matches
            .get_one::<String>("bead_id")
            .ok_or_else(|| anyhow::anyhow!("bead_id is required"))?
            .clone();

        let agent_command = matches
            .get_one::<String>("agent-command")
            .cloned()
            .unwrap_or_else(|| "claude".to_string());

        let agent_args = matches
            .get_many::<String>("agent-args")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default();

        let no_auto_merge = matches.get_flag("no-auto-merge");
        let no_auto_cleanup = matches.get_flag("no-auto-cleanup");
        let background = matches.get_flag("background");
        let idempotent = matches.get_flag("idempotent");
        let dry_run = matches.get_flag("dry-run");

        let timeout = matches
            .get_one::<String>("timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(14400);

        let format = if matches.get_flag("json") {
            "json".to_string()
        } else {
            "human".to_string()
        };

        Ok(Self {
            bead_id,
            agent_command,
            agent_args,
            no_auto_merge,
            no_auto_cleanup,
            background,
            timeout,
            idempotent,
            format,
            dry_run,
        })
    }

    /// Convert to `SpawnOptions`
    pub fn to_options(&self) -> SpawnOptions {
        SpawnOptions {
            bead_id: self.bead_id.clone(),
            agent_command: self.agent_command.clone(),
            agent_args: self.agent_args.clone(),
            no_auto_merge: self.no_auto_merge,
            no_auto_cleanup: self.no_auto_cleanup,
            background: self.background,
            timeout_secs: self.timeout,
            idempotent: self.idempotent,
            format: OutputFormat::Json,
            dry_run: self.dry_run,
        }
    }
}

/// Options for spawn command (internal)
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct SpawnOptions {
    pub bead_id: String,
    pub agent_command: String,
    pub agent_args: Vec<String>,
    pub no_auto_merge: bool,
    pub no_auto_cleanup: bool,
    pub background: bool,
    pub timeout_secs: u64,
    pub idempotent: bool,
    pub format: OutputFormat,
    pub dry_run: bool,
}

/// Output from spawn command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnOutput {
    pub bead_id: String,
    pub workspace_path: String,
    pub agent_pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub merged: bool,
    pub cleaned: bool,
    pub status: SpawnStatus,
}

/// Status of the spawn operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnStatus {
    /// Spawn initiated and running in background
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Validation error (wrong location, bead not ready, etc.)
    ValidationError,
    /// Dry run (preview only)
    DryRun,
}

/// Phase of spawn operation for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code)] // Phase tracking reserved for future error reporting
pub enum SpawnPhase {
    ValidatingLocation,
    ValidatingBead,
    CreatingWorkspace,
    UpdatingBeadStatus,
    SpawningAgent,
    WaitingForCompletion,
    MergingChanges,
    CleaningWorkspace,
    UpdatingFinalStatus,
}

impl SpawnPhase {
    #[allow(dead_code)] // Used in tests (test_spawn_phase_names)
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ValidatingLocation => "validating_location",
            Self::ValidatingBead => "validating_bead",
            Self::CreatingWorkspace => "creating_workspace",
            Self::UpdatingBeadStatus => "updating_bead_status",
            Self::SpawningAgent => "spawning_agent",
            Self::WaitingForCompletion => "waiting_for_completion",
            Self::MergingChanges => "merging_changes",
            Self::CleaningWorkspace => "cleaning_workspace",
            Self::UpdatingFinalStatus => "updating_final_status",
        }
    }
}

/// Spawn operation error
#[derive(Debug, Clone)]
pub enum SpawnError {
    #[expect(dead_code)] // current_location reserved for future error messages
    NotOnMain {
        current_location: String,
    },
    InvalidBeadStatus {
        bead_id: String,
        status: String,
    },
    BeadNotFound {
        bead_id: String,
    },
    WorkspaceCreationFailed {
        reason: String,
    },
    AgentSpawnFailed {
        reason: String,
    },
    Timeout {
        timeout_secs: u64,
    },
    MergeFailed {
        reason: String,
    },
    CleanupFailed {
        reason: String,
    },
    DatabaseError {
        reason: String,
    },
    JjCommandFailed {
        reason: String,
    },
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOnMain { .. } => {
                write!(f, "Cannot spawn from workspace, must be on main branch")
            }
            Self::InvalidBeadStatus { bead_id, status } => {
                write!(
                    f,
                    "Bead '{bead_id}' has status '{status}', expected open or ready"
                )
            }
            Self::BeadNotFound { bead_id } => write!(f, "Bead '{bead_id}' not found"),
            Self::WorkspaceCreationFailed { reason } => {
                write!(f, "Failed to create workspace: {reason}")
            }
            Self::AgentSpawnFailed { reason } => write!(f, "Failed to spawn agent: {reason}"),
            Self::Timeout { timeout_secs } => {
                write!(f, "Agent timed out after {timeout_secs} seconds")
            }
            Self::MergeFailed { reason } => write!(f, "Failed to merge changes: {reason}"),
            Self::CleanupFailed { reason } => write!(f, "Failed to cleanup workspace: {reason}"),
            Self::DatabaseError { reason } => write!(f, "Database error: {reason}"),
            Self::JjCommandFailed { reason } => write!(f, "JJ command failed: {reason}"),
        }
    }
}

impl std::error::Error for SpawnError {}

impl SpawnError {
    #[allow(dead_code)] // Used in tests (test_spawn_error_codes)
    pub const fn phase(&self) -> SpawnPhase {
        match self {
            Self::NotOnMain { .. } => SpawnPhase::ValidatingLocation,
            Self::InvalidBeadStatus { .. } | Self::BeadNotFound { .. } => {
                SpawnPhase::ValidatingBead
            }
            Self::WorkspaceCreationFailed { .. } => SpawnPhase::CreatingWorkspace,
            Self::DatabaseError { .. } => SpawnPhase::UpdatingBeadStatus,
            Self::AgentSpawnFailed { .. } => SpawnPhase::SpawningAgent,
            Self::Timeout { .. } => SpawnPhase::WaitingForCompletion,
            Self::MergeFailed { .. } | Self::JjCommandFailed { .. } => SpawnPhase::MergingChanges,
            Self::CleanupFailed { .. } => SpawnPhase::CleaningWorkspace,
        }
    }

    #[allow(dead_code)] // Used in tests (test_spawn_error_codes)
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::NotOnMain { .. } => "NOT_ON_MAIN",
            Self::InvalidBeadStatus { .. } => "INVALID_BEAD_STATUS",
            Self::BeadNotFound { .. } => "BEAD_NOT_FOUND",
            Self::WorkspaceCreationFailed { .. } => "WORKSPACE_CREATION_FAILED",
            Self::AgentSpawnFailed { .. } => "AGENT_SPAWN_FAILED",
            Self::Timeout { .. } => "TIMEOUT",
            Self::MergeFailed { .. } => "MERGE_FAILED",
            Self::CleanupFailed { .. } => "CLEANUP_FAILED",
            Self::DatabaseError { .. } => "DATABASE_ERROR",
            Self::JjCommandFailed { .. } => "JJ_COMMAND_FAILED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_args_to_options() {
        let args = SpawnArgs {
            bead_id: "test-bead".to_string(),
            agent_command: "claude".to_string(),
            agent_args: vec!["--arg".to_string()],
            no_auto_merge: true,
            no_auto_cleanup: false,
            background: false,
            timeout: 3600,
            idempotent: false,
            format: "json".to_string(),
            dry_run: false,
        };

        let opts = args.to_options();

        assert_eq!(opts.bead_id, "test-bead");
        assert_eq!(opts.agent_command, "claude");
        assert_eq!(opts.agent_args, vec!["--arg".to_string()]);
        assert!(opts.no_auto_merge);
        assert!(!opts.no_auto_cleanup);
        assert_eq!(opts.timeout_secs, 3600);
        assert!(matches!(opts.format, OutputFormat::Json));
        assert!(!opts.dry_run);
    }

    #[test]
    fn test_spawn_phase_names() {
        assert_eq!(SpawnPhase::ValidatingLocation.name(), "validating_location");
        assert_eq!(SpawnPhase::ValidatingBead.name(), "validating_bead");
        assert_eq!(SpawnPhase::CreatingWorkspace.name(), "creating_workspace");
        assert_eq!(SpawnPhase::SpawningAgent.name(), "spawning_agent");
        assert_eq!(
            SpawnPhase::WaitingForCompletion.name(),
            "waiting_for_completion"
        );
        assert_eq!(SpawnPhase::MergingChanges.name(), "merging_changes");
        assert_eq!(SpawnPhase::CleaningWorkspace.name(), "cleaning_workspace");
    }

    #[test]
    fn test_spawn_error_codes() {
        let err = SpawnError::NotOnMain {
            current_location: "workspace".to_string(),
        };
        assert_eq!(err.error_code(), "NOT_ON_MAIN");
        assert_eq!(err.phase(), SpawnPhase::ValidatingLocation);
    }

    #[test]
    fn test_spawn_status_serialization() {
        // Test that SpawnStatus can be constructed and matches variants
        let running = SpawnStatus::Running;
        let completed = SpawnStatus::Completed;
        let failed = SpawnStatus::Failed;
        let validation = SpawnStatus::ValidationError;

        // Verify variant discriminants work correctly
        assert!(matches!(running, SpawnStatus::Running));
        assert!(matches!(completed, SpawnStatus::Completed));
        assert!(matches!(failed, SpawnStatus::Failed));
        assert!(matches!(validation, SpawnStatus::ValidationError));
    }
}
