//! Session and workspace validation for sync operations

use anyhow::Result;

use crate::session::SessionStatus;

/// Validate that a session's status allows syncing
pub fn validate_session_status(status: &SessionStatus, name: &str) -> Result<()> {
    match status {
        SessionStatus::Active | SessionStatus::Paused => Ok(()),
        SessionStatus::Creating => {
            anyhow::bail!(
                "Cannot sync session '{name}': session is still being created\n\
                 \n\
                 The session is not yet ready for sync operations.\n\
                 \n\
                 Suggestions:\n\
                 • Wait for creation to complete\n\
                 • Cancel with: jjz remove {name}"
            );
        }
        SessionStatus::Failed => {
            anyhow::bail!(
                "Cannot sync session '{name}': session creation failed\n\
                 \n\
                 This session is in a failed state and cannot be synced.\n\
                 \n\
                 Suggestions:\n\
                 • Remove the failed session: jjz remove {name}\n\
                 • Recreate the session: jjz add {name}"
            );
        }
        SessionStatus::Completed => {
            anyhow::bail!(
                "Cannot sync session '{name}': session is already completed\n\
                 \n\
                 Completed sessions cannot be synced.\n\
                 \n\
                 If you want to work on this session again:\n\
                 • Create a new session: jjz add {name}-v2\n\
                 • Or reopen this session by updating its status"
            );
        }
    }
}


#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_validate_active_status() {
        let result = validate_session_status(&SessionStatus::Active, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_paused_status() {
        let result = validate_session_status(&SessionStatus::Paused, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_creating_status() {
        let result = validate_session_status(&SessionStatus::Creating, "test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("still being created"));
        }
    }

    #[test]
    fn test_validate_failed_status() {
        let result = validate_session_status(&SessionStatus::Failed, "test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("creation failed"));
        }
    }

    #[test]
    fn test_validate_completed_status() {
        let result = validate_session_status(&SessionStatus::Completed, "test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("already completed"));
        }
    }

}
