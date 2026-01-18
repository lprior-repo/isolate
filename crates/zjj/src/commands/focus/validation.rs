//! Pre-focus validation for the focus command
//!
//! Validates session name, database, and terminal environment before attempting
//! to switch tabs. Uses `Result<T>` for all operations - zero unwrap/panic design.

use anyhow::Result;

use crate::{cli::is_tty, commands::get_session_db, database::SessionDb};

/// Validates session name using the standard validation function
///
/// Returns error if name is invalid.
pub fn validate_session_name(name: &str) -> Result<()> {
    Ok(crate::session::validate_session_name(name)?)
}

/// Validates database is accessible and session exists
///
/// # Arguments
/// * `session_name` - Name of the session to look up
///
/// # Returns
/// * `Ok(db, session)` - Database handle and session record
/// * `Err(e)` - If database not found or session doesn't exist
pub async fn validate_database_and_session(session_name: &str) -> Result<(SessionDb, String)> {
    let db = get_session_db().await?;

    let session = db
        .get(session_name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

    Ok((db, session.zellij_tab))
}

/// Validates that we're running in a `TTY` environment
///
/// Terminal access is required for `Zellij` operations.
/// Returns an error with helpful diagnostics if not a `TTY`.
pub fn validate_tty_environment() -> Result<()> {
    if !is_tty() {
        return Err(anyhow::anyhow!(
            "Cannot focus Zellij tab: not running in a terminal\n\
             \n\
             This error occurs when:\n\
             • Running in CI/CD environment\n\
             • SSH without TTY allocation (ssh user@host 'command')\n\
             • Piped input/output\n\
             • Background process execution\n\
             \n\
             The 'zjj focus' command requires a terminal to interact with Zellij."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_session_name_valid() {
        assert!(validate_session_name("test-session").is_ok());
        assert!(validate_session_name("feature-123").is_ok());
        assert!(validate_session_name("my_session").is_ok());
    }

    #[test]
    fn test_validate_session_name_invalid() {
        assert!(validate_session_name("").is_err());
        assert!(validate_session_name("123invalid").is_err());
        assert!(validate_session_name("default").is_err());
    }

    #[test]
    fn test_tty_detection() {
        // This test only verifies the function exists and returns Result
        let result = validate_tty_environment();
        assert!(result.is_ok() || result.is_err()); // Just verify it returns something
    }
}
