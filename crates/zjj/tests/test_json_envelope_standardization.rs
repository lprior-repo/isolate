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
fn test_undo_list_json_empty_history_still_has_envelope() -> Result<(), serde_json::Error> {
    // Given an initialized repo with no undo history
    // When undo history is listed in JSON mode
    // Then the response uses the same schema envelope
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["undo", "--list", "--json"]);
    assert!(result.success, "undo --list --json should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "undo-response");
    assert_eq!(
        parsed
            .get("total")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        0
    );
    Ok(())
}

#[test]
fn test_undo_list_json_reports_malformed_undo_log() -> Result<(), serde_json::Error> {
    // Given a malformed undo log file
    // When undo list is requested
    // Then command fails with structured malformed-log error
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let _ = harness.create_file(".zjj/undo.log", "{not-json}\n");

    let result = harness.zjj(&["undo", "--list", "--json"]);
    assert!(!result.success, "malformed undo log should fail");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "error-response");
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("MALFORMED_UNDO_LOG")
    );
    Ok(())
}

#[test]
fn test_undo_list_json_reports_malformed_line_number() -> Result<(), serde_json::Error> {
    // Given a partially valid undo log with a malformed trailing line
    // When undo list is requested in JSON mode
    // Then the error message identifies the malformed source line
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let content = format!(
        "{{\"session_name\":\"ok\",\"commit_id\":\"c1\",\"pre_merge_commit_id\":\"p1\",\"timestamp\":{timestamp},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n\n{{broken-json}}\n"
    );
    let _ = harness.create_file(".zjj/undo.log", &content);

    let result = harness.zjj(&["undo", "--list", "--json"]);
    assert!(!result.success, "malformed undo log should fail");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "error-response");
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("MALFORMED_UNDO_LOG")
    );
    assert!(
        parsed
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|message| message.contains("line 3")),
        "error message should include malformed line number"
    );
    Ok(())
}

#[test]
fn test_undo_list_json_reports_read_error_when_log_is_directory() -> Result<(), serde_json::Error> {
    // Given an undo.log path that is a directory (corrupted state)
    // When undo list is requested in JSON mode
    // Then command fails with structured READ_UNDO_LOG_FAILED
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let undo_log_dir = harness.repo_path.join(".zjj").join("undo.log");
    let _ = std::fs::remove_file(&undo_log_dir);
    let _ = std::fs::create_dir_all(&undo_log_dir);

    let result = harness.zjj(&["undo", "--list", "--json"]);
    assert!(!result.success, "directory undo.log should fail");
    assert_eq!(
        result.exit_code,
        Some(4),
        "read errors should map to exit 4"
    );

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "error-response");
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("READ_UNDO_LOG_FAILED")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_undo_list_json_reports_read_error_when_log_is_unreadable() -> Result<(), serde_json::Error>
{
    use std::os::unix::fs::PermissionsExt;

    // Given an unreadable undo.log file (permission corruption)
    // When undo list is requested in JSON mode
    // Then command fails with structured READ_UNDO_LOG_FAILED
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let undo_log_path = harness.repo_path.join(".zjj").join("undo.log");
    let _ = std::fs::write(&undo_log_path, "{}\n");
    let _ = std::fs::set_permissions(&undo_log_path, std::fs::Permissions::from_mode(0o000));

    let result = harness.zjj(&["undo", "--list", "--json"]);

    // Restore permissions so temp cleanup always succeeds.
    let _ = std::fs::set_permissions(&undo_log_path, std::fs::Permissions::from_mode(0o644));

    assert!(!result.success, "unreadable undo.log should fail");
    assert_eq!(
        result.exit_code,
        Some(4),
        "read errors should map to exit 4"
    );

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_schema_envelope(&parsed, "error-response");
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("READ_UNDO_LOG_FAILED")
    );
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
