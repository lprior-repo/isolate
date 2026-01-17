//! Validator for checking if a session already exists in the database
//!
//! This module validates that a session name is not already in use
//! by checking the session database.

use anyhow::{bail, Context, Result};

use crate::commands::add::error_messages;

/// Validate that session does not already exist
///
/// This check prevents creating duplicate sessions with the same name.
///
/// # Arguments
/// * `session_db` - Database connection for checking existing sessions
/// * `name` - Session name to check
///
/// # Errors
/// Returns error if:
/// - Failed to query the database
/// - Session with the given name already exists
pub async fn validate_not_exists(
    session_db: &crate::database::SessionDb,
    name: &str,
) -> Result<()> {
    session_db
        .get(name)
        .await
        .context(error_messages::SESSION_DB_QUERY_FAILED)?
        .map_or(Ok(()), |_session| {
            bail!(error_messages::session_already_exists(name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Async tests would require database fixtures.
    // These are typically tested through integration tests.
    // Unit tests here focus on error message generation.

    #[test]
    fn test_error_message_session_exists() {
        let name = "test-session";
        let msg = error_messages::session_already_exists(name);
        assert!(msg.contains(name));
        assert!(msg.contains("already exists"));
    }
}
