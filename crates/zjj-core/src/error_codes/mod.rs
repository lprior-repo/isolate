//! Error code definitions for structured JSON error responses
//!
//! This module provides comprehensive error codes organized into three categories:
//! - **Validation**: Input validation and format errors
//! - **Execution**: Operations that failed during execution
//! - **System**: Resource state, missing dependencies, and system failures
//!
//! All error codes use `SCREAMING_SNAKE_CASE` and map to specific error conditions.

pub mod execution;
pub mod system;
pub mod validation;

pub use execution::ExecutionError;
pub use system::SystemError;
pub use validation::ValidationError;

use serde::{Deserialize, Serialize};

/// Comprehensive error codes for machine-readable errors
///
/// This enum provides a unified interface that combines all error categories.
/// For new code, consider using specific error types (ValidationError, ExecutionError, SystemError)
/// for better type safety and categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // ═══════════════════════════════════════════════════════════════════════
    // Validation Errors
    // ═══════════════════════════════════════════════════════════════════════
    /// Session name fails validation rules
    SessionNameInvalid,
    /// Session status transition not allowed
    SessionInvalidTransition,
    /// Configuration file has syntax errors
    ConfigParseError,
    /// Configuration value has invalid type
    ConfigInvalidValue,
    /// Generic validation error
    InvalidArgument,

    // ═══════════════════════════════════════════════════════════════════════
    // Execution Errors
    // ═══════════════════════════════════════════════════════════════════════
    /// JJ command failed during execution
    JjCommandFailed,
    /// Zellij command failed during execution
    ZellijCommandFailed,
    /// Hook command returned non-zero exit code
    HookFailed,
    /// Hook command failed to execute
    HookExecutionError,
    /// Hook timeout exceeded
    HookTimeout,
    /// Beads database query failed
    BeadsDbQueryFailed,
    /// Failed to create workspace directory
    WorkspaceCreationFailed,
    /// Workspace cannot be deleted
    WorkspaceDeletionFailed,
    /// State database migration failed
    StateDbMigrationFailed,

    // ═══════════════════════════════════════════════════════════════════════
    // System Errors
    // ═══════════════════════════════════════════════════════════════════════
    /// Session with given name was not found
    SessionNotFound,
    /// Session with given name already exists
    SessionAlreadyExists,
    /// Session database operation failed
    SessionDbError,
    /// Workspace directory not found
    WorkspaceNotFound,
    /// Workspace is in an inconsistent state
    WorkspaceCorrupted,
    /// JJ command-line tool not installed or not in PATH
    JjNotInstalled,
    /// Not inside a JJ repository
    NotJjRepository,
    /// JJ repository is corrupted or invalid
    JjRepositoryCorrupted,
    /// JJ workspace operation failed
    JjWorkspaceError,
    /// Not currently inside a Zellij session
    ZellijNotRunning,
    /// Zellij tab not found
    ZellijTabNotFound,
    /// Zellij session creation failed
    ZellijSessionCreationFailed,
    /// Configuration file not found
    ConfigNotFound,
    /// Configuration key not found
    ConfigKeyNotFound,
    /// Configuration write failed
    ConfigWriteFailed,
    /// State database file is corrupted
    StateDbCorrupted,
    /// State database is locked by another process
    StateDbLocked,
    /// State database not initialized
    StateDbNotInitialized,
    /// Beads database not found
    BeadsDbNotFound,
    /// Beads issue not found
    BeadsIssueNotFound,
    /// File or directory not found
    FileNotFound,
    /// Permission denied
    PermissionDenied,
    /// Disk space exhausted
    DiskFull,
    /// I/O operation failed
    IoError,
    /// Operation not permitted in current state
    OperationNotPermitted,
    /// Resource is busy or locked
    ResourceBusy,
    /// Unknown or unclassified error
    Unknown,
}

impl ErrorCode {
    /// Get the string representation of the error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            // Validation Errors
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::SessionInvalidTransition => "SESSION_INVALID_TRANSITION",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigInvalidValue => "CONFIG_INVALID_VALUE",
            Self::InvalidArgument => "INVALID_ARGUMENT",

            // Execution Errors
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::HookTimeout => "HOOK_TIMEOUT",
            Self::BeadsDbQueryFailed => "BEADS_DB_QUERY_FAILED",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceDeletionFailed => "WORKSPACE_DELETION_FAILED",
            Self::StateDbMigrationFailed => "STATE_DB_MIGRATION_FAILED",

            // System Errors
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyExists => "SESSION_ALREADY_EXISTS",
            Self::SessionDbError => "SESSION_DB_ERROR",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::WorkspaceCorrupted => "WORKSPACE_CORRUPTED",
            Self::JjNotInstalled => "JJ_NOT_INSTALLED",
            Self::NotJjRepository => "NOT_JJ_REPOSITORY",
            Self::JjRepositoryCorrupted => "JJ_REPOSITORY_CORRUPTED",
            Self::JjWorkspaceError => "JJ_WORKSPACE_ERROR",
            Self::ZellijNotRunning => "ZELLIJ_NOT_RUNNING",
            Self::ZellijTabNotFound => "ZELLIJ_TAB_NOT_FOUND",
            Self::ZellijSessionCreationFailed => "ZELLIJ_SESSION_CREATION_FAILED",
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigKeyNotFound => "CONFIG_KEY_NOT_FOUND",
            Self::ConfigWriteFailed => "CONFIG_WRITE_FAILED",
            Self::StateDbCorrupted => "STATE_DB_CORRUPTED",
            Self::StateDbLocked => "STATE_DB_LOCKED",
            Self::StateDbNotInitialized => "STATE_DB_NOT_INITIALIZED",
            Self::BeadsDbNotFound => "BEADS_DB_NOT_FOUND",
            Self::BeadsIssueNotFound => "BEADS_ISSUE_NOT_FOUND",
            Self::FileNotFound => "FILE_NOT_FOUND",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::DiskFull => "DISK_FULL",
            Self::IoError => "IO_ERROR",
            Self::OperationNotPermitted => "OPERATION_NOT_PERMITTED",
            Self::ResourceBusy => "RESOURCE_BUSY",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Get a human-readable description of the error code
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            // Validation Errors
            Self::SessionNameInvalid => "Session name contains invalid characters or format",
            Self::SessionInvalidTransition => "Cannot transition session to requested state",
            Self::ConfigParseError => "Configuration file has syntax errors",
            Self::ConfigInvalidValue => "Configuration value has invalid type",
            Self::InvalidArgument => "Invalid argument provided",

            // Execution Errors
            Self::JjCommandFailed => "JJ command execution failed",
            Self::ZellijCommandFailed => "Zellij command execution failed",
            Self::HookFailed => "Hook command returned non-zero exit code",
            Self::HookExecutionError => "Failed to execute hook command",
            Self::HookTimeout => "Hook execution exceeded timeout",
            Self::BeadsDbQueryFailed => "Beads database query failed",
            Self::WorkspaceCreationFailed => "Failed to create workspace directory",
            Self::WorkspaceDeletionFailed => "Failed to delete workspace directory",
            Self::StateDbMigrationFailed => "Database schema migration failed",

            // System Errors
            Self::SessionNotFound => "The specified session does not exist",
            Self::SessionAlreadyExists => "A session with this name already exists",
            Self::SessionDbError => "Session database operation failed",
            Self::WorkspaceNotFound => "Workspace directory does not exist",
            Self::WorkspaceCorrupted => "Workspace is in an inconsistent state",
            Self::JjNotInstalled => "JJ (Jujutsu) is not installed or not in PATH",
            Self::NotJjRepository => "Current directory is not a JJ repository",
            Self::JjRepositoryCorrupted => "JJ repository is corrupted or invalid",
            Self::JjWorkspaceError => "JJ workspace operation failed",
            Self::ZellijNotRunning => "Not currently inside a Zellij session",
            Self::ZellijTabNotFound => "Zellij tab not found",
            Self::ZellijSessionCreationFailed => "Failed to create Zellij session",
            Self::ConfigNotFound => "Configuration file not found",
            Self::ConfigKeyNotFound => "Configuration key does not exist",
            Self::ConfigWriteFailed => "Failed to write configuration file",
            Self::StateDbCorrupted => "State database is corrupted",
            Self::StateDbLocked => "State database is locked by another process",
            Self::StateDbNotInitialized => "State database not initialized - run 'zjj init'",
            Self::BeadsDbNotFound => "Beads database not found in repository",
            Self::BeadsIssueNotFound => "Beads issue not found",
            Self::FileNotFound => "File or directory not found",
            Self::PermissionDenied => "Permission denied",
            Self::DiskFull => "Disk space exhausted",
            Self::IoError => "I/O operation failed",
            Self::OperationNotPermitted => "Operation not permitted in current state",
            Self::ResourceBusy => "Resource is busy or locked",
            Self::Unknown => "An unknown error occurred",
        }
    }

    /// Get suggested action to resolve the error
    #[must_use]
    pub const fn suggestion(self) -> Option<&'static str> {
        match self {
            // Validation Errors
            Self::SessionNameInvalid => Some(
                "Use only letters, numbers, hyphens, and underscores. Must start with a letter",
            ),
            Self::SessionInvalidTransition => {
                Some("Check session status with 'zjj status' before attempting operation")
            }
            Self::ConfigParseError => {
                Some("Check configuration file syntax. Refer to documentation for format")
            }
            Self::ConfigInvalidValue => {
                Some("Check configuration value type matches expected format")
            }
            Self::InvalidArgument => Some("Check command syntax with 'zjj --help'"),

            // Execution Errors
            Self::JjCommandFailed => Some("Check JJ repository status with 'jj status'"),
            Self::ZellijCommandFailed => Some("Check Zellij status with 'zellij list-sessions'"),
            Self::HookFailed => Some("Check hook output and fix errors. Use --no-hooks to skip"),
            Self::HookExecutionError => Some("Ensure hook command exists and is executable"),
            Self::HookTimeout => Some("Increase hook timeout in config or optimize hook script"),
            Self::BeadsDbQueryFailed => Some("Check beads database integrity"),
            Self::WorkspaceCreationFailed => Some("Check disk space and permissions"),
            Self::WorkspaceDeletionFailed => {
                Some("Check permissions and ensure no processes are using the workspace")
            }
            Self::StateDbMigrationFailed => Some("Backup your data and try 'zjj init --force'"),

            // System Errors
            Self::SessionNotFound => Some("Use 'zjj list' to see available sessions"),
            Self::SessionAlreadyExists => {
                Some("Choose a different name or remove the existing session")
            }
            Self::SessionDbError => Some("Try running 'zjj doctor --fix' to repair the database"),
            Self::WorkspaceNotFound => {
                Some("The workspace may have been deleted. Use 'zjj remove' to clean up")
            }
            Self::WorkspaceCorrupted => Some("Try removing and recreating the session"),
            Self::JjNotInstalled => Some("Install JJ: 'cargo install jj-cli' or 'brew install jj'"),
            Self::NotJjRepository => {
                Some("Initialize a JJ repository with 'jj init' or 'zjj init'")
            }
            Self::JjRepositoryCorrupted => {
                Some("Repository may be corrupted. Check with 'jj doctor'")
            }
            Self::JjWorkspaceError => Some("Check workspace with 'jj workspace list'"),
            Self::ZellijNotRunning => Some("Start Zellij first with 'zellij'"),
            Self::ZellijTabNotFound => {
                Some("Tab may have been closed. Use 'zjj list' to see active sessions")
            }
            Self::ZellijSessionCreationFailed => {
                Some("Check Zellij installation and configuration")
            }
            Self::ConfigNotFound => {
                Some("Create config with 'zjj init' or manually create ~/.config/zjj/config.toml")
            }
            Self::ConfigKeyNotFound => {
                Some("Use 'zjj config' to see all available configuration keys")
            }
            Self::ConfigWriteFailed => Some("Check file permissions on configuration directory"),
            Self::StateDbCorrupted => Some("Run 'zjj doctor --fix' to attempt repair"),
            Self::StateDbLocked => {
                Some("Another zjj process may be running. Wait or kill stuck processes")
            }
            Self::StateDbNotInitialized => {
                Some("Run 'zjj init' to initialize zjj in this repository")
            }
            Self::BeadsDbNotFound => Some("Initialize beads with 'beads init' first"),
            Self::BeadsIssueNotFound => Some("Use 'beads list' to see available issues"),
            Self::FileNotFound => Some("Check that the file or directory exists"),
            Self::PermissionDenied => {
                Some("Check file permissions or run with appropriate privileges")
            }
            Self::DiskFull => Some("Free up disk space and try again"),
            Self::IoError => Some("Check filesystem status and permissions"),
            Self::OperationNotPermitted => Some("Verify session state allows this operation"),
            Self::ResourceBusy => {
                Some("Wait for the resource to become available or kill competing processes")
            }
            Self::Unknown => Some("Check logs for more details or report this as a bug"),
        }
    }

    /// HTTP status code equivalent (for potential future REST API)
    #[must_use]
    pub const fn http_status(self) -> u16 {
        match self {
            // 404: Not Found
            Self::SessionNotFound
            | Self::WorkspaceNotFound
            | Self::ConfigNotFound
            | Self::ConfigKeyNotFound
            | Self::ZellijTabNotFound
            | Self::BeadsDbNotFound
            | Self::BeadsIssueNotFound
            | Self::FileNotFound => 404,

            // 409: Conflict
            Self::SessionAlreadyExists | Self::ResourceBusy => 409,

            // 422: Unprocessable Entity (validation errors)
            Self::SessionNameInvalid
            | Self::SessionInvalidTransition
            | Self::ConfigParseError
            | Self::ConfigInvalidValue
            | Self::InvalidArgument => 422,

            // 403: Forbidden
            Self::PermissionDenied | Self::OperationNotPermitted => 403,

            // 503: Service Unavailable
            Self::StateDbLocked | Self::ZellijNotRunning | Self::JjNotInstalled => 503,

            // 500: Internal Server Error
            Self::SessionDbError
            | Self::WorkspaceCreationFailed
            | Self::WorkspaceCorrupted
            | Self::WorkspaceDeletionFailed
            | Self::JjCommandFailed
            | Self::NotJjRepository
            | Self::JjRepositoryCorrupted
            | Self::JjWorkspaceError
            | Self::ZellijCommandFailed
            | Self::ZellijSessionCreationFailed
            | Self::ConfigWriteFailed
            | Self::HookFailed
            | Self::HookExecutionError
            | Self::HookTimeout
            | Self::StateDbCorrupted
            | Self::StateDbMigrationFailed
            | Self::StateDbNotInitialized
            | Self::BeadsDbQueryFailed
            | Self::DiskFull
            | Self::IoError
            | Self::Unknown => 500,
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(ErrorCode::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
        assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
        assert_eq!(ErrorCode::Unknown.as_str(), "UNKNOWN");
    }

    #[test]
    fn test_error_code_description() {
        let desc = ErrorCode::SessionNotFound.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("session"));
    }

    #[test]
    fn test_error_code_suggestion() {
        let suggestion = ErrorCode::SessionNotFound.suggestion();
        assert!(suggestion.is_some());
        let text = suggestion
            .map(|s| s.contains("list"))
            .unwrap_or_else(|| false);
        assert!(text);
    }

    #[test]
    fn test_error_code_http_status() {
        assert_eq!(ErrorCode::SessionNotFound.http_status(), 404);
        assert_eq!(ErrorCode::SessionAlreadyExists.http_status(), 409);
        assert_eq!(ErrorCode::InvalidArgument.http_status(), 422);
        assert_eq!(ErrorCode::PermissionDenied.http_status(), 403);
        assert_eq!(ErrorCode::Unknown.http_status(), 500);
    }

    #[test]
    fn test_error_code_to_string() {
        let code: String = ErrorCode::SessionNotFound.into();
        assert_eq!(code, "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_error_code_display() {
        let code = ErrorCode::JjNotInstalled;
        assert_eq!(format!("{code}"), "JJ_NOT_INSTALLED");
    }

    #[test]
    fn test_all_error_codes_have_descriptions() {
        let codes = [
            ErrorCode::SessionNotFound,
            ErrorCode::SessionAlreadyExists,
            ErrorCode::JjNotInstalled,
            ErrorCode::ConfigNotFound,
            ErrorCode::HookFailed,
            ErrorCode::StateDbCorrupted,
            ErrorCode::Unknown,
        ];

        for code in &codes {
            assert!(!code.description().is_empty());
        }
    }

    #[test]
    fn test_serialization() {
        let code = ErrorCode::SessionNotFound;
        let result = serde_json::to_string(&code);
        assert!(result.is_ok(), "Serialization should succeed");
        assert_eq!(result.ok(), Some("\"SESSION_NOT_FOUND\"".to_string()));
    }

    #[test]
    fn test_deserialization() {
        let json = "\"SESSION_NOT_FOUND\"";
        let result = serde_json::from_str::<ErrorCode>(json);
        assert!(result.is_ok(), "Deserialization should succeed");
        assert_eq!(result.ok(), Some(ErrorCode::SessionNotFound));
    }
}
