//! Pure validation functions for done command
//!
//! These functions have no side effects and are easy to test.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::Path;

use thiserror::Error;

/// Validation errors
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    #[error("Invalid commit ID: {0}")]
    InvalidCommitId(String),

    #[error("Invalid repository path: {0}")]
    InvalidRepoPath(String),
}

/// Validate workspace name for security and correctness
pub fn validate_workspace_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::InvalidWorkspaceName(
            "Workspace name cannot be empty".to_string(),
        ));
    }

    // Security: reject path traversal
    if name.contains("..") {
        return Err(ValidationError::InvalidWorkspaceName(
            "Workspace name cannot contain '..'".to_string(),
        ));
    }

    // Security: reject path separators
    if name.contains('/') || name.contains('\\') {
        return Err(ValidationError::InvalidWorkspaceName(
            "Workspace name cannot contain path separators".to_string(),
        ));
    }

    // Security: reject null bytes
    if name.contains('\0') {
        return Err(ValidationError::InvalidWorkspaceName(
            "Workspace name cannot contain null bytes".to_string(),
        ));
    }

    Ok(())
}

/// Validate bead ID format
pub fn validate_bead_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::InvalidBeadId(
            "Bead ID cannot be empty".to_string(),
        ));
    }

    // Validate format (alphanumeric + dashes + underscores)
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ValidationError::InvalidBeadId(
            "Bead ID must contain only alphanumeric characters, dashes, or underscores".to_string(),
        ));
    }

    Ok(())
}

/// Validate commit ID (hexadecimal)
pub fn validate_commit_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::InvalidCommitId(
            "Commit ID cannot be empty".to_string(),
        ));
    }

    // Validate hexadecimal format
    if !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValidationError::InvalidCommitId(
            "Commit ID must contain only hexadecimal characters".to_string(),
        ));
    }

    Ok(())
}

/// Validate repository path
pub fn validate_repo_path(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::InvalidRepoPath(
            "Path does not exist".to_string(),
        ));
    }

    if !path.is_dir() {
        return Err(ValidationError::InvalidRepoPath(
            "Path is not a directory".to_string(),
        ));
    }

    // Check for .jj directory
    let jj_dir = path.join(".jj");
    if !jj_dir.exists() {
        return Err(ValidationError::InvalidRepoPath(
            "Not a JJ repository (no .jj directory found)".to_string(),
        ));
    }

    Ok(())
}

/// Check if workspace name is safe (no path traversal, no special chars)
pub fn is_safe_workspace_name(name: &str) -> bool {
    validate_workspace_name(name).is_ok()
}
