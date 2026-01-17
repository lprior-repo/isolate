//! Query filtering and error categorization logic
//!
//! This module handles filtering operations and error categorization
//! for query operations, using pure functional transformations.

use anyhow::Result;
use zjj_core::introspection::QueryError;

/// Categorize database errors into error codes and messages
///
/// Uses pattern matching to identify common error conditions and provide
/// appropriate error codes and user-friendly messages.
pub fn categorize_db_error(error: &anyhow::Error) -> (String, String) {
    let error_msg = error.to_string();

    // Check for JJ not installed
    if error_msg.contains("JJ not installed") || error_msg.contains("jj: not found") {
        return (
            "JJ_NOT_INSTALLED".to_string(),
            "Cannot check session - JJ not installed".to_string(),
        );
    }

    // Check for not in JJ repo
    if error_msg.contains("Not in a JJ repository") || error_msg.contains("not a jj repo") {
        return (
            "NOT_JJ_REPO".to_string(),
            "Cannot check session - not in a JJ repository".to_string(),
        );
    }

    // Check for not initialized
    if error_msg.contains("not initialized") || error_msg.contains("Run 'jjz init'") {
        return (
            "NOT_INITIALIZED".to_string(),
            "Cannot check session - jjz not initialized".to_string(),
        );
    }

    // Generic database error
    (
        "DATABASE_ERROR".to_string(),
        format!("Cannot check session - {error_msg}"),
    )
}

/// Parse and extract status filter from filter argument
///
/// Handles the `--status=<value>` format, returning the status value
/// if the filter matches, or None otherwise.
pub fn extract_status_filter(filter: Option<&str>) -> Option<String> {
    filter
        .and_then(|f| f.strip_prefix("--status="))
        .map(|s| s.to_string())
}

/// Apply status filter to a slice of sessions
///
/// Functional approach that filters sessions based on their status string.
/// Returns the filtered sessions as a filtered iterator.
pub fn filter_sessions_by_status<'a, S: AsRef<str>>(
    sessions: &'a [(String, &'a str)],
    status: &'a str,
) -> impl Iterator<Item = &'a (String, &'a str)> {
    sessions.iter().filter(move |(_, s)| *s == status)
}

/// Create a QueryError from error details
pub fn query_error(code: impl Into<String>, message: impl Into<String>) -> QueryError {
    QueryError {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_jj_not_installed() {
        let err = anyhow::anyhow!("jj: not found");
        let (code, msg) = categorize_db_error(&err);
        assert_eq!(code, "JJ_NOT_INSTALLED");
        assert!(msg.contains("JJ not installed"));
    }

    #[test]
    fn test_categorize_not_jj_repo() {
        let err = anyhow::anyhow!("Not in a JJ repository");
        let (code, msg) = categorize_db_error(&err);
        assert_eq!(code, "NOT_JJ_REPO");
        assert!(msg.contains("not in a JJ repository"));
    }

    #[test]
    fn test_categorize_not_initialized() {
        let err = anyhow::anyhow!("jjz not initialized, Run 'jjz init'");
        let (code, msg) = categorize_db_error(&err);
        assert_eq!(code, "NOT_INITIALIZED");
        assert!(msg.contains("not initialized"));
    }

    #[test]
    fn test_categorize_generic_error() {
        let err = anyhow::anyhow!("Some generic error");
        let (code, msg) = categorize_db_error(&err);
        assert_eq!(code, "DATABASE_ERROR");
        assert!(msg.contains("Some generic error"));
    }

    #[test]
    fn test_extract_status_filter_with_status() {
        let result = extract_status_filter(Some("--status=active"));
        assert_eq!(result, Some("active".to_string()));
    }

    #[test]
    fn test_extract_status_filter_without_prefix() {
        let result = extract_status_filter(Some("active"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_status_filter_none() {
        let result = extract_status_filter(None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_filter_sessions_by_status() {
        let sessions = vec![
            ("session1".to_string(), "active"),
            ("session2".to_string(), "inactive"),
            ("session3".to_string(), "active"),
        ];

        let active_sessions: Vec<_> = filter_sessions_by_status(&sessions, "active").collect();
        assert_eq!(active_sessions.len(), 2);
    }

    #[test]
    fn test_query_error_creation() {
        let err = query_error("TEST_CODE", "Test message");
        assert_eq!(err.code, "TEST_CODE");
        assert_eq!(err.message, "Test message");
    }
}
