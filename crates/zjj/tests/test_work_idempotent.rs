//! Idempotent flag tests for `work` command
//!
//! These tests verify that the `--idempotent` flag works correctly for the work command.
//! Tests follow Martin Fowler's Given-When-Then format with descriptive names.

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

mod common;
use common::{payload, session_entries, TestHarness};
use serde_json::Value as JsonValue;

// ============================================================================
// P0 Tests: Happy Path - Must Pass
// ============================================================================

#[test]
fn test_work_idempotent_succeeds_when_already_in_target_workspace() {
    // GIVEN: User is in workspace "feature-auth"
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "feature-auth", "--no-open"]);
    harness.current_dir = harness.workspace_path("feature-auth");

    // WHEN: User runs `zjj work feature-auth --idempotent`
    let result = harness.zjj(&[
        "work",
        "feature-auth",
        "--idempotent",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed when already in target workspace with --idempotent\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    // THEN: No error raised
    assert!(
        !result.stdout.to_lowercase().contains("error"),
        "Should not show error\nstdout: {}",
        result.stdout
    );

    // THEN: Output includes workspace path
    assert!(
        result.stdout.contains("feature-auth") || result.stdout.contains("workspace"),
        "Output should include workspace information\nstdout: {}",
        result.stdout
    );
}

#[test]
fn test_work_idempotent_creates_workspace_when_not_exists() {
    // GIVEN: User is on main branch (not in a workspace)
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj work new-feature --idempotent`
    let result = harness.zjj(&[
        "work",
        "new-feature",
        "--idempotent",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed when creating new workspace\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    // THEN: Workspace is created
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success, "List should succeed");

    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List should be valid JSON: {e}"),
    };
    let sessions = match session_entries(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    };

    assert!(
        sessions.iter().any(|s| s["name"] == "new-feature"),
        "Workspace should be created"
    );
}

#[test]
fn test_work_idempotent_json_output_includes_created_field() {
    // GIVEN: User is on main branch
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj work new-feature --idempotent --json`
    let result = harness.zjj(&[
        "work",
        "new-feature",
        "--idempotent",
        "--json",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Command succeeds
    assert!(
        result.success,
        "Command should succeed\nstdout: {}",
        result.stdout
    );

    // THEN: Output is valid JSON
    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Output should be valid JSON: {e}"),
    };

    // THEN: JSON includes created: true for new workspace
    let response = payload(&json);
    assert_eq!(
        response["created"], true,
        "created should be true for new workspace"
    );
    assert_eq!(response["name"], "new-feature");

    // WHEN: Already in workspace, run again with --idempotent --json
    harness.current_dir = harness.workspace_path("new-feature");
    let result2 = harness.zjj(&[
        "work",
        "new-feature",
        "--idempotent",
        "--json",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Command succeeds (idempotent)
    assert!(
        result2.success,
        "Second command should succeed (idempotent)\nstdout: {}",
        result2.stdout
    );

    // THEN: JSON includes created: false
    let json2: JsonValue = match serde_json::from_str(&result2.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Second output should be valid JSON: {e}"),
    };

    let response2 = payload(&json2);
    assert_eq!(
        response2["created"], false,
        "created should be false when already in workspace"
    );
}

#[test]
fn test_work_idempotent_with_agent_id_reregisters_successfully() {
    // GIVEN: An existing workspace with agent
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "agent-task", "--no-open"]);
    harness.current_dir = harness.workspace_path("agent-task");

    // WHEN: User runs `zjj work agent-task --idempotent --agent-id agent-1`
    let result = harness.zjj(&[
        "work",
        "agent-task",
        "--idempotent",
        "--agent-id",
        "agent-1",
        "--no-zellij",
    ]);

    // THEN: Command succeeds with exit code 0
    assert!(
        result.success,
        "Command should succeed with idempotent agent re-registration\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    // THEN: No duplicate agent registration error
    assert!(
        !result.stdout.to_lowercase().contains("error"),
        "Should not show registration error\nstdout: {}",
        result.stdout
    );
}

// ============================================================================
// P0 Tests: Error Path - Must Pass
// ============================================================================

#[test]
fn test_work_idempotent_fails_when_in_different_workspace() {
    // GIVEN: User is already in workspace "feature-auth"
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "feature-auth", "--no-open"]);
    harness.current_dir = harness.workspace_path("feature-auth");

    // WHEN: User runs `zjj work different-feature --idempotent`
    let result = harness.zjj(&[
        "work",
        "different-feature",
        "--idempotent",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Command fails with exit code 1
    assert!(
        !result.success,
        "Command should fail when in different workspace"
    );

    // THEN: Error message indicates already in workspace
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("already in workspace") || output.contains("already workspace"),
        "Error should indicate already in workspace\noutput: {}",
        output
    );

    // THEN: No new workspace created
    let list_result = harness.zjj(&["list", "--json"]);
    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List should be valid JSON: {e}"),
    };
    let sessions = match session_entries(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    };

    assert!(
        !sessions.iter().any(|s| s["name"] == "different-feature"),
        "Should not create different workspace"
    );
}

#[test]
fn test_work_idempotent_fails_when_not_in_jj_repo() {
    // GIVEN: A directory that is not a JJ repository
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Change to a non-JJ directory
    let temp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => panic!("Failed to create temp dir: {e}"),
    };
    harness.current_dir = temp_dir.path().to_path_buf();

    // WHEN: User runs `zjj work test --idempotent`
    let result = harness.zjj(&["work", "test", "--idempotent", "--no-zellij", "--no-agent"]);

    // THEN: Command fails with exit code 1
    assert!(!result.success, "Command should fail when not in JJ repo");

    // THEN: Error message indicates not in JJ repository
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("not a jj repo")
            || output.contains("jj repository")
            || output.contains("failed to create workspace")
            || output.contains("not in"),
        "Error should indicate not in JJ repo\noutput: {}",
        output
    );
}

#[test]
fn test_work_without_idempotent_existing_session_fails() {
    // GIVEN: User is on main and session exists
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "existing-session", "--no-open"]);

    // WHEN: User runs `zjj work existing-session` WITHOUT --idempotent
    let result = harness.zjj(&["work", "existing-session", "--no-zellij", "--no-agent"]);

    // THEN: Command fails with exit code 1
    assert!(!result.success, "Command should fail without --idempotent");

    // THEN: Error message indicates session exists
    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("already exists"),
        "Error should indicate session exists\noutput: {}",
        output
    );
}

#[test]
fn test_work_rejects_path_traversal_name() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&[
        "work",
        "../../../etc/passwd",
        "--idempotent",
        "--no-zellij",
        "--no-agent",
    ]);

    assert!(!result.success, "Path traversal name must be rejected");

    let output = result.stdout.to_lowercase() + &result.stderr.to_lowercase();
    assert!(
        output.contains("invalid session name") || output.contains("validation"),
        "Expected validation error for invalid name\noutput: {}",
        output
    );
}

// ============================================================================
// P1 Tests: Edge Cases - Should Pass
// ============================================================================

#[test]
fn test_work_idempotent_human_readable_output_format() {
    // GIVEN: User is on main branch
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj work output-test --idempotent` (without --json)
    let result = harness.zjj(&[
        "work",
        "output-test",
        "--idempotent",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Output is human-readable text (not JSON)
    assert!(
        !result.stdout.starts_with('{'),
        "Output should not be JSON when --json flag not used\nstdout: {}",
        result.stdout
    );

    // THEN: Includes session name
    assert!(
        result.stdout.contains("output-test"),
        "Output should include session name\nstdout: {}",
        result.stdout
    );

    // THEN: Includes workspace path or status information
    assert!(
        result.stdout.contains("workspace") || result.stdout.contains("active"),
        "Output should include workspace info\nstdout: {}",
        result.stdout
    );
}

#[test]
fn test_work_idempotent_with_dry_run() {
    // GIVEN: User is on main branch
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj work dry-test --idempotent --dry-run`
    let result = harness.zjj(&[
        "work",
        "dry-test",
        "--idempotent",
        "--dry-run",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Command succeeds
    assert!(result.success, "Dry run should succeed");

    // THEN: Output indicates dry run
    let output = result.stdout.to_lowercase();
    assert!(
        output.contains("dry run") || output.contains("[dry run]"),
        "Output should indicate dry run\noutput: {}",
        result.stdout
    );

    // THEN: No workspace created
    let list_result = harness.zjj(&["list", "--json"]);
    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List should be valid JSON: {e}"),
    };
    let sessions = match session_entries(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    };

    assert!(
        !sessions.iter().any(|s| s["name"] == "dry-test"),
        "Workspace should not be created in dry run"
    );
}

#[test]
fn test_work_idempotent_json_output_schema_validation() {
    // GIVEN: User is on main branch
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // WHEN: User runs `zjj work schema-test --idempotent --json`
    let result = harness.zjj(&[
        "work",
        "schema-test",
        "--idempotent",
        "--json",
        "--no-zellij",
        "--no-agent",
    ]);

    // THEN: Output is valid JSON
    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Output should be valid JSON: {e}"),
    };

    // THEN: JSON matches schema
    assert_eq!(json["$schema"], "zjj://work-response/v1");
    assert_eq!(json["schema_type"], "single");

    // THEN: response includes required fields
    let response = payload(&json);
    assert!(response.get("name").is_some(), "Should have 'name' field");
    assert!(
        response.get("workspace_path").is_some(),
        "Should have 'workspace_path' field"
    );
    assert!(
        response.get("zellij_tab").is_some(),
        "Should have 'zellij_tab' field"
    );
    assert!(
        response.get("created").is_some(),
        "Should have 'created' field"
    );
    assert!(
        response.get("enter_command").is_some(),
        "Should have 'enter_command' field"
    );
}
