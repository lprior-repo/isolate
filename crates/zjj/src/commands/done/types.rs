//! Types for the done command
//!
//! This module provides zero-panic, type-safe types for completing work in a workspace.

use serde::{Deserialize, Serialize};
use std::fmt;

use zjj_core::OutputFormat;

/// CLI arguments for the done command (parsed in main.rs)
#[derive(Debug, Clone)]
pub struct DoneArgs {
    /// Commit message (auto-generated if not provided)
    pub message: Option<String>,

    /// Keep workspace after merge
    pub keep_workspace: bool,

    /// Squash all commits into one
    pub squash: bool,

    /// Preview without executing
    pub dry_run: bool,

    /// Skip bead status update
    pub no_bead_update: bool,

    /// Output format
    pub format: String,
}

impl DoneArgs {
    /// Convert to DoneOptions
    pub fn to_options(&self) -> DoneOptions {
        DoneOptions {
            message: self.message.clone(),
            keep_workspace: self.keep_workspace,
            squash: self.squash,
            dry_run: self.dry_run,
            no_bead_update: self.no_bead_update,
            format: if self.format == "json" {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
        }
    }
}

/// Internal options for the done command
#[derive(Debug, Clone)]
pub struct DoneOptions {
    pub message: Option<String>,
    pub keep_workspace: bool,
    pub squash: bool,
    pub dry_run: bool,
    pub no_bead_update: bool,
    pub format: OutputFormat,
}

/// Output from the done command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoneOutput {
    pub workspace_name: String,
    pub bead_id: Option<String>,
    pub files_committed: usize,
    pub commits_merged: usize,
    pub merged: bool,
    pub cleaned: bool,
    pub bead_closed: bool,
    pub dry_run: bool,
    pub preview: Option<DonePreview>,
    pub error: Option<String>,
}

impl Default for DoneOutput {
    fn default() -> Self {
        Self {
            workspace_name: String::new(),
            bead_id: None,
            files_committed: 0,
            commits_merged: 0,
            merged: false,
            cleaned: false,
            bead_closed: false,
            dry_run: false,
            preview: None,
            error: None,
        }
    }
}

/// Preview information for dry-run mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonePreview {
    pub uncommitted_files: Vec<String>,
    pub commits_to_merge: Vec<CommitInfo>,
    pub potential_conflicts: Vec<String>,
    pub bead_to_close: Option<String>,
    pub workspace_path: String,
}

/// Information about a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub change_id: String,
    pub commit_id: String,
    pub description: String,
    pub timestamp: String,
}

/// Phase of the done operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonePhase {
    ValidatingLocation,
    CheckingUncommitted,
    CommittingChanges,
    CheckingConflicts,
    MergingToMain,
    UpdatingBeadStatus,
    CleaningWorkspace,
    SwitchingToMain,
}

impl DonePhase {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ValidatingLocation => "validating_location",
            Self::CheckingUncommitted => "checking_uncommitted",
            Self::CommittingChanges => "committing_changes",
            Self::CheckingConflicts => "checking_conflicts",
            Self::MergingToMain => "merging_to_main",
            Self::UpdatingBeadStatus => "updating_bead_status",
            Self::CleaningWorkspace => "cleaning_workspace",
            Self::SwitchingToMain => "switching_to_main",
        }
    }
}

/// Done operation error (zero-panic, no unwraps)
#[derive(Debug, Clone)]
pub enum DoneError {
    NotInWorkspace { current_location: String },
    NotAJjRepo,
    WorkspaceNotFound { workspace_name: String },
    CommitFailed { reason: String },
    MergeConflict { conflicts: Vec<String> },
    MergeFailed { reason: String },
    CleanupFailed { reason: String },
    BeadUpdateFailed { reason: String },
    JjCommandFailed { command: String, reason: String },
    InvalidState { reason: String },
}

impl fmt::Display for DoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInWorkspace { current_location } => write!(
                f,
                "Not in a workspace (currently at: {}). Use 'zjj focus <name>' to switch to a workspace first.",
                current_location
            ),
            Self::NotAJjRepo => write!(f, "Not in a JJ repository. Run 'zjj init' first."),
            Self::WorkspaceNotFound { workspace_name } => {
                write!(f, "Workspace '{}' not found", workspace_name)
            }
            Self::CommitFailed { reason } => write!(f, "Failed to commit changes: {}", reason),
            Self::MergeConflict { conflicts } => {
                write!(f, "Merge conflicts detected: {}", conflicts.join(", "))
            }
            Self::MergeFailed { reason } => write!(f, "Failed to merge to main: {}", reason),
            Self::CleanupFailed { reason } => write!(f, "Failed to cleanup workspace: {}", reason),
            Self::BeadUpdateFailed { reason } => {
                write!(f, "Failed to update bead status: {}", reason)
            }
            Self::JjCommandFailed { command, reason } => {
                write!(f, "JJ command '{}' failed: {}", command, reason)
            }
            Self::InvalidState { reason } => write!(f, "Invalid state: {}", reason),
        }
    }
}

impl std::error::Error for DoneError {}

impl DoneError {
    pub fn phase(&self) -> DonePhase {
        match self {
            Self::NotInWorkspace { .. } | Self::NotAJjRepo => DonePhase::ValidatingLocation,
            Self::WorkspaceNotFound { .. } => DonePhase::ValidatingLocation,
            Self::CommitFailed { .. } => DonePhase::CommittingChanges,
            Self::MergeConflict { .. } | Self::MergeFailed { .. } => DonePhase::MergingToMain,
            Self::CleanupFailed { .. } => DonePhase::CleaningWorkspace,
            Self::BeadUpdateFailed { .. } => DonePhase::UpdatingBeadStatus,
            Self::JjCommandFailed { .. } => DonePhase::MergingToMain,
            Self::InvalidState { .. } => DonePhase::ValidatingLocation,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NotInWorkspace { .. } => "NOT_IN_WORKSPACE",
            Self::NotAJjRepo => "NOT_A_JJ_REPO",
            Self::WorkspaceNotFound { .. } => "WORKSPACE_NOT_FOUND",
            Self::CommitFailed { .. } => "COMMIT_FAILED",
            Self::MergeConflict { .. } => "MERGE_CONFLICT",
            Self::MergeFailed { .. } => "MERGE_FAILED",
            Self::CleanupFailed { .. } => "CLEANUP_FAILED",
            Self::BeadUpdateFailed { .. } => "BEAD_UPDATE_FAILED",
            Self::JjCommandFailed { .. } => "JJ_COMMAND_FAILED",
            Self::InvalidState { .. } => "INVALID_STATE",
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::MergeConflict { .. })
    }
}

/// Exit codes for the done command
#[derive(Debug, Clone, Copy)]
pub enum DoneExitCode {
    Success = 0,
    MergeConflict = 1,
    NotInWorkspace = 2,
    OtherError = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_done_args_to_options() {
        let args = DoneArgs {
            message: Some("test commit".to_string()),
            keep_workspace: true,
            squash: false,
            dry_run: false,
            no_bead_update: false,
            format: "json".to_string(),
        };

        let opts = args.to_options();

        assert_eq!(opts.message, Some("test commit".to_string()));
        assert!(opts.keep_workspace);
        assert!(!opts.squash);
        assert!(!opts.dry_run);
        assert!(matches!(opts.format, OutputFormat::Json));
    }

    #[test]
    fn test_done_phase_names() {
        assert_eq!(DonePhase::ValidatingLocation.name(), "validating_location");
        assert_eq!(DonePhase::CommittingChanges.name(), "committing_changes");
        assert_eq!(DonePhase::MergingToMain.name(), "merging_to_main");
    }

    #[test]
    fn test_done_error_codes() {
        let err = DoneError::NotInWorkspace {
            current_location: "main".to_string(),
        };
        assert_eq!(err.error_code(), "NOT_IN_WORKSPACE");
        assert_eq!(err.phase(), DonePhase::ValidatingLocation);
    }

    #[test]
    fn test_merge_conflict_is_recoverable() {
        let err = DoneError::MergeConflict {
            conflicts: vec!["file.txt".to_string()],
        };
        assert!(err.is_recoverable());

        let err2 = DoneError::CommitFailed {
            reason: "test".to_string(),
        };
        assert!(!err2.is_recoverable());
    }

    #[test]
    fn test_done_output_serialization() {
        let output = DoneOutput {
            workspace_name: "test-ws".to_string(),
            bead_id: Some("zjj-test".to_string()),
            files_committed: 2,
            commits_merged: 1,
            merged: true,
            cleaned: true,
            bead_closed: true,
            dry_run: false,
            preview: None,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"test-ws\""));
        assert!(json.contains("\"merged\":true"));
        assert!(json.contains("\"cleaned\":true"));
    }

    #[test]
    fn test_done_preview_serialization() {
        let preview = DonePreview {
            uncommitted_files: vec!["file.txt".to_string()],
            commits_to_merge: vec![CommitInfo {
                change_id: "abc123".to_string(),
                commit_id: "xyz789".to_string(),
                description: "test commit".to_string(),
                timestamp: "2025-01-26T00:00:00Z".to_string(),
            }],
            potential_conflicts: vec![],
            bead_to_close: Some("zjj-test".to_string()),
            workspace_path: "/path/to/workspace".to_string(),
        };

        let json = serde_json::to_string(&preview).unwrap();
        assert!(json.contains("\"file.txt\""));
        assert!(json.contains("\"zjj-test\""));
    }
}
