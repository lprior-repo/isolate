//! Types for revert command
//!
//! This module provides zero-panic, type-safe types for reverting operations.

use std::fmt;

use serde::{Deserialize, Serialize};
use zjj_core::OutputFormat;

/// CLI arguments for revert command (parsed in main.rs)
#[derive(Debug, Clone)]
pub struct RevertArgs {
    /// Session name to revert
    pub session_name: String,

    /// Preview without executing
    pub dry_run: bool,

    /// Output format
    pub format: OutputFormat,
}

impl RevertArgs {
    /// Convert to `RevertOptions`
    pub fn to_options(&self) -> RevertOptions {
        RevertOptions {
            session_name: self.session_name.clone(),
            dry_run: self.dry_run,
            format: self.format,
        }
    }
}

/// Internal options for revert command
#[derive(Debug, Clone)]
pub struct RevertOptions {
    pub session_name: String,
    pub dry_run: bool,
    pub format: OutputFormat,
}

/// Output from revert command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RevertOutput {
    pub session_name: String,
    pub dry_run: bool,
    pub commit_id: String,
    pub pushed_to_remote: bool,
    pub error: Option<String>,
}

/// Revert operation error (zero-panic, no unwraps)
#[derive(Debug, Clone)]
pub enum RevertError {
    NotInMain {
        workspace: String,
    },
    SessionNotFound {
        session_name: String,
    },
    #[allow(dead_code)]
    NoUndoHistory,
    AlreadyPushedToRemote {
        commit_id: String,
    },
    RebaseFailed {
        reason: String,
    },
    JjCommandFailed {
        command: String,
        reason: String,
    },
    ReadUndoLogFailed {
        reason: String,
    },
    WriteUndoLogFailed {
        reason: String,
    },
    SerializationError {
        reason: String,
    },
    InvalidState {
        reason: String,
    },
}

impl fmt::Display for RevertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInMain { workspace } => {
                write!(f, "Not in main branch (currently in workspace: {workspace}). Switch to main first: 'zjj focus main'")
            }
            Self::SessionNotFound { session_name } => {
                write!(f, "Session '{session_name}' not found in undo history")
            }
            Self::NoUndoHistory => write!(f, "No undo history found"),
            Self::AlreadyPushedToRemote { commit_id } => {
                write!(
                    f,
                    "Cannot revert: commit {commit_id} has already been pushed to remote"
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
        }
    }
}

impl std::error::Error for RevertError {}

impl RevertError {
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::NotInMain { .. } => "NOT_IN_MAIN",
            Self::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            Self::NoUndoHistory => "NO_UNDO_HISTORY",
            Self::AlreadyPushedToRemote { .. } => "ALREADY_PUSHED_TO_REMOTE",
            Self::RebaseFailed { .. } => "REBASE_FAILED",
            Self::JjCommandFailed { .. } => "JJ_COMMAND_FAILED",
            Self::ReadUndoLogFailed { .. } => "READ_UNDO_LOG_FAILED",
            Self::WriteUndoLogFailed { .. } => "WRITE_UNDO_LOG_FAILED",
            Self::SerializationError { .. } => "SERIALIZATION_ERROR",
            Self::InvalidState { .. } => "INVALID_STATE",
        }
    }
}

/// Exit codes for revert command
#[derive(Debug, Clone, Copy)]
pub enum RevertExitCode {
    Success = 0,
    AlreadyPushed = 1,
    SessionNotFound = 2,
    InvalidState = 3,
    OtherError = 4,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revert_args_to_options() {
        let args = RevertArgs {
            session_name: "test-session".to_string(),
            dry_run: true,
            format: OutputFormat::Json,
        };

        let opts = args.to_options();

        assert_eq!(opts.session_name, "test-session");
        assert!(opts.dry_run);
        assert!(matches!(opts.format, OutputFormat::Json));
    }

    #[test]
    fn test_revert_error_codes() {
        let err = RevertError::NoUndoHistory;
        assert_eq!(err.error_code(), "NO_UNDO_HISTORY");

        let err2 = RevertError::SessionNotFound {
            session_name: "test".to_string(),
        };
        assert_eq!(err2.error_code(), "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_revert_output_serialization() {
        let output = RevertOutput {
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

    // ── RevertError Display Tests ────────────────────────────────────────

    #[test]
    fn test_revert_error_not_in_main_display() {
        let err = RevertError::NotInMain {
            workspace: "feature-auth".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("Not in main branch"));
        assert!(display.contains("feature-auth"));
    }

    #[test]
    fn test_revert_error_session_not_found_display() {
        let err = RevertError::SessionNotFound {
            session_name: "missing-session".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("not found"));
        assert!(display.contains("missing-session"));
    }

    #[test]
    fn test_revert_error_already_pushed_display() {
        let err = RevertError::AlreadyPushedToRemote {
            commit_id: "abc123".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("Cannot revert"));
        assert!(display.contains("pushed to remote"));
    }

    // ── All RevertError Codes Tests ──────────────────────────────────────

    #[test]
    fn test_all_revert_error_codes() {
        assert_eq!(
            RevertError::NotInMain {
                workspace: String::new()
            }
            .error_code(),
            "NOT_IN_MAIN"
        );
        assert_eq!(
            RevertError::SessionNotFound {
                session_name: String::new()
            }
            .error_code(),
            "SESSION_NOT_FOUND"
        );
        assert_eq!(RevertError::NoUndoHistory.error_code(), "NO_UNDO_HISTORY");
        assert_eq!(
            RevertError::AlreadyPushedToRemote {
                commit_id: String::new()
            }
            .error_code(),
            "ALREADY_PUSHED_TO_REMOTE"
        );
        assert_eq!(
            RevertError::RebaseFailed {
                reason: String::new()
            }
            .error_code(),
            "REBASE_FAILED"
        );
        assert_eq!(
            RevertError::JjCommandFailed {
                command: String::new(),
                reason: String::new()
            }
            .error_code(),
            "JJ_COMMAND_FAILED"
        );
        assert_eq!(
            RevertError::ReadUndoLogFailed {
                reason: String::new()
            }
            .error_code(),
            "READ_UNDO_LOG_FAILED"
        );
        assert_eq!(
            RevertError::WriteUndoLogFailed {
                reason: String::new()
            }
            .error_code(),
            "WRITE_UNDO_LOG_FAILED"
        );
        assert_eq!(
            RevertError::SerializationError {
                reason: String::new()
            }
            .error_code(),
            "SERIALIZATION_ERROR"
        );
        assert_eq!(
            RevertError::InvalidState {
                reason: String::new()
            }
            .error_code(),
            "INVALID_STATE"
        );
    }

    // ── RevertExitCode Tests ─────────────────────────────────────────────

    #[test]
    fn test_revert_exit_code_values() {
        assert_eq!(RevertExitCode::Success as i32, 0);
        assert_eq!(RevertExitCode::AlreadyPushed as i32, 1);
        assert_eq!(RevertExitCode::SessionNotFound as i32, 2);
        assert_eq!(RevertExitCode::InvalidState as i32, 3);
        assert_eq!(RevertExitCode::OtherError as i32, 4);
    }

    // ── RevertOutput Tests ───────────────────────────────────────────────

    #[test]
    fn test_revert_output_default() {
        let output = RevertOutput::default();
        assert!(output.session_name.is_empty());
        assert!(output.commit_id.is_empty());
        assert!(!output.dry_run);
        assert!(!output.pushed_to_remote);
        assert!(output.error.is_none());
    }

    #[test]
    fn test_revert_output_with_error() {
        let output = RevertOutput {
            session_name: "test".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
            error: Some("failed to revert".to_string()),
        };
        assert!(output.error.is_some());
        assert_eq!(output.error, Some("failed to revert".to_string()));
    }

    #[test]
    fn test_revert_output_json_serialization() {
        let output = RevertOutput {
            session_name: "test-ws".to_string(),
            dry_run: true,
            commit_id: "xyz789".to_string(),
            pushed_to_remote: false,
            error: None,
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("test-ws"));
        assert!(json_str.contains("xyz789"));
        assert!(json_str.contains("\"dry_run\":true"));
    }

    // ── RevertOptions Tests ──────────────────────────────────────────────

    #[test]
    fn test_revert_options_from_args() {
        let args = RevertArgs {
            session_name: "session-1".to_string(),
            dry_run: false,
            format: OutputFormat::Human,
        };
        let opts = args.to_options();
        assert_eq!(opts.session_name, "session-1");
        assert!(!opts.dry_run);
        assert!(!opts.format.is_json());
    }
}
