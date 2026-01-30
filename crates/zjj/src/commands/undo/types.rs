//! Types for undo command
//!
//! This module provides zero-panic, type-safe types for undoing operations.

use std::fmt;

use serde::{Deserialize, Serialize};
use zjj_core::OutputFormat;

/// CLI arguments for undo command (parsed in main.rs)
#[derive(Debug, Clone)]
pub struct UndoArgs {
    /// Preview without executing
    pub dry_run: bool,

    /// Output format
    pub format: OutputFormat,
}

impl UndoArgs {
    /// Convert to `UndoOptions`
    pub const fn to_options(&self) -> UndoOptions {
        UndoOptions {
            dry_run: self.dry_run,
            format: self.format,
        }
    }
}

/// Internal options for undo command
#[derive(Debug, Clone)]
pub struct UndoOptions {
    pub dry_run: bool,
    pub format: OutputFormat,
}

/// Output from undo command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UndoOutput {
    pub session_name: String,
    pub dry_run: bool,
    pub commit_id: String,
    pub pushed_to_remote: bool,
    pub error: Option<String>,
}

/// Undo operation error (zero-panic, no unwraps)
#[derive(Debug, Clone)]
pub enum UndoError {
    NotInMain { workspace: String },
    NoUndoHistory,
    AlreadyPushedToRemote { commit_id: String },
    WorkspaceExpired { session_name: String, hours: u64 },
    RebaseFailed { reason: String },
    JjCommandFailed { command: String, reason: String },
    ReadUndoLogFailed { reason: String },
    WriteUndoLogFailed { reason: String },
    SerializationError { reason: String },
    InvalidState { reason: String },
    SystemTimeError { reason: String },
}

impl fmt::Display for UndoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInMain { workspace } => {
                write!(f, "Not in main branch (currently in workspace: {workspace}). Switch to main first: 'zjj focus main'")
            }
            Self::NoUndoHistory => write!(f, "No undo history found. Cannot undo."),
            Self::AlreadyPushedToRemote { commit_id } => {
                write!(
                    f,
                    "Cannot undo: commit {commit_id} has already been pushed to remote"
                )
            }
            Self::WorkspaceExpired {
                session_name,
                hours,
            } => {
                write!(
                    f,
                    "Cannot undo: workspace '{session_name}' expired after {hours} hours"
                )
            }
            Self::RebaseFailed { reason } => write!(f, "Failed to revert merge: {reason}"),
            Self::JjCommandFailed { command, reason } => {
                write!(f, "JJ command '{command}' failed: {reason}")
            }
            Self::ReadUndoLogFailed { reason } => {
                write!(f, "Failed to read undo log: {reason}")
            }
            Self::WriteUndoLogFailed { reason } => {
                write!(f, "Failed to write undo log: {reason}")
            }
            Self::SerializationError { reason } => {
                write!(f, "Failed to serialize undo entry: {reason}")
            }
            Self::InvalidState { reason } => write!(f, "Invalid state: {reason}"),
            Self::SystemTimeError { reason } => {
                write!(f, "System time error: {reason}")
            }
        }
    }
}

impl std::error::Error for UndoError {}

impl UndoError {
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::NotInMain { .. } => "NOT_IN_MAIN",
            Self::NoUndoHistory => "NO_UNDO_HISTORY",
            Self::AlreadyPushedToRemote { .. } => "ALREADY_PUSHED_TO_REMOTE",
            Self::WorkspaceExpired { .. } => "WORKSPACE_EXPIRED",
            Self::RebaseFailed { .. } => "REBASE_FAILED",
            Self::JjCommandFailed { .. } => "JJ_COMMAND_FAILED",
            Self::ReadUndoLogFailed { .. } => "READ_UNDO_LOG_FAILED",
            Self::WriteUndoLogFailed { .. } => "WRITE_UNDO_LOG_FAILED",
            Self::SerializationError { .. } => "SERIALIZATION_ERROR",
            Self::InvalidState { .. } => "INVALID_STATE",
            Self::SystemTimeError { .. } => "SYSTEM_TIME_ERROR",
        }
    }
}

/// Exit codes for undo command
#[derive(Debug, Clone, Copy)]
pub enum UndoExitCode {
    Success = 0,
    AlreadyPushed = 1,
    NoHistory = 2,
    InvalidState = 3,
    OtherError = 4,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_args_to_options() {
        let args = UndoArgs {
            dry_run: true,
            format: OutputFormat::Json,
        };

        let opts = args.to_options();

        assert!(opts.dry_run);
        assert!(matches!(opts.format, OutputFormat::Json));
    }

    #[test]
    fn test_undo_error_codes() {
        let err = UndoError::NoUndoHistory;
        assert_eq!(err.error_code(), "NO_UNDO_HISTORY");

        let err2 = UndoError::AlreadyPushedToRemote {
            commit_id: "abc123".to_string(),
        };
        assert_eq!(err2.error_code(), "ALREADY_PUSHED_TO_REMOTE");
    }

    #[test]
    fn test_undo_output_serialization() {
        let output = UndoOutput {
            session_name: "test-session".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
            error: None,
        };

        assert_eq!(output.session_name, "test-session");
        assert_eq!(output.commit_id, "abc123");
        assert!(!output.dry_run);
    }
}
