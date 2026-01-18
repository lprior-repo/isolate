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
                 • Cancel with: zjj remove {name}"
            );
        }
        SessionStatus::Failed => {
            anyhow::bail!(
                "Cannot sync session '{name}': session creation failed\n\
                 \n\
                 This session is in a failed state and cannot be synced.\n\
                 \n\
                 Suggestions:\n\
                 • Remove the failed session: zjj remove {name}\n\
                 • Recreate the session: zjj add {name}"
            );
        }
        SessionStatus::Completed => {
            anyhow::bail!(
                "Cannot sync session '{name}': session is already completed\n\
                 \n\
                 Completed sessions cannot be synced.\n\
                 \n\
                 If you want to work on this session again:\n\
                 • Create a new session: zjj add {name}-v2\n\
                 • Or reopen this session by updating its status"
            );
        }
    }
}

/// Validate that a workspace exists and is a directory
pub fn validate_workspace(workspace_path: &str, name: &str) -> anyhow::Result<()> {
    let workspace_pathbuf = std::path::Path::new(workspace_path);

    if !workspace_pathbuf.exists() {
        anyhow::bail!(
            "Workspace directory not found: {workspace_path}\n\
             \n\
             The workspace may have been deleted manually.\n\
             \n\
             Suggestions:\n\
             • Run 'zjj doctor' to detect and fix orphaned sessions\n\
             • Remove the session: zjj remove {name} --force\n\
             • Recreate the session: zjj add {name}"
        );
    }

    if !workspace_pathbuf.is_dir() {
        anyhow::bail!(
            "Workspace path is not a directory: {workspace_path}\n\
             \n\
             Expected a directory but found a file.\n\
             This indicates database corruption or manual file system changes."
        );
    }

    Ok(())
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

    #[test]
    fn test_validate_workspace_exists() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().to_string_lossy().to_string();
        let result = validate_workspace(&path, "test");
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_workspace_not_exists() {
        let result = validate_workspace("/nonexistent/path", "test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not found"));
        }
    }

    #[test]
    fn test_validate_workspace_not_directory() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let file_path = dir.path().join("file.txt");
        std::fs::write(&file_path, "test")?;
        let path = file_path.to_string_lossy().to_string();
        let result = validate_workspace(&path, "test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not a directory"));
        }
        Ok(())
    }
}
