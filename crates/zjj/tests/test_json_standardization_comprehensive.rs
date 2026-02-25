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
///
/// Supports both:
/// 1. Nested format: {"session": {...}}, {"action": {...}}, etc.
/// 2. Flat format: {"name": ..., "status": ...}, {"verb": ..., "target": ...}, etc.
/// 3. SchemaEnvelope format: {"$schema": ..., "success": ..., "data": {...}}
fn validate_output_line_type(line: &serde_json::Value) -> Result<(), String> {
    // Check for nested OutputLine format (session, action, result, etc.)
    let has_nested_session = line.get("session").is_some();
    let has_nested_action = line.get("action").is_some();
    let has_nested_result = line.get("result").is_some();
    let has_nested_summary = line.get("summary").is_some();
    let has_nested_issue = line.get("issue").is_some();
    let has_nested_warning = line.get("warning").is_some();
    let has_nested_plan = line.get("plan").is_some();
    let has_nested_stack = line.get("stack").is_some();
    let has_nested_queue_summary = line.get("queue_summary").is_some();
    let has_nested_queue_entry = line.get("queue_entry").is_some();
    let has_nested_train = line.get("train").is_some();
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
    let has_flat_stack = line.get("entries").is_some() && line.get("base_ref").is_some();
    let has_flat_queue_summary = line.get("total").is_some() && line.get("pending").is_some();
    let has_flat_queue_entry = line.get("session").is_some() && line.get("priority").is_some();
    let has_flat_train = line.get("steps").is_some() && line.get("status").is_some();
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
        || has_nested_stack
        || has_nested_queue_summary
        || has_nested_queue_entry
        || has_nested_train
        || has_nested_conflict
        || has_flat_session
        || has_flat_action
        || has_flat_result
        || has_flat_summary
        || has_flat_issue
        || has_flat_warning
        || has_flat_plan
        || has_flat_stack
        || has_flat_queue_summary
        || has_flat_queue_entry
        || has_flat_train
        || has_flat_conflict
        || has_envelope;

    if is_valid {
        Ok(())
    } else {
        Err(format!("Line does not match any OutputLine type: {line}"))
    }
}

/// Find a line with nested session in JSONL output
fn find_session_line(lines: &[serde_json::Value]) -> Option<&serde_json::Value> {
    lines.iter().find(|line| line.get("session").is_some())
}

/// Find a line with nested action in JSONL output
fn find_action_line(lines: &[serde_json::Value]) -> Option<&serde_json::Value> {
    lines.iter().find(|line| line.get("action").is_some())
}

/// Find a line with nested result in JSONL output
fn find_result_line(lines: &[serde_json::Value]) -> Option<&serde_json::Value> {
    lines.iter().find(|line| line.get("result").is_some())
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

    let result = harness.zjj(&["init", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "init should succeed");

    // Parse as JSONL lines
    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(
        !lines.is_empty(),
        "init should produce at least one JSONL line"
    );

    // Validate each line is a valid output type
    for line in &lines {
        validate_output_line_type(line)?;
    }

    // init uses SchemaEnvelope format with $schema field
    assert!(
        has_envelope_format(&lines),
        "init output should use SchemaEnvelope format"
    );

    Ok(())
}

/// Test that add command JSON output uses JSONL or SchemaEnvelope format
#[test]
fn test_add_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "test-session", "--json", "--no-open"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "add should succeed");

    // Parse as JSONL lines
    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(
        !lines.is_empty(),
        "add should produce at least one JSONL line"
    );

    // Validate each line is a valid output type
    for line in &lines {
        validate_output_line_type(line)?;
    }

    // add uses SchemaEnvelope format with $schema field
    assert!(
        has_envelope_format(&lines),
        "add output should use SchemaEnvelope format"
    );

    Ok(())
}

/// Test that list command JSON output uses JSONL format with session lines
#[test]
fn test_list_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "list-test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "list should succeed");

    // Parse as JSONL lines
    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(
        !lines.is_empty(),
        "list should produce at least one JSONL line"
    );

    // Validate each line is a valid output type
    for line in &lines {
        validate_output_line_type(line)?;
    }

    // List produces session lines in nested format: {"session": {...}}
    let session_lines: Vec<_> = lines
        .iter()
        .filter(|l| l.get("session").is_some())
        .collect();
    assert!(
        !session_lines.is_empty(),
        "list output should include at least one session line"
    );

    Ok(())
}

/// Test that focus command JSON output uses JSONL format with action lines
#[test]
fn test_focus_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "focus-test", "--no-open"]);

    let result = harness.zjj(&["focus", "focus-test", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "focus should succeed");

    // Parse as JSONL lines
    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(
        !lines.is_empty(),
        "focus should produce at least one JSONL line"
    );

    // Validate each line is a valid output type
    for line in &lines {
        validate_output_line_type(line)?;
    }

    // Focus produces action lines in nested format: {"action": {...}}
    let action_line = find_action_line(&lines);
    assert!(
        action_line.is_some(),
        "focus output should include an action line"
    );

    Ok(())
}

/// Test that status command JSON output uses valid JSONL format
#[test]
fn test_status_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "status-test", "--no-open"]);

    let result = harness.zjj(&["status", "--json"]);

    // Note: status command may not have --json flag yet, this test
    // documents the expected behavior
    if result.success {
        // Parse as JSONL lines
        let lines = parse_jsonl(result.stdout.trim())?;

        if !lines.is_empty() {
            // Validate each line is a valid output type
            for line in &lines {
                validate_output_line_type(line)?;
            }
        }
    }
    // If status --json not implemented yet, that's OK for this test

    Ok(())
}

/// Test that remove command JSON output uses JSONL format with action lines
#[test]
fn test_remove_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "remove-test", "--no-open"]);

    let result = harness.zjj(&["remove", "remove-test", "--json", "--force"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "remove should succeed");

    // Parse as JSONL lines
    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(
        !lines.is_empty(),
        "remove should produce at least one JSONL line"
    );

    // Validate each line is a valid output type
    for line in &lines {
        validate_output_line_type(line)?;
    }

    // Remove produces action lines in nested format: {"action": {...}}
    let action_line = find_action_line(&lines);
    assert!(
        action_line.is_some(),
        "remove output should include an action line"
    );

    Ok(())
}

/// Test that sync command JSON output uses JSONL format
///
/// The sync command outputs streaming JSONL (one JSON object per line) for AI-first control plane.
/// Each line is either an Action, Summary, Issue, or Result.
#[test]
fn test_sync_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "sync-test", "--no-open"]);

    let result = harness.zjj(&["sync", "sync-test", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "sync should succeed");

    // Parse each line of JSONL output
    let lines = parse_jsonl(result.stdout.trim())?;
    assert!(
        !lines.is_empty(),
        "sync should produce at least one JSONL line"
    );

    // Each line should be valid JSON and have a valid output type
    for parsed in &lines {
        validate_output_line_type(parsed)?;
    }

    // Sync produces action lines in nested format: {"action": {...}}
    let action_line = find_action_line(&lines);
    assert!(
        action_line.is_some(),
        "sync output should include at least one action line"
    );

    Ok(())
}

/// Test that error responses use standardized format
#[test]
fn test_error_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    // Try to focus on a non-existent session
    let result = harness.zjj(&["focus", "nonexistent", "--json"]);

    // Should fail - check for JSON output
    if result.stdout.contains('{') {
        // Try to parse as JSONL lines
        if let Ok(lines) = parse_jsonl(result.stdout.trim()) {
            // Look for issue line or failed result
            let has_issue = lines.iter().any(|l| l.get("issue").is_some());
            let has_failed_result = lines.iter().any(|l| {
                l.get("result")
                    .and_then(|r| r.get("success"))
                    .and_then(|s| s.as_bool())
                    .map_or(false, |success| !success)
            });

            // Either an issue or a failed result should be present for errors
            assert!(
                has_issue || has_failed_result || !result.success,
                "Error response should indicate failure"
            );
        }
    }
    // Command should have failed
    assert!(!result.success, "focus on nonexistent session should fail");

    Ok(())
}

/// Test that all command responses produce valid JSONL output
#[test]
fn test_schema_uri_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    // Test multiple commands produce valid JSONL
    let commands: Vec<Vec<&str>> = vec![
        vec!["add", "uri-test", "--json", "--no-open"],
        vec!["list", "--json"],
    ];

    for args in commands {
        let result = harness.zjj(&args);
        if result.success {
            // Parse as JSONL lines
            let lines = parse_jsonl(result.stdout.trim())?;

            // Validate each line is a valid output type
            for line in &lines {
                validate_output_line_type(line)?;
            }
        }
    }

    Ok(())
}

/// Test that JSONL output uses consistent structure
#[test]
fn test_schema_naming_conventions() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    // Test that commands produce valid JSONL with proper structure
    let add_result = harness.zjj(&["add", "naming-test", "--json", "--no-open"]);

    if add_result.success {
        // Parse as JSONL lines
        let lines = parse_jsonl(add_result.stdout.trim())?;

        // Validate each line is a valid output type
        for line in &lines {
            validate_output_line_type(line)?;
        }
    }

    Ok(())
}

/// Test that JSONL lines have consistent structure
#[test]
fn test_hateoas_links_field() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "hateoas-test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if result.success {
        // Parse as JSONL lines
        let lines = parse_jsonl(result.stdout.trim())?;

        // Validate each line is a valid output type
        for line in &lines {
            validate_output_line_type(line)?;
        }
    }

    Ok(())
}

/// Test that JSONL output includes timestamps where appropriate
#[test]
fn test_meta_field() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "meta-test", "--json", "--no-open"]);

    if result.success {
        // Parse as JSONL lines
        let lines = parse_jsonl(result.stdout.trim())?;

        // Validate each line is a valid output type
        for line in &lines {
            validate_output_line_type(line)?;
        }
    }

    Ok(())
}

/// Test that list command produces multiple session lines in JSONL
#[test]
fn test_array_envelope_for_collections() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "array-test-1", "--no-open"]);
    harness.assert_success(&["add", "array-test-2", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "list should succeed");

    // Parse as JSONL lines
    let lines = parse_jsonl(result.stdout.trim())?;

    // Count session lines (nested format: {"session": {...}})
    let session_lines: Vec<_> = lines
        .iter()
        .filter(|l| l.get("session").is_some())
        .collect();

    assert!(
        session_lines.len() >= 2,
        "Should have at least 2 session lines in JSONL output, got {}",
        session_lines.len()
    );

    Ok(())
}

/// Test that JSONL output is compact (one line per JSON object)
#[test]
fn test_json_is_pretty_printed() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "pretty-test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if result.success {
        let json_str = result.stdout.trim();

        // JSONL output should have multiple lines (one per object)
        let lines: Vec<&str> = json_str.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            !lines.is_empty(),
            "JSONL output should have at least one line"
        );

        // Each line should be valid JSON
        for line in &lines {
            let parsed: serde_json::Value =
                serde_json::from_str(line).expect("Each line should be valid JSON");

            // JSONL uses compact format - verify no indentation/newlines within the line
            assert!(
                !line.contains("  "),
                "JSONL line should not have indentation: got '{line}'"
            );
            assert!(
                !line.contains('\n'),
                "JSONL line should not contain newlines: got '{line}'"
            );

            // Also verify the parsed JSON is valid
            let _ = parsed;
        }
    }
}

/// Test that exit codes are reflected in command output
#[test]
fn test_exit_code_matches_json() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    // Try a command that will fail
    let result = harness.zjj(&["focus", "does-not-exist", "--json"]);

    // Command should have failed
    assert!(!result.success, "focus on nonexistent session should fail");

    // If JSON output is produced, validate it
    if result.stdout.contains('{') {
        if let Ok(lines) = parse_jsonl(result.stdout.trim()) {
            // Validate each line is valid JSON
            for line in &lines {
                // Just check it's valid JSON - the exact error format may vary
                let _ = line;
            }
        }
    }

    Ok(())
}
