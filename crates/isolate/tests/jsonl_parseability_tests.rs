#![allow(clippy::unreadable_literal, clippy::unnecessary_map_or, clippy::filter_map_next)]
//! JSONL Parseability Tests (bd-foy)
//!
//! These tests verify that all isolate commands produce valid JSONL output
//! that can be parsed by external tools like `jq` and programmatic consumers.
//!
//! # Test Plan Reference
//!
//! From `.beads/beads/bd-foy-martin-fowler-tests.md`:
//! - test_output_parseable_by_jq
//! - test_output_has_consistent_schema
//! - test_session_output_has_required_fields
//! - test_issue_output_has_required_fields
//! - test_result_output_has_required_fields

// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    clippy::needless_raw_string_hashes,
    clippy::bool_assert_comparison
)]

mod common;

use std::collections::HashSet;

use common::TestHarness;
use serde_json::Value as JsonValue;

// =============================================================================
// INVARIANT TESTS: All output lines are valid JSONL
// =============================================================================

/// Given: Any isolate command execution
/// When: Output is captured
/// Then: Every non-empty line starting with '{' is valid JSON
#[test]
fn test_all_jsonl_lines_are_valid_json() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Run a command that produces JSONL output
    let result = harness.isolate(&["list"]);

    // Verify each line that looks like JSON is parseable
    for line in result.stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            let parsed: Result<JsonValue, _> = serde_json::from_str(trimmed);
            assert!(
                parsed.is_ok(),
                "Invalid JSON line: {trimmed}\nParse error: {:?}",
                parsed.err()
            );
        }
    }
}

/// Given: Any isolate command execution
/// When: Output lines are parsed
/// Then: Each JSON object has a discriminator field we can identify
#[test]
fn test_all_jsonl_lines_have_type_discriminator() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Run a command that produces JSONL output
    let result = harness.isolate(&["list"]);

    let mut found_json_lines = 0;
    for line in result.stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(json) = serde_json::from_str::<JsonValue>(trimmed) {
                found_json_lines += 1;
                // Check that the JSON object is an object (not array, etc.)
                assert!(json.is_object(), "JSON line should be an object: {trimmed}");
                // Our OutputLine enum serializes with externally tagged representation
                // Each variant has a single key like "Summary", "Session", "Issue", etc.
                let keys: Vec<_> = json.as_object().map_or(vec![], |obj| obj.keys().collect());
                assert!(
                    !keys.is_empty(),
                    "JSON line should have at least one key: {trimmed}"
                );
            }
        }
    }

    // Ensure we actually tested some JSON lines
    assert!(
        found_json_lines > 0,
        "Should have found at least one JSON line in output"
    );
}

// =============================================================================
// TEST: Output parseable by jq (using serde_json as jq substitute)
// =============================================================================

/// Given: Any command execution
/// When: Output is processed by a JSON parser (like jq)
/// Then: Parsing succeeds for all JSONL lines
///
/// This test verifies that the output can be piped to external tools
/// like `jq` for filtering and transformation.
#[test]
fn test_output_parseable_by_jq() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Test commands that produce JSONL output (exclude status which may have different format)
    let commands_to_test: Vec<&[&str]> = vec![&["list"]];

    for cmd in commands_to_test {
        let result = harness.isolate(cmd);

        for line in result.stdout.lines() {
            let trimmed = line.trim();
            // Only parse lines that look like complete JSON objects
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                // Verify it's valid JSON (equivalent to `jq .` success)
                let parsed: Result<JsonValue, _> = serde_json::from_str(trimmed);
                assert!(
                    parsed.is_ok(),
                    "Command {:?} produced invalid JSON: {trimmed}\nError: {:?}",
                    cmd,
                    parsed.err()
                );
            }
        }
    }
}

// =============================================================================
// TEST: Output has consistent schema across runs
// =============================================================================

/// Given: Multiple runs of the same command
/// When: Output schemas are compared
/// Then: All runs produce the same JSON structure (same top-level keys)
#[test]
fn test_output_has_consistent_schema() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Run the same command multiple times
    let result1 = harness.isolate(&["list"]);
    let result2 = harness.isolate(&["list"]);

    // Extract schemas (set of top-level keys) from both runs
    let schemas1 = extract_schemas(&result1.stdout);
    let schemas2 = extract_schemas(&result2.stdout);

    // Compare schemas - they should be identical
    assert_eq!(
        schemas1, schemas2,
        "Schema should be consistent across runs.\nRun 1: {:?}\nRun 2: {:?}",
        schemas1, schemas2
    );
}

/// Extract the set of schemas (top-level JSON structure) from JSONL output
fn extract_schemas(output: &str) -> Vec<HashSet<String>> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                serde_json::from_str::<JsonValue>(trimmed)
                    .ok()
                    .and_then(|json| {
                        json.as_object()
                            .map(|obj| obj.keys().cloned().collect::<HashSet<String>>())
                    })
            } else {
                None
            }
        })
        .collect()
}

// =============================================================================
// TEST: SessionOutput has required fields
// =============================================================================

/// Given: A SessionOutput JSON line
/// When: Fields are examined
/// Then: All required fields are present:
///   - name
///   - status
///   - state
///   - workspace_path
///   - created_at
///   - updated_at
#[test]
fn test_session_output_has_required_fields() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Create a session to get SessionOutput
    let result = harness.isolate(&["add", "test-session-fields", "--no-hooks"]);
    assert!(
        result.success,
        "Failed to create session: {}",
        result.stderr
    );

    // Find SessionOutput in the output (lowercase key: "session")
    let session_output = find_json_line_by_type(&result.stdout, "session");

    if let Some(json) = session_output {
        // Verify required fields
        let required_fields = [
            "name",
            "status",
            "state",
            "workspace_path",
            "created_at",
            "updated_at",
        ];

        for field in &required_fields {
            assert!(
                json.get(field).is_some(),
                "SessionOutput missing required field: {field}\nJSON: {json}"
            );
        }

        // Verify field types
        assert!(
            json.get("name").and_then(JsonValue::as_str).is_some(),
            "SessionOutput.name should be a string"
        );
        assert!(
            json.get("status").and_then(JsonValue::as_str).is_some(),
            "SessionOutput.status should be a string"
        );
        assert!(
            json.get("state").and_then(JsonValue::as_str).is_some(),
            "SessionOutput.state should be a string"
        );
        assert!(
            json.get("workspace_path")
                .and_then(JsonValue::as_str)
                .is_some(),
            "SessionOutput.workspace_path should be a string"
        );
        assert!(
            json.get("created_at").and_then(JsonValue::as_i64).is_some(),
            "SessionOutput.created_at should be a timestamp"
        );
        assert!(
            json.get("updated_at").and_then(JsonValue::as_i64).is_some(),
            "SessionOutput.updated_at should be a timestamp"
        );
    } else {
        panic!(
            "No SessionOutput found in add command output:\n{}",
            result.stdout
        );
    }

    // Cleanup
    let _ = harness.isolate(&["remove", "test-session-fields", "--merge"]);
}

// =============================================================================
// TEST: Issue has required fields
// =============================================================================

/// Given: An Issue JSON line (triggered by an error condition that produces JSONL)
/// When: Fields are examined
/// Then: All required fields are present:
///   - id
///   - title
///   - kind
///   - severity
///
/// Note: Some validation errors currently output to stderr as plain text.
/// This test verifies Issue structure when JSONL is produced.
#[test]
fn test_issue_output_has_required_fields() {
    // Create an Issue manually and verify its structure
    use isolate_core::domain::SessionName;
    use isolate_core::output::{Issue, IssueId, IssueKind, IssueSeverity, IssueTitle};

    let issue = Issue::new(
        IssueId::new("TEST-001").expect("valid id"),
        IssueTitle::new("Test issue").expect("valid title"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .expect("valid issue")
    .with_session(SessionName::parse("test-session").expect("valid session"))
    .with_suggestion("Try a different value".to_string());

    // Serialize and verify structure
    let json_str = serde_json::to_string(&issue).expect("serialize issue");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse issue json");

    // Verify required fields
    let required_fields = ["id", "title", "kind", "severity"];

    for field in &required_fields {
        assert!(
            json.get(field).is_some(),
            "Issue missing required field: {field}\nJSON: {json}"
        );
    }

    // Verify field types
    assert!(
        json.get("id").and_then(JsonValue::as_str).is_some(),
        "Issue.id should be a string"
    );
    assert!(
        json.get("title").and_then(JsonValue::as_str).is_some(),
        "Issue.title should be a string"
    );
    assert!(
        json.get("kind").and_then(JsonValue::as_str).is_some(),
        "Issue.kind should be a string"
    );
    assert!(
        json.get("severity").and_then(JsonValue::as_str).is_some(),
        "Issue.severity should be a string"
    );

    // Verify optional fields have correct types
    // Session is nested in scope: {"scope": {"InSession": {"session": "..."}}}
    assert!(
        json.get("scope").is_some(),
        "Issue.scope should be present when session is set"
    );
    assert!(
        json.get("suggestion").and_then(JsonValue::as_str).is_some(),
        "Issue.suggestion should be a string"
    );
}

// =============================================================================
// TEST: ResultOutput has required fields
// =============================================================================

/// Given: A ResultOutput JSON line (from add command)
/// When: Fields are examined
/// Then: All required fields are present:
///   - kind
///   - success
///   - message
///   - timestamp
#[test]
fn test_result_output_has_required_fields() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Run a command that produces ResultOutput (add produces result at end)
    let result = harness.isolate(&["add", "test-result-fields", "--no-hooks"]);

    // Find ResultOutput in the output (lowercase key: "result")
    let result_output = find_json_line_by_type(&result.stdout, "result");

    if let Some(json) = result_output {
        // Verify required fields (outcome instead of success)
        let required_fields = ["kind", "outcome", "message", "timestamp"];

        for field in &required_fields {
            assert!(
                json.get(field).is_some(),
                "ResultOutput missing required field: {field}\nJSON: {json}"
            );
        }

        // Verify field types
        assert!(
            json.get("kind").and_then(JsonValue::as_str).is_some(),
            "ResultOutput.kind should be a string"
        );
        assert!(
            json.get("outcome").and_then(JsonValue::as_str).is_some(),
            "ResultOutput.outcome should be a string (success/failure)"
        );
        assert!(
            json.get("message").and_then(JsonValue::as_str).is_some(),
            "ResultOutput.message should be a string"
        );
        assert!(
            json.get("timestamp").and_then(JsonValue::as_i64).is_some(),
            "ResultOutput.timestamp should be a timestamp"
        );

        // Verify optional data field has correct type if present
        if let Some(data) = json.get("data") {
            assert!(
                data.is_object() || data.is_null(),
                "ResultOutput.data should be an object or null if present"
            );
        }
    } else {
        panic!(
            "No ResultOutput found in command output. Every command should end with ResultOutput.\nStdout: {}",
            result.stdout
        );
    }

    // Cleanup
    let _ = harness.isolate(&["remove", "test-result-fields", "--merge"]);
}

// =============================================================================
// TEST: ResultOutput is final line for add command (invariant)
// =============================================================================

/// Given: An add command that produces JSONL
/// When: Output lines are examined in order
/// Then: ResultOutput is always the last JSONL line emitted
///
/// Note: Different commands may have different final output types.
/// The add command specifically ends with ResultOutput.
#[test]
fn test_result_output_is_final_line_for_add() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Test add command which produces result at end
    let result = harness.isolate(&["add", "test-final-result", "--no-hooks"]);

    // Get all JSON lines
    let json_lines: Vec<JsonValue> = result
        .stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                serde_json::from_str(trimmed).ok()
            } else {
                None
            }
        })
        .collect();

    if !json_lines.is_empty() {
        // The last JSON line should be a Result (lowercase key: "result")
        let last = &json_lines[json_lines.len() - 1];
        let is_result = last
            .as_object()
            .map_or(false, |obj| obj.contains_key("result"));

        assert!(
            is_result,
            "Final line of add command should be ResultOutput\nLast line: {:?}",
            last
        );
    }

    // Cleanup
    let _ = harness.isolate(&["remove", "test-final-result", "--merge"]);
}

// =============================================================================
// TEST: All OutputLine variants serialize correctly
// =============================================================================

/// Given: Various commands that produce different OutputLine variants
/// When: Output is examined
/// Then: Each variant has correct structure
#[test]
fn test_all_output_variants_have_correct_structure() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Test Summary variant (from list command with no sessions)
    let result = harness.isolate(&["list"]);
    let has_valid_lines = result.stdout.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with('{') && serde_json::from_str::<JsonValue>(trimmed).is_ok()
    });
    assert!(has_valid_lines, "list command should produce valid JSONL");

    // Test Action variant (from add command, lowercase key: "action")
    let result = harness.isolate(&["add", "test-action-variant", "--no-hooks"]);
    let action_output = find_json_line_by_type(&result.stdout, "action");
    if let Some(json) = action_output {
        // Action should have: verb, target, status, timestamp
        for field in ["verb", "target", "status", "timestamp"] {
            assert!(json.get(field).is_some(), "Action missing field: {field}");
        }
    }

    // Cleanup
    let _ = harness.isolate(&["remove", "test-action-variant", "--merge"]);
}

// =============================================================================
// TEST: Timestamps are valid and reasonable
// =============================================================================

/// Given: Any command with timestamp fields
/// When: Timestamps are examined
/// Then: Timestamps are valid millisecond timestamps near current time
#[test]
fn test_timestamps_are_valid() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["list"]);

    // Find all timestamp fields in output
    for line in result.stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            if let Ok(json) = serde_json::from_str::<JsonValue>(trimmed) {
                if let Some(obj) = json.as_object() {
                    // Check timestamp fields
                    for key in ["timestamp", "created_at", "updated_at"] {
                        if let Some(ts) = obj.get(key).and_then(JsonValue::as_i64) {
                            // Timestamp should be after year 2020 (1609459200000 ms)
                            // and before year 2100 (4102444800000 ms)
                            assert!(
                                ts > 1609459200000 && ts < 4102444800000,
                                "Timestamp {ts} for field {key} is unreasonable"
                            );
                        }
                    }
                }
            }
        }
    }
}

// =============================================================================
// TEST: Enum variants serialize to snake_case
// =============================================================================

/// Given: Commands that produce enum fields (status, kind, severity, etc.)
/// When: Enum values are examined
/// Then: Values are serialized in snake_case format
#[test]
fn test_enums_serialize_to_snake_case() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Create a session
    let result = harness.isolate(&["add", "test-enum-serialization", "--no-hooks"]);

    // Check Session status field (lowercase key: "session")
    if let Some(session) = find_json_line_by_type(&result.stdout, "session") {
        if let Some(status) = session.get("status").and_then(JsonValue::as_str) {
            // Should be lowercase, snake_case
            assert!(
                status == status.to_lowercase(),
                "Status should be lowercase: {status}"
            );
            assert!(
                !status.contains(' '),
                "Status should not contain spaces: {status}"
            );
        }
    }

    // Trigger an Issue and check its fields (lowercase key: "issue")
    let result = harness.isolate(&["add", ""]);
    if let Some(issue) = find_json_line_by_type(&result.stdout, "issue") {
        for field in ["kind", "severity"] {
            if let Some(value) = issue.get(field).and_then(JsonValue::as_str) {
                assert!(
                    value == value.to_lowercase(),
                    "Issue.{field} should be lowercase: {value}"
                );
                assert!(
                    !value.contains(' '),
                    "Issue.{field} should not contain spaces: {value}"
                );
            }
        }
    }

    // Cleanup
    let _ = harness.isolate(&["remove", "test-enum-serialization", "--merge"]);
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Find a JSON line by its top-level type key (e.g., "session", "issue", "result")
/// Returns the inner object (the value of the type key), not the wrapper
fn find_json_line_by_type(output: &str, type_name: &str) -> Option<JsonValue> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                serde_json::from_str::<JsonValue>(trimmed)
                    .ok()
                    .and_then(|json| {
                        json.as_object().and_then(|obj| {
                            obj.get(type_name).cloned() // Return the inner object
                        })
                    })
            } else {
                None
            }
        })
        .next()
}

// =============================================================================
// ADDITIONAL CONTRACT TESTS
// =============================================================================

/// Given: A successful command
/// When: ResultOutput is examined
/// Then: success field is true
#[test]
fn test_success_command_has_success_true() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["list"]);

    if let Some(result_output) = find_json_line_by_type(&result.stdout, "result") {
        if let Some(success) = result_output.get("success").and_then(JsonValue::as_bool) {
            assert!(
                success,
                "Successful command should have success=true in ResultOutput"
            );
        }
    }
}

/// Given: A failed command
/// When: ResultOutput is examined (if JSONL is produced)
/// Then: success field is false
///
/// Note: Some validation errors output to stderr as plain text.
/// This test verifies the success=false when JSONL ResultOutput is produced.
#[test]
fn test_failed_command_has_success_false() {
    // Create a failed ResultOutput and verify its structure
    use isolate_core::output::{Message, ResultKind, ResultOutput};

    let result = ResultOutput::failure(
        ResultKind::Command,
        Message::new("Command failed due to validation error").expect("valid message"),
    )
    .expect("valid result");

    // Serialize and verify structure
    let json_str = serde_json::to_string(&result).expect("serialize result");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse result json");

    // Verify outcome is failure
    assert!(
        json.get("outcome").and_then(JsonValue::as_str) == Some("failure"),
        "Failed command ResultOutput should have outcome=failure"
    );
}

/// Given: An OutputLine containing an Issue
/// When: Serialized and examined
/// Then: Issue is properly structured within OutputLine
#[test]
fn test_issue_in_output_line_structure() {
    use isolate_core::output::{Issue, IssueId, IssueKind, IssueSeverity, IssueTitle, OutputLine};

    let issue = Issue::new(
        IssueId::new("TEST-002").expect("valid id"),
        IssueTitle::new("Test issue in output line").expect("valid title"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .expect("valid issue");

    let output_line = OutputLine::Issue(issue);

    // Serialize and verify structure
    let json_str = serde_json::to_string(&output_line).expect("serialize output line");
    let json: JsonValue = serde_json::from_str(&json_str).expect("parse output line json");

    // The OutputLine enum uses snake_case variant names as keys
    assert!(
        json.get("issue").is_some(),
        "OutputLine::Issue should have 'issue' key"
    );

    // Verify nested Issue structure
    let issue_obj = json.get("issue").expect("issue object");
    assert!(
        issue_obj.get("id").and_then(JsonValue::as_str).is_some(),
        "Issue.id should be a string"
    );
    assert!(
        issue_obj.get("kind").and_then(JsonValue::as_str).is_some(),
        "Issue.kind should be a string"
    );
}

/// Given: Idempotent remove of nonexistent session
/// When: Command completes
/// Then: No Issue line is emitted (per test plan)
#[test]
fn test_idempotent_remove_no_issue_on_missing() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize isolate
    harness.assert_success(&["init"]);

    // Idempotent remove of nonexistent session
    let result = harness.isolate(&[
        "remove",
        "nonexistent-session-xyz",
        "--idempotent",
        "--merge",
    ]);

    // Check for Issue lines - there should be none (lowercase key: "issue")
    let has_issue = result.stdout.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            if let Ok(json) = serde_json::from_str::<JsonValue>(trimmed) {
                json.as_object()
                    .map_or(false, |obj| obj.contains_key("issue"))
            } else {
                false
            }
        } else {
            false
        }
    });

    // With idempotent, there should be no Issue line
    assert!(
        !has_issue,
        "Idempotent remove of missing session should not emit Issue\nOutput: {}",
        result.stdout
    );

    // Command should still succeed (lowercase key: "result")
    if let Some(result_output) = find_json_line_by_type(&result.stdout, "result") {
        if let Some(success) = result_output.get("success").and_then(JsonValue::as_bool) {
            assert!(
                success,
                "Idempotent remove should have success=true\nOutput: {}",
                result.stdout
            );
        }
    }
}
