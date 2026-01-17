//! System error codes for resource state, missing dependencies, and system failures.
//!
//! These errors occur when system resources, permissions, or required tools are missing.

use serde::{Deserialize, Serialize};

/// System error codes for system state, dependencies, and resource errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemError {
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

impl SystemError {
    /// Get the string representation of the system error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
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

    /// Get a human-readable description of the system error
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
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
            Self::StateDbNotInitialized => "State database not initialized - run 'jjz init'",
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

    /// Get suggested action to resolve the system error
    #[must_use]
    pub const fn suggestion(self) -> Option<&'static str> {
        match self {
            Self::SessionNotFound => Some("Use 'jjz list' to see available sessions"),
            Self::SessionAlreadyExists => {
                Some("Choose a different name or remove the existing session")
            }
            Self::SessionDbError => Some("Try running 'jjz doctor --fix' to repair the database"),
            Self::WorkspaceNotFound => {
                Some("The workspace may have been deleted. Use 'jjz remove' to clean up")
            }
            Self::WorkspaceCorrupted => Some("Try removing and recreating the session"),
            Self::JjNotInstalled => Some("Install JJ: 'cargo install jj-cli' or 'brew install jj'"),
            Self::NotJjRepository => {
                Some("Initialize a JJ repository with 'jj init' or 'jjz init'")
            }
            Self::JjRepositoryCorrupted => {
                Some("Repository may be corrupted. Check with 'jj doctor'")
            }
            Self::JjWorkspaceError => Some("Check workspace with 'jj workspace list'"),
            Self::ZellijNotRunning => Some("Start Zellij first with 'zellij'"),
            Self::ZellijTabNotFound => {
                Some("Tab may have been closed. Use 'jjz list' to see active sessions")
            }
            Self::ZellijSessionCreationFailed => {
                Some("Check Zellij installation and configuration")
            }
            Self::ConfigNotFound => {
                Some("Create config with 'jjz init' or manually create ~/.config/jjz/config.toml")
            }
            Self::ConfigKeyNotFound => {
                Some("Use 'jjz config' to see all available configuration keys")
            }
            Self::ConfigWriteFailed => Some("Check file permissions on configuration directory"),
            Self::StateDbCorrupted => Some("Run 'jjz doctor --fix' to attempt repair"),
            Self::StateDbLocked => {
                Some("Another jjz process may be running. Wait or kill stuck processes")
            }
            Self::StateDbNotInitialized => {
                Some("Run 'jjz init' to initialize jjz in this repository")
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

            // 403: Forbidden
            Self::PermissionDenied | Self::OperationNotPermitted => 403,

            // 503: Service Unavailable
            Self::StateDbLocked | Self::ZellijNotRunning | Self::JjNotInstalled => 503,

            // 500: Internal Server Error
            Self::SessionDbError
            | Self::WorkspaceCorrupted
            | Self::NotJjRepository
            | Self::JjRepositoryCorrupted
            | Self::JjWorkspaceError
            | Self::ZellijSessionCreationFailed
            | Self::ConfigWriteFailed
            | Self::StateDbCorrupted
            | Self::StateDbNotInitialized
            | Self::DiskFull
            | Self::IoError
            | Self::Unknown => 500,
        }
    }
}

impl From<SystemError> for String {
    fn from(code: SystemError) -> Self {
        code.as_str().to_string()
    }
}

impl std::fmt::Display for SystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_error_as_str() {
        assert_eq!(SystemError::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(SystemError::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
        assert_eq!(SystemError::FileNotFound.as_str(), "FILE_NOT_FOUND");
    }

    #[test]
    fn test_system_error_description() {
        let desc = SystemError::SessionNotFound.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("session"));
    }

    #[test]
    fn test_system_error_suggestion() {
        let suggestion = SystemError::SessionNotFound.suggestion();
        assert!(suggestion.is_some());
        let text = suggestion
            .map(|s| s.contains("list"))
            .unwrap_or_else(|| false);
        assert!(text);
    }

    #[test]
    fn test_system_error_http_status() {
        assert_eq!(SystemError::SessionNotFound.http_status(), 404);
        assert_eq!(SystemError::SessionAlreadyExists.http_status(), 409);
        assert_eq!(SystemError::PermissionDenied.http_status(), 403);
        assert_eq!(SystemError::Unknown.http_status(), 500);
    }

    #[test]
    fn test_system_error_to_string() {
        let code: String = SystemError::JjNotInstalled.into();
        assert_eq!(code, "JJ_NOT_INSTALLED");
    }

    #[test]
    fn test_system_error_display() {
        let code = SystemError::ConfigNotFound;
        assert_eq!(format!("{code}"), "CONFIG_NOT_FOUND");
    }

    #[test]
    fn test_system_serialization() {
        let code = SystemError::FileNotFound;
        let result = serde_json::to_string(&code);
        assert!(result.is_ok(), "Serialization should succeed");
        assert_eq!(result.ok(), Some("\"FILE_NOT_FOUND\"".to_string()));
    }

    #[test]
    fn test_system_deserialization() {
        let json = "\"JJ_NOT_INSTALLED\"";
        let result = serde_json::from_str::<SystemError>(json);
        assert!(result.is_ok(), "Deserialization should succeed");
        assert_eq!(result.ok(), Some(SystemError::JjNotInstalled));
    }
}
