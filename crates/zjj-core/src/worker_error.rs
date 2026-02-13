//! Error classification for queue worker operations.
//!
//! This module provides error classification logic to determine whether
//! failures should be retryable or terminal based on error characteristics
//! and attempt count.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use thiserror::Error;

/// Classification of worker errors for retry decision-making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Error is transient and can be retried.
    Retryable,
    /// Error is permanent and should not be retried.
    Terminal,
}

/// Errors that can occur during worker processing.
#[derive(Debug, Clone, Error)]
pub enum WorkerError {
    /// Merge conflict detected.
    #[error("merge conflict: {0}")]
    MergeConflict(String),

    /// Workspace not found or invalid.
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),

    /// Git operation failed.
    #[error("git operation failed: {0}")]
    GitOperationFailed(String),

    /// Network or I/O error (transient).
    #[error("I/O error: {0}")]
    IoError(String),

    /// Database error (may be transient).
    #[error("database error: {0}")]
    DatabaseError(String),

    /// Lock acquisition failed (transient).
    #[error("lock contention: {0}")]
    LockContention(String),

    /// Validation error (terminal).
    #[error("validation error: {0}")]
    ValidationError(String),

    /// Configuration error (terminal).
    #[error("configuration error: {0}")]
    ConfigurationError(String),

    /// Permission denied (terminal).
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Generic processing error.
    #[error("processing error: {0}")]
    ProcessingError(String),

    /// Operation timeout (retryable).
    #[error("operation timed out: {0}")]
    Timeout(String),

    /// External service unavailable (retryable).
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl WorkerError {
    /// Classify the error as retryable or terminal.
    #[must_use]
    pub const fn classify(&self) -> ErrorClass {
        match self {
            Self::IoError(_)
            | Self::DatabaseError(_)
            | Self::LockContention(_)
            | Self::Timeout(_)
            | Self::ServiceUnavailable(_) => ErrorClass::Retryable,

            Self::MergeConflict(_)
            | Self::WorkspaceNotFound(_)
            | Self::ValidationError(_)
            | Self::ConfigurationError(_)
            | Self::PermissionDenied(_) => ErrorClass::Terminal,

            Self::GitOperationFailed(_) | Self::ProcessingError(_) => ErrorClass::Terminal,
        }
    }

    /// Check if this error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self.classify(), ErrorClass::Retryable)
    }

    /// Check if this error is terminal.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self.classify(), ErrorClass::Terminal)
    }
}

/// Classify an error from a string representation.
#[must_use]
pub fn classify_error_message(error_msg: &str) -> ErrorClass {
    let lower = error_msg.to_lowercase();

    let terminal_patterns = [
        "conflict",
        "not in a workspace",
        "workspace not found",
        "validation failed",
        "invalid config",
        "permission denied",
        "access denied",
        "authentication failed",
        "not authorized",
        "branch diverged",
        "no such file",
        "does not exist",
        "malformed",
        "corrupt",
        "invalid format",
        "parse error",
        "syntax error",
    ];

    let retryable_patterns = [
        "timeout",
        "timed out",
        "connection refused",
        "connection reset",
        "network unreachable",
        "temporarily unavailable",
        "resource temporarily",
        "would block",
        "try again",
        "database is locked",
        "sqlite_busy",
        "too many connections",
        "rate limit",
        "throttl",
        "backoff",
        "retry",
        "transient",
        "interrupted",
        "deadline exceeded",
    ];

    for pattern in &terminal_patterns {
        if lower.contains(pattern) {
            return ErrorClass::Terminal;
        }
    }

    for pattern in &retryable_patterns {
        if lower.contains(pattern) {
            return ErrorClass::Retryable;
        }
    }

    ErrorClass::Terminal
}

/// Classify an error with attempt count consideration.
#[must_use]
pub fn classify_with_attempts(
    error_msg: &str,
    attempt_count: i32,
    max_attempts: i32,
) -> ErrorClass {
    if attempt_count >= max_attempts {
        return ErrorClass::Terminal;
    }

    classify_error_message(error_msg)
}

/// Determine if an error should be retried.
#[must_use]
pub fn should_retry(error_msg: &str, attempt_count: i32, max_attempts: i32) -> bool {
    matches!(
        classify_with_attempts(error_msg, attempt_count, max_attempts),
        ErrorClass::Retryable
    )
}

/// Convert an anyhow::Error to a WorkerError.
#[must_use]
pub fn from_anyhow(error: &anyhow::Error) -> WorkerError {
    let error_str = error.to_string();
    let lower = error_str.to_lowercase();

    let chain: Vec<String> = error
        .chain()
        .map(std::string::ToString::to_string)
        .collect();
    let chain_str = chain.join(" ").to_lowercase();

    if lower.contains("conflict") || chain_str.contains("conflict") {
        return WorkerError::MergeConflict(error_str);
    }

    if lower.contains("not in a workspace")
        || lower.contains("workspace not found")
        || lower.contains("no such workspace")
    {
        return WorkerError::WorkspaceNotFound(error_str);
    }

    if lower.contains("io error")
        || lower.contains("os error")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
        || lower.contains("broken pipe")
        || lower.contains("unexpected eof")
        || chain.iter().any(|s| s.contains("std::io::"))
    {
        return WorkerError::IoError(error_str);
    }

    if lower.contains("database") || lower.contains("sqlite") || lower.contains("sqlx") {
        return WorkerError::DatabaseError(error_str);
    }

    if lower.contains("lock") || lower.contains("contention") {
        return WorkerError::LockContention(error_str);
    }

    if lower.contains("timeout") || lower.contains("deadline") {
        return WorkerError::Timeout(error_str);
    }

    if lower.contains("permission") || lower.contains("access denied") {
        return WorkerError::PermissionDenied(error_str);
    }

    if lower.contains("validation") || lower.contains("invalid") {
        return WorkerError::ValidationError(error_str);
    }

    if lower.contains("config") || lower.contains("configuration") {
        return WorkerError::ConfigurationError(error_str);
    }

    WorkerError::ProcessingError(error_str)
}

/// Classify an anyhow::Error considering attempt count.
#[must_use]
pub fn classify_anyhow_with_attempts(
    error: &anyhow::Error,
    attempt_count: i32,
    max_attempts: i32,
) -> ErrorClass {
    if attempt_count >= max_attempts {
        return ErrorClass::Terminal;
    }

    let worker_error = from_anyhow(error);
    worker_error.classify()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_conflict_is_terminal() {
        let error = WorkerError::MergeConflict("branches diverged".into());
        assert_eq!(error.classify(), ErrorClass::Terminal);
        assert!(!error.is_retryable());
        assert!(error.is_terminal());
    }

    #[test]
    fn test_io_error_is_retryable() {
        let error = WorkerError::IoError("connection reset".into());
        assert_eq!(error.classify(), ErrorClass::Retryable);
        assert!(error.is_retryable());
        assert!(!error.is_terminal());
    }

    #[test]
    fn test_database_error_is_retryable() {
        let error = WorkerError::DatabaseError("database is locked".into());
        assert_eq!(error.classify(), ErrorClass::Retryable);
    }

    #[test]
    fn test_timeout_is_retryable() {
        let error = WorkerError::Timeout("operation timed out".into());
        assert_eq!(error.classify(), ErrorClass::Retryable);
    }

    #[test]
    fn test_validation_error_is_terminal() {
        let error = WorkerError::ValidationError("invalid input".into());
        assert_eq!(error.classify(), ErrorClass::Terminal);
    }

    #[test]
    fn test_permission_denied_is_terminal() {
        let error = WorkerError::PermissionDenied("access denied".into());
        assert_eq!(error.classify(), ErrorClass::Terminal);
    }

    #[test]
    fn test_classify_error_message_conflict() {
        assert_eq!(
            classify_error_message("merge conflict in file.rs"),
            ErrorClass::Terminal
        );
    }

    #[test]
    fn test_classify_error_message_timeout() {
        assert_eq!(
            classify_error_message("operation timed out after 30s"),
            ErrorClass::Retryable
        );
    }

    #[test]
    fn test_classify_error_message_database_locked() {
        assert_eq!(
            classify_error_message("database is locked"),
            ErrorClass::Retryable
        );
    }

    #[test]
    fn test_classify_error_message_not_found() {
        assert_eq!(
            classify_error_message("workspace not found"),
            ErrorClass::Terminal
        );
    }

    #[test]
    fn test_classify_error_message_unknown_defaults_terminal() {
        assert_eq!(
            classify_error_message("something weird happened"),
            ErrorClass::Terminal
        );
    }

    #[test]
    fn test_classify_with_attempts_under_max() {
        let result = classify_with_attempts("database is locked", 1, 3);
        assert_eq!(result, ErrorClass::Retryable);
    }

    #[test]
    fn test_classify_with_attempts_at_max() {
        let result = classify_with_attempts("database is locked", 3, 3);
        assert_eq!(result, ErrorClass::Terminal);
    }

    #[test]
    fn test_classify_with_attempts_over_max() {
        let result = classify_with_attempts("database is locked", 4, 3);
        assert_eq!(result, ErrorClass::Terminal);
    }

    #[test]
    fn test_should_retry_under_max() {
        assert!(should_retry("database is locked", 1, 3));
    }

    #[test]
    fn test_should_retry_at_max() {
        assert!(!should_retry("database is locked", 3, 3));
    }

    #[test]
    fn test_should_retry_terminal_error() {
        assert!(!should_retry("merge conflict", 1, 3));
    }

    #[test]
    fn test_from_anyhow_conflict() {
        let error = anyhow::anyhow!("merge conflict detected");
        let worker_error = from_anyhow(&error);
        assert!(matches!(worker_error, WorkerError::MergeConflict(_)));
    }

    #[test]
    fn test_from_anyhow_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection reset");
        let error = anyhow::Error::from(io_error);
        let worker_error = from_anyhow(&error);
        assert!(matches!(worker_error, WorkerError::IoError(_)));
    }

    #[test]
    fn test_classify_anyhow_with_attempts() {
        let error = anyhow::anyhow!("database is locked");

        let class = classify_anyhow_with_attempts(&error, 1, 3);
        assert_eq!(class, ErrorClass::Retryable);

        let class = classify_anyhow_with_attempts(&error, 3, 3);
        assert_eq!(class, ErrorClass::Terminal);
    }
}
