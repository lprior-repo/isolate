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

    /// List undo history without reverting
    pub list: bool,

    /// Output format
    pub format: OutputFormat,
}

impl UndoArgs {
    /// Convert to `UndoOptions`
    pub const fn to_options(&self) -> UndoOptions {
        UndoOptions {
            dry_run: self.dry_run,
            list: self.list,
            format: self.format,
        }
    }
}

/// Internal options for undo command
#[derive(Debug, Clone)]
pub struct UndoOptions {
    pub dry_run: bool,
    pub list: bool,
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
    MalformedUndoLog { reason: String },
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
            Self::MalformedUndoLog { reason } => {
                write!(f, "Malformed undo log: {reason}")
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
            Self::MalformedUndoLog { .. } => "MALFORMED_UNDO_LOG",
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
            list: false,
            format: OutputFormat::Json,
        };

        let opts = args.to_options();

        assert!(opts.dry_run);
        assert!(!opts.list);
        assert!(matches!(opts.format, OutputFormat::Json));
    }

    #[test]
    fn test_undo_args_with_list() {
        let args = UndoArgs {
            dry_run: false,
            list: true,
            format: OutputFormat::Human,
        };

        let opts = args.to_options();

        assert!(!opts.dry_run);
        assert!(opts.list);
        assert!(matches!(opts.format, OutputFormat::Human));
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

    // ── UndoError Display Tests ──────────────────────────────────────────

    #[test]
    fn test_undo_error_not_in_main_display() {
        let err = UndoError::NotInMain {
            workspace: "feature-auth".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("Not in main branch"));
        assert!(display.contains("feature-auth"));
    }

    #[test]
    fn test_undo_error_no_history_display() {
        let err = UndoError::NoUndoHistory;
        let display = format!("{err}");
        assert!(display.contains("No undo history"));
    }

    #[test]
    fn test_undo_error_already_pushed_display() {
        let err = UndoError::AlreadyPushedToRemote {
            commit_id: "abc123".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("Cannot undo"));
        assert!(display.contains("pushed to remote"));
    }

    #[test]
    fn test_undo_error_workspace_expired_display() {
        let err = UndoError::WorkspaceExpired {
            session_name: "old-session".to_string(),
            hours: 24,
        };
        let display = format!("{err}");
        assert!(display.contains("expired"));
        assert!(display.contains("old-session"));
        assert!(display.contains("24"));
    }

    #[test]
    fn test_undo_error_rebase_failed_display() {
        let err = UndoError::RebaseFailed {
            reason: "conflict".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("Failed to revert"));
        assert!(display.contains("conflict"));
    }

    // ── UndoError Code Tests ─────────────────────────────────────────────

    #[test]
    fn test_all_undo_error_codes() {
        assert_eq!(
            UndoError::NotInMain {
                workspace: String::new()
            }
            .error_code(),
            "NOT_IN_MAIN"
        );
        assert_eq!(UndoError::NoUndoHistory.error_code(), "NO_UNDO_HISTORY");
        assert_eq!(
            UndoError::AlreadyPushedToRemote {
                commit_id: String::new()
            }
            .error_code(),
            "ALREADY_PUSHED_TO_REMOTE"
        );
        assert_eq!(
            UndoError::WorkspaceExpired {
                session_name: String::new(),
                hours: 0
            }
            .error_code(),
            "WORKSPACE_EXPIRED"
        );
        assert_eq!(
            UndoError::RebaseFailed {
                reason: String::new()
            }
            .error_code(),
            "REBASE_FAILED"
        );
        assert_eq!(
            UndoError::JjCommandFailed {
                command: String::new(),
                reason: String::new()
            }
            .error_code(),
            "JJ_COMMAND_FAILED"
        );
        assert_eq!(
            UndoError::ReadUndoLogFailed {
                reason: String::new()
            }
            .error_code(),
            "READ_UNDO_LOG_FAILED"
        );
        assert_eq!(
            UndoError::WriteUndoLogFailed {
                reason: String::new()
            }
            .error_code(),
            "WRITE_UNDO_LOG_FAILED"
        );
        assert_eq!(
            UndoError::SerializationError {
                reason: String::new()
            }
            .error_code(),
            "SERIALIZATION_ERROR"
        );
        assert_eq!(
            UndoError::InvalidState {
                reason: String::new()
            }
            .error_code(),
            "INVALID_STATE"
        );
        assert_eq!(
            UndoError::SystemTimeError {
                reason: String::new()
            }
            .error_code(),
            "SYSTEM_TIME_ERROR"
        );
    }

    // ── UndoExitCode Tests ───────────────────────────────────────────────

    #[test]
    fn test_undo_exit_code_values() {
        assert_eq!(UndoExitCode::Success as i32, 0);
        assert_eq!(UndoExitCode::AlreadyPushed as i32, 1);
        assert_eq!(UndoExitCode::NoHistory as i32, 2);
        assert_eq!(UndoExitCode::InvalidState as i32, 3);
        assert_eq!(UndoExitCode::OtherError as i32, 4);
    }

    // ── UndoOutput Tests ─────────────────────────────────────────────────

    #[test]
    fn test_undo_output_default() {
        let output = UndoOutput::default();
        assert!(output.session_name.is_empty());
        assert!(output.commit_id.is_empty());
        assert!(!output.dry_run);
        assert!(!output.pushed_to_remote);
        assert!(output.error.is_none());
    }

    #[test]
    fn test_undo_output_with_error() {
        let output = UndoOutput {
            session_name: "test".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
            error: Some("failed to undo".to_string()),
        };
        assert!(output.error.is_some());
        assert_eq!(output.error, Some("failed to undo".to_string()));
    }

    #[test]
    fn test_undo_output_json_serialization() {
        let output = UndoOutput {
            session_name: "test-ws".to_string(),
            dry_run: true,
            commit_id: "xyz789".to_string(),
            pushed_to_remote: false,
            error: None,
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("test-ws"));
        assert!(json_str.contains("xyz789"));
        assert!(json_str.contains("\"dry_run\":true"));
    }
}
