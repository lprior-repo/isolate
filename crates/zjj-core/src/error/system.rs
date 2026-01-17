//! System error types for IO, commands, and hook execution.
//!
//! These errors represent failures in system operations that are typically
//! out of the user's direct control.

use std::fmt;

/// System errors represent failures in IO, external commands, or hooks.
#[derive(Debug, Clone)]
pub enum SystemError {
    /// IO operation failed
    IoError(String),
    /// External command execution failed
    Command(String),
    /// Hook execution failed with exit code
    HookFailed {
        hook_type: String,
        command: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    /// Hook execution setup failed
    HookExecutionFailed { command: String, source: String },
    /// JJ command execution failed
    JjCommandError {
        operation: String,
        source: String,
        is_not_found: bool,
    },
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
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
        }
    }
}

impl SystemError {
    /// Get exit code for system errors.
    /// - JJ not found: 3
    /// - Other system errors: 2
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::JjCommandError {
                is_not_found: true, ..
            } => 3,
            _ => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_display() {
        let err = SystemError::io_error("file not found".into());
        assert_eq!(err.to_string(), "IO error: file not found");
    }

    #[test]
    fn test_command_error_display() {
        let err = SystemError::command_error("command failed".into());
        assert_eq!(err.to_string(), "Command error: command failed");
    }

    #[test]
    fn test_hook_failed_display() {
        let err = SystemError::HookFailed {
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
    fn test_hook_execution_failed_display() {
        let err = SystemError::HookExecutionFailed {
            command: "invalid-shell".to_string(),
            source: "No such file or directory".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Failed to execute hook"));
        assert!(display.contains("invalid-shell"));
        assert!(display.contains("No such file or directory"));
    }

    #[test]
    fn test_jj_command_not_found_display() {
        let err = SystemError::JjCommandError {
            operation: "create workspace".to_string(),
            source: "No such file or directory (os error 2)".to_string(),
            is_not_found: true,
        };
        let display = err.to_string();
        assert!(display.contains("Failed to create workspace"));
        assert!(display.contains("JJ is not installed"));
        assert!(display.contains("cargo install jj-cli"));
    }

    #[test]
    fn test_jj_command_error_display() {
        let err = SystemError::JjCommandError {
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
    fn test_exit_code_system_errors() {
        assert_eq!(SystemError::IoError("test".into()).exit_code(), 2);
        assert_eq!(SystemError::Command("test".into()).exit_code(), 2);
        assert_eq!(
            SystemError::HookFailed {
                hook_type: "post".to_string(),
                command: "test".to_string(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: String::new(),
            }
            .exit_code(),
            2
        );
        assert_eq!(
            SystemError::HookExecutionFailed {
                command: "test".to_string(),
                source: "error".to_string(),
            }
            .exit_code(),
            2
        );
    }

    #[test]
    fn test_exit_code_jj_not_found() {
        assert_eq!(
            SystemError::JjCommandError {
                operation: "test".to_string(),
                source: "not found".to_string(),
                is_not_found: true,
            }
            .exit_code(),
            3
        );
    }

    #[test]
    fn test_exit_code_jj_other_error() {
        assert_eq!(
            SystemError::JjCommandError {
                operation: "test".to_string(),
                source: "error".to_string(),
                is_not_found: false,
            }
            .exit_code(),
            2
        );
    }
}
