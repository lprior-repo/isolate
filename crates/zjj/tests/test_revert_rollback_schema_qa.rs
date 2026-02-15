#![allow(clippy::expect_used, clippy::unwrap_used)]
// Integration tests for adversarial QA hardening of:
// - zjj revert
// - zjj rollback
// - zjj schema

mod common;

use common::TestHarness;

#[test]
fn schema_rejects_conflicting_flag_combinations() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let conflicting_cases = vec![
        vec!["schema", "--list", "--all"],
        vec!["schema", "--list", "add-response"],
        vec!["schema", "--all", "add-response"],
    ];

    for args in conflicting_cases {
        let result = harness.zjj(&args);
        assert!(!result.success, "Expected failure for args: {args:?}");
        result.assert_output_contains("cannot be used with");
    }
}

#[test]
fn schema_unknown_name_has_consistent_not_found_exit_code() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let human = harness.zjj(&["schema", "unknown-schema"]);
    assert!(!human.success, "Unknown schema should fail");
    assert_eq!(human.exit_code, Some(2));
    human.assert_output_contains("not found");

    let json = harness.zjj(&["schema", "unknown-schema", "--json"]);
    assert!(!json.success, "Unknown schema should fail in JSON mode");
    assert_eq!(json.exit_code, Some(2));
    let parsed: serde_json::Value =
        serde_json::from_str(&json.stdout).unwrap_or_default();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["exit_code"], 2);
}

#[test]
fn rollback_dry_run_invalid_checkpoint_returns_nonzero_json_once() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "rb-check", "--no-open"]);

    // GIVEN: an existing session and invalid checkpoint
    // WHEN: rollback dry-run executes in JSON mode
    // THEN: output is a single JSON envelope with success=false and non-zero exit
    let result = harness.zjj(&[
        "rollback",
        "rb-check",
        "--to",
        "not-a-real-checkpoint",
        "--dry-run",
        "--json",
    ]);

    assert!(!result.success, "Invalid checkpoint dry-run should fail");
    assert_eq!(result.exit_code, Some(4));
    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).unwrap_or_default();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["operation_succeeded"], false);
}

#[test]
fn rollback_missing_session_returns_not_found_without_duplicate_output() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["rollback", "missing-session", "--to", "abc", "--json"]);
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(2));
    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).unwrap_or_default();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["operation_succeeded"], false);
}

#[test]
fn revert_malformed_undo_log_is_reported_as_read_error() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let undo_log = harness.zjj_dir().join("undo.log");
    std::fs::write(&undo_log, "{invalid-json}\n").expect("should write malformed undo log");

    // GIVEN: malformed undo log content
    // WHEN: revert is executed
    // THEN: read/parse failure is surfaced (not silently treated as empty history)
    let result = harness.zjj(&["revert", "some-session", "--json"]);
    assert!(!result.success, "Malformed undo log should fail revert");
    assert_eq!(result.exit_code, Some(4));
    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).unwrap_or_default();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "READ_UNDO_LOG_FAILED");
}

#[test]
fn revert_missing_session_returns_semantic_exit_code_without_duplicate_json() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["revert", "missing", "--json"]);
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(2));
    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).unwrap_or_default();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "SESSION_NOT_FOUND");
}
