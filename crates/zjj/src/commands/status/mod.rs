//! Show detailed session status
//!
//! This module provides comprehensive status reporting for zjj sessions,
//! including workspace changes, diff statistics, and beads integration.

mod execution;
mod formatting;
mod gathering;
mod types;

// Re-export the main entry point
pub use execution::run;
