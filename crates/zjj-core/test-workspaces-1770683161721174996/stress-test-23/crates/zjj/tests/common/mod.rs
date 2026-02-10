//! Common test utilities and relaxed clippy settings for integration tests
//!
//! Integration tests are separate compilation units from the main crate,
//! so they need their own lint relaxations. This module provides:
//! - Relaxed clippy allowances for brutal test scenarios
//! - Common test utilities
//! - Shared test fixtures

// #![warn(clippy::all)] // Keep useful warnings
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::indexing_slicing,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    // Test-specific patterns
    clippy::needless_raw_string_hashes,
    clippy::bool_assert_comparison,
)]

pub use std::{path::PathBuf, process::Command};

/// Get the zjj binary path for testing
pub fn zjj_bin() -> PathBuf {
    std::env::var("ZJJ_BIN")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../../target/release/zjj"))
}

/// Get test workspaces directory
pub fn test_workspaces_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("zjj-test-{}", name))
}

/// Clean up test workspace directory
pub fn cleanup_test_dir(path: &PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}
