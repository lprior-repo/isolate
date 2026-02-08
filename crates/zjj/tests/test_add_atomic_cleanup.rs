//! Atomic add failure cleanup tests

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

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
    assert_eq!(
        session_status_from_list_json(&parsed, session_name).as_deref(),
        Some("creating"),
        "workspace-creation failure should leave db status as creating"
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
