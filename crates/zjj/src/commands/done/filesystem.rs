//! Filesystem abstraction for testability
//!
//! This module provides a trait for filesystem operations, allowing
//! for in-memory testing.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use thiserror::Error;

/// Filesystem operation errors
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FsError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid UTF-8 in file: {0}")]
    InvalidUtf8(String),

    #[error("IO error: {0}")]
    IoError(String),
}

/// Trait for filesystem operations
pub trait FileSystem {
    /// Read a file to string
    fn read_to_string(&self, path: &Path) -> Result<String, FsError>;

    /// Write string to file
    fn write(&self, path: &Path, contents: &str) -> Result<(), FsError>;

    /// Check if file exists
    fn exists(&self, path: &Path) -> bool;

    /// Remove a file
    fn remove_file(&self, path: &Path) -> Result<(), FsError>;

    /// Remove a directory and all its contents
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError>;
}

/// Real filesystem implementation
#[derive(Debug, Default)]
pub struct RealFileSystem;

impl RealFileSystem {
    /// Create a new RealFileSystem
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FsError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FsError::PermissionDenied(path.display().to_string())
            } else {
                FsError::IoError(e.to_string())
            }
        })
    }

    fn write(&self, path: &Path, contents: &str) -> Result<(), FsError> {
        fs::write(path, contents).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                FsError::PermissionDenied(path.display().to_string())
            } else {
                FsError::IoError(e.to_string())
            }
        })
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        fs::remove_file(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FsError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FsError::PermissionDenied(path.display().to_string())
            } else {
                FsError::IoError(e.to_string())
            }
        })
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        fs::remove_dir_all(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FsError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FsError::PermissionDenied(path.display().to_string())
            } else {
                FsError::IoError(e.to_string())
            }
        })
    }
}

/// In-memory filesystem for testing
#[derive(Debug, Clone)]
pub struct InMemoryFileSystem {
    files: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryFileSystem {
    /// Create a new InMemoryFileSystem
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for InMemoryFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        let key = path.display().to_string();
        self.files
            .lock()
            .map_err(|e| FsError::IoError(format!("Lock poisoned: {}", e)))?
            .get(&key)
            .cloned()
            .ok_or_else(|| FsError::NotFound(key))
    }

    fn write(&self, path: &Path, contents: &str) -> Result<(), FsError> {
        let key = path.display().to_string();
        self.files
            .lock()
            .map_err(|e| FsError::IoError(format!("Lock poisoned: {}", e)))?
            .insert(key, contents.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let key = path.display().to_string();
        self.files
            .lock()
            .map(|files| files.contains_key(&key))
            .unwrap_or(false)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        let key = path.display().to_string();
        self.files
            .lock()
            .map_err(|e| FsError::IoError(format!("Lock poisoned: {}", e)))?
            .remove(&key)
            .ok_or_else(|| FsError::NotFound(key))?;
        Ok(())
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let prefix = path.display().to_string();
        let mut files = self
            .files
            .lock()
            .map_err(|e| FsError::IoError(format!("Lock poisoned: {}", e)))?;

        // Remove all files that start with this path prefix
        files.retain(|k, _| !k.starts_with(&prefix));
        Ok(())
    }
}
