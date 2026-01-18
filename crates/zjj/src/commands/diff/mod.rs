//! Show diff between session and main branch
//!
//! This module provides diff functionality for zjj sessions,
//! showing changes between the session workspace and the main branch.

mod execution;
mod formatting;
mod parsing;
mod types;

// Re-export the main entry point and types
pub use execution::run_with_options;
pub use types::DiffOptions;
