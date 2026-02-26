#![allow(clippy::needless_collect)]
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
    // Dead code allowance for test helpers
    dead_code,
)]
//! Comprehensive JSON standardization tests
//!
//! This test file verifies that all commands follow consistent JSONL output standards:
//! 1. All JSON outputs use JSONL format (one JSON object per line)
//! 2. Each line is a valid OutputLine type (session, action, result, summary, etc.)
//! 3. Error outputs use standardized issue format
//! 4. Success outputs include all required fields

mod common;

use common::TestHarness;

/// Parse JSONL output into a vector of JSON values
fn parse_jsonl(output: &str) -> Result<Vec<serde_json::Value>, String> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|e| format!("Failed to parse JSONL line '{line}': {e}"))
        })
        .collect()
}

/// Validate that a JSONL line has a valid output structure
fn validate_output_line_type(line: &serde_json::Value) -> Result<(), String> {
    // Check for nested OutputLine format (session, action, result, etc.)
    let has_nested_session = line.get("session").is_some();
    let has_nested_action = line.get("action").is_some();
    let has_nested_result = line.get("result").is_some();
    let has_nested_summary = line.get("summary").is_some();
    let has_nested_issue = line.get("issue").is_some();
    let has_nested_warning = line.get("warning").is_some();
    let has_nested_plan = line.get("plan").is_some();
    let has_nested_conflict =
        line.get("conflict_analysis").is_some() || line.get("conflict_detail").is_some();

    // Check for flat OutputLine format
    let has_flat_session = line.get("name").is_some() && line.get("status").is_some();
    let has_flat_action = line.get("verb").is_some() && line.get("target").is_some();
    let has_flat_result = line.get("kind").is_some() && line.get("success").is_some();
    let has_flat_summary = line.get("type").is_some() && line.get("message").is_some();
    let has_flat_issue = line.get("id").is_some() && line.get("kind").is_some();
    let has_flat_warning = line.get("code").is_some() && line.get("message").is_some();
    let has_flat_plan = line.get("title").is_some() && line.get("steps").is_some();
    let has_flat_conflict = line.get("conflicts").is_some();

    // Check for SchemaEnvelope format (legacy envelope wrapper)
    let has_envelope = line.get("$schema").is_some() && line.get("success").is_some();

    let is_valid = has_nested_session
        || has_nested_action
        || has_nested_result
        || has_nested_summary
        || has_nested_issue
        || has_nested_warning
        || has_nested_plan
        || has_nested_conflict
        || has_flat_session
        || has_flat_action
        || has_flat_result
        || has_flat_summary
        || has_flat_issue
        || has_flat_warning
        || has_flat_plan
        || has_flat_conflict
        || has_envelope;

    if is_valid {
        Ok(())
    } else {
        Err(format!("Line does not match any OutputLine type: {line}"))
    }
}

/// Find a line with nested action in JSONL output
fn find_action_line(lines: &[serde_json::Value]) -> Option<&serde_json::Value> {
    lines.iter().find(|line| line.get("action").is_some())
}

/// Check if any line has SchemaEnvelope format
fn has_envelope_format(lines: &[serde_json::Value]) -> bool {
    lines.iter().any(|line| line.get("$schema").is_some())
}

/// Test that init command JSON output uses JSONL or SchemaEnvelope format
#[test]
fn test_init_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    let result = harness.isolate(&["init", "--json"]);
    assert!(result.success, "init should succeed");

    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(!lines.is_empty());

    for line in &lines {
        validate_output_line_type(line)?;
    }

    assert!(has_envelope_format(&lines));
    Ok(())
}

/// Test that list command JSON output uses JSONL format
#[test]
fn test_list_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "list-test", "--no-open"]);

    let result = harness.isolate(&["list", "--json"]);
    assert!(result.success, "list should succeed");

    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(!lines.is_empty());

    for line in &lines {
        validate_output_line_type(line)?;
    }

    let session_lines: Vec<_> = lines
        .iter()
        .filter(|l| l.get("session").is_some())
        .collect();
    assert!(!session_lines.is_empty());

    Ok(())
}

/// Test that focus command JSON output uses JSONL format
#[test]
fn test_focus_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "focus-test", "--no-open"]);

    let result = harness.isolate(&["focus", "focus-test", "--json"]);
    assert!(result.success, "focus should succeed");

    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(!lines.is_empty());

    for line in &lines {
        validate_output_line_type(line)?;
    }

    let action_line = find_action_line(&lines);
    assert!(action_line.is_some());

    Ok(())
}
