//! Infrastructure validation layer for sessions
//!
//! This module provides I/O-based validation that should be called from
//! infrastructure/services layer, not from domain types directly.
//!
//! # Design Principle
//!
//! Domain types (`Session`, etc.) should have pure validation methods that
//! don't perform I/O operations. Filesystem and network checks belong here
//! in the infrastructure layer.

use crate::{
    types::{Session, SessionStatus},
    Error, Result,
};

/// Validate that a session's workspace path exists on the filesystem.
///
/// This is an I/O operation and should only be called from the infrastructure
/// layer, not from domain type constructors or pure validation methods.
///
/// # Errors
///
/// Returns `Error::ValidationError` if:
/// - Session status is not `Creating` AND
/// - The workspace path does not exist on the filesystem
///
/// # Example
///
/// ```ignore
/// use zjj_core::validation::validate_session_workspace_exists;
///
/// let session = Session { /* ... */ };
/// validate_session_workspace_exists(&session)?;
/// ```
pub fn validate_session_workspace_exists(session: &Session) -> Result<()> {
    if session.status != SessionStatus::Creating && !session.workspace_path.exists() {
        return Err(Error::ValidationError {
            message: format!(
                "Workspace '{}' does not exist",
                session.workspace_path.display()
            ),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }
    Ok(())
}

/// Validate that a path exists on the filesystem.
///
/// This is an I/O operation for infrastructure layer use.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path does not exist.
pub fn validate_path_exists(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::ValidationError {
            message: format!("Path '{}' does not exist", path.display()),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }
    Ok(())
}

/// Validate that a path is a directory.
///
/// This is an I/O operation for infrastructure layer use.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path is not a directory.
pub fn validate_is_directory(path: &std::path::Path) -> Result<()> {
    if !path.is_dir() {
        return Err(Error::ValidationError {
            message: format!("Path '{}' is not a directory", path.display()),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }
    Ok(())
}

/// Combined validation: path exists and is a directory.
///
/// This is an I/O operation for infrastructure layer use.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path does not exist or is not a directory.
pub fn validate_workspace_path(path: &std::path::Path) -> Result<()> {
    validate_path_exists(path)?;
    validate_is_directory(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_validate_path_exists_for_tmp() {
        let result = validate_path_exists(PathBuf::from("/tmp").as_path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_exists_rejects_nonexistent() {
        let result = validate_path_exists(
            PathBuf::from("/nonexistent/path/that/should/not/exist").as_path(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_is_directory_for_tmp() {
        let result = validate_is_directory(PathBuf::from("/tmp").as_path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_is_directory_rejects_file() {
        let result = validate_is_directory(PathBuf::from("/etc/hosts").as_path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_path_for_tmp() {
        let result = validate_workspace_path(PathBuf::from("/tmp").as_path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_workspace_path_rejects_nonexistent() {
        let result = validate_workspace_path(PathBuf::from("/nonexistent/path").as_path());
        assert!(result.is_err());
    }
}
