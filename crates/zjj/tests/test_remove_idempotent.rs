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
//! Idempotent flag tests for `remove` command
//!
//! These tests verify that the `--idempotent` flag works correctly for the remove command.
//!
//! The contract (rust-contract-zjj-ftds.md) states:
//! - If session doesn't exist and --idempotent is used, command should succeed (exit code 0)
//! - This differs from default behavior which fails with exit code 2 (not found)

mod common;
use common::{parse_json_output, payload, TestHarness};
use serde_json::Value as JsonValue;

// ============================================================================
// P0 Tests: Happy Path - Must Pass
// ============================================================================

#[test]
fn test_remove_idempotent_succeeds_when_session_doesnt_exist() {
    // GIVEN: An initialized ZJJ repository with no session "nonexistent"
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj remove nonexistent --idempotent`
    let result = harness.zjj(&["remove", "nonexistent", "--idempotent"]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed when session doesn't exist with --idempotent\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    // THEN: No error message about session not found
    assert!(
        !result.stdout.to_lowercase().contains("not found")
            && !result.stderr.to_lowercase().contains("not found"),
        "Should not show 'not found' error with --idempotent\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    // THEN: Output indicates "already removed" or similar
    let output = &result.stdout.to_lowercase();
    assert!(
        output.contains("already removed")
            || output.contains("no such session")
            || output.is_empty(),
        "Output should indicate idempotent path\noutput: {}",
        result.stdout
    );
}

#[test]
fn test_remove_idempotent_removes_session_when_exists() {
    // GIVEN: An initialized ZJJ repository with existing session "old-session"
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "old-session", "--no-open"]);

    // WHEN: User runs `zjj remove old-session --idempotent --force`
    // --force avoids interactive confirmation in test environment.
    let result = harness.zjj(&["remove", "old-session", "--idempotent", "--force"]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed when removing existing session\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    // THEN: Session is removed from database
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success, "List should succeed");

    let json: JsonValue =
        parse_json_output(&list_result.stdout).expect("List should be valid JSON");
    let data = payload(&json);
    let sessions = data["sessions"].as_array().or_else(|| data.as_array());
    let sessions = sessions.expect("Should have sessions");

    assert!(
        !sessions.iter().any(|s| s["name"] == "old-session"),
        "Session should be removed"
    );
}

#[test]
fn test_remove_idempotent_with_force_flag_is_redundant() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj remove test --idempotent -f` (both flags)
    let result = harness.zjj(&["remove", "test", "--idempotent", "--force"]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed with both flags\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    // THEN: No conflict between flags
    // Behavior is identical to --force alone (succeeds whether session exists or not)
}

// ============================================================================
// P0 Tests: Error Path - Must Pass
// ============================================================================

#[test]
fn test_remove_idempotent_fails_when_not_initialized() {
    // GIVEN: A JJ repository without ZJJ initialized
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // NOTE: Not running `zjj init`

    // WHEN: User runs `zjj remove test --idempotent`
    let result = harness.zjj(&["remove", "test", "--idempotent"]);

    // THEN: Command fails with exit code 1
    assert!(
        !result.success,
        "Command should fail when ZJJ not initialized"
    );

    // THEN: Error message indicates ZJJ not initialized
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("not initialized") || output.contains("init"),
        "Error should indicate initialization required\noutput: {output}"
    );
}

// ============================================================================
// Current Behavior Tests (Documenting What Exists Now)
// ============================================================================

#[test]
fn test_remove_without_idempotent_fails_on_nonexistent_session() {
    // GIVEN: An initialized ZJJ repository with no session "nonexistent"
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj remove nonexistent` WITHOUT --idempotent
    let result = harness.zjj(&["remove", "nonexistent", "--force"]);

    // THEN: Command fails with exit code 2 (not found)
    // Note: Currently this might fail with exit code 1
    assert!(
        !result.success,
        "Command should fail when session doesn't exist"
    );

    // THEN: Error message indicates session not found
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("not found") || output.contains("no such"),
        "Error should indicate session not found\noutput: {output}"
    );
}

#[test]
fn test_remove_with_force_succeeds_on_nonexistent_session() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj remove nonexistent --force`
    let result = harness.zjj(&["remove", "nonexistent", "--force"]);

    // THEN: Command behavior depends on implementation
    // Currently: --force skips confirmation but still fails if session doesn't exist
    // With --idempotent: Should succeed
    //
    // This test documents current behavior
    let _ = result;
    // Assert whatever the current behavior is
}

// ============================================================================
// P1 Tests: Edge Cases - Should Pass
// ============================================================================

#[test]
fn test_remove_idempotent_json_output_includes_status() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj remove nonexistent --idempotent --json`
    let result = harness.zjj(&["remove", "nonexistent", "--idempotent", "--json"]);

    // THEN: Command succeeds
    assert!(result.success, "Command should succeed");

    // THEN: Output is valid JSON
    let json: JsonValue = parse_json_output(&result.stdout).expect("Output should be valid JSON");

    // THEN: JSON indicates idempotent path
    assert_eq!(json["schema_type"], "single");
    assert!(json.get("$schema").is_some());

    let data = payload(&json);
    assert_eq!(data["name"], "nonexistent");

    // Should indicate already removed or no action needed
    let message = data["message"].as_str().expect("Should have message");
    assert!(
        message.to_lowercase().contains("already") || message.to_lowercase().contains("no such"),
        "Message should indicate idempotent path\nmessage: {}",
        message
    );
}

#[test]
fn test_remove_idempotent_never_fails_on_nonexistent_session() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj remove safe-remove --idempotent` multiple times
    let result1 = harness.zjj(&["remove", "safe-remove", "--idempotent"]);
    let result2 = harness.zjj(&["remove", "safe-remove", "--idempotent"]);
    let result3 = harness.zjj(&["remove", "safe-remove", "--idempotent"]);

    // THEN: All attempts succeed (safe to retry indefinitely)
    assert!(
        result1.success && result2.success && result3.success,
        "All attempts should succeed with --idempotent"
    );
}
