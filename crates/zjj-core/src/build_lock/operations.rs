//! Lock operations for build lock system.
//!
//! File operations, lock acquisition, and cleanup procedures with concurrent access patterns.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    thread,
    time::{Duration, Instant},
};

use fs2::FileExt;

use super::queries::{is_process_alive, parse_pid, validate_poll_interval, validate_timeout};
use super::types::{
    BuildCoordinator, BuildLock, BuildLockError, IoErrorKind, LockAcquisition, LockContention,
};

/// Create lock directory if it doesn't exist.
pub(super) fn create_lock_directory(lock_dir: &Path) -> Result<(), BuildLockError> {
    fs::create_dir_all(lock_dir).map_err(|e| BuildLockError::LockDirectoryCreationFailed {
        path: lock_dir.to_path_buf(),
        source: e.into(),
    })
}

/// Read PID from lock file.
pub(super) fn read_lock_file_pid(lock_path: &Path) -> Result<u32, BuildLockError> {
    let mut file =
        File::open(lock_path).map_err(|e| BuildLockError::PidReadFailed { source: e.into() })?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| BuildLockError::PidReadFailed { source: e.into() })?;

    parse_pid(&content)
}

/// Attempt to acquire exclusive lock on file using fs2 (cross-platform).
pub(super) fn try_lock_file(file: &File) -> Result<(), BuildLockError> {
    // Use fs2's try_lock_exclusive for non-blocking exclusive lock
    file.try_lock_exclusive().map_err(|err| {
        if err.kind() == io::ErrorKind::WouldBlock {
            BuildLockError::LockOperationFailed {
                source: IoErrorKind::WouldBlock,
            }
        } else {
            BuildLockError::LockOperationFailed { source: err.into() }
        }
    })
}

/// Cleanup stale lock file.
pub(super) fn cleanup_stale_lock(lock_path: &Path) -> Result<(), BuildLockError> {
    fs::remove_file(lock_path).map_err(|e| BuildLockError::LockReleaseFailed {
        path: lock_path.to_path_buf(),
        source: e.into(),
    })
}

/// Write current PID to lock file.
pub(super) fn write_pid_to_lock(file: &mut File) -> Result<(), BuildLockError> {
    let pid = std::process::id();
    file.write_all(pid.to_string().as_bytes())
        .map_err(|e| BuildLockError::LockOperationFailed { source: e.into() })
}

impl BuildCoordinator {
    /// Create a new build coordinator with validated configuration.
    ///
    /// # Errors
    ///
    /// - `InvalidConfiguration` if timeout is zero or `poll_interval` >= timeout
    /// - `LockDirectoryCreationFailed` if lock directory cannot be created
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::{path::PathBuf, time::Duration};
    ///
    /// use zjj_core::build_lock::BuildCoordinator;
    ///
    /// let coordinator = BuildCoordinator::new(
    ///     PathBuf::from("/tmp/zjj-locks"),
    ///     Duration::from_secs(300),
    ///     Duration::from_millis(500),
    /// )?;
    /// # Ok::<(), zjj_core::build_lock::BuildLockError>(())
    /// ```
    pub fn new(
        lock_dir: std::path::PathBuf,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<Self, BuildLockError> {
        // Validate preconditions (Railway pattern: fail fast)
        validate_timeout(timeout)
            .and_then(|()| validate_poll_interval(poll_interval, timeout))
            .and_then(|()| create_lock_directory(&lock_dir).map(|()| lock_dir))
            .map(|validated_dir| Self {
                lock_dir: validated_dir,
                timeout,
                poll_interval,
            })
    }

    /// Acquire build lock with timeout and automatic stale lock cleanup.
    ///
    /// This method will:
    /// 1. Attempt to acquire the lock atomically
    /// 2. If lock held, check if holder process is alive
    /// 3. If holder is dead, cleanup stale lock and retry
    /// 4. If holder is alive, poll with backoff until timeout
    ///
    /// # Errors
    ///
    /// - `LockFileOpenFailed` if lock file cannot be created
    /// - `StaleDetectionFailed` if process liveness check fails
    /// - `LockReleaseFailed` if stale lock cleanup fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zjj_core::build_lock::{BuildCoordinator, LockAcquisition};
    /// # use std::time::Duration;
    /// # use std::path::PathBuf;
    ///
    /// # let coordinator = BuildCoordinator::new(
    /// #     PathBuf::from("/tmp/zjj-locks"),
    /// #     Duration::from_secs(10),
    /// #     Duration::from_millis(100),
    /// # )?;
    ///
    /// match coordinator.acquire()? {
    ///     LockAcquisition::Acquired(lock) => {
    ///         // Lock held, proceed with build
    ///         println!("Lock acquired!");
    ///         // lock auto-released when dropped
    ///     }
    ///     LockAcquisition::AlreadyHeld { holder_pid } => {
    ///         println!("Build already running (PID: {})", holder_pid);
    ///     }
    ///     LockAcquisition::Timeout => {
    ///         println!("Timeout waiting for lock");
    ///     }
    /// }
    /// # Ok::<(), zjj_core::build_lock::BuildLockError>(())
    /// ```
    pub fn acquire(&self) -> Result<LockAcquisition, BuildLockError> {
        let lock_path = self.lock_dir.join("build.lock");
        let start = Instant::now();

        loop {
            match Self::try_acquire_lock_once(&lock_path) {
                Ok(lock) => return Ok(LockAcquisition::Acquired(lock)),
                Err(LockContention::AlreadyLocked(pid)) => {
                    // Check if holder process is still alive
                    if is_process_alive(pid) {
                        // Process alive, check timeout
                        if start.elapsed() >= self.timeout {
                            return Ok(LockAcquisition::Timeout);
                        }
                        // Poll again after interval
                        thread::sleep(self.poll_interval);
                    } else {
                        // Stale lock detected, cleanup and retry
                        cleanup_stale_lock(&lock_path)?;
                    }
                }
                Err(LockContention::IoError(e)) => return Err(e),
            }
        }
    }

    /// Single attempt to acquire lock (atomic operation).
    fn try_acquire_lock_once(lock_path: &Path) -> Result<BuildLock, LockContention> {
        // Try to create lock file (don't truncate yet - need to read PID first)
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| {
                LockContention::IoError(BuildLockError::LockFileOpenFailed {
                    path: lock_path.to_path_buf(),
                    source: e.into(),
                })
            })?;

        // Try to acquire exclusive lock
        match try_lock_file(&file) {
            Ok(()) => {
                // Lock acquired, NOW safe to truncate and write our PID
                file.set_len(0).map_err(|e| {
                    LockContention::IoError(BuildLockError::LockOperationFailed {
                        source: e.into(),
                    })
                })?;
                write_pid_to_lock(&mut file).map_err(LockContention::IoError)?;

                Ok(BuildLock {
                    lock_file: lock_path.to_path_buf(),
                    lock_fd: file,
                })
            }
            Err(BuildLockError::LockOperationFailed {
                source: IoErrorKind::WouldBlock,
            }) => {
                // Lock held by another process, read their PID
                let pid = read_lock_file_pid(lock_path).map_err(|e| {
                    // Log error context before discarding
                    eprintln!("Warning: failed to read lock file PID: {e}");
                    // If we can't read PID, treat as contention with unknown PID
                    LockContention::AlreadyLocked(0)
                })?;
                Err(LockContention::AlreadyLocked(pid))
            }
            Err(e) => Err(LockContention::IoError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_lock_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("zjj-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn cleanup_lock_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_coordinator_creation_validates_timeout() {
        let dir = temp_lock_dir();

        let result = BuildCoordinator::new(
            dir.clone(),
            Duration::from_secs(0), // Invalid: zero timeout
            Duration::from_millis(100),
        );

        assert!(matches!(
            result,
            Err(BuildLockError::InvalidConfiguration { .. })
        ));

        cleanup_lock_dir(&dir);
    }

    #[test]
    fn test_coordinator_creation_validates_poll_interval() {
        let dir = temp_lock_dir();

        let result = BuildCoordinator::new(
            dir.clone(),
            Duration::from_secs(1),
            Duration::from_secs(2), // Invalid: poll > timeout
        );

        assert!(matches!(
            result,
            Err(BuildLockError::InvalidConfiguration { .. })
        ));

        cleanup_lock_dir(&dir);
    }

    #[test]
    fn test_coordinator_creation_success() {
        let dir = temp_lock_dir();

        let result = BuildCoordinator::new(
            dir.clone(),
            Duration::from_secs(10),
            Duration::from_millis(100),
        );

        assert!(result.is_ok());
        cleanup_lock_dir(&dir);
    }

    #[test]
    fn test_lock_acquisition_and_release() -> Result<(), BuildLockError> {
        let dir = temp_lock_dir();
        let lock_path = dir.join("build.lock");

        let coordinator = BuildCoordinator::new(
            dir.clone(),
            Duration::from_secs(10),
            Duration::from_millis(100),
        )?;

        let result = coordinator.acquire()?;

        if let LockAcquisition::Acquired(lock) = result {
            // Lock acquired successfully, file should exist while held
            assert!(lock_path.exists());
            drop(lock);
            // Lock should be cleaned up after drop
            assert!(
                !lock_path.exists(),
                "Lock file should be removed after drop"
            );
        } else {
            cleanup_lock_dir(&dir);
            return Err(BuildLockError::InvalidConfiguration {
                reason: "Expected lock to be acquired".to_string(),
            });
        }

        cleanup_lock_dir(&dir);
        Ok(())
    }
}
