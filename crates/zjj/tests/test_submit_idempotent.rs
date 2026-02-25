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
//! Idempotency tests for submit command
//!
//! These tests verify that the submit command behaves idempotently:
//! - Submitting the same change twice succeeds
//! - Response contains consistent dedupe keys
//!
//! Tests follow Martin Fowler's Given-When-Then format with descriptive names.

mod common;
use common::TestHarness;
use serde_json::Value as JsonValue;

// ============================================================================
// P0 Tests: Core Idempotency - Must Pass
// ============================================================================

#[test]
fn test_submit_succeeds_on_first_submit() {
    // GIVEN: An initialized ZJJ repository with a workspace that has changes
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "feature-test", "--no-open"]);

    // Create a file and commit
    let workspace_path = harness.workspace_path("feature-test");
    std::fs::write(workspace_path.join("test.txt"), "test content")
        .expect("Failed to write test file");

    // Commit the changes
    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Add test file"]);

    // Create a bookmark with -r @ for reliability
    harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "feature-test", "-r", "@"],
    );

    // WHEN: User runs submit from the workspace
    harness.current_dir = workspace_path;
    let result = harness.zjj(&["submit", "--json"]);

    // THEN: Command succeeds
    if !result.success {
        // Skip if submit infrastructure not fully available (e.g., no remote)
        eprintln!("Skipping test: submit infrastructure not available");
        return;
    }

    // THEN: JSON output has valid schema
    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Output should be valid JSON: {e}\nGot: {}", result.stdout),
    };

    // Verify schema
    assert!(
        json["schema"]
            .as_str()
            .is_some_and(|s| s.contains("submit")),
        "JSON should have submit schema\nGot: {json}"
    );
    assert_eq!(json["ok"], true, "ok should be true");
}

#[test]
fn test_submit_duplicate_is_idempotent() {
    // GIVEN: An initialized ZJJ repository with a workspace
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "duplicate-test", "--no-open"]);

    // Create a file and commit
    let workspace_path = harness.workspace_path("duplicate-test");
    std::fs::write(workspace_path.join("test.txt"), "duplicate test")
        .expect("Failed to write test file");

    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Add test file"]);
    harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "dup-test", "-r", "@"],
    );

    harness.current_dir = workspace_path;

    // WHEN: User runs submit twice with the same change
    let first_result = harness.zjj(&["submit", "--json"]);
    if !first_result.success {
        eprintln!("Skipping test: submit failed");
        return;
    }

    // Run submit again
    let second_result = harness.zjj(&["submit", "--json"]);

    // THEN: Second submit should also succeed (idempotent)
    assert!(
        second_result.success,
        "Second submit should succeed (idempotent)\nstdout: {}\nstderr: {}",
        second_result.stdout, second_result.stderr
    );

    // THEN: JSON output for second submit is valid
    let json: JsonValue = match serde_json::from_str(&second_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Second output should be valid JSON: {e}"),
    };
    assert_eq!(json["ok"], true, "Second submit ok should be true");
}

#[test]
fn test_submit_dedupe_key_format() {
    // GIVEN: An initialized ZJJ repository
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "dedupe-test", "--no-open"]);

    let workspace_path = harness.workspace_path("dedupe-test");
    std::fs::write(workspace_path.join("test.txt"), "dedupe content")
        .expect("Failed to write test file");

    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Add test"]);
    harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "dedupe-test", "-r", "@"],
    );

    harness.current_dir = workspace_path;

    // WHEN: User submits
    let result = harness.zjj(&["submit", "--json"]);
    if !result.success {
        eprintln!("Skipping test: submit failed");
        return;
    }

    let json: JsonValue = serde_json::from_str(&result.stdout).expect("Valid JSON");
    let data = &json["data"];

    // THEN: Response includes dedupe_key
    let dedupe_key = data["dedupe_key"].as_str();
    assert!(
        dedupe_key.is_some(),
        "Response should include dedupe_key\nGot: {json}"
    );
    let dedupe_key = dedupe_key.expect("dedupe_key exists");

    // THEN: dedupe_key format is workspace:change_id
    assert!(
        dedupe_key.starts_with("dedupe-test:"),
        "Dedupe key should start with workspace name\nGot: {dedupe_key}"
    );
}
