use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidConfig(String),
    IoError(String),
    ParseError(String),
    ValidationError(String),
    NotFound(String),
    DatabaseError(String),
    Command(String),
    HookFailed {
        hook_type: String,
        command: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    HookExecutionFailed {
        command: String,
        source: String,
    },
    JjCommandError {
        operation: String,
        source: String,
        is_not_found: bool,
    },
    SessionLocked {
        session: String,
        holder: String,
    },
    NotLockHolder {
        session: String,
        agent_id: String,
    },
    Unknown(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::Command(msg) => write!(f, "Command error: {msg}"),
            Self::HookFailed {
                hook_type,
                command,
                exit_code,
                stdout: _,
                stderr,
            } => {
                write!(
                    f,
                    "Hook '{hook_type}' failed: {command}\nExit code: {exit_code:?}\nStderr: {stderr}"
                )
            }
            Self::HookExecutionFailed { command, source } => {
                write!(f, "Failed to execute hook '{command}': {source}")
            }
            Self::JjCommandError {
                operation,
                source,
                is_not_found,
            } => {
                if *is_not_found {
                    write!(
                        f,
                        "Failed to {operation}: JJ is not installed or not in PATH.\n\n\nInstall JJ:\n\n  cargo install jj-cli\n\nor:\n\n  brew install jj\n\nor visit: https://github.com/martinvonz/jj#installation\n\nError: {source}"
                    )
                } else {
                    write!(f, "Failed to {operation}: {source}")
                }
            }
            Self::SessionLocked { session, holder } => {
                write!(f, "Session '{session}' is locked by agent '{holder}'")
            }
            Self::NotLockHolder { session, agent_id } => {
                write!(
                    f,
                    "Agent '{agent_id}' does not hold the lock for session '{session}'"
                )
            }
            Self::Unknown(msg) => write!(f, "Unknown error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::ParseError(format!("Failed to parse config: {err}"))
    }
}

impl Error {
    /// Returns the machine-readable error code for this error.
    ///
    /// Error codes are always in `SCREAMING_SNAKE_CASE` format.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidConfig(_) => "INVALID_CONFIG",
            Self::IoError(_) => "IO_ERROR",
            Self::ParseError(_) => "PARSE_ERROR",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::Command(_) => "COMMAND_ERROR",
            Self::HookFailed { .. } => "HOOK_FAILED",
            Self::HookExecutionFailed { .. } => "HOOK_EXECUTION_FAILED",
            Self::JjCommandError { .. } => "JJ_COMMAND_ERROR",
            Self::SessionLocked { .. } => "SESSION_LOCKED",
            Self::NotLockHolder { .. } => "NOT_LOCK_HOLDER",
            Self::Unknown(_) => "UNKNOWN",
        }
    }

    /// Returns structured context information for this error as a JSON value.
    ///
    /// This provides machine-readable context that can be used by AI agents
    /// or tools to understand the error in detail.
    #[must_use]
    pub fn context_map(&self) -> Option<serde_json::Value> {
        match self {
            Self::InvalidConfig(msg) => Some(serde_json::json!({
                "input": msg,
                "expected_format": "valid TOML configuration"
            })),
            Self::ValidationError(msg) => Some(serde_json::json!({
                "input": msg,
                "expected_format": "alphanumeric, dash, underscore only"
            })),
            Self::NotFound(msg) => Some(serde_json::json!({
                "resource_type": "session",
                "resource_id": msg,
                "searched_in": "database"
            })),
            Self::IoError(msg) => Some(serde_json::json!({
                "operation": "file_io",
                "error": msg
            })),
            Self::DatabaseError(msg) => Some(serde_json::json!({
                "operation": "database",
                "error": msg
            })),
            Self::Command(msg) => Some(serde_json::json!({
                "operation": "command_execution",
                "error": msg
            })),
            Self::HookFailed {
                hook_type,
                command,
                exit_code,
                stdout: _,
                stderr,
            } => Some(serde_json::json!({
                "hook_type": hook_type,
                "command": command,
                "exit_code": exit_code,
                "stderr": stderr
            })),
            Self::HookExecutionFailed { command, source } => Some(serde_json::json!({
                "command": command,
                "source": source
            })),
            Self::JjCommandError {
                operation,
                source,
                is_not_found,
            } => Some(serde_json::json!({
                "operation": operation,
                "source": source,
                "is_not_found": is_not_found
            })),
            Self::SessionLocked { session, holder } => Some(serde_json::json!({
                "session": session,
                "holder": holder
            })),
            Self::NotLockHolder { session, agent_id } => Some(serde_json::json!({
                "session": session,
                "agent_id": agent_id
            })),
            _ => None,
        }
    }

    /// Returns a helpful suggestion for resolving this error, if available.
    ///
    /// Suggestions are actionable and guide the user toward a solution.
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match self {
             Self::NotFound(_) => Some("Try 'zjj list' to see available sessions".to_string()),
             Self::ValidationError(msg) if msg.contains("name") => Some(
                 "Session name must start with letter and contain only alphanumeric, dash, underscore"
                     .to_string(),
             ),
             Self::DatabaseError(_) => {
                 Some("Try 'zjj doctor' to check database health".to_string())
             }
Self::JjCommandError {
                  is_not_found: true,
                  ..
              } => Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
             Self::HookFailed { .. } => Some(
                 "Check your hook configuration and ensure the command is correct".to_string(),
             ),
             Self::HookExecutionFailed { .. } => {
                 Some("Ensure the hook command exists and is executable".to_string())
             }
             _ => None,
         }
    }

    /// Returns the semantic exit code for this error.
    ///
    /// Exit codes follow this semantic mapping:
    /// - 1: Validation errors (user input issues)
    /// - 2: Not found errors (missing resources)
    /// - 3: System errors (IO, database issues)
    /// - 4: External command errors (JJ, hooks, etc.)
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            // Validation errors: exit code 1
            Self::InvalidConfig(_) | Self::ValidationError(_) | Self::ParseError(_) => 1,
            // Not found errors: exit code 2
            Self::NotFound(_) => 2,
            // System errors: exit code 3
            Self::IoError(_) | Self::DatabaseError(_) => 3,
            // External command errors: exit code 4
            Self::Command(_)
            | Self::JjCommandError { .. }
            | Self::HookFailed { .. }
            | Self::HookExecutionFailed { .. }
            | Self::Unknown(_) => 4,
            // Lock contention errors: exit code 5
            Self::SessionLocked { .. } | Self::NotLockHolder { .. } => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_config() {
        let err = Error::InvalidConfig("test error".into());
        assert_eq!(err.to_string(), "Invalid configuration: test error");
    }

    #[test]
    fn test_error_display_database_error() {
        let err = Error::DatabaseError("connection failed".into());
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::IoError(_)));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::InvalidConfig("test".into());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("InvalidConfig"));
    }

    #[test]
    fn test_error_display_hook_failed() {
        let err = Error::HookFailed {
            hook_type: "post_create".to_string(),
            command: "npm install".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "Package not found".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Hook 'post_create' failed"));
        assert!(display.contains("npm install"));
        assert!(display.contains("Exit code: Some(1)"));
        assert!(display.contains("Package not found"));
    }

    #[test]
    fn test_error_display_hook_execution_failed() {
        let err = Error::HookExecutionFailed {
            command: "invalid-shell".to_string(),
            source: "No such file or directory".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Failed to execute hook"));
        assert!(display.contains("invalid-shell"));
        assert!(display.contains("No such file or directory"));
    }

    #[test]
    fn test_error_display_jj_command_not_found() {
        let err = Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: "No such file or directory (os error 2)".to_string(),
            is_not_found: true,
        };
        let display = err.to_string();
        assert!(display.contains("Failed to create workspace"));
        assert!(display.contains("JJ is not installed"));
        assert!(display.contains("cargo install jj-cli"));
        assert!(display.contains("brew install jj"));
    }

    #[test]
    fn test_error_display_jj_command_other_error() {
        let err = Error::JjCommandError {
            operation: "list workspaces".to_string(),
            source: "Permission denied".to_string(),
            is_not_found: false,
        };
        let display = err.to_string();
        assert!(display.contains("Failed to list workspaces"));
        assert!(display.contains("Permission denied"));
        assert!(!display.contains("JJ is not installed"));
    }

    #[test]
    fn test_error_code_validation_error() {
        let err = Error::ValidationError("invalid input".into());
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_error_code_not_found() {
        let err = Error::NotFound("session not found".into());
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn test_error_code_io_error() {
        let err = Error::IoError("file not found".into());
        assert_eq!(err.code(), "IO_ERROR");
    }

    #[test]
    fn test_error_code_database_error() {
        let err = Error::DatabaseError("connection failed".into());
        assert_eq!(err.code(), "DATABASE_ERROR");
    }

    #[test]
    fn test_error_code_uppercase() {
        let err = Error::InvalidConfig("bad config".into());
        let code = err.code();
        assert_eq!(code, code.to_uppercase(), "Error code must be uppercase");
    }

    #[test]
    fn test_validation_error_context_has_field() {
        let err = Error::ValidationError("Session name must be alphanumeric".into());
        let context = err.context_map();
        assert!(context.is_some());
        if let Some(ctx) = context {
            assert!(ctx.get("input").is_some());
        }
    }

    #[test]
    fn test_not_found_error_context_has_resource() {
        let err = Error::NotFound("session 'test-123' not found".into());
        let context = err.context_map();
        assert!(context.is_some());
        if let Some(ctx) = context {
            assert!(ctx.get("resource_type").is_some());
        }
    }

    #[test]
    fn test_io_error_context_has_path() {
        let err = Error::IoError("Failed to read file".into());
        let context = err.context_map();
        assert!(context.is_some());
        if let Some(ctx) = context {
            assert!(ctx.get("operation").is_some());
        }
    }

    #[test]
    fn test_session_not_found_suggests_list() {
        let err = Error::NotFound("session not found".into());
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        if let Some(sugg) = suggestion {
            assert!(sugg.contains("zjj list") || sugg.contains("list"));
        }
    }

    #[test]
    fn test_validation_error_suggests_format() {
        let err = Error::ValidationError("invalid session name".into());
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_validation_error_maps_to_exit_code_1() {
        let err = Error::ValidationError("invalid input".into());
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_invalid_config_maps_to_exit_code_1() {
        let err = Error::InvalidConfig("bad config".into());
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_parse_error_maps_to_exit_code_1() {
        let err = Error::ParseError("malformed input".into());
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_not_found_error_maps_to_exit_code_2() {
        let err = Error::NotFound("session not found".into());
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_io_error_maps_to_exit_code_3() {
        let err = Error::IoError("file not found".into());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_database_error_maps_to_exit_code_3() {
        let err = Error::DatabaseError("connection failed".into());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_command_error_maps_to_exit_code_4() {
        let err = Error::Command("command failed".into());
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_jj_command_error_maps_to_exit_code_4() {
        let err = Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: "error".to_string(),
            is_not_found: false,
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_hook_failed_maps_to_exit_code_4() {
        let err = Error::HookFailed {
            hook_type: "post_create".to_string(),
            command: "npm install".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "Package not found".to_string(),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_hook_execution_failed_maps_to_exit_code_4() {
        let err = Error::HookExecutionFailed {
            command: "invalid-shell".to_string(),
            source: "No such file or directory".to_string(),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_unknown_error_maps_to_exit_code_4() {
        let err = Error::Unknown("unknown error".into());
        assert_eq!(err.exit_code(), 4);
    }
}
