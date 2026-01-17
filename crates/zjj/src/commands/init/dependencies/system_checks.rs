//! System utility functions for dependency checking
//!
//! This module provides version parsing and common utilities used by all dependency checkers.
//! All functions are pure and panic-free.

use anyhow::{bail, Context, Result};
use im;

/// Parse version string like "jj 0.36.0-..." or "zellij 0.43.1" into (major, minor, patch)
///
/// This function handles various version string formats:
/// - "tool 1.2.3" -> (1, 2, 3)
/// - "tool 1.2.3-commit-hash" -> (1, 2, 3)
/// - "tool 1.2" -> (1, 2, 0)
///
/// # Arguments
///
/// * `version_str` - Version string output from tool --version command
/// * `tool_name` - Name of tool for error messages
///
/// # Examples
///
/// ```
/// # use zjj::commands::init::dependencies::parse_version;
/// assert_eq!(parse_version("jj 0.36.0", "jj").ok(), Some((0, 36, 0)));
/// assert_eq!(parse_version("jj 0.36.0-abc123", "jj").ok(), Some((0, 36, 0)));
/// assert_eq!(
///     parse_version("zellij 0.43.1", "zellij").ok(),
///     Some((0, 43, 1))
/// );
/// ```
///
/// # Errors
///
/// Returns error if:
/// - No version number found in string
/// - Version has fewer than 2 components (major.minor required)
/// - Version components are not valid u32 integers
pub fn parse_version(version_str: &str, tool_name: &str) -> Result<(u32, u32, u32)> {
    // Find the version number after the tool name
    let parts: im::Vector<&str> = version_str.split_whitespace().collect();
    let version_part = parts
        .iter()
        .find(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .context(format!("Could not find version number in: {version_str}"))?;

    // Split by '-' to handle versions like "0.36.0-commit-hash"
    // Use pattern matching to avoid unwrap
    let version_clean = version_part.split('-').next().context(
        "Failed to extract version from string (this should not happen as split always returns at least one element)"
    )?;

    // Parse major.minor.patch
    let nums: im::Vector<&str> = version_clean.split('.').collect();
    if nums.len() < 2 {
        bail!("Invalid version format for {tool_name}: {version_str}");
    }

    let major = nums[0]
        .parse::<u32>()
        .context(format!("Invalid major version: {}", nums[0]))?;
    let minor = nums[1]
        .parse::<u32>()
        .context(format!("Invalid minor version: {}", nums[1]))?;
    let patch = if nums.len() >= 3 {
        nums[2]
            .parse::<u32>()
            .context(format!("Invalid patch version: {}", nums[2]))?
    } else {
        0
    };

    Ok((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_full() -> Result<()> {
        assert_eq!(parse_version("jj 0.36.0", "jj")?, (0, 36, 0));
        assert_eq!(parse_version("zellij 0.43.1", "zellij")?, (0, 43, 1));
        Ok(())
    }

    #[test]
    fn test_parse_version_with_commit_hash() -> Result<()> {
        assert_eq!(
            parse_version("jj 0.36.0-70fd8f7697fbc20a9329a6e2f790ef86a8e284d1", "jj")?,
            (0, 36, 0)
        );
        Ok(())
    }

    #[test]
    fn test_parse_version_major_minor_only() -> Result<()> {
        assert_eq!(parse_version("tool 1.2", "tool")?, (1, 2, 0));
        Ok(())
    }

    #[test]
    fn test_parse_version_with_extra_text() -> Result<()> {
        assert_eq!(
            parse_version("My Tool Version 2.5.1 (build 123)", "tool")?,
            (2, 5, 1)
        );
        Ok(())
    }

    #[test]
    fn test_parse_version_invalid_no_version() {
        let result = parse_version("jj without version", "jj");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_version_invalid_single_component() {
        let result = parse_version("tool 1", "tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_version_invalid_non_numeric() {
        let result = parse_version("tool a.b.c", "tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_version_tuple_comparison() {
        // Test tuple ordering works correctly for version comparison
        assert!((0, 8, 0) < (0, 8, 1));
        assert!((0, 8, 0) < (0, 9, 0));
        assert!((0, 8, 0) < (1, 0, 0));
        assert!((0, 36, 0) >= (0, 8, 0));
        assert!((0, 43, 1) >= (0, 35, 1));
    }
}
