//! Generic JSON response wrapper for consistent API output
//!
//! All commands that support --json should use these types to ensure:
//! - Consistent `success` field in all responses
//! - Consistent `error` structure when operations fail
//! - Type-safe JSON serialization

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::Serialize;

/// Generic JSON response wrapper
///
/// All zjj commands that support --json should wrap their output in this type.
/// This ensures consistent API structure across all commands.
///
/// # Success Response
/// ```json
/// {
///   "success": true,
///   ...data fields
/// }
/// ```
///
/// # Error Response
/// ```json
/// {
///   "success": false,
///   "error": {
///     "code": "SESSION_NOT_FOUND",
///     "message": "Session 'foo' not found",
///     "suggestion": "Run 'zjj list' to see available sessions"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct JsonResponse<T> {
    /// Whether the operation succeeded
    pub success: bool,

    /// Error details (only present when success=false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,

    /// Response data (flattened into top level)
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// Standard error structure for all JSON responses
///
/// Provides semantic error codes and actionable suggestions.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorDetail {
    /// Semantic error code (e.g., "SESSION_NOT_FOUND", "VALIDATION_ERROR")
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Optional suggestion for recovery
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl<T: Serialize> JsonResponse<T> {
    /// Create a successful response with data
    #[must_use]
    pub const fn success(data: T) -> Self {
        Self {
            success: true,
            error: None,
            data: Some(data),
        }
    }

    /// Create an error response
    #[must_use]
    pub const fn failure(error: ErrorDetail) -> Self {
        Self {
            success: false,
            error: Some(error),
            data: None,
        }
    }
}

impl ErrorDetail {
    /// Create a new error detail
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            suggestion: None,
        }
    }

    /// Add details to the error
    #[must_use]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add a suggestion for recovery
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Serialize)]
    struct TestData {
        name: String,
        count: u32,
    }

    #[test]
    fn test_success_response_structure() {
        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };
        let response = JsonResponse::success(data);

        let json = serde_json::to_value(response).unwrap_or_else(|_| json!({}));
        assert_eq!(json.get("success").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(json.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(json.get("count").and_then(|v| v.as_u64()), Some(42));
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_error_response_structure() {
        let error =
            ErrorDetail::new("TEST_ERROR", "Something went wrong").with_suggestion("Try again");
        let response: JsonResponse<TestData> = JsonResponse::failure(error);

        let json = serde_json::to_value(response).unwrap_or_else(|_| json!({}));
        assert_eq!(json.get("success").and_then(|v| v.as_bool()), Some(false));

        let error_obj = json.get("error").and_then(|v| v.as_object());
        assert!(error_obj.is_some());

        let error = error_obj.unwrap_or_else(|| panic!("error object missing"));
        assert_eq!(
            error.get("code").and_then(|v| v.as_str()),
            Some("TEST_ERROR")
        );
        assert_eq!(
            error.get("message").and_then(|v| v.as_str()),
            Some("Something went wrong")
        );
        assert_eq!(
            error.get("suggestion").and_then(|v| v.as_str()),
            Some("Try again")
        );
    }
}
