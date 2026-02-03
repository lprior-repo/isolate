//! NewType wrappers for done command
//!
//! These types provide compile-time validation and type safety.
//! All validation happens at construction time via TryFrom/From.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{
    fmt,
    path::{Path, PathBuf},
};

use thiserror::Error;

/// Validation errors for NewType constructors
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Invalid repository path: {0}")]
    InvalidRepoPath(String),

    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    #[error("Invalid commit ID: {0}")]
    InvalidCommitId(String),

    #[error("Invalid JJ output: {0}")]
    InvalidJjOutput(String),
}

/// Validated repository root path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRoot(PathBuf);

impl RepoRoot {
    /// Create a new RepoRoot with validation
    pub fn new(path: PathBuf) -> Result<Self, ValidationError> {
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

        Ok(Self(path))
    }

    /// Get the inner `PathBuf`
    pub const fn inner(&self) -> &PathBuf {
        &self.0
    }

    /// Get as `Path`
    pub const fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for RepoRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// Validated workspace name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    /// Create a new `WorkspaceName` with validation
    pub fn new(name: String) -> Result<Self, ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::InvalidWorkspaceName(
                "Workspace name cannot be empty".to_string(),
            ));
        }

        // Security: reject path traversal attempts
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

        Ok(Self(name))
    }

    /// Get the inner String
    pub fn inner(&self) -> &str {
        &self.0
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated bead ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadId(String);

impl BeadId {
    /// Create a new `BeadId` with validation
    pub fn new(id: String) -> Result<Self, ValidationError> {
        if id.is_empty() {
            return Err(ValidationError::InvalidBeadId(
                "Bead ID cannot be empty".to_string(),
            ));
        }

        // Validate format (basic alphanumeric + dashes)
        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::InvalidBeadId(
                "Bead ID must contain only alphanumeric characters, dashes, or underscores"
                    .to_string(),
            ));
        }

        Ok(Self(id))
    }

    /// Get the inner String
    pub fn inner(&self) -> &str {
        &self.0
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BeadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated commit ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new `CommitId` with validation
    pub fn new(id: String) -> Result<Self, ValidationError> {
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

        Ok(Self(id))
    }

    /// Get the inner String
    pub fn inner(&self) -> &str {
        &self.0
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated JJ command output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjOutput(String);

impl JjOutput {
    /// Create a new `JjOutput` with validation
    pub fn new(output: String) -> Result<Self, ValidationError> {
        // Validate UTF-8 (already validated by String type, but we check for control chars)
        if output.chars().any(|c| c == '\0') {
            return Err(ValidationError::InvalidJjOutput(
                "Output contains null bytes".to_string(),
            ));
        }

        Ok(Self(output))
    }

    /// Get the inner String
    pub fn inner(&self) -> &str {
        &self.0
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JjOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
