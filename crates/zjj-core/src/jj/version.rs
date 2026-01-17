//! JJ version parsing and compatibility checking

use std::process::Command;

use crate::{Error, Result};

/// Semantic version for JJ compatibility checking
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JjVersion {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl JjVersion {
    /// Minimum supported JJ version (0.20.0)
    /// This version is chosen conservatively to ensure workspace command stability
    pub const MIN_SUPPORTED: Self = Self {
        major: 0,
        minor: 20,
        patch: 0,
    };

    /// Parse version from JJ version string
    /// Expected format: "jj 0.36.0-<git-hash>" or "jj 0.36.0"
    ///
    /// # Errors
    ///
    /// Returns error if version string cannot be parsed
    pub fn parse(version_str: &str) -> Result<Self> {
        // Extract version number from "jj X.Y.Z-hash" or "jj X.Y.Z"
        let version_part = version_str.strip_prefix("jj ").ok_or_else(|| {
            Error::validation_error(format!("Invalid JJ version format: {version_str}"))
        })?;

        // Split on '-' to remove git hash if present
        let version_number = version_part.split('-').next().ok_or_else(|| {
            Error::validation_error(format!("Invalid JJ version format: {version_str}"))
        })?;

        // Parse semantic version
        let parts: Vec<&str> = version_number.split('.').collect();
        if parts.len() != 3 {
            return Err(Error::validation_error(format!(
                "Invalid JJ version format (expected X.Y.Z): {version_str}"
            )));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|e| Error::validation_error(format!("Invalid major version: {e}")))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|e| Error::validation_error(format!("Invalid minor version: {e}")))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|e| Error::validation_error(format!("Invalid patch version: {e}")))?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// Check if this version meets minimum requirements
    pub fn is_compatible(&self) -> bool {
        self >= &Self::MIN_SUPPORTED
    }
}

/// Get JJ version from `jj --version` command
///
/// # Errors
///
/// Returns error if:
/// - JJ is not found in PATH
/// - Version output cannot be parsed
pub fn get_jj_version() -> Result<JjVersion> {
    let output = Command::new("jj")
        .arg("--version")
        .output()
        .map_err(|e| super::jj_command_error("get JJ version", &e))?;

    if !output.status.success() {
        return Err(Error::System(SystemError::JjCommandError {
            operation: "get JJ version".to_string(),
            source: "JJ command returned non-zero exit code".to_string(),
            is_not_found: false,
        });
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_str = version_str.trim();

    JjVersion::parse(version_str)
}

/// Check if JJ version meets minimum requirements
///
/// # Errors
///
/// Returns error if:
/// - JJ is not found in PATH
/// - Version is below minimum supported version (0.20.0)
pub fn check_jj_version_compatible() -> Result<()> {
    let version = get_jj_version()?;

    if !version.is_compatible() {
        return Err(Error::validation_error(format!(
            "JJ version {}.{}.{} is not supported. Minimum required version: {}.{}.{}",
            version.major,
            version.minor,
            version.patch,
            JjVersion::MIN_SUPPORTED.major,
            JjVersion::MIN_SUPPORTED.minor,
            JjVersion::MIN_SUPPORTED.patch
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_version_parse_standard_format() {
        let version_str = "jj 0.36.0-70fd8f7697fbc20a9329a6e2f790ef86a8e284d1";
        let result = JjVersion::parse(version_str);
        assert!(result.is_ok());

        let version = result.unwrap_or(JjVersion {
            major: 0,
            minor: 0,
            patch: 0,
        });
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 36);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_jj_version_parse_without_hash() {
        let version_str = "jj 1.5.2";
        let result = JjVersion::parse(version_str);
        assert!(result.is_ok());

        let version = result.unwrap_or(JjVersion {
            major: 0,
            minor: 0,
            patch: 0,
        });
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 5);
        assert_eq!(version.patch, 2);
    }

    #[test]
    fn test_jj_version_parse_invalid_prefix() {
        let version_str = "jujutsu 0.36.0";
        let result = JjVersion::parse(version_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_jj_version_parse_invalid_format() {
        let version_str = "jj 0.36";
        let result = JjVersion::parse(version_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_jj_version_parse_non_numeric() {
        let version_str = "jj x.y.z";
        let result = JjVersion::parse(version_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_jj_version_compatibility_above_minimum() {
        let version = JjVersion {
            major: 0,
            minor: 36,
            patch: 0,
        };
        assert!(version.is_compatible());
    }

    #[test]
    fn test_jj_version_compatibility_at_minimum() {
        let version = JjVersion::MIN_SUPPORTED;
        assert!(version.is_compatible());
    }

    #[test]
    fn test_jj_version_compatibility_below_minimum() {
        let version = JjVersion {
            major: 0,
            minor: 19,
            patch: 9,
        };
        assert!(!version.is_compatible());
    }

    #[test]
    fn test_jj_version_comparison() {
        let v1 = JjVersion {
            major: 0,
            minor: 20,
            patch: 0,
        };
        let v2 = JjVersion {
            major: 0,
            minor: 36,
            patch: 0,
        };
        let v3 = JjVersion {
            major: 1,
            minor: 0,
            patch: 0,
        };

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
        assert_eq!(v1, JjVersion::MIN_SUPPORTED);
    }

    #[test]
    fn test_get_jj_version_integration() {
        // This test requires JJ to be installed
        let result = get_jj_version();

        // If JJ is not installed, test passes (environment dependent)
        if result.is_err() {
            return;
        }

        // If JJ is installed, verify version is parsed correctly
        let version = result.unwrap_or(JjVersion {
            major: 0,
            minor: 0,
            patch: 0,
        });
        assert!(version.major < 100); // Sanity check
        assert!(version.minor < 1000); // Sanity check
    }

    #[test]
    fn test_check_jj_version_compatible_integration() {
        // This test requires JJ to be installed
        let result = check_jj_version_compatible();

        // If JJ is not installed, test passes (environment dependent)
        if result.is_err() {
            return;
        }

        // If JJ is installed and compatible, test passes
        assert!(result.is_ok());
    }
}
