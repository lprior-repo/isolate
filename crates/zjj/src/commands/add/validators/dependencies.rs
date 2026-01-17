//! Validator for checking required dependencies
//!
//! This module validates that all required external commands (jj and zellij)
//! are installed and available in the system PATH.

use anyhow::Context;
use anyhow::Result;

use crate::commands::add::error_messages;

/// Validate that required dependencies (jj and zellij) are installed
///
/// This check verifies that both jj (Jujutsu) and zellij are installed
/// and available in the system PATH before attempting to use them.
///
/// # Errors
/// Returns error if:
/// - jj executable is not found in PATH
/// - zellij executable is not found in PATH
pub fn validate_dependencies() -> Result<()> {
    // Check for jj
    which::which("jj").context(error_messages::JJ_NOT_FOUND)?;

    // Check for zellij
    which::which("zellij").context(error_messages::ZELLIJ_NOT_FOUND)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_dependencies_jj_exists() {
        // This test will pass if jj is installed, skip otherwise
        if which::which("jj").is_ok() {
            let result = validate_dependencies();
            // Should pass if both jj and zellij are installed
            // If zellij is missing, it's ok for this test
            match result {
                Ok(()) => {}
                Err(e) => {
                    // It's ok if zellij is missing, but jj must be found
                    assert!(
                        e.to_string().contains("zellij"),
                        "Expected zellij error if dependencies failed, got: {e}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_error_messages_defined() {
        assert!(!error_messages::JJ_NOT_FOUND.is_empty());
        assert!(!error_messages::ZELLIJ_NOT_FOUND.is_empty());
        assert!(error_messages::JJ_NOT_FOUND.contains("jj"));
        assert!(error_messages::ZELLIJ_NOT_FOUND.contains("zellij"));
    }
}
