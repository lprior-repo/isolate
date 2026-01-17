//! Build lock coordination system for preventing concurrent build contention.
//!
//! This module provides a file-based locking mechanism to ensure at most ONE build
//! runs at a time, preventing resource waste from 24+ concurrent agents running
//! duplicate moon/cargo builds.
//!
//! # Guarantees
//!
//! - **Mutual Exclusion**: At most ONE process holds build lock
//! - **Automatic Cleanup**: RAII via Drop trait guarantees lock release
//! - **Stale Lock Detection**: Dead process locks are auto-cleaned
//! - **Timeout-Based Waiting**: Configurable timeout prevents infinite blocking
//! - **Zero Panics**: All errors handled via Result types
//!
//! # Example
//!
//! ```no_run
//! use std::{path::PathBuf, time::Duration};
//!
//! use zjj_core::build_lock::{BuildCoordinator, LockAcquisition};
//!
//! let coordinator = BuildCoordinator::new(
//!     PathBuf::from("/tmp/zjj-build-locks"),
//!     Duration::from_secs(300),   // 5 min timeout
//!     Duration::from_millis(500), // 500ms poll interval
//! )?;
//!
//! match coordinator.acquire()? {
//!     LockAcquisition::Acquired(lock) => {
//!         // Run build - lock auto-released when dropped
//!         println!("Build lock acquired, proceeding with build");
//!     }
//!     LockAcquisition::AlreadyHeld { holder_pid } => {
//!         println!("Build already running (PID: {})", holder_pid);
//!     }
//!     LockAcquisition::Timeout => {
//!         println!("Timeout waiting for build lock");
//!     }
//! }
//! # Ok::<(), zjj_core::build_lock::BuildLockError>(())
//! ```

mod operations;
mod queries;
pub mod types;

// Re-export public API
pub use types::{BuildCoordinator, BuildLock, BuildLockError, IoErrorKind, LockAcquisition};
