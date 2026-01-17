//! Command specifications module
//!
//! This module is organized into sub-modules for maintainability:
//!
//! - `builders`: Individual command specification builders
//! - `commands`: Command dispatcher and public API
//!
//! The module provides a clean interface for command introspection with
//! zero-panic, zero-unwrap functional Rust patterns.

mod builders;
pub mod commands;

// Re-export the public API at module level for convenience
pub use commands::{all_command_names, get_command_spec};
