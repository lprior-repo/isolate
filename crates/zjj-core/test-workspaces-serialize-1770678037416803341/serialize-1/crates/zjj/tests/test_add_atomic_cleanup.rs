
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
//! Atomic add failure cleanup tests

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::ignored_unit_patterns,
    clippy::doc_markdown
)]
mod common;

use common::TestHarness;
use serde_json::Value as JsonValue;

fn session_status_from_list_json(json: &JsonValue, name: &str) -> Option<String> {
    json["data"]["sessions"]
        .as_array()
        .or_else(|| json["data"].as_array())
        .and_then(|sessions| {
            sessions
                .iter()
                .find(|session| session["name"].as_str() == Some(name))
                .and_then(|session| session["status"].as_str())
        })
        .map(ToString::to_string)
}

#[test]
fn test_add_workspace_creation_failure_rolls_back_file_workspace_path() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "atomic-file-rollback";
    let workspace_path = harness.workspace_path(session_name);
    if let Some(parent) = workspace_path.parent() {
        let create_parent_result = std::fs::create_dir_all(parent);
        assert!(
            create_parent_result.is_ok(),
            "Failed to create workspace parent: {:?}",
            create_parent_result.err()
        );
    }
    let write_file_result = std::fs::write(&workspace_path, "pre-existing file");
    assert!(
        write_file_result.is_ok(),
        "Failed to create blocking workspace file: {:?}",
        write_file_result.err()
    );

    let result = harness.zjj(&["add", session_name, "--no-open", "--no-hooks"]);
    assert!(
        !result.success,
        "add should fail when workspace path is a file\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        result.stderr.contains("Failed to create workspace")
            || result.stdout.contains("Failed to create workspace"),
        "failure should surface workspace creation error\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );
    assert!(
        result.stderr.contains("Recovery:") || result.stdout.contains("Recovery:"),
        "failure should include recovery guidance\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    assert!(
        !workspace_path.exists(),
        "rollback should remove workspace file path: {}",
        workspace_path.display()
    );

    let list_result = harness.zjj(&["list", "--all", "--json"]);
    assert!(
        list_result.success,
        "list should succeed: {}",
        list_result.stderr
    );
    let parsed_result: Result<JsonValue, _> = serde_json::from_str(&list_result.stdout);
    assert!(
        parsed_result.is_ok(),
        "list output should be valid json: {}",
        list_result.stdout
    );

    let Some(parsed) = parsed_result.ok() else {
        return;
    };
    assert!(
        session_status_from_list_json(&parsed, session_name).is_none(),
        "workspace-creation failure should remove db session record"
    );

    let retry_result = harness.zjj(&["add", session_name, "--no-open", "--no-hooks"]);
    assert!(
        retry_result.success,
        "retry should succeed after rollback cleanup\nstdout: {}\nstderr: {}",
        retry_result.stdout, retry_result.stderr
    );
    harness.assert_workspace_exists(session_name);

    let retry_list_result = harness.zjj(&["list", "--all", "--json"]);
    assert!(
        retry_list_result.success,
        "list should succeed after retry: {}",
        retry_list_result.stderr
    );
    let retry_parsed_result: Result<JsonValue, _> = serde_json::from_str(&retry_list_result.stdout);
    assert!(
        retry_parsed_result.is_ok(),
        "list output should be valid json after retry: {}",
        retry_list_result.stdout
    );
    let Some(retry_parsed) = retry_parsed_result.ok() else {
        return;
    };
    assert_eq!(
        session_status_from_list_json(&retry_parsed, session_name).as_deref(),
        Some("active"),
        "retry should produce active session"
    );
}

#[test]
fn test_add_hook_failure_marks_failed_and_rolls_back_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "hook-failure-rollback";
    let result = harness.zjj_with_env(
        &["add", session_name, "--no-open"],
        &[("ZJJ_TEST_FAIL_POST_CREATE_HOOK", "1")],
    );
    assert!(
        !result.success,
        "add should fail when post_create hook fails\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        result.stderr.contains("post_create hook failed")
            || result.stdout.contains("post_create hook failed"),
        "failure should surface hook error\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    harness.assert_workspace_not_exists(session_name);

    let list_result = harness.zjj(&["list", "--all", "--json"]);
    assert!(
        list_result.success,
        "list should succeed: {}",
        list_result.stderr
    );

    let parsed_result: Result<JsonValue, _> = serde_json::from_str(&list_result.stdout);
    assert!(
        parsed_result.is_ok(),
        "list output should be valid json: {}",
        list_result.stdout
    );

    let Some(parsed) = parsed_result.ok() else {
        return;
    };
    assert_eq!(
        session_status_from_list_json(&parsed, session_name).as_deref(),
        Some("failed"),
        "hook failure should mark session status as failed"
    );
}

#[test]
fn test_add_hook_failure_recoverable_via_remove_then_retry() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "hook-failure-recoverable";
    let failed_add = harness.zjj_with_env(
        &["add", session_name, "--no-open"],
        &[("ZJJ_TEST_FAIL_POST_CREATE_HOOK", "1")],
    );
    assert!(
        !failed_add.success,
        "add should fail when hook fails\nstdout: {}\nstderr: {}",
        failed_add.stdout, failed_add.stderr
    );

    let remove_result = harness.zjj(&["remove", session_name, "--force"]);
    assert!(
        remove_result.success,
        "remove --force should recover failed session\nstdout: {}\nstderr: {}",
        remove_result.stdout, remove_result.stderr
    );

    let retry_result = harness.zjj(&["add", session_name, "--no-open", "--no-hooks"]);
    assert!(
        retry_result.success,
        "add retry should succeed after remove\nstdout: {}\nstderr: {}",
        retry_result.stdout, retry_result.stderr
    );
    harness.assert_workspace_exists(session_name);
}
