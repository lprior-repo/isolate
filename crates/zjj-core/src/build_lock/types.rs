//! Type definitions for build lock system.
//!
//! Provides error types, domain types, and the lock acquisition result enum.

use std::{io, path::PathBuf};
use thiserror::Error;

/// Comprehensive error type for build lock operations.
///
/// All failure modes are represented as distinct variants with full context.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BuildLockError {
    /// Lock directory could not be created
    #[error("failed to create lock directory at {path}: {source}")]
    LockDirectoryCreationFailed {
        path: PathBuf,
        #[source]
        source: IoErrorKind,
    },

    /// Lock file could not be opened or created
    #[error("failed to open lock file at {path}: {source}")]
    LockFileOpenFailed {
        path: PathBuf,
        #[source]
        source: IoErrorKind,
    },

    /// Lock file could not be released (best-effort, logged not fatal)
    #[error("failed to release lock file at {path}: {source}")]
    LockReleaseFailed {
        path: PathBuf,
        #[source]
        source: IoErrorKind,
    },

    /// Timeout waiting to acquire lock
    #[error("timeout waiting for build lock after {timeout_secs} seconds")]
    LockAcquisitionTimeout { timeout_secs: u64 },

    /// Failed to read PID from lock file
    #[error("failed to read PID from lock file: {source}")]
    PidReadFailed {
        #[source]
        source: IoErrorKind,
    },

    /// Lock file contains invalid PID data
    #[error("invalid PID in lock file: '{raw}'")]
    InvalidPid { raw: String },

    /// Failed to detect if process is still alive
    #[error("failed to check if process is alive: {source}")]
    StaleDetectionFailed {
        #[source]
        source: IoErrorKind,
    },

    /// Invalid configuration (precondition violation)
    #[error("invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },

    /// File locking operation failed
    #[error("file locking failed: {source}")]
    LockOperationFailed {
        #[source]
        source: IoErrorKind,
    },
}

/// IO error kinds (cloneable, no source chain issues)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    TimedOut,
    WriteZero,
    Interrupted,
    UnexpectedEof,
    OutOfMemory,
    Other(String),
}

impl std::fmt::Display for IoErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::AlreadyExists => write!(f, "already exists"),
            Self::WouldBlock => write!(f, "would block"),
            Self::InvalidInput => write!(f, "invalid input"),
            Self::TimedOut => write!(f, "timed out"),
            Self::WriteZero => write!(f, "write zero"),
            Self::Interrupted => write!(f, "interrupted"),
            Self::UnexpectedEof => write!(f, "unexpected EOF"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for IoErrorKind {}

impl From<io::Error> for IoErrorKind {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            io::ErrorKind::WouldBlock => Self::WouldBlock,
            io::ErrorKind::InvalidInput => Self::InvalidInput,
            io::ErrorKind::TimedOut => Self::TimedOut,
            io::ErrorKind::WriteZero => Self::WriteZero,
            io::ErrorKind::Interrupted => Self::Interrupted,
            io::ErrorKind::UnexpectedEof => Self::UnexpectedEof,
            io::ErrorKind::OutOfMemory => Self::OutOfMemory,
            _ => Self::Other(err.to_string()),
        }
    }
}

/// Build lock handle with guaranteed RAII cleanup.
///
/// When dropped, the lock file is automatically removed, ensuring other processes
/// can acquire the lock. This prevents resource leaks even on panic.
#[derive(Debug)]
pub struct BuildLock {
    pub(super) lock_file: PathBuf,
    #[allow(dead_code)] // Held for lifetime, dropped for side effect
    pub(super) lock_fd: std::fs::File,
}

impl Drop for BuildLock {
    fn drop(&mut self) {
        // Best-effort cleanup - don't panic in Drop

        // Check if lock file still exists (detect external deletion)
        if !self.lock_file.exists() {
            eprintln!(
                "CRITICAL: Lock file {} was deleted while held! Mutual exclusion may be violated.",
                self.lock_file.display()
            );
            return;
        }

        let _ = std::fs::remove_file(&self.lock_file).map_err(|e| {
            eprintln!(
                "Warning: failed to remove lock file {}: {}",
                self.lock_file.display(),
                e
            );
        });
    }
}

/// Build coordinator managing lock acquisition with timeout and polling.
///
/// # Invariants
///
/// - `timeout > Duration::ZERO`
/// - `poll_interval < timeout`
/// - `lock_dir` exists and is writable
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCoordinator {
    pub(super) lock_dir: PathBuf,
    pub(super) timeout: std::time::Duration,
    pub(super) poll_interval: std::time::Duration,
}

/// Result of lock acquisition attempt (discriminated union, no panic)
#[derive(Debug)]
pub enum LockAcquisition {
    /// Lock successfully acquired
    Acquired(BuildLock),
    /// Lock already held by another process
    AlreadyHeld { holder_pid: u32 },
    /// Timeout waiting for lock
    Timeout,
}

/// Internal type for lock contention handling.
pub(super) enum LockContention {
    AlreadyLocked(u32),
    IoError(BuildLockError),
}
