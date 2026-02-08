//! Idempotent flag tests for `add` command
//!
//! These tests verify that the `--idempotent` flag works correctly for the add command.
//! Tests follow Martin Fowler's Given-When-Then format with descriptive names.

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

mod common;
use common::TestHarness;
use serde_json::Value as JsonValue;

// ============================================================================
// P0 Tests: Happy Path - Must Pass
// ============================================================================

#[test]
fn test_add_idempotent_succeeds_when_session_already_exists() {
    // GIVEN: An initialized ZJJ repository with an existing session
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "existing-session", "--no-open"]);

    // WHEN: User runs `zjj add existing-session --idempotent`
    let result = harness.zjj(&["add", "existing-session", "--idempotent", "--no-open"]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed when session exists with --idempotent flag\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    // THEN: No error message is displayed
    assert!(
        !result.stdout.to_lowercase().contains("error"),
        "Output should not contain error messages\nstdout: {}",
        result.stdout
    );

    // THEN: Output indicates session already exists
    assert!(
        result.stdout.contains("already exists") || result.stdout.contains("(idempotent)"),
        "Output should indicate idempotent path was taken\nstdout: {}",
        result.stdout
    );
}

#[test]
fn test_add_idempotent_creates_session_when_not_exists() {
    // GIVEN: An initialized ZJJ repository with no existing session
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj add new-session --idempotent`
    let result = harness.zjj(&["add", "new-session", "--idempotent", "--no-open"]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed when creating new session with --idempotent\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    // THEN: Session is created
    assert!(
        result.stdout.contains("new-session"),
        "Output should mention the session name\nstdout: {}",
        result.stdout
    );

    // THEN: Verify session exists in list
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success, "List command should succeed");

    let json: JsonValue =
        serde_json::from_str(&list_result.stdout).expect("List output should be valid JSON");
    let sessions = json["data"].as_array().expect("Should have sessions array");

    assert!(
        sessions.iter().any(|s| s["name"] == "new-session"),
        "Session should be in list"
    );
}

#[test]
fn test_add_idempotent_with_json_output_includes_created_field() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj add new-session --idempotent --json --no-open`
    let result = harness.zjj(&["add", "new-session", "--idempotent", "--json", "--no-open"]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed\nstdout: {}",
        result.stdout
    );

    // THEN: Output is valid JSON
    let json: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // THEN: JSON matches SchemaEnvelope structure
    assert_eq!(json["$schema"], "zjj://add-response/v1");
    assert_eq!(json["schema_type"], "single");

    // THEN: Payload fields are at top level (flattened by SchemaEnvelope)
    assert_eq!(json["name"], "new-session");
    assert!(json["workspace_path"].is_string());
    assert!(json["zellij_tab"].is_string());

    // WHEN: Run again with --idempotent
    let result2 = harness.zjj(&["add", "new-session", "--idempotent", "--json", "--no-open"]);

    // THEN: Command still succeeds
    assert!(
        result2.success,
        "Second command should succeed (idempotent)\nstdout: {}",
        result2.stdout
    );

    // THEN: JSON indicates already exists
    let json2: JsonValue =
        serde_json::from_str(&result2.stdout).expect("Second output should be valid JSON");

    // Payload fields are at top level (flattened by SchemaEnvelope)
    assert_eq!(json2["name"], "new-session");
    assert!(
        json2["status"]
            .as_str()
            .is_some_and(|s| s.contains("idempotent") || s.contains("already")),
        "Status should indicate idempotent path\ngot: {:?}",
        json2["status"]
    );
}

#[test]
fn test_add_idempotent_with_bead_id_succeeds_on_duplicate() {
    // GIVEN: An initialized ZJJ repository with existing session
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "bugfix-123", "--bead", "zjj-abc", "--no-open"]);

    // WHEN: User runs same command with --idempotent
    let result = harness.zjj(&[
        "add",
        "bugfix-123",
        "--idempotent",
        "--bead",
        "zjj-abc",
        "--no-open",
    ]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed with --idempotent on duplicate\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    // THEN: No duplicate session created
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success, "List should succeed");

    let json: JsonValue =
        serde_json::from_str(&list_result.stdout).expect("List output should be valid JSON");
    let sessions = json["data"].as_array().expect("Should have sessions");

    let count = sessions
        .iter()
        .filter(|s| s["name"] == "bugfix-123")
        .count();

    assert_eq!(count, 1, "Should only have one session with this name");
}

// ============================================================================
// P0 Tests: Error Path - Must Pass
// ============================================================================

#[test]
fn test_add_idempotent_fails_on_invalid_session_name() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj add 123-invalid --idempotent`
    let result = harness.zjj(&["add", "123-invalid", "--idempotent", "--no-open"]);

    // THEN: Command fails with exit code 1
    assert!(!result.success, "Command should fail on invalid name");

    // THEN: Error message indicates invalid session name
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("invalid") || output.contains("must start with letter"),
        "Error should indicate invalid name\noutput: {output}"
    );

    // THEN: No session created
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success, "List should succeed");

    let json: JsonValue =
        serde_json::from_str(&list_result.stdout).expect("List output should be valid JSON");
    let sessions = json["data"].as_array().expect("Should have sessions");

    assert!(
        sessions.is_empty(),
        "No sessions should be created with invalid name"
    );
}

#[test]
fn test_add_idempotent_fails_when_not_initialized() {
    // GIVEN: A JJ repository without ZJJ initialized
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // NOTE: Not running `zjj init`

    // WHEN: User runs `zjj add test --idempotent`
    let result = harness.zjj(&["add", "test", "--idempotent", "--no-open"]);

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

#[test]
fn test_add_without_idempotent_existing_session_fails() {
    // GIVEN: An initialized ZJJ repository with existing session
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "existing-session", "--no-open"]);

    // WHEN: User runs `zjj add existing-session` WITHOUT --idempotent
    let result = harness.zjj(&["add", "existing-session", "--no-open"]);

    // THEN: Command fails with exit code 1
    assert!(
        !result.success,
        "Command should fail without --idempotent flag"
    );

    // THEN: Error message indicates session already exists
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("already exists"),
        "Error should indicate session exists\noutput: {output}"
    );
}

// ============================================================================
// P1 Tests: Edge Cases - Should Pass
// ============================================================================

#[test]
fn test_add_idempotent_with_dry_run_shows_existing_session() {
    // GIVEN: An initialized ZJJ repository with existing session
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "dry-test", "--no-open"]);

    // WHEN: User runs `zjj add dry-test --idempotent --dry-run`
    let result = harness.zjj(&["add", "dry-test", "--idempotent", "--dry-run"]);

    // THEN: Command succeeds with exit code 0
    assert!(result.success, "Dry run with idempotent should succeed");

    // THEN: Output indicates session exists (idempotent path taken before dry-run branch)
    let output = result.stdout.to_lowercase();
    assert!(
        output.contains("already exists") || output.contains("idempotent"),
        "Output should indicate idempotent path\noutput: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Concurrent execution test requires threading/forking implementation"]
fn test_add_idempotent_concurrent_calls_handle_race_condition() {
    // NOTE: This test requires concurrent execution which is complex to implement
    // Skipping for now as it requires threading/forking in tests
    // This is a P1 test that can be implemented later
}

#[test]
fn test_add_idempotent_json_output_schema_validation() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj add schema-test --idempotent --json --no-open`
    let result = harness.zjj(&["add", "schema-test", "--idempotent", "--json", "--no-open"]);

    // THEN: Output is valid JSON
    let json: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // THEN: JSON matches SchemaEnvelope structure
    assert_eq!(json["$schema"], "zjj://add-response/v1");
    assert_eq!(json["schema_type"], "single");

    // THEN: Payload fields are at top level (flattened by SchemaEnvelope)
    assert!(json.get("name").is_some(), "Should have 'name' field");
    assert!(
        json.get("workspace_path").is_some(),
        "Should have 'workspace_path' field"
    );
    assert!(
        json.get("zellij_tab").is_some(),
        "Should have 'zellij_tab' field"
    );
}

// ============================================================================
// P2 Tests: Nice to Have - Don't Block
// ============================================================================

#[test]
fn test_add_idempotent_preserves_existing_session_metadata() {
    // GIVEN: An initialized ZJJ repository with session and bead_id
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "metadata-test", "--bead", "zjj-123", "--no-open"]);

    // Get initial session info
    let list1 = harness.zjj(&["list", "--json"]);
    let json1: JsonValue = serde_json::from_str(&list1.stdout).expect("List should be valid JSON");
    let session1 = json1["data"]
        .as_array()
        .expect("Should have sessions")
        .iter()
        .find(|s| s["name"] == "metadata-test")
        .expect("Should find session");

    // WHEN: User runs with different bead_id and --idempotent
    let result = harness.zjj(&[
        "add",
        "metadata-test",
        "--idempotent",
        "--bead",
        "zjj-456",
        "--no-open",
    ]);

    // THEN: Command succeeds
    assert!(result.success, "Idempotent add should succeed");

    // THEN: bead_id remains unchanged
    let list2 = harness.zjj(&["list", "--json"]);
    let json2: JsonValue = serde_json::from_str(&list2.stdout).expect("List should be valid JSON");
    let session2 = json2["data"]
        .as_array()
        .expect("Should have sessions")
        .iter()
        .find(|s| s["name"] == "metadata-test")
        .expect("Should find session");

    assert_eq!(
        session1["bead_id"], session2["bead_id"],
        "bead_id should not change with --idempotent"
    );
}
