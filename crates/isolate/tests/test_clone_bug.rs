// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
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
)]
//! Test for clone "source session not found" bug
//!
//! Bug: isolate clone reports 'Source session not found' error even when the source session exists.
//! This test reproduces the issue to verify the fix.

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
    let result = harness.isolate(&["list", "--json"]);
    assert!(result.success, "list should succeed");

    let parsed = common::parse_jsonl_output(&result.stdout)
        .unwrap_or_else(|e| panic!("list JSON should be valid: {e}"));

    let found = parsed.iter().any(|line| {
        line.get("session")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
            == Some("source-session")
    });

    assert!(
        found,
        "source-session should exist in database"
    );

    // Attempt to clone - this should succeed but currently fails with "not found"
    harness.assert_success(&["clone", "source-session", "cloned-session"]);
}
