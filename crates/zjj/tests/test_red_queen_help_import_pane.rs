#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

// Martin Fowler-style adversarial regression tests for help/import/pane.
//
// These tests lock in error behavior found through QA/Red-Queen style attacks.

mod common;

use common::TestHarness;

#[test]
fn bdd_help_supports_nested_subcommand_paths() {
    // Given a compiled zjj binary
    // When I ask for nested help
    // Then it shows the nested command help instead of a parse error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["help", "pane", "focus"]);

    assert!(
        result.success,
        "Expected nested help to succeed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Focus a specific pane");
    result.assert_output_contains("--direction");
}

#[test]
fn bdd_help_unknown_nested_path_reports_full_path() {
    // Given a valid top-level command and an invalid nested command
    // When I ask for help for that nested path
    // Then it fails with a full-path unknown-command error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["help", "pane", "does-not-exist"]);

    assert!(
        !result.success,
        "Expected unknown nested help path to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Unknown command 'pane does-not-exist'");
}

#[test]
fn bdd_import_rejects_conflicting_force_and_skip_existing_flags() {
    // Given conflicting import conflict-resolution flags
    // When I run import
    // Then CLI parsing rejects the command before execution
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["import", "--force", "--skip-existing", "input.json"]);

    assert!(
        !result.success,
        "Expected conflicting flags to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("cannot be used with");
    result.assert_output_contains("--skip-existing");
}

#[test]
fn bdd_import_rejects_pre_epoch_timestamps() {
    // Given an import file with pre-1970 created_at timestamp
    // When I run import
    // Then import fails with an actionable timestamp-range error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let import_file = harness.repo_path.join("pre_epoch_import.json");
    let import_json = r#"{
  "version": "1.0",
  "exported_at": "2026-01-01T00:00:00Z",
  "count": 1,
  "sessions": [
    {
      "name": "legacy-session",
      "status": "active",
      "workspace_path": "workspaces/legacy-session",
      "created_at": "1969-12-31T23:59:59Z",
      "commits": []
    }
  ]
}"#;
    std::fs::write(&import_file, import_json).expect("write import test file");

    let import_path = import_file.to_string_lossy().to_string();
    let result = harness.zjj(&["import", &import_path]);

    assert!(
        !result.success,
        "Expected pre-epoch timestamp import to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("legacy-session");
    result.assert_output_contains("must be >= Unix epoch");
}

#[test]
fn bdd_pane_focus_rejects_pane_and_direction_together() {
    // Given mutually-exclusive pane focus selectors
    // When I provide both pane id and direction
    // Then CLI parsing rejects the command with a conflict error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["pane", "focus", "test-session", "3", "--direction", "left"]);

    assert!(
        !result.success,
        "Expected pane+direction conflict to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("cannot be used with");
    result.assert_output_contains("--direction");
}
