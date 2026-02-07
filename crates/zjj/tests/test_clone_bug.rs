//! Test for clone "source session not found" bug
//!
//! Bug: zjj clone reports 'Source session not found' error even when the source session exists.
//! This test reproduces the issue to verify the fix.

mod common;

use common::TestHarness;

/// RED: Clone should find existing source session
///
/// This test reproduces the bug where clone reports "not found" even when
/// the source session exists in the database.
#[test]
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
        serde_json::from_str(&result.stdout).expect("list JSON should be valid");

    let sessions = json["data"]["sessions"]
        .as_array()
        .expect("sessions should be an array");

    assert!(
        sessions.iter().any(|s| s["name"] == "source-session"),
        "source-session should exist in database"
    );

    // Attempt to clone - this should succeed but currently fails with "not found"
    harness.assert_success(&["clone", "source-session", "cloned-session"]);
}
