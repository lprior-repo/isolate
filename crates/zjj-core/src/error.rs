//! Error types for ZJJ with categorization:
//!
//! - **Validation errors**: Input validation and configuration (exit code 1)
//! - **System errors**: IO, commands, hooks, external operations (exit code 2 or 3)
//! - **Execution errors**: Database, repository state, resources (exit code 3 or 4)
//!
//! The module organizes errors into three logical categories using functional patterns
//! and clean separation of concerns.

pub mod execution;
pub mod system;
pub mod validation;

pub use execution::ExecutionError;
use std::fmt;
pub use system::SystemError;
pub use validation::ValidationError;

/// Top-level error type that can represent any error in the system.
///
/// Errors are logically separated into three categories:
/// - Validation errors (user input/config issues)
/// - System errors (IO, commands, external operations)
/// - Execution errors (database, repository state)
/// - Unknown errors (fallback for unexpected cases)
#[derive(Debug, Clone)]
pub enum Error {
    /// Validation error from input or configuration
    Validation(ValidationError),
    /// System error from IO or external operations
    System(SystemError),
    /// Execution error from database or repository state
    Execution(ExecutionError),
    /// Unknown error (fallback)
    Unknown(String),
}

// Convenience constructors using functional patterns
impl Error {
    /// Create a validation error from an invalid config.
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::Validation(ValidationError::InvalidConfig(msg.into()))
    }

    /// Create a validation error from a parse error.
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::Validation(ValidationError::ParseError(msg.into()))
    }

    /// Create a validation error from a validation failure.
    pub fn validation_error(msg: impl Into<String>) -> Self {
        Self::Validation(ValidationError::ValidationError(msg.into()))
    }

    /// Create a system error from an IO error.
    pub fn io_error(msg: impl Into<String>) -> Self {
        Self::System(SystemError::IoError(msg.into()))
    }

    /// Create a system error from a command failure.
    pub fn command_error(msg: impl Into<String>) -> Self {
        Self::System(SystemError::Command(msg.into()))
    }

    /// Create a system error from a hook failure.
    pub fn hook_failed(
        hook_type: impl Into<String>,
        command: impl Into<String>,
        exit_code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::System(SystemError::HookFailed {
            hook_type: hook_type.into(),
            command: command.into(),
            exit_code,
            stdout: stdout.into(),
            stderr: stderr.into(),
        })
    }

    /// Create a system error from hook execution setup failure.
    pub fn hook_execution_failed(command: impl Into<String>, source: impl Into<String>) -> Self {
        Self::System(SystemError::HookExecutionFailed {
            command: command.into(),
            source: source.into(),
        })
    }

    /// Create a system error from a JJ command failure.
    pub fn jj_command_error(
        operation: impl Into<String>,
        source: impl Into<String>,
        is_not_found: bool,
    ) -> Self {
        Self::System(SystemError::JjCommandError {
            operation: operation.into(),
            source: source.into(),
            is_not_found,
        })
    }

    /// Create an execution error from a database failure.
    pub fn database_error(msg: impl Into<String>) -> Self {
        Self::Execution(ExecutionError::DatabaseError(msg.into()))
    }

    /// Create an execution error for missing repository commits.
    pub fn no_commits_yet(workspace_path: impl Into<String>) -> Self {
        Self::Execution(ExecutionError::NoCommitsYet {
            workspace_path: workspace_path.into(),
        })
    }

    /// Create an execution error for missing main bookmark.
    pub fn main_bookmark_missing(
        workspace_path: impl Into<String>,
        bookmark_name: impl Into<String>,
        commit_count: usize,
    ) -> Self {
        Self::Execution(ExecutionError::MainBookmarkMissing {
            workspace_path: workspace_path.into(),
            bookmark_name: bookmark_name.into(),
            commit_count,
        })
    }

    /// Create an execution error for a not found resource.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::Execution(ExecutionError::NotFound(msg.into()))
    }

    /// Create an unknown error.
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Unknown(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(err) => write!(f, "{err}"),
            Self::System(err) => write!(f, "{err}"),
            Self::Execution(err) => write!(f, "{err}"),
            Self::Unknown(msg) => write!(f, "Unknown error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Returns the appropriate exit code for this error type.
    ///
    /// Exit code scheme:
    /// - 1: User error (validation, invalid input, bad configuration)
    /// - 2: System error (IO, external commands, hooks)
    /// - 3: Not found (sessions, resources, JJ not installed)
    /// - 4: Invalid state (database corruption, inconsistent state)
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Validation(err) => err.exit_code(),
            Self::System(err) => err.exit_code(),
            Self::Execution(err) => err.exit_code(),
            Self::Unknown(_) => 2,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::parse_error(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::parse_error(format!("Failed to parse config: {err}"))
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::database_error(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_config() {
        let err = Error::invalid_config("test error");
        assert_eq!(err.to_string(), "Invalid configuration: test error");
    }

    #[test]
    fn test_error_display_database_error() {
        let err = Error::database_error("connection failed");
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::System(_)));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::invalid_config("test");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Validation"));
    }

    #[test]
    fn test_error_display_hook_failed() {
        let err = Error::hook_failed(
            "post_create",
            "npm install",
            Some(1),
            "",
            "Package not found",
        );
        let display = err.to_string();
        assert!(display.contains("Hook 'post_create' failed"));
        assert!(display.contains("npm install"));
        assert!(display.contains("Exit code: Some(1)"));
        assert!(display.contains("Package not found"));
    }

    #[test]
    fn test_error_display_hook_execution_failed() {
        let err = Error::hook_execution_failed("invalid-shell", "No such file or directory");
        let display = err.to_string();
        assert!(display.contains("Failed to execute hook"));
        assert!(display.contains("invalid-shell"));
        assert!(display.contains("No such file or directory"));
    }

    #[test]
    fn test_error_display_jj_command_not_found() {
        let err = Error::jj_command_error(
            "create workspace",
            "No such file or directory (os error 2)",
            true,
        );
        let display = err.to_string();
        assert!(display.contains("Failed to create workspace"));
        assert!(display.contains("JJ is not installed"));
        assert!(display.contains("cargo install jj-cli"));
        assert!(display.contains("brew install jj"));
    }

    #[test]
    fn test_error_display_jj_command_other_error() {
        let err = Error::jj_command_error("list workspaces", "Permission denied", false);
        let display = err.to_string();
        assert!(display.contains("Failed to list workspaces"));
        assert!(display.contains("Permission denied"));
        assert!(!display.contains("JJ is not installed"));
    }

    #[test]
    fn test_exit_code_user_errors() {
        // User errors should exit with code 1
        assert_eq!(Error::validation_error("test").exit_code(), 1);
        assert_eq!(Error::invalid_config("test").exit_code(), 1);
        assert_eq!(Error::parse_error("test").exit_code(), 1);
    }

    #[test]
    fn test_exit_code_system_errors() {
        // System errors should exit with code 2
        assert_eq!(Error::io_error("test").exit_code(), 2);
        assert_eq!(Error::command_error("test").exit_code(), 2);
        assert_eq!(Error::unknown("test").exit_code(), 2);
        assert_eq!(
            Error::hook_failed("post_create", "test", Some(1), "", "").exit_code(),
            2
        );
        assert_eq!(Error::hook_execution_failed("test", "error").exit_code(), 2);
        assert_eq!(
            Error::jj_command_error("test", "error", false).exit_code(),
            2
        );
    }

    #[test]
    fn test_exit_code_not_found() {
        // Not found errors should exit with code 3
        assert_eq!(Error::not_found("session").exit_code(), 3);
        assert_eq!(
            Error::jj_command_error("test", "jj not found", true).exit_code(),
            3
        );
    }

    #[test]
    fn test_exit_code_invalid_state() {
        // Database/state errors should exit with code 4
        assert_eq!(Error::database_error("corrupt").exit_code(), 4);
    }

    #[test]
    fn test_error_constructor_convenience_methods() {
        let err1 = Error::invalid_config("test");
        assert_eq!(err1.exit_code(), 1);

        let err2 = Error::io_error("test");
        assert_eq!(err2.exit_code(), 2);

        let err3 = Error::not_found("test");
        assert_eq!(err3.exit_code(), 3);

        let err4 = Error::database_error("test");
        assert_eq!(err4.exit_code(), 4);
    }

    #[test]
    fn test_no_commits_yet_error() {
        let err = Error::no_commits_yet("/tmp/repo");
        let display = err.to_string();
        assert!(display.contains("Cannot sync"));
        assert!(display.contains("No commits"));
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_main_bookmark_missing_error() {
        let err = Error::main_bookmark_missing("/tmp/repo", "main", 5);
        let display = err.to_string();
        assert!(display.contains("bookmark 'main' doesn't exist"));
        assert!(display.contains("5 commit"));
        assert_eq!(err.exit_code(), 4);
    }
}
