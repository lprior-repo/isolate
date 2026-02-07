//! Filesystem abstraction for testability
//!
//! This module provides a trait for filesystem operations, allowing
//! for in-memory testing.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{
    collections::HashMap,
    future::Future,
    path::Path,
    pin::Pin,
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

    #[expect(dead_code)] // Reserved for binary file handling
    #[error("Invalid UTF-8 in file: {0}")]
    InvalidUtf8(String),

    #[error("IO error: {0}")]
    IoError(String),
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait for filesystem operations
pub trait FileSystem: Send + Sync {
    /// Read a file to string
    fn read_to_string<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<String, FsError>>;

    /// Write string to file
    fn write<'a>(&'a self, path: &'a Path, contents: &'a str)
        -> BoxFuture<'a, Result<(), FsError>>;

    /// Check if file exists
    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool>;

    /// Remove a file
    #[expect(dead_code)] // For future file cleanup operations
    fn remove_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>>;

    /// Remove a directory and all its contents
    fn remove_dir_all<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>>;
}

/// Real filesystem implementation
#[derive(Debug, Default)]
pub struct RealFileSystem;

impl RealFileSystem {
    /// Create a new `RealFileSystem`
    pub const fn new() -> Self {
        Self
    }
}

impl FileSystem for RealFileSystem {
    fn read_to_string<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<String, FsError>> {
        Box::pin(async move {
            tokio::fs::read_to_string(path).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    FsError::NotFound(path.display().to_string())
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FsError::PermissionDenied(path.display().to_string())
                } else {
                    FsError::IoError(e.to_string())
                }
            })
        })
    }

    fn write<'a>(
        &'a self,
        path: &'a Path,
        contents: &'a str,
    ) -> BoxFuture<'a, Result<(), FsError>> {
        Box::pin(async move {
            tokio::fs::write(path, contents).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FsError::PermissionDenied(path.display().to_string())
                } else {
                    FsError::IoError(e.to_string())
                }
            })
        })
    }

    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool> {
        Box::pin(async move { tokio::fs::try_exists(path).await.unwrap_or(false) })
    }

    fn remove_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>> {
        Box::pin(async move {
            tokio::fs::remove_file(path).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    FsError::NotFound(path.display().to_string())
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FsError::PermissionDenied(path.display().to_string())
                } else {
                    FsError::IoError(e.to_string())
                }
            })
        })
    }

    fn remove_dir_all<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>> {
        Box::pin(async move {
            tokio::fs::remove_dir_all(path).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    FsError::NotFound(path.display().to_string())
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FsError::PermissionDenied(path.display().to_string())
                } else {
                    FsError::IoError(e.to_string())
                }
            })
        })
    }
}

/// In-memory filesystem for testing
#[allow(dead_code)] // Reserved for future test expansion
#[derive(Debug, Clone)]
pub struct InMemoryFileSystem {
    files: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryFileSystem {
    /// Create a new `InMemoryFileSystem`
    #[allow(dead_code)] // Reserved for future test expansion
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
    fn read_to_string<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<String, FsError>> {
        Box::pin(async move {
            let key = path.display().to_string();
            self.files
                .lock()
                .map_err(|e| FsError::IoError(format!("Lock poisoned: {e}")))?
                .get(&key)
                .cloned()
                .ok_or(FsError::NotFound(key))
        })
    }

    fn write<'a>(
        &'a self,
        path: &'a Path,
        contents: &'a str,
    ) -> BoxFuture<'a, Result<(), FsError>> {
        let contents = contents.to_string();
        Box::pin(async move {
            let key = path.display().to_string();
            self.files
                .lock()
                .map_err(|e| FsError::IoError(format!("Lock poisoned: {e}")))?
                .insert(key, contents);
            Ok(())
        })
    }

    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            let key = path.display().to_string();
            self.files
                .lock()
                .map(|files| files.contains_key(&key))
                .unwrap_or(false)
        })
    }

    fn remove_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>> {
        Box::pin(async move {
            let key = path.display().to_string();
            self.files
                .lock()
                .map_err(|e| FsError::IoError(format!("Lock poisoned: {e}")))?
                .remove(&key)
                .ok_or(FsError::NotFound(key))?;
            Ok(())
        })
    }

    fn remove_dir_all<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>> {
        Box::pin(async move {
            let prefix = path.display().to_string();
            self.files
                .lock()
                .map_err(|e| FsError::IoError(format!("Lock poisoned: {e}")))?
                .retain(|k, _| !k.starts_with(&prefix));
            Ok(())
        })
    }
}
