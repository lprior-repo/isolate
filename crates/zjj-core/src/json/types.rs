//! Core JSON output types for standardized API responses.
//!
//! Provides the fundamental data structures for consistent JSON serialization
//! across all CLI commands in an AI-first design.

use serde::{Deserialize, Serialize};

/// Standard JSON success response wrapper
///
/// Flattens the generic data payload into the JSON response while adding
/// a success indicator for consistent response format detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSuccess<T> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T> JsonSuccess<T> {
    /// Create a new success response with the given data.
    pub const fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Standard JSON error response structure.
///
/// Provides a consistent error format with a success indicator and detailed
/// error information for programmatic and human consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub success: bool,
    pub error: ErrorDetail,
}

impl Default for JsonError {
    fn default() -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "UNKNOWN".to_string(),
                message: "An unknown error occurred".to_string(),
                details: None,
                suggestion: None,
            },
        }
    }
}

/// Detailed error information with optional context and resolution hints.
///
/// Designed for both machine-readable error codes and human-readable messages,
/// with optional structured details and user guidance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code in `SCREAMING_SNAKE_CASE` format
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional structured additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggestion for resolving the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Machine-readable error codes for programmatic error handling.
///
/// Organized by domain (Session, Workspace, JJ, Zellij, Config, Hook, State, Generic)
/// to enable domain-specific error recovery strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Session errors
    SessionNotFound,
    SessionAlreadyExists,
    SessionNameInvalid,

    // Workspace errors
    WorkspaceCreationFailed,
    WorkspaceNotFound,

    // JJ errors
    JjNotInstalled,
    JjCommandFailed,
    NotJjRepository,

    // Zellij errors
    ZellijNotRunning,
    ZellijCommandFailed,

    // Config errors
    ConfigNotFound,
    ConfigParseError,
    ConfigKeyNotFound,

    // Hook errors
    HookFailed,
    HookExecutionError,

    // State errors
    StateDbCorrupted,
    StateDbLocked,
    InvalidState,

    // Generic errors
    InvalidArgument,
    Unknown,
}

impl ErrorCode {
    /// Get the static string representation of the error code.
    ///
    /// Returns the `SCREAMING_SNAKE_CASE` format suitable for JSON and
    /// programmatic comparison.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyExists => "SESSION_ALREADY_EXISTS",
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::JjNotInstalled => "JJ_NOT_INSTALLED",
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::NotJjRepository => "NOT_JJ_REPOSITORY",
            Self::ZellijNotRunning => "ZELLIJ_NOT_RUNNING",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigKeyNotFound => "CONFIG_KEY_NOT_FOUND",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::StateDbCorrupted => "STATE_DB_CORRUPTED",
            Self::StateDbLocked => "STATE_DB_LOCKED",
            Self::InvalidState => "INVALID_STATE",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_success_creation() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "test".to_string(),
        };
        let success = JsonSuccess::new(data);
        assert!(success.success);
        assert_eq!(success.data.name, "test");
    }

    #[test]
    fn test_json_error_default() {
        let err = JsonError::default();
        assert!(!err.success);
        assert_eq!(err.error.code, "UNKNOWN");
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(ErrorCode::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
        assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
    }

    #[test]
    fn test_error_code_to_string() {
        let code: String = ErrorCode::SessionNotFound.into();
        assert_eq!(code, "SESSION_NOT_FOUND");
    }
}
