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
//! Integration tests for standardized JSON envelopes

mod common;

use common::TestHarness;

fn assert_schema_envelope(json: &serde_json::Value, schema_name: &str) {
    assert!(json.get("$schema").is_some(), "Missing $schema field");
    assert_eq!(
        json.get("schema_type").and_then(serde_json::Value::as_str),
        Some("single")
    );
    let expected_schema = format!("zjj://{schema_name}/v1");
    assert_eq!(
        json.get("$schema").and_then(serde_json::Value::as_str),
        Some(expected_schema.as_str())
    );
}

#[test]
fn test_done_dry_run_json_has_envelope() -> Result<(), serde_json::Error> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "done-test", "--no-open"]);

    let workspace_path = harness.workspace_path("done-test");
    let result = harness.zjj_in_dir(&workspace_path, &["done", "--dry-run", "--json"]);

    assert!(result.success, "done dry-run should succeed");
    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "done-response");
    assert_eq!(
        parsed
            .get("workspace_name")
            .and_then(serde_json::Value::as_str),
        Some("done-test")
    );
    Ok(())
}

#[test]
fn test_undo_list_json_has_envelope() -> Result<(), serde_json::Error> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let entry = format!(
        "{{\"session_name\":\"undo-test\",\"commit_id\":\"c1\",\"pre_merge_commit_id\":\"p1\",\"timestamp\":{timestamp},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n"
    );
    let _ = harness.create_file(".zjj/undo.log", &entry);

    let result = harness.zjj(&["undo", "--list", "--json"]);
    assert!(result.success, "undo --list should succeed");
    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "undo-response");
    assert!(parsed.get("entries").is_some(), "entries should be present");
    Ok(())
}

#[test]
fn test_revert_dry_run_json_has_envelope() -> Result<(), serde_json::Error> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let entry = "{\"session_name\":\"revert-test\",\"commit_id\":\"c1\",\"pre_merge_commit_id\":\"p1\",\"timestamp\":1700000000,\"pushed_to_remote\":false,\"status\":\"completed\"}\n";
    let _ = harness.create_file(".zjj/undo.log", entry);

    let result = harness.zjj(&["revert", "revert-test", "--dry-run", "--json"]);
    assert!(result.success, "revert --dry-run should succeed");
    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "revert-response");
    assert_eq!(
        parsed
            .get("session_name")
            .and_then(serde_json::Value::as_str),
        Some("revert-test")
    );
    Ok(())
}

#[test]
fn test_export_json_has_envelope() -> Result<(), serde_json::Error> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "export-test", "--no-open"]);

    let result = harness.zjj(&["export", "--json"]);
    assert!(result.success, "export --json should succeed");
    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "export-response");
    assert!(
        parsed.get("sessions").is_some(),
        "sessions should be present"
    );
    Ok(())
}
