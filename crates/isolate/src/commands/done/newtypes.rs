//! `NewType` wrappers for done command
//!
//! These types provide compile-time validation and type safety.
//! All validation happens at construction time via TryFrom/From.
//!
//! # Single Source of Truth
//!
//! This module re-exports BeadId from isolate_core::domain::identifiers to maintain
//! consistency across the codebase.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::{
    fmt,
    path::{Path, PathBuf},
};

use thiserror::Error;

// Re-export BeadId from domain layer (single source of truth)
//
// BeadId is a type alias for TaskId in the domain layer, validating bd-{hex} format.
pub use isolate_core::domain::BeadId;

// Re-export WorkspaceName from domain layer (single source of truth)
//
// WorkspaceName validates workspace names (non-empty, no path separators, max 255 chars).
// The canonical implementation uses `WorkspaceName::parse()` for construction.
pub use isolate_core::domain::WorkspaceName;

/// Validation errors for `NewType` constructors
#[expect(clippy::enum_variant_names)] // All validation errors should be "Invalid*"
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[allow(dead_code)] // Reserved for future path validation
    #[error("Invalid repository path: {0}")]
    InvalidRepoPath(String),

    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    #[allow(dead_code)] // Reserved for future commit validation
    #[error("Invalid commit ID: {0}")]
    InvalidCommitId(String),

    #[error("Invalid JJ output: {0}")]
    InvalidJjOutput(String),
}

/// Validated repository root path
#[allow(dead_code)] // Reserved for future strict path validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRoot(PathBuf);

impl RepoRoot {
    /// Create a new `RepoRoot` with validation
    #[allow(dead_code)] // Reserved for future strict path validation
    pub async fn new(path: PathBuf) -> Result<Self, ValidationError> {
        match tokio::fs::try_exists(&path).await {
            Ok(true) => {
                if !path.is_dir() {
                    return Err(ValidationError::InvalidRepoPath(
                        "Path is not a directory".to_string(),
                    ));
                }

                // Check for .jj directory
                let jj_dir = path.join(".jj");
                match tokio::fs::try_exists(&jj_dir).await {
                    Ok(true) => Ok(Self(path)),
                    _ => Err(ValidationError::InvalidRepoPath(
                        "Not a JJ repository (no .jj directory found)".to_string(),
                    )),
                }
            }
            _ => Err(ValidationError::InvalidRepoPath(
                "Path does not exist".to_string(),
            )),
        }
    }

    /// Get the inner `PathBuf`
    #[expect(dead_code)] // For future strict path validation
    pub const fn inner(&self) -> &PathBuf {
        &self.0
    }

    /// Get as `Path`
    #[expect(dead_code)] // For future strict path validation
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl fmt::Display for RepoRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// Validated commit ID
#[allow(dead_code)] // Reserved for future commit ID validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new `CommitId` with validation
    #[allow(dead_code)] // Reserved for future commit ID validation
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
    #[expect(dead_code)] // For future direct access needs
    pub fn inner(&self) -> &str {
        &self.0
    }

    /// Get as string slice
    #[expect(dead_code)] // For future direct access needs
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
    #[expect(dead_code)] // For future direct access needs
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
