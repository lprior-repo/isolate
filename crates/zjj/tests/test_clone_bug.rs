//! Test for clone "source session not found" bug
//!
//! Bug: zjj clone reports 'Source session not found' error even when the source session exists.
//! This test reproduces the issue to verify the fix.

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

mod common;

use common::TestHarness;

/// RED: Clone should find existing source session
///
/// This test reproduces the bug where clone reports "not found" even when
/// the source session exists in the database.
#[test]
#[allow(clippy::expect_used)]
fn test_clone_finds_existing_source_session() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a source session
    harness.assert_success(&["add", "source-session", "--no-open"]);

    // Verify source session exists in database
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success, "list should succeed");

    let json: serde_json::Value =
        match serde_json::from_str(&result.stdout) {
            Ok(v) => v,
            Err(e) => panic!("list JSON should be valid: {e}"),
        };

    let sessions = match json["data"].as_array() {
        Some(arr) => arr,
        None => panic!("sessions should be an array"),
    };

    assert!(
        sessions.iter().any(|s| s["name"] == "source-session"),
        "source-session should exist in database"
    );

    // Attempt to clone - this should succeed but currently fails with "not found"
    harness.assert_success(&["clone", "source-session", "cloned-session"]);
}
