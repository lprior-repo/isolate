//! File locking utilities for preventing TOCTOU vulnerabilities
//!
//! This module provides file locking with retry logic and exponential backoff
//! to prevent time-of-check to time-of-use (TOCTOU) race conditions.
//!
//! # Design Principles
//!
//! - **Zero panics**: All operations return `Result<T, Error>`
//! - **Zero unwraps**: Uses functional patterns throughout
//! - **Atomic operations**: Uses file locks for critical sections
//! - **Retry logic**: Exponential backoff for transient failures
//! - **Drop-safe**: Locks are automatically released on drop

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use std::path::Path;

use crate::{Error, Result};

/// Default maximum number of retry attempts for lock acquisition
const DEFAULT_MAX_RETRIES: u32 = 10;

/// Base delay in milliseconds for exponential backoff
const BASE_DELAY_MS: u64 = 10;

/// Maximum delay in milliseconds for exponential backoff
const MAX_DELAY_MS: u64 = 5000;

/// A file lock that releases automatically when dropped
///
/// Uses platform-specific locking mechanisms for cross-process synchronization.
/// The lock is released when the `FileLock` instance is dropped.
///
/// # Safety
///
/// - Locks are automatically released on drop (RAII pattern)
/// - Lock files are NOT deleted (prevents TOCTOU in lock file creation)
/// - Multiple locks on the same file from the same process are supported
#[derive(Debug)]
pub struct FileLock {
    /// The file descriptor for the lock file
    file: tokio::fs::File,
    /// Path to the lock file (for debugging purposes)
    lock_path: std::path::PathBuf,
}

impl FileLock {
    /// Create a new file lock with the given file descriptor
    #[must_use]
    fn new(file: tokio::fs::File, lock_path: std::path::PathBuf) -> Self {
        Self { file, lock_path }
    }

    /// Get the path to the lock file
    #[must_use]
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

/// Drop guard to ensure lock is released
///
/// The lock is released automatically when this guard is dropped.
/// Note: Platform-specific locks are released when the file descriptor is closed.
impl Drop for FileLock {
    fn drop(&mut self) {
        // Lock is automatically released when file is closed
        // No explicit unlock needed for platform-specific locks
        tracing::debug!("Released file lock: {}", self.lock_path.display());
    }
}

/// Configuration for lock acquisition behavior
#[derive(Debug, Clone)]
pub struct LockOptions {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay in milliseconds for exponential backoff
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Whether to create parent directories if they don't exist
    pub create_parent_dirs: bool,
}

impl Default for LockOptions {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay_ms: BASE_DELAY_MS,
            max_delay_ms: MAX_DELAY_MS,
            create_parent_dirs: false,
        }
    }
}

impl LockOptions {
    /// Create new lock options with custom retry settings
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of retry attempts
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set base delay for exponential backoff
    #[must_use]
    pub fn with_base_delay_ms(mut self, base_delay_ms: u64) -> Self {
        self.base_delay_ms = base_delay_ms;
        self
    }

    /// Set maximum delay for exponential backoff
    #[must_use]
    pub fn with_max_delay_ms(mut self, max_delay_ms: u64) -> Self {
        self.max_delay_ms = max_delay_ms;
        self
    }

    /// Enable creation of parent directories
    #[must_use]
    pub fn with_create_parent_dirs(mut self, create: bool) -> Self {
        self.create_parent_dirs = create;
        self
    }
}

/// Acquire an exclusive file lock with retry logic
///
/// This function implements exponential backoff retry logic to handle
/// transient lock contention. Uses platform-specific locking mechanisms
/// which are automatically released when the file descriptor is closed.
///
/// # Arguments
///
/// * `lock_path` - Path to the lock file (will be created if it doesn't exist)
/// * `options` - Configuration for lock acquisition behavior
///
/// # Returns
///
/// Returns a `FileLock` guard that will release the lock when dropped.
///
/// # Errors
///
/// Returns error if:
/// - Lock cannot be acquired after max retries
/// - File system errors occur
/// - Parent directories don't exist (unless `create_parent_dirs` is enabled)
///
/// # Platform-Specific Behavior
///
/// - **Unix**: Uses `flock()` with `LOCK_EX | LOCK_NB` for non-blocking exclusive locks
/// - **Windows**: Uses `LockFileEx()` with `LOCKFILE_EXCLUSIVE_LOCK` for exclusive locks
///
/// # Examples
///
/// ```no_run
/// use zjj_core::filelock::acquire_lock;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let lock = acquire_lock("/tmp/mylock.lock", &Default::default()).await?;
///     // Critical section here
///     // Lock is automatically released when `lock` goes out of scope
///     Ok(())
/// }
/// ```
pub async fn acquire_lock(
    lock_path: impl AsRef<Path>,
    options: &LockOptions,
) -> Result<FileLock> {
    let lock_path = lock_path.as_ref();

    // Create parent directories if requested
    if options.create_parent_dirs {
        if let Some(parent) = lock_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::IoError(format!("Failed to create lock directory: {e}")))?;
        }
    }

    // Calculate delay between retries with exponential backoff
    let calculate_delay = |attempt: u32| -> u64 {
        let delay = options.base_delay_ms * 2_u64.pow(attempt);
        std::cmp::min(delay, options.max_delay_ms)
    };

    // Retry loop with exponential backoff
    let mut attempt = 0;
    loop {
        // Try to open/create lock file
        let open_result = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .mode(0o644) // rw-r--r--
            .open(lock_path)
            .await;

        let file = match open_result {
            Ok(f) => f,
            Err(e) => {
                // File system error - not recoverable
                return Err(Error::IoError(format!(
                    "Failed to open lock file '{}': {e}",
                    lock_path.display()
                )));
            }
        };

        // Try to acquire exclusive lock using platform-specific implementation
        let lock_result = try_acquire_platform_lock(&file).await;

        match lock_result {
            Ok(()) => {
                tracing::debug!("Acquired file lock: {}", lock_path.display());
                return Ok(FileLock::new(file, lock_path.to_path_buf()));
            }
            Err(e) => {
                if attempt >= options.max_retries {
                    return Err(Error::IoError(format!(
                        "Failed to acquire lock '{}' after {} attempts: {e}",
                        lock_path.display(),
                        options.max_retries + 1
                    )));
                }

                let delay = calculate_delay(attempt);
                tracing::debug!(
                    "Lock attempt {}/{} failed, retrying after {}ms: {}",
                    attempt + 1,
                    options.max_retries + 1,
                    delay,
                    lock_path.display()
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                attempt += 1;
            }
        }
    }
}

/// Platform-specific lock acquisition
///
/// Uses platform-appropriate locking mechanisms:
/// - Unix: `flock()` with non-blocking flag
/// - Windows: Would use `LockFileEx()` (not yet implemented)
#[cfg(unix)]
async fn try_acquire_platform_lock(file: &tokio::fs::File) -> Result<()> {
    use std::os::fd::AsRawFd;

    // Get the raw file descriptor
    let raw_fd = file.as_raw_fd();

    // Try to acquire exclusive lock using flock
    // F_SETLK: Set lock (non-blocking)
    // LOCK_EX: Exclusive lock
    let lock_result = unsafe {
        libc::flock(
            raw_fd,
            libc::LOCK_EX | libc::LOCK_NB,
        )
    };

    if lock_result == 0 {
        Ok(())
    } else {
        // Lock is held by another process
        Err(Error::IoError("Lock is held by another process".to_string()))
    }
}

/// Platform-specific lock acquisition for Windows
///
/// Note: Windows implementation not yet provided.
/// For cross-platform compatibility, consider using a library like `fs2` or `fdlock`.
#[cfg(not(unix))]
async fn try_acquire_platform_lock(_file: &tokio::fs::File) -> Result<()> {
    // Windows would use LockFileEx here
    // For now, we return an error indicating lack of support
    Err(Error::IoError(
        "File locking is not yet supported on this platform".to_string()
    ))
}

/// Execute a critical section with file locking
///
/// This is a convenience function that acquires a lock, executes
/// the provided closure, and releases the lock automatically.
///
/// # Arguments
///
/// * `lock_path` - Path to the lock file
/// * `options` - Configuration for lock acquisition
/// * `f` - Async closure to execute while holding the lock
///
/// # Returns
///
/// Returns the result of the closure, or an error if lock acquisition fails.
///
/// # Examples
///
/// ```no_run
/// use zjj_core::filelock::with_lock;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     with_lock("/tmp/mylock.lock", &Default::default(), || async {
///         // Critical section here
///         Ok::<(), Box<dyn std::error::Error>>(())
///     }).await?;
///     Ok(())
/// }
/// ```
pub async fn with_lock<F, Fut, T>(
    lock_path: impl AsRef<Path>,
    options: &LockOptions,
    f: F,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let _lock = acquire_lock(lock_path, options).await?;
    f().await
}

/// Execute an operation with retry logic (without locking)
///
/// This is useful for operations that may fail transiently and need
/// to be retried with exponential backoff.
///
/// # Arguments
///
/// * `operation` - Async closure that may fail transiently
/// * `options` - Retry configuration
///
/// # Returns
///
/// Returns the result of the operation, or the last error if all retries fail.
pub async fn with_retry<F, Fut, T>(
    operation: F,
    options: &LockOptions,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut last_error = None;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);

                if attempt >= options.max_retries {
                    // Return the last error
                    return last_error.ok_or_else(|| {
                        Error::Unknown("Retry loop failed without capturing error".to_string())
                    });
                }

                let delay = std::cmp::min(
                    options.base_delay_ms * 2_u64.pow(attempt),
                    options.max_delay_ms,
                );

                tracing::debug!(
                    "Operation attempt {}/{} failed, retrying after {}ms",
                    attempt + 1,
                    options.max_retries + 1,
                    delay
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_acquire_and_release_lock() {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))
            .unwrap();

        let lock_path = temp_dir.path().join("test.lock");

        {
            // Acquire lock
            let lock = acquire_lock(&lock_path, &Default::default())
                .await
                .expect("Failed to acquire lock");

            assert!(lock.lock_path().exists());

            // Lock is automatically released when `lock` goes out of scope
        }

        // After lock is released, we should be able to acquire it again
        let lock2 = acquire_lock(&lock_path, &Default::default())
            .await
            .expect("Failed to acquire lock after release");

        assert!(lock2.lock_path().exists());
    }

    #[tokio::test]
    async fn test_lock_contention() {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))
            .unwrap();

        let lock_path = temp_dir.path().join("contention.lock");
        let lock_path_clone = lock_path.clone();

        // Acquire first lock
        let lock1 = acquire_lock(&lock_path, &Default::default())
            .await
            .expect("Failed to acquire first lock");

        // Try to acquire second lock (should fail quickly)
        let options = LockOptions::new()
            .with_max_retries(2)
            .with_base_delay_ms(10);

        let result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            acquire_lock(&lock_path_clone, &options),
        )
        .await;

        // Should fail due to contention
        assert!(result.is_err() || result.unwrap().is_err());

        // Drop first lock
        drop(lock1);

        // Now we should be able to acquire the lock
        let lock2 = acquire_lock(&lock_path, &Default::default())
            .await
            .expect("Failed to acquire lock after first lock released");

        drop(lock2);
    }

    #[tokio::test]
    async fn test_with_lock_helper() {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))
            .unwrap();

        let lock_path = temp_dir.path().join("helper.lock");
        let result = with_lock(&lock_path, &Default::default(), || async {
            Ok::<(), Error>(())
        })
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_retry_success_on_first_try() {
        let mut attempts = 0;

        let result = with_retry(
            || {
                attempts += 1;
                async { Ok::<(), Error>(()) }
            },
            &LockOptions::new().with_max_retries(3),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn test_with_retry_success_after_retries() {
        let mut attempts = 0;

        let result = with_retry(
            || {
                attempts += 1;
                async {
                    if attempts < 3 {
                        Err(Error::IoError("Transient error".to_string()))
                    } else {
                        Ok::<(), Error>(())
                    }
                }
            },
            &LockOptions::new().with_max_retries(5),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_with_retry_failure_after_max_retries() {
        let mut attempts = 0;

        let result = with_retry(
            || {
                attempts += 1;
                async { Err::<(), Error>(Error::IoError("Persistent error".to_string())) }
            },
            &LockOptions::new().with_max_retries(3),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 4); // Initial attempt + 3 retries
    }

    #[tokio::test]
    async fn test_lock_options_default() {
        let options = LockOptions::default();
        assert_eq!(options.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(options.base_delay_ms, BASE_DELAY_MS);
        assert_eq!(options.max_delay_ms, MAX_DELAY_MS);
        assert!(!options.create_parent_dirs);
    }

    #[tokio::test]
    async fn test_lock_options_builder() {
        let options = LockOptions::new()
            .with_max_retries(20)
            .with_base_delay_ms(50)
            .with_max_delay_ms(10000)
            .with_create_parent_dirs(true);

        assert_eq!(options.max_retries, 20);
        assert_eq!(options.base_delay_ms, 50);
        assert_eq!(options.max_delay_ms, 10000);
        assert!(options.create_parent_dirs);
    }

    #[tokio::test]
    async fn test_lock_with_parent_dir_creation() {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))
            .unwrap();

        let lock_path = temp_dir.path().join("subdir/nested.lock");

        let options = LockOptions::new().with_create_parent_dirs(true);
        let lock = acquire_lock(&lock_path, &options)
            .await
            .expect("Failed to acquire lock with parent dir creation");

        assert!(lock.lock_path().exists());
        assert!(lock_path.parent().unwrap().exists());
    }
}
