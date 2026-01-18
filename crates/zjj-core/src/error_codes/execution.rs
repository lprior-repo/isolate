//! Execution error codes for operations that failed during execution.
//!
//! These errors occur when commands or operations were attempted but failed.

use serde::{Deserialize, Serialize};

/// Execution error codes for operations that failed at runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionError {
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
}

impl ExecutionError {
    /// Get the string representation of the execution error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::HookTimeout => "HOOK_TIMEOUT",
            Self::BeadsDbQueryFailed => "BEADS_DB_QUERY_FAILED",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceDeletionFailed => "WORKSPACE_DELETION_FAILED",
            Self::StateDbMigrationFailed => "STATE_DB_MIGRATION_FAILED",
        }
    }

    /// Get a human-readable description of the execution error
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::JjCommandFailed => "JJ command execution failed",
            Self::ZellijCommandFailed => "Zellij command execution failed",
            Self::HookFailed => "Hook command returned non-zero exit code",
            Self::HookExecutionError => "Failed to execute hook command",
            Self::HookTimeout => "Hook execution exceeded timeout",
            Self::BeadsDbQueryFailed => "Beads database query failed",
            Self::WorkspaceCreationFailed => "Failed to create workspace directory",
            Self::WorkspaceDeletionFailed => "Failed to delete workspace directory",
            Self::StateDbMigrationFailed => "Database schema migration failed",
        }
    }

    /// Get suggested action to resolve the execution error
    #[must_use]
    pub const fn suggestion(self) -> Option<&'static str> {
        match self {
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
        }
    }

    /// HTTP status code equivalent (for potential future REST API)
    #[must_use]
    pub const fn http_status(self) -> u16 {
        // 500: Internal Server Error (execution failures)
        500
    }
}

impl From<ExecutionError> for String {
    fn from(code: ExecutionError) -> Self {
        code.as_str().to_string()
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_error_as_str() {
        assert_eq!(
            ExecutionError::JjCommandFailed.as_str(),
            "JJ_COMMAND_FAILED"
        );
        assert_eq!(
            ExecutionError::ZellijCommandFailed.as_str(),
            "ZELLIJ_COMMAND_FAILED"
        );
        assert_eq!(ExecutionError::HookFailed.as_str(), "HOOK_FAILED");
    }

    #[test]
    fn test_execution_error_description() {
        let desc = ExecutionError::JjCommandFailed.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("JJ"));
    }

    #[test]
    fn test_execution_error_suggestion() {
        let suggestion = ExecutionError::HookFailed.suggestion();
        assert!(suggestion.is_some());
        let text = suggestion
            .map(|s| s.contains("hook"))
            .unwrap_or_else(|| false);
        assert!(text);
    }

    #[test]
    fn test_execution_error_http_status() {
        assert_eq!(ExecutionError::JjCommandFailed.http_status(), 500);
        assert_eq!(ExecutionError::HookFailed.http_status(), 500);
        assert_eq!(ExecutionError::WorkspaceCreationFailed.http_status(), 500);
    }

    #[test]
    fn test_execution_error_to_string() {
        let code: String = ExecutionError::HookTimeout.into();
        assert_eq!(code, "HOOK_TIMEOUT");
    }

    #[test]
    fn test_execution_error_display() {
        let code = ExecutionError::BeadsDbQueryFailed;
        assert_eq!(format!("{code}"), "BEADS_DB_QUERY_FAILED");
    }

    #[test]
    fn test_execution_serialization() {
        let code = ExecutionError::JjCommandFailed;
        let result = serde_json::to_string(&code);
        assert!(result.is_ok(), "Serialization should succeed");
        assert_eq!(result.ok(), Some("\"JJ_COMMAND_FAILED\"".to_string()));
    }

    #[test]
    fn test_execution_deserialization() {
        let json = "\"HOOK_FAILED\"";
        let result = serde_json::from_str::<ExecutionError>(json);
        assert!(result.is_ok(), "Deserialization should succeed");
        assert_eq!(result.ok(), Some(ExecutionError::HookFailed));
    }
}
