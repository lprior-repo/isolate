//! Validation error codes for input validation and parsing failures.
//!
//! These errors occur when inputs fail validation or have syntax/format issues.

use serde::{Deserialize, Serialize};

/// Validation error codes for input validation and format errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationError {
    /// Session name fails validation rules
    SessionNameInvalid,
    /// Session status transition not allowed
    SessionInvalidTransition,
    /// Configuration file has syntax errors
    ConfigParseError,
    /// Configuration value has invalid type
    ConfigInvalidValue,
    /// Generic validation error
    InvalidArgument,
}

impl ValidationError {
    /// Get the string representation of the validation error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::SessionInvalidTransition => "SESSION_INVALID_TRANSITION",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigInvalidValue => "CONFIG_INVALID_VALUE",
            Self::InvalidArgument => "INVALID_ARGUMENT",
        }
    }

    /// Get a human-readable description of the validation error
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::SessionNameInvalid => "Session name contains invalid characters or format",
            Self::SessionInvalidTransition => "Cannot transition session to requested state",
            Self::ConfigParseError => "Configuration file has syntax errors",
            Self::ConfigInvalidValue => "Configuration value has invalid type",
            Self::InvalidArgument => "Invalid argument provided",
        }
    }

    /// Get suggested action to resolve the validation error
    #[must_use]
    pub const fn suggestion(self) -> Option<&'static str> {
        match self {
            Self::SessionNameInvalid => Some(
                "Use only letters, numbers, hyphens, and underscores. Must start with a letter",
            ),
            Self::SessionInvalidTransition => {
                Some("Check session status with 'jjz status' before attempting operation")
            }
            Self::ConfigParseError => {
                Some("Check configuration file syntax. Refer to documentation for format")
            }
            Self::ConfigInvalidValue => {
                Some("Check configuration value type matches expected format")
            }
            Self::InvalidArgument => Some("Check command syntax with 'jjz --help'"),
        }
    }

    /// HTTP status code equivalent (for potential future REST API)
    #[must_use]
    pub const fn http_status(self) -> u16 {
        // 422: Unprocessable Entity (validation errors)
        422
    }
}

impl From<ValidationError> for String {
    fn from(code: ValidationError) -> Self {
        code.as_str().to_string()
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_as_str() {
        assert_eq!(
            ValidationError::SessionNameInvalid.as_str(),
            "SESSION_NAME_INVALID"
        );
        assert_eq!(
            ValidationError::ConfigParseError.as_str(),
            "CONFIG_PARSE_ERROR"
        );
        assert_eq!(
            ValidationError::InvalidArgument.as_str(),
            "INVALID_ARGUMENT"
        );
    }

    #[test]
    fn test_validation_error_description() {
        let desc = ValidationError::SessionNameInvalid.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("invalid"));
    }

    #[test]
    fn test_validation_error_suggestion() {
        let suggestion = ValidationError::SessionNameInvalid.suggestion();
        assert!(suggestion.is_some());
        let text = suggestion
            .map(|s| s.contains("letters"))
            .unwrap_or_else(|| false);
        assert!(text);
    }

    #[test]
    fn test_validation_error_http_status() {
        assert_eq!(ValidationError::SessionNameInvalid.http_status(), 422);
        assert_eq!(ValidationError::ConfigParseError.http_status(), 422);
        assert_eq!(ValidationError::InvalidArgument.http_status(), 422);
    }

    #[test]
    fn test_validation_error_to_string() {
        let code: String = ValidationError::SessionNameInvalid.into();
        assert_eq!(code, "SESSION_NAME_INVALID");
    }

    #[test]
    fn test_validation_error_display() {
        let code = ValidationError::ConfigInvalidValue;
        assert_eq!(format!("{code}"), "CONFIG_INVALID_VALUE");
    }

    #[test]
    fn test_validation_serialization() {
        let code = ValidationError::SessionNameInvalid;
        let result = serde_json::to_string(&code);
        assert!(result.is_ok(), "Serialization should succeed");
        assert_eq!(result.ok(), Some("\"SESSION_NAME_INVALID\"".to_string()));
    }

    #[test]
    fn test_validation_deserialization() {
        let json = "\"SESSION_NAME_INVALID\"";
        let result = serde_json::from_str::<ValidationError>(json);
        assert!(result.is_ok(), "Deserialization should succeed");
        assert_eq!(result.ok(), Some(ValidationError::SessionNameInvalid));
    }
}
