//! Validator for session name format and rules
//!
//! This module validates session names according to the rules defined
//! in the `crate::session` module, delegating the actual validation logic
//! to maintain a single source of truth for name validation rules.

use anyhow::{Context, Result};

/// Validate session name format and rules
///
/// Session names must:
/// - Start with a letter (a-z, A-Z)
/// - Contain only ASCII alphanumeric characters, dashes, underscores, and periods
/// - Be non-empty and not exceed 255 characters
/// - Not be reserved names (default, root)
///
/// # Errors
/// Returns error if name is invalid, empty, too long, contains invalid characters,
/// or is a reserved name
pub fn validate_session_name(name: &str) -> Result<()> {
    // Use the existing session validation function
    crate::session::validate_session_name(name).context("Session name validation failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_session_names() {
        let valid_names = vec![
            "feature",
            "my-feature",
            "myFeature",
            "feature123",
            "f",
            "Feature-123",
        ];

        for name in valid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_ok(),
                "Expected '{name}' to be valid, got: {result:?}"
            );
        }
    }

    #[test]
    fn test_invalid_session_names_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_session_names_special_chars() {
        let invalid_names = vec![
            "my session",  // space
            "my@session",  // @
            "my/session",  // slash
            "my\\session", // backslash
            "my#session",  // hash
        ];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Expected '{name}' to be invalid, but validation passed"
            );
        }
    }

    #[test]
    fn test_valid_session_names_with_period() {
        let valid_names = vec!["my.session", "feature.v1", "test.feature.branch"];

        for name in valid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_ok(),
                "Expected '{name}' to be valid, got: {result:?}"
            );
        }
    }

    #[test]
    fn test_invalid_session_names_unicode() {
        let unicode_names = vec!["café", "session名", "🚀rocket", "naïve"];

        for name in unicode_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Expected unicode name '{name}' to be rejected"
            );
        }
    }

    #[test]
    fn test_invalid_session_names_starts_with_dash() {
        let result = validate_session_name("-session");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_session_names_starts_with_underscore() {
        let result = validate_session_name("_session");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_session_names_starts_with_digit() {
        let result = validate_session_name("123session");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_session_names_too_long() {
        let long_name = "a".repeat(256);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }
}
