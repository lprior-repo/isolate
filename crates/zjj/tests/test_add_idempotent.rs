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

fn sessions_from_list_json(json: &JsonValue) -> Option<&Vec<JsonValue>> {
    json["data"]["sessions"]
        .as_array()
        .or_else(|| json["data"].as_array())
}

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

    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List output should be valid JSON: {e}"),
    };
    let sessions = match sessions_from_list_json(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions array"),
    };

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
    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Output should be valid JSON: {e}"),
    };

    // THEN: JSON matches schema
    assert_eq!(json["schema"], "add-response");
    assert_eq!(json["type"], "single");

    // THEN: JSON includes created field (true for new session)
    let data = &json["data"];
    assert_eq!(data["name"], "new-session");
    assert_eq!(data["created"], true);
    assert!(data["workspace_path"].is_string());
    assert!(data["zellij_tab"].is_string());

    // WHEN: Run again with --idempotent
    let result2 = harness.zjj(&["add", "new-session", "--idempotent", "--json", "--no-open"]);

    // THEN: Command still succeeds
    assert!(
        result2.success,
        "Second command should succeed (idempotent)\nstdout: {}",
        result2.stdout
    );

    // THEN: JSON indicates already exists
    let json2: JsonValue = match serde_json::from_str(&result2.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Second output should be valid JSON: {e}"),
    };

    let data2 = &json2["data"];
    assert_eq!(data2["name"], "new-session");
    assert!(
        json2["data"]["status"]
            .as_str()
            .map_or(false, |s| s.contains("idempotent"))
            || json2["data"]["status"]
                .as_str()
                .map_or(false, |s| s.contains("already")),
        "Status should indicate idempotent path\ngot: {:?}",
        json2["data"]["status"]
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

    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List output should be valid JSON: {e}"),
    };
    let sessions = match sessions_from_list_json(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    };

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
        "Error should indicate invalid name\noutput: {}",
        output
    );

    // THEN: No session created
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success, "List should succeed");

    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List output should be valid JSON: {e}"),
    };
    let sessions = match sessions_from_list_json(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    };

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
        "Error should indicate initialization required\noutput: {}",
        output
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
        "Error should indicate session exists\noutput: {}",
        output
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

    // THEN: Output indicates dry run
    let output = result.stdout.to_lowercase();
    assert!(
        output.contains("dry run") || output.contains("[dry run]"),
        "Output should indicate dry run\noutput: {}",
        result.stdout
    );
}

#[test]
fn test_add_idempotent_concurrent_calls_handle_race_condition() {
    // GIVEN: An initialized ZJJ repository
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let session_name = "race-idempotent";
    let command_id = "test-race-same-command-id";
    let seed_result = harness.zjj(&["add", session_name, "--no-open"]);
    if !seed_result.success {
        // Environment-specific JJ invocation issues can prevent workspace creation.
        // Skip this concurrency regression in that case.
        return;
    }
    let zjj_bin = harness.zjj_bin.clone();
    let repo_path = harness.repo_path.clone();
    let state_db = repo_path.join(".zjj").join("state.db");

    let path_with_system_dirs = format!(
        "/usr/bin:/usr/local/bin:{}",
        std::env::var("PATH").unwrap_or_default()
    );
    let jj_path = std::env::var("ZJJ_JJ_PATH").unwrap_or_else(|_| "/usr/bin/jj".to_string());

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

    let spawn_add = |barrier: std::sync::Arc<std::sync::Barrier>| {
        let zjj_bin = zjj_bin.clone();
        let repo_path = repo_path.clone();
        let state_db = state_db.clone();
        let path_with_system_dirs = path_with_system_dirs.clone();
        let jj_path = jj_path.clone();

        std::thread::spawn(move || {
            barrier.wait();

            let output = std::process::Command::new(&zjj_bin)
                .args([
                    "add",
                    session_name,
                    "--idempotent",
                    "--command-id",
                    command_id,
                    "--no-open",
                ])
                .current_dir(&repo_path)
                .env("NO_COLOR", "1")
                .env("ZJJ_TEST_MODE", "1")
                .env("ZJJ_WORKSPACE_DIR", "workspaces")
                .env("ZJJ_STATE_DB", &state_db)
                .env("ZJJ_JJ_PATH", &jj_path)
                .env("PATH", &path_with_system_dirs)
                .output()
                .unwrap_or_else(|e| panic!("failed to execute concurrent add command: {e}"));

            (
                output.status.success(),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            )
        })
    };

    let first = spawn_add(barrier.clone());
    let second = spawn_add(barrier.clone());

    barrier.wait();

    let first_result = first.join().expect("first thread panicked");
    let second_result = second.join().expect("second thread panicked");

    // THEN: Both concurrent commands succeed under idempotent semantics
    assert!(
        first_result.0,
        "first concurrent add should succeed\nexit={:?}\nstdout={}\nstderr={}",
        first_result.1, first_result.2, first_result.3
    );
    assert!(
        second_result.0,
        "second concurrent add should succeed\nexit={:?}\nstdout={}\nstderr={}",
        second_result.1, second_result.2, second_result.3
    );

    // THEN: Exactly one session exists (no duplicate created by race)
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(
        list_result.success,
        "List should succeed\nstdout: {}\nstderr: {}",
        list_result.stdout, list_result.stderr
    );

    let json: JsonValue = match serde_json::from_str(&list_result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List output should be valid JSON: {e}"),
    };
    let sessions = match sessions_from_list_json(&json) {
        Some(arr) => arr,
        None => panic!("Should have sessions array"),
    };

    let count = sessions
        .iter()
        .filter(|s| s["name"] == session_name)
        .count();
    assert_eq!(
        count, 1,
        "Concurrent idempotent add with same --command-id should create exactly one session"
    );
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
    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(e) => panic!("Output should be valid JSON: {e}"),
    };

    // THEN: JSON matches SchemaEnvelope structure
    assert_eq!(json["schema"], "add-response");
    assert_eq!(json["type"], "single");

    // THEN: `data` field includes all required fields
    let data = &json["data"];
    assert!(data.is_object(), "data should be an object");

    // Required fields
    assert!(data.get("name").is_some(), "Should have 'name' field");
    assert!(
        data.get("workspace_path").is_some(),
        "Should have 'workspace_path' field"
    );
    assert!(
        data.get("zellij_tab").is_some(),
        "Should have 'zellij_tab' field"
    );
    assert!(data.get("status").is_some(), "Should have 'status' field");
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
    let json1: JsonValue = match serde_json::from_str(&list1.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List should be valid JSON: {e}"),
    };
    let session1 = match sessions_from_list_json(&json1) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    }
    .iter()
    .find(|s| s["name"] == "metadata-test");
    let session1 = match session1 {
        Some(s) => s,
        None => panic!("Should find session"),
    };

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
    let json2: JsonValue = match serde_json::from_str(&list2.stdout) {
        Ok(v) => v,
        Err(e) => panic!("List should be valid JSON: {e}"),
    };
    let session2 = match sessions_from_list_json(&json2) {
        Some(arr) => arr,
        None => panic!("Should have sessions"),
    }
    .iter()
    .find(|s| s["name"] == "metadata-test");
    let session2 = match session2 {
        Some(s) => s,
        None => panic!("Should find session"),
    };

    assert_eq!(
        session1["bead_id"], session2["bead_id"],
        "bead_id should not change with --idempotent"
    );
}
