//! File watching for beads database changes
//!
//! Monitors `.beads/beads.db` in all workspace directories and emits
//! events when changes are detected. Events are debounced to prevent
//! excessive updates during bulk changes.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//!
//! use zjj_core::{
//!     config::WatchConfig,
//!     watcher::{FileWatcher, WatchEvent},
//! };
//!
//! # async fn example() -> zjj_core::Result<()> {
//! let config = WatchConfig {
//!     enabled: true,
//!     debounce_ms: 100,
//!     paths: vec![".beads/beads.db".to_string()],
//! };
//!
//! let workspaces = vec![PathBuf::from("/path/to/workspace")];
//! let mut rx = FileWatcher::watch_workspaces(&config, workspaces)?;
//!
//! while let Some(event) = rx.recv().await {
//!     match event {
//!         WatchEvent::BeadsChanged { workspace_path } => {
//!             // Update UI
//!             println!("Beads changed in {:?}", workspace_path);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

// ═══════════════════════════════════════════════════════════════════════════
// MODULE DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════

pub mod callbacks;
pub mod state;
pub mod watching;

// ═══════════════════════════════════════════════════════════════════════════
// RE-EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

// Core types and functions
pub use state::{query_beads_status, BeadsStatus};
pub use watching::{FileWatcher, WatchEvent};

// Callbacks for internal use (but exported for testing/advanced usage)
pub use callbacks::extract_workspace_path;
