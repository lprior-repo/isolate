//! Validator for checking Zellij availability
//!
//! This module validates that Zellij is running and accessible
//! by checking environment variables and socket availability.

use anyhow::Context;
use anyhow::Result;

use crate::commands::add::error_messages;

/// Validate that Zellij is running and accessible
///
/// This check verifies that we are inside a Zellij session
/// by checking if the ZELLIJ environment variable is set.
///
/// # Errors
/// Returns error if:
/// - ZELLIJ environment variable is not set (not inside Zellij)
pub fn validate_zellij_running() -> Result<()> {
    // Check if ZELLIJ environment variable is set
    std::env::var("ZELLIJ").context(error_messages::ZELLIJ_NOT_SET)?;

    // Additional check: verify zellij socket exists if we can determine it
    // For now, just checking env var is sufficient as it indicates we're inside Zellij
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_zellij_running_outside_zellij() {
        // When running outside Zellij, this should fail
        if std::env::var("ZELLIJ").is_err() {
            let result = validate_zellij_running();
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_error_message_zellij_not_set() {
        assert!(!error_messages::ZELLIJ_NOT_SET.is_empty());
        // Message should mention Zellij and suggest --no-open flag
        let msg_lower = error_messages::ZELLIJ_NOT_SET.to_lowercase();
        assert!(msg_lower.contains("zellij"));
        assert!(error_messages::ZELLIJ_NOT_SET.contains("--no-open"));
    }
}
