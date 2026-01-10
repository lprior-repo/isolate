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
                        "Failed to {operation}: JJ is not installed or not in PATH.\n\n\
                        Install JJ:\n\
                          cargo install jj-cli\n\
                        or:\n\
                          brew install jj\n\
                        or visit: https://github.com/martinvonz/jj#installation\n\n\
                        Error: {source}"
                    )
                } else {
                    write!(f, "Failed to {operation}: {source}")
                }
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
}
