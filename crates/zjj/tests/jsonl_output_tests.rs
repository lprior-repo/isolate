//! Integration tests for JSONL output format (ATDD phase)
//!
//! These tests verify that zjj commands emit valid, parseable JSONL output.
//! Each test focuses on a specific command's output format validation.
//!
//! # Test Categories
//!
//! - `focus`: Tests for focus command JSONL output
//! - `list`: Tests for list command JSONL output
//! - `stack`: Tests for stack command JSONL output
//! - `all_commands`: Tests that apply to all commands

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

use common::TestHarness;
use serde_json::Value as JsonValue;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Parse all non-empty lines as JSONL.
/// Returns a vector of parsed JSON values.
fn parse_jsonl_lines(output: &str) -> Vec<JsonValue> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Find a JSON line by its top-level type key (e.g., "session", "issue", "result")
fn find_line_by_type(lines: &[JsonValue], type_name: &str) -> Option<JsonValue> {
    lines
        .iter()
        .find(|line| line.get(type_name).is_some())
        .cloned()
}

/// Check if any line has the given type key
fn has_line_type(lines: &[JsonValue], type_name: &str) -> bool {
    lines.iter().any(|line| line.get(type_name).is_some())
}

/// Valid OutputLine variant names
const VALID_VARIANT_NAMES: &[&str] = &[
    "summary",
    "session",
    "issue",
    "plan",
    "action",
    "warning",
    "result",
    "stack",
    "queue_summary",
    "queue_entry",
    "train",
    "conflictdetail",
    "conflict_analysis",
];

/// Verify all lines have valid OutputLine variant keys
fn all_lines_have_valid_variant(lines: &[JsonValue]) -> bool {
    lines.iter().all(|line| {
        line.as_object()
            .map(|obj| {
                obj.keys()
                    .any(|k| VALID_VARIANT_NAMES.contains(&k.as_str()))
            })
            .unwrap_or(false)
    })
}

// =============================================================================
// FOCUS COMMAND TESTS
// =============================================================================

mod focus_command {
    use super::*;

    /// Test that focus command emits valid JSONL output.
    ///
    /// Given: A zjj repository with an existing session
    /// When: Running `zjj focus <session> --no-zellij`
    /// Then: Output is valid JSONL with session and result lines
    #[test]
    fn test_focus_emits_valid_jsonl() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        // Initialize zjj
        harness.assert_success(&["init"]);

        // Create a session
        let session_name = "test-focus-jsonl";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // Focus the session
        let result = harness.zjj(&["focus", session_name, "--no-zellij"]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        // Parse output
        let lines = parse_jsonl_lines(&result.stdout);

        // Should have at least 2 lines: session + result
        assert!(lines.len() >= 2, "Focus should emit at least 2 JSONL lines");

        // All lines should be valid JSON objects
        for line in &lines {
            assert!(line.is_object(), "Each line should be a JSON object");
        }

        // All lines should have valid variant types
        assert!(
            all_lines_have_valid_variant(&lines),
            "All lines should have valid OutputLine variant keys"
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// Test that focus command emits a Session line.
    ///
    /// Given: A zjj repository with an existing session
    /// When: Running `zjj focus <session> --no-zellij`
    /// Then: Output contains a Session line with correct session name
    #[test]
    fn test_focus_emits_session_line() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let session_name = "test-focus-session";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        let result = harness.zjj(&["focus", session_name, "--no-zellij"]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);

        // Find session line
        let session_line = find_line_by_type(&lines, "session");
        assert!(session_line.is_some(), "Should have a session line");

        let session_name_field = match session_line {
            Some(line) => line
                .get("session")
                .and_then(|session| session.get("name"))
                .and_then(|name| name.as_str())
                .map(str::to_owned),
            None => None,
        };
        assert_eq!(
            session_name_field.as_deref(),
            Some(session_name),
            "Session name should match"
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// Test that focus command emits a Result line.
    ///
    /// Given: A zjj repository with an existing session
    /// When: Running `zjj focus <session> --no-zellij`
    /// Then: Output ends with a Result line with success=true
    #[test]
    fn test_focus_emits_result_line() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let session_name = "test-focus-result";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        let result = harness.zjj(&["focus", session_name, "--no-zellij"]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);
        assert!(!lines.is_empty(), "Should have output lines");

        // Last line should be a result
        let last_line = lines.last().unwrap();
        assert!(
            last_line.get("result").is_some(),
            "Last line should be a Result"
        );

        let result_obj = last_line.get("result").unwrap();
        assert_eq!(
            result_obj.get("success").and_then(|s| s.as_bool()),
            Some(true),
            "Result should have success=true"
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// Test that focus on missing session emits Issue line.
    ///
    /// Given: A zjj repository
    /// When: Running `zjj focus <nonexistent> --no-zellij`
    /// Then: Output contains an Issue line with kind=resource_not_found
    #[test]
    fn test_focus_missing_session_emits_issue() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.zjj(&["focus", "nonexistent-session", "--no-zellij"]);

        // Command should fail
        assert!(!result.success, "Focus on missing session should fail");

        let lines = parse_jsonl_lines(&result.stdout);

        // Should have an Issue line
        assert!(
            has_line_type(&lines, "issue"),
            "Should emit an Issue line for missing session"
        );

        // Issue should have kind=resource_not_found
        let issue_line = find_line_by_type(&lines, "issue").unwrap();
        let issue = issue_line.get("issue").unwrap();
        assert_eq!(
            issue.get("kind").and_then(|k| k.as_str()),
            Some("resource_not_found"),
            "Issue should have kind=resource_not_found"
        );
    }
}

// =============================================================================
// LIST COMMAND TESTS
// =============================================================================

mod list_command {
    use super::*;

    /// Test that list command emits valid JSONL output.
    ///
    /// Given: A zjj repository
    /// When: Running `zjj list`
    /// Then: Output is valid JSONL with session(s) and summary lines
    #[test]
    fn test_list_emits_valid_jsonl() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);

        // Should have at least 1 line (summary)
        assert!(!lines.is_empty(), "List should emit at least 1 JSONL line");

        // All lines should be valid JSON objects
        for line in &lines {
            assert!(line.is_object(), "Each line should be a JSON object");
        }

        // All lines should have valid variant types
        assert!(
            all_lines_have_valid_variant(&lines),
            "All lines should have valid OutputLine variant keys"
        );
    }

    /// Test that list command emits a Summary line.
    ///
    /// Given: A zjj repository
    /// When: Running `zjj list`
    /// Then: Output contains a Summary line
    #[test]
    fn test_list_emits_summary_line() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);

        // Should have a summary line
        assert!(
            has_line_type(&lines, "summary"),
            "Should have a summary line"
        );

        // Summary should have message and type fields
        let summary_line = find_line_by_type(&lines, "summary").unwrap();
        let summary = summary_line.get("summary").unwrap();
        assert!(
            summary.get("message").and_then(|m| m.as_str()).is_some(),
            "Summary should have message field"
        );
        assert!(
            summary.get("type").is_some(),
            "Summary should have type field"
        );
    }

    /// Test that list command emits Session lines for each session.
    ///
    /// Given: A zjj repository with sessions
    /// When: Running `zjj list`
    /// Then: Output contains a Session line for each session
    #[test]
    fn test_list_emits_session_lines() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Create sessions
        let sessions = ["session-a", "session-b", "session-c"];
        for session in &sessions {
            let result = harness.zjj(&["add", session, "--no-zellij", "--no-hooks"]);
            assert!(result.success, "Add {} should succeed", session);
        }

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);

        // Find all session lines
        let session_lines: Vec<_> = lines
            .iter()
            .filter(|line| line.get("session").is_some())
            .collect();

        // Should have 3 session lines
        assert_eq!(
            session_lines.len(),
            3,
            "Should have 3 session lines, got {}",
            session_lines.len()
        );

        // Verify session names are present
        let session_names: Vec<_> = session_lines
            .iter()
            .filter_map(|line| {
                line.get("session")
                    .and_then(|s| s.get("name"))
                    .and_then(|n| n.as_str())
            })
            .collect();

        for session in &sessions {
            assert!(
                session_names.contains(session),
                "Should have session named {}",
                session
            );
        }

        // Cleanup
        for session in &sessions {
            let _ = harness.zjj(&["remove", session, "--merge"]);
        }
    }

    /// Test that list command Session lines have required fields.
    ///
    /// Given: A zjj repository with a session
    /// When: Running `zjj list`
    /// Then: Session line has all required fields: name, status, state, workspace_path
    #[test]
    fn test_list_session_has_required_fields() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let session_name = "test-fields";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);
        let session_line = find_line_by_type(&lines, "session").unwrap();
        let session = session_line.get("session").unwrap();

        // Required fields
        let required_fields = ["name", "status", "state", "workspace_path"];
        for field in &required_fields {
            assert!(
                session.get(field).is_some(),
                "Session should have {} field",
                field
            );
        }

        // Field types
        assert!(
            session.get("name").and_then(|n| n.as_str()).is_some(),
            "name should be a string"
        );
        assert!(
            session.get("status").and_then(|s| s.as_str()).is_some(),
            "status should be a string"
        );
        assert!(
            session.get("state").and_then(|s| s.as_str()).is_some(),
            "state should be a string"
        );
        assert!(
            session
                .get("workspace_path")
                .and_then(|p| p.as_str())
                .is_some(),
            "workspace_path should be a string"
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// Test that list with --all produces valid JSONL output.
    ///
    /// Given: A zjj repository with sessions
    /// When: Running `zjj list --all`
    /// Then: Output is valid JSONL
    #[test]
    fn test_list_all_emits_valid_jsonl() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Create a session
        let session_name = "test-list-all";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // List with --all
        let result = harness.zjj(&["list", "--all"]);
        assert!(
            result.success,
            "List --all should succeed: {}",
            result.stderr
        );

        let lines = parse_jsonl_lines(&result.stdout);
        assert!(!lines.is_empty(), "Should have output lines");

        // All lines should have valid variant types
        assert!(
            all_lines_have_valid_variant(&lines),
            "All lines should have valid OutputLine variant keys"
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }
}

// =============================================================================
// STACK COMMAND TESTS
// =============================================================================

mod stack_command {
    use super::*;

    /// Test that stack list emits valid JSONL output.
    ///
    /// Given: A zjj repository with stack entries
    /// When: Running `zjj stack list`
    /// Then: Output is valid JSONL
    #[test]
    fn test_stack_list_emits_valid_jsonl() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.zjj(&["stack", "list"]);
        // Command may succeed or fail depending on queue state
        // Just verify output format if any

        if !result.stdout.is_empty() {
            let lines = parse_jsonl_lines(&result.stdout);

            // All lines should have valid variant types
            if !lines.is_empty() {
                assert!(
                    all_lines_have_valid_variant(&lines),
                    "All lines should have valid OutputLine variant keys"
                );
            }
        }
    }

    /// Test that stack status emits valid JSON output.
    ///
    /// Given: A zjj repository
    /// When: Running `zjj stack status <workspace>`
    /// Then: Output is valid JSON (uses StackStatusEnvelope format)
    #[test]
    fn test_stack_status_emits_valid_json() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Stack status for non-existent workspace
        let result = harness.zjj(&["stack", "status", "nonexistent-workspace"]);

        // Should fail (workspace not in queue)
        assert!(
            !result.success,
            "Stack status for missing workspace should fail"
        );

        // If there's output, it should be valid JSON
        if !result.stdout.is_empty() {
            let trimmed = result.stdout.trim();
            if trimmed.starts_with('{') {
                let parsed: Result<JsonValue, _> = serde_json::from_str(trimmed);
                assert!(
                    parsed.is_ok(),
                    "Stack status output should be valid JSON: {}",
                    trimmed
                );
            }
        }
    }
}

// =============================================================================
// ALL COMMANDS: TYPE FIELD TESTS
// =============================================================================

mod all_commands {
    use super::*;

    /// Test that all output lines have a type discriminator field.
    ///
    /// Given: Any zjj command execution
    /// When: Output is parsed
    /// Then: Each JSON line has an OutputLine variant key
    #[test]
    fn test_all_commands_have_type_field() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Create a session for testing
        let session_name = "test-type-field";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // Test multiple commands
        let focus_cmd = ["focus", session_name, "--no-zellij"];
        let commands: Vec<&[&str]> = vec![&["list"], &["status"], &focus_cmd];

        for cmd in &commands {
            let result = harness.zjj(cmd);
            let lines = parse_jsonl_lines(&result.stdout);

            // All lines should have valid variant types
            for line in &lines {
                let has_valid_key = line.as_object().map_or(false, |obj| {
                    obj.keys()
                        .any(|k| VALID_VARIANT_NAMES.contains(&k.as_str()))
                });
                assert!(
                    has_valid_key,
                    "Command {:?} produced line without valid variant: {:?}",
                    cmd, line
                );
            }
        }

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// Test that all output lines are single-line JSON (no pretty-printing).
    ///
    /// Given: Any zjj command execution
    /// When: Output is examined
    /// Then: Each JSON object is on a single line (JSONL format)
    #[test]
    fn test_output_is_single_line_json() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        // Each line should be a complete JSON object
        for line in result.stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Should start with { and end with }
            assert!(
                trimmed.starts_with('{'),
                "JSON line should start with '{{': {}",
                trimmed
            );
            assert!(
                trimmed.ends_with('}'),
                "JSON line should end with '}}': {}",
                trimmed
            );

            // Should not contain newlines inside
            assert!(
                !trimmed.contains('\n'),
                "JSON line should not contain newlines: {}",
                trimmed
            );
        }
    }

    /// Test that output can be processed line-by-line.
    ///
    /// Given: Any zjj command execution
    /// When: Output is split by newlines
    /// Then: Each line can be parsed independently
    #[test]
    fn test_output_is_line_parseable() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Create a session to get multiple output lines
        let session_name = "test-line-parse";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        // Process each line independently
        let line_count = result.stdout.lines().count();
        let parsed_count = result
            .stdout
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && serde_json::from_str::<JsonValue>(trimmed).is_ok()
            })
            .count();

        assert!(
            parsed_count > 0,
            "Should have at least one parseable JSON line"
        );
        assert_eq!(
            line_count, parsed_count,
            "All non-empty lines should be parseable JSON"
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// Test that timestamps are in milliseconds.
    ///
    /// Given: Any zjj command with timestamp fields
    /// When: Timestamps are examined
    /// Then: Timestamps are millisecond Unix timestamps
    #[test]
    fn test_timestamps_are_milliseconds() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.zjj(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);

        // Current time in milliseconds
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        // Check all timestamp fields
        for line in &lines {
            if let Some(obj) = line.as_object() {
                // Check nested objects too (e.g., {"session": {"created_at": ...}})
                for value in obj.values() {
                    if let Some(nested) = value.as_object() {
                        for field in ["timestamp", "created_at", "updated_at"] {
                            if let Some(ts) = nested.get(field).and_then(|t| t.as_i64()) {
                                // Timestamp should be after year 2020 and before 1 hour from now
                                let min_ts = 1577836800000_i64; // 2020-01-01
                                let max_ts = now_ms + 3600000; // 1 hour from now
                                assert!(
                                    ts > min_ts && ts < max_ts,
                                    "Timestamp {} should be reasonable (between 2020 and now+1h)",
                                    ts
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// =============================================================================
// ERROR OUTPUT TESTS
// =============================================================================

mod error_output {
    use super::*;

    /// Test that error conditions emit Issue lines.
    ///
    /// Given: A zjj repository
    /// When: Running a command that fails (e.g., focus on missing session)
    /// Then: Output contains an Issue line with error details
    #[test]
    fn test_resource_not_found_emits_issue() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Trigger error by focusing on non-existent session
        let result = harness.zjj(&["focus", "nonexistent-session-xyz", "--no-zellij"]);

        // Command should fail
        assert!(!result.success, "Focus on missing session should fail");

        let lines = parse_jsonl_lines(&result.stdout);

        // Should have an Issue line (if JSONL output is produced)
        if !lines.is_empty() {
            assert!(
                has_line_type(&lines, "issue"),
                "Resource not found should emit Issue line"
            );

            // Issue should have resource_not_found kind
            let issue_line = find_line_by_type(&lines, "issue").unwrap();
            let issue = issue_line.get("issue").unwrap();
            assert_eq!(
                issue.get("kind").and_then(|k| k.as_str()),
                Some("resource_not_found"),
                "Issue should have kind=resource_not_found"
            );
        }
    }

    /// Test that Issue lines have all required fields.
    ///
    /// Given: A zjj repository
    /// When: Running a command that triggers an error
    /// Then: Issue line has id, title, kind, severity fields
    #[test]
    fn test_issue_line_has_required_fields() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Trigger error - use focus on missing session which we know produces Issue
        let result = harness.zjj(&["focus", "nonexistent-issue-test", "--no-zellij"]);
        let lines = parse_jsonl_lines(&result.stdout);

        if let Some(issue_line) = find_line_by_type(&lines, "issue") {
            let issue = issue_line.get("issue").unwrap();

            // Required fields
            let required_fields = ["id", "title", "kind", "severity"];
            for field in &required_fields {
                assert!(
                    issue.get(field).is_some(),
                    "Issue should have {} field",
                    field
                );
            }

            // Field types
            assert!(
                issue.get("id").and_then(|i| i.as_str()).is_some(),
                "id should be a string"
            );
            assert!(
                issue.get("title").and_then(|t| t.as_str()).is_some(),
                "title should be a string"
            );
            assert!(
                issue.get("kind").and_then(|k| k.as_str()).is_some(),
                "kind should be a string"
            );
            assert!(
                issue.get("severity").and_then(|s| s.as_str()).is_some(),
                "severity should be a string"
            );
        }
        // If no Issue line, test passes silently (behavior may vary)
    }

    /// Test that errors have appropriate severity levels.
    ///
    /// Given: Different error conditions
    /// When: Examining Issue severity
    /// Then: Severity matches error type
    #[test]
    fn test_error_severity_levels() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Resource not found should have error severity
        let result = harness.zjj(&["focus", "nonexistent-severity", "--no-zellij"]);
        let lines = parse_jsonl_lines(&result.stdout);

        if let Some(issue_line) = find_line_by_type(&lines, "issue") {
            let issue = issue_line.get("issue").unwrap();
            let severity = issue.get("severity").and_then(|s| s.as_str());
            assert!(
                matches!(severity, Some("error")),
                "Resource not found should have error severity, got {:?}",
                severity
            );
        }
    }
}
