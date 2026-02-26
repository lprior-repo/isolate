#![allow(clippy::unnecessary_map_or)]
//! Integration tests for JSONL output format (ATDD phase)
//!
//! These tests verify that isolate commands emit valid, parseable JSONL output.
//! Each test focuses on a specific command's output format validation.
//!
//! # Test Categories
//!
//! - `focus`: Tests for focus command JSONL output
//! - `list`: Tests for list command JSONL output
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
    #[test]
    fn test_focus_emits_valid_jsonl() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        // Initialize isolate
        harness.assert_success(&["init"]);

        // Create a session
        let session_name = "test-focus-jsonl";
        let result = harness.isolate(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // Focus the session
        let result = harness.isolate(&["focus", session_name]);
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
        let _ = harness.isolate(&["remove", session_name, "--merge"]);
    }

    /// Test that focus command emits a Session line.
    #[test]
    fn test_focus_emits_session_line() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let session_name = "test-focus-session";
        let result = harness.isolate(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        let result = harness.isolate(&["focus", session_name]);
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
        let _ = harness.isolate(&["remove", session_name, "--merge"]);
    }
}

// =============================================================================
// LIST COMMAND TESTS
// =============================================================================

mod list_command {
    use super::*;

    /// Test that list command emits valid JSONL output.
    #[test]
    fn test_list_emits_valid_jsonl() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.isolate(&["list"]);
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
    #[test]
    fn test_list_emits_summary_line() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        let result = harness.isolate(&["list"]);
        assert!(result.success, "List should succeed: {}", result.stderr);

        let lines = parse_jsonl_lines(&result.stdout);

        // Should have a summary line
        assert!(
            has_line_type(&lines, "summary"),
            "Should have a summary line"
        );
    }
}

// =============================================================================
// ALL COMMANDS: TYPE FIELD TESTS
// =============================================================================

mod all_commands {
    use super::*;

    /// Test that all output lines have a type discriminator field.
    #[test]
    fn test_all_commands_have_type_field() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        harness.assert_success(&["init"]);

        // Create a session for testing
        let session_name = "test-type-field";
        let result = harness.isolate(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // Test multiple commands
        let commands: Vec<&[&str]> = vec![&["list"], &["status"]];

        for cmd in &commands {
            let result = harness.isolate(cmd);
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
        let _ = harness.isolate(&["remove", session_name, "--merge"]);
    }
}
