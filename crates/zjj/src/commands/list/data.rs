//! Data operations for list command
//!
//! This module provides a unified interface for session data operations.
//! It coordinates three specialized submodules:
//!
//! - `enrichment`: Session metadata extraction (beads, agents, workspace changes)
//! - `query`: Session filtering with functional patterns
//! - `output`: Data formatting for display
//!
//! The public API re-exports key functions while keeping internal details hidden.

pub mod enrichment;
pub mod output;
pub mod query;
pub mod types;

// Re-export the public API from submodules
pub use enrichment::get_beads_count;
pub use output::format_sessions;
pub use query::apply_filters;
pub use types::{ListFilter, SessionListItem, SessionListResponse};
