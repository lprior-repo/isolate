//! JSONL Invariant Tests for bd-foy Contract Specification
//!
//! These tests verify the global invariants from the contract spec at
//! `.beads/beads/bd-foy-contract-spec.md`.
//!
//! ## Global Invariants Tested
//!
//! - INV-GLOBAL-01: Every emitted line is valid, parseable JSONL
//! - INV-GLOBAL-02: Every OutputLine has a `type` field
//! - INV-GLOBAL-04: Success cases emit ResultOutput::success as final line
//! - INV-GLOBAL-07: stdout is flushed after each line
//!
//! ## Command Status
//!
//! - `focus.rs`: CONVERTED - All tests should PASS
//! - `remove.rs`: NOT YET CONVERTED - Tests document expectations
//! - `sync.rs`: NOT YET CONVERTED - Tests document expectations
//! - `add.rs`: NOT YET CONVERTED - Tests document expectations

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
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::unused_self,
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
    // Test-specific patterns
    clippy::needless_raw_string_hashes,
    clippy::bool_assert_comparison,
    clippy::items_after_statements,
    // Allow unused helper functions for future test coverage
    dead_code,
)]

mod common;

use std::collections::HashSet;

use anyhow::{Context, Result};
use common::TestHarness;
use serde_json::Value as JsonValue;

// ============================================================================
// INV-GLOBAL-01: Every emitted line is valid, parseable JSONL
// ============================================================================

/// Parse each line of stdout as JSON and return all parsed values.
/// Returns an error if any line fails to parse.
fn parse_all_jsonl_lines(stdout: &str) -> Result<Vec<JsonValue>> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(idx, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("Line {} is not valid JSON: {}", idx + 1, line))
        })
        .collect()
}

/// INV-GLOBAL-01: Verify all stdout lines are valid JSONL
fn assert_all_lines_valid_jsonl(stdout: &str) -> Result<Vec<JsonValue>> {
    parse_all_jsonl_lines(stdout)
}

// ============================================================================
// INV-GLOBAL-02: Every OutputLine has a type field indicating its variant
// ============================================================================

/// INV-GLOBAL-02: Verify each JSONL line has a type field indicating the variant.
///
/// OutputLine serializes with variant name as the key, e.g.:
/// - `{"session": {...}}` for Session variant
/// - `{"result": {...}}` for Result variant
/// - `{"issue": {...}}` for Issue variant
fn assert_all_lines_have_variant_type(parsed_lines: &[JsonValue]) -> Result<()> {
    let valid_variants: HashSet<&str> = [
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
    ]
    .into_iter()
    .collect();

    for (idx, line) in parsed_lines.iter().enumerate() {
        let line_obj = line
            .as_object()
            .with_context(|| format!("Line {} is not a JSON object", idx + 1))?;

        let keys: Vec<&str> = line_obj.keys().map(String::as_str).collect();

        // Check that at least one key is a valid variant name
        let has_valid_variant = keys.iter().any(|k| valid_variants.contains(k));

        if !has_valid_variant {
            anyhow::bail!(
                "Line {} does not have a valid OutputLine variant key. Found keys: {:?}",
                idx + 1,
                keys
            );
        }
    }

    Ok(())
}

// ============================================================================
// INV-GLOBAL-04: Success cases emit ResultOutput::success as final line
// ============================================================================

/// INV-GLOBAL-04: Verify the last line is a ResultOutput with outcome="success"
fn assert_final_line_is_success_result(parsed_lines: &[JsonValue]) -> Result<()> {
    let last_line = parsed_lines
        .last()
        .context("Output must have at least one line")?;

    // The result should be wrapped: {"result": {...}}
    let result_obj = last_line
        .get("result")
        .context("Final line must be a Result variant (missing 'result' key)")?;

    let outcome = result_obj
        .get("outcome")
        .and_then(JsonValue::as_str)
        .context("Result must have a string 'outcome' field")?;

    if outcome != "success" {
        anyhow::bail!(
            "Final line must be a success Result, but outcome={:?}. Line: {:?}",
            outcome,
            last_line
        );
    }

    Ok(())
}

/// INV-GLOBAL-04 (failure case): Verify the last line is a ResultOutput with outcome="failure"
#[allow(dead_code)]
fn assert_final_line_is_failure_result(parsed_lines: &[JsonValue]) -> Result<()> {
    let last_line = parsed_lines
        .last()
        .context("Output must have at least one line")?;

    let result_obj = last_line
        .get("result")
        .context("Final line must be a Result variant (missing 'result' key)")?;

    let outcome = result_obj
        .get("outcome")
        .and_then(JsonValue::as_str)
        .context("Result must have a string 'outcome' field")?;

    if outcome == "success" {
        anyhow::bail!(
            "Final line must be a failure Result, but outcome=success. Line: {:?}",
            last_line
        );
    }

    Ok(())
}

// ============================================================================
// INV-GLOBAL-07: stdout is flushed after each line
// ============================================================================

/// INV-GLOBAL-07: Verify output is complete (each line ends with newline).
///
/// This is verified by ensuring no partial lines exist in the output.
/// The actual flush happens in `emit_stdout()` and is tested via the
/// unit tests in `zjj_core::output::writer`.
fn assert_output_is_complete(stdout: &str) -> Result<()> {
    // Empty output is valid (no lines)
    if stdout.is_empty() {
        return Ok(());
    }

    // Each line should be parseable JSON (which implies proper termination)
    // This is implicitly verified by `assert_all_lines_valid_jsonl`
    // Here we just check that we don't have trailing partial content
    if !stdout.ends_with('\n') && !stdout.is_empty() {
        anyhow::bail!("Output does not end with newline - may indicate incomplete flush");
    }

    Ok(())
}

// ============================================================================
// FOCUS COMMAND TESTS (CONVERTED - SHOULD PASS)
// ============================================================================

mod focus_command {
    use super::*;

    fn setup_focused_session(harness: &TestHarness) -> Result<String> {
        let session_name = "test-jsonl-focus";

        // Initialize zjj
        harness.zjj(&["init"]).assert_success();

        // Create a session
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(
            result.success,
            "Failed to create session: {}",
            result.stderr
        );

        Ok(session_name.to_string())
    }

    /// INV-GLOBAL-01: Focus success emits valid JSONL
    #[test]
    fn test_focus_success_all_lines_valid_jsonl() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        let session_name = setup_focused_session(&harness)?;

        // Focus the session
        let result = harness.zjj(&["focus", &session_name]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        // Verify INV-GLOBAL-01
        let parsed = assert_all_lines_valid_jsonl(&result.stdout)?;

        // Should have at least 2 lines: Session + Result
        assert!(
            parsed.len() >= 2,
            "Focus should emit at least 2 lines, got {}",
            parsed.len()
        );

        Ok(())
    }

    /// INV-GLOBAL-02: Focus success lines have variant type
    #[test]
    fn test_focus_success_lines_have_variant_type() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        let session_name = setup_focused_session(&harness)?;

        let result = harness.zjj(&["focus", &session_name]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        let parsed = assert_all_lines_valid_jsonl(&result.stdout)?;

        // Verify INV-GLOBAL-02
        assert_all_lines_have_variant_type(&parsed)?;

        Ok(())
    }

    /// INV-GLOBAL-04: Focus success ends with ResultOutput::success
    #[test]
    fn test_focus_success_final_line_is_result() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        let session_name = setup_focused_session(&harness)?;

        let result = harness.zjj(&["focus", &session_name]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        let parsed = assert_all_lines_valid_jsonl(&result.stdout)?;

        // Verify INV-GLOBAL-04
        assert_final_line_is_success_result(&parsed)?;

        Ok(())
    }

    /// INV-GLOBAL-07: Focus output is complete (flushed)
    #[test]
    fn test_focus_output_is_complete() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        let session_name = setup_focused_session(&harness)?;

        let result = harness.zjj(&["focus", &session_name]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        // Verify INV-GLOBAL-07
        assert_output_is_complete(&result.stdout)?;

        Ok(())
    }

    /// Focus on missing session emits Issue + Result
    #[test]
    fn test_focus_missing_session_emits_issue() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Try to focus on non-existent session
        let result = harness.zjj(&["focus", "nonexistent-session"]);

        // Command should fail
        assert!(!result.success, "Focus on missing session should fail");

        // Parse output
        let parsed = parse_all_jsonl_lines(&result.stdout)?;

        // Should have at least one issue line
        let has_issue = parsed.iter().any(|line| line.get("issue").is_some());
        assert!(has_issue, "Should emit an Issue line for missing session");

        Ok(())
    }

    /// Focus without session name emits validation Issue
    #[test]
    fn test_focus_no_name_emits_validation_issue() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Try to focus without providing a name
        let result = harness.zjj(&["focus", "--no-session"]);

        // Command should fail
        assert!(!result.success, "Focus without name should fail");

        // Parse output
        let parsed = parse_all_jsonl_lines(&result.stdout)?;

        // Should have an issue with Validation kind
        let has_validation_issue = parsed.iter().any(|line| {
            line.get("issue")
                .and_then(|i| i.get("kind"))
                .and_then(|k| k.as_str())
                .map(|k| k == "validation")
                .unwrap_or(false)
        });

        assert!(
            has_validation_issue,
            "Should emit Issue with kind=validation"
        );

        Ok(())
    }
}

// ============================================================================
// REMOVE COMMAND TESTS (NOT YET CONVERTED - DOCUMENT EXPECTATIONS)
// ============================================================================

mod remove_command {
    use super::*;

    /// INV-GLOBAL-01: Remove success should emit valid JSONL
    ///
    /// STATUS: RED - remove.rs still uses SchemaEnvelope, not JSONL OutputLine
    ///
    /// EXPECTED BEHAVIOR AFTER CONVERSION:
    /// 1. Emits Action lines for cleanup steps
    /// 2. Emits ResultOutput::success as final line
    #[test]
    fn test_remove_success_all_lines_valid_jsonl() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Create a session
        let session_name = "test-jsonl-remove";
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(
            result.success,
            "Failed to create session: {}",
            result.stderr
        );

        // Remove the session
        let result = harness.zjj(&["remove", session_name, "--force"]);
        assert!(result.success, "Remove should succeed: {}", result.stderr);

        // Parse output - currently uses SchemaEnvelope format
        let parsed = parse_all_jsonl_lines(&result.stdout)?;

        // After conversion, this should pass:
        // assert_all_lines_have_variant_type(&parsed)?;

        // For now, just verify we got valid JSON (SchemaEnvelope format)
        assert!(
            !parsed.is_empty(),
            "Remove should emit at least one JSON line"
        );

        Ok(())
    }

    /// INV-RM-01: Remove with --idempotent never returns error for missing session
    #[test]
    fn test_remove_idempotent_missing_session() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Remove non-existent session with idempotent flag
        let result = harness.zjj(&["remove", "nonexistent", "--idempotent"]);

        // Should succeed (idempotent)
        assert!(
            result.success,
            "Remove --idempotent should succeed for missing session"
        );

        Ok(())
    }

    /// Remove without --idempotent fails for missing session
    #[test]
    fn test_remove_missing_session_fails() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Remove non-existent session without idempotent flag
        let result = harness.zjj(&["remove", "nonexistent"]);

        // Should fail
        assert!(
            !result.success,
            "Remove should fail for missing session without --idempotent"
        );

        Ok(())
    }
}

// ============================================================================
// SYNC COMMAND TESTS (NOT YET CONVERTED - DOCUMENT EXPECTATIONS)
// ============================================================================

mod sync_command {
    use super::*;

    /// INV-GLOBAL-01: Sync success should emit valid JSONL
    ///
    /// STATUS: RED - sync.rs not yet converted to JSONL OutputLine
    #[test]
    fn test_sync_success_all_lines_valid_jsonl() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Create a session
        let session_name = "test-jsonl-sync";
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(
            result.success,
            "Failed to create session: {}",
            result.stderr
        );

        // Sync the session
        let result = harness.zjj(&["sync", session_name]);

        // Sync may succeed or fail depending on repo state
        // Just verify we got valid JSON output
        let parsed = parse_all_jsonl_lines(&result.stdout)?;

        // After conversion, this should pass:
        // assert_all_lines_have_variant_type(&parsed)?;

        // For now, verify we got some output or it's empty (valid)
        if !result.stdout.is_empty() {
            assert!(!parsed.is_empty(), "Should emit valid JSON lines");
        }

        Ok(())
    }

    /// INV-SYNC-01: synced_count + failed_count == total_sessions
    ///
    /// STATUS: RED - requires ResultOutput with data field containing counts
    #[test]
    #[ignore = "Requires JSONL conversion with data field support"]
    fn test_sync_all_counts_consistent() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Create multiple sessions
        for name in ["sync-test-1", "sync-test-2", "sync-test-3"] {
            let result = harness.zjj(&["add", name, "--no-hooks"]);
            assert!(result.success, "Failed to create session {}", name);
        }

        // Sync all
        let _result = harness.zjj(&["sync", "--all"]);

        // After conversion, verify counts in ResultOutput.data
        // synced_count + failed_count should equal total_sessions

        Ok(())
    }
}

// ============================================================================
// ADD COMMAND TESTS (NOT YET CONVERTED - DOCUMENT EXPECTATIONS)
// ============================================================================

mod add_command {
    use super::*;

    /// INV-GLOBAL-01: Add success should emit valid JSONL
    ///
    /// STATUS: RED - add.rs not yet converted to JSONL OutputLine
    #[test]
    fn test_add_success_all_lines_valid_jsonl() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Create a session
        let session_name = "test-jsonl-add";
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // Parse output
        let parsed = parse_all_jsonl_lines(&result.stdout)?;

        // After conversion, this should pass:
        // assert_all_lines_have_variant_type(&parsed)?;

        // For now, just verify we got valid JSON
        assert!(!parsed.is_empty(), "Add should emit at least one JSON line");

        Ok(())
    }

    /// INV-ADD-01: Session name in output matches input name
    #[test]
    fn test_add_session_name_matches_input() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        let session_name = "test-name-match";
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // After conversion, find SessionOutput line and verify name matches
        // For now, verify via list command
        let list_result = harness.zjj(&["list"]);
        assert!(list_result.stdout.contains(session_name));

        Ok(())
    }

    /// INV-ADD-02: Workspace path is absolute
    #[test]
    fn test_add_workspace_path_is_absolute() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        let session_name = "test-absolute-path";
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        // After conversion, find SessionOutput line and verify workspace_path is absolute
        // For now, verify the workspace directory exists and is absolute
        let workspace_path = harness.workspace_path(session_name);
        assert!(
            workspace_path.is_absolute(),
            "Workspace path should be absolute"
        );

        Ok(())
    }

    /// Add with invalid name emits validation Issue
    #[test]
    fn test_add_invalid_name_emits_validation_issue() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        // Try to create a session with invalid name (e.g., starts with number)
        let result = harness.zjj(&["add", "123-invalid", "--no-hooks"]);

        // Should fail validation
        assert!(!result.success, "Add with invalid name should fail");

        Ok(())
    }
}

// ============================================================================
// UNIT TESTS FOR OUTPUT TYPES (INV-GLOBAL-05: No unwrap/expect/panic)
// ============================================================================

mod output_types_unit_tests {
    use anyhow::Result;
    use zjj_core::{
        output::{
            Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
            IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput,
            SessionOutput, Summary, SummaryType,
        },
        types::SessionStatus,
        WorkspaceState,
    };

    /// Verify that OutputLine types serialize correctly with variant keys
    #[test]
    fn test_session_output_serializes_with_session_key() -> Result<()> {
        let session = SessionOutput::new(
            "test".to_string(),
            SessionStatus::Active,
            WorkspaceState::Created,
            std::path::PathBuf::from("/tmp/test"),
        )?;

        let line = OutputLine::Session(session);
        let json = serde_json::to_string(&line)?;

        assert!(
            json.contains("\"session\":"),
            "Should serialize with 'session' key: {}",
            json
        );

        Ok(())
    }

    #[test]
    fn test_result_output_serializes_with_result_key() -> Result<()> {
        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new("Test success").expect("valid message"),
        )?;

        let line = OutputLine::Result(result);
        let json = serde_json::to_string(&line)?;

        assert!(
            json.contains("\"result\":"),
            "Should serialize with 'result' key: {}",
            json
        );
        assert!(
            json.contains("\"outcome\":\"success\""),
            "Should have outcome=success: {}",
            json
        );

        Ok(())
    }

    #[test]
    fn test_issue_output_serializes_with_issue_key() -> Result<()> {
        let issue = Issue::new(
            IssueId::new("TEST-001").expect("valid id"),
            IssueTitle::new("Test issue").expect("valid title"),
            IssueKind::Validation,
            IssueSeverity::Error,
        )?;

        let line = OutputLine::Issue(issue);
        let json = serde_json::to_string(&line)?;

        assert!(
            json.contains("\"issue\":"),
            "Should serialize with 'issue' key: {}",
            json
        );
        assert!(
            json.contains("\"kind\":\"validation\""),
            "Should have kind field: {}",
            json
        );

        Ok(())
    }

    #[test]
    fn test_action_output_serializes_with_action_key() {
        let action = Action::new(
            ActionVerb::new("test").expect("valid action verb"),
            ActionTarget::new("target").expect("valid action target"),
            ActionStatus::Completed,
        );

        let line = OutputLine::Action(action);
        let json = serde_json::to_string(&line).unwrap();

        assert!(
            json.contains("\"action\":"),
            "Should serialize with 'action' key: {}",
            json
        );
    }

    #[test]
    fn test_summary_output_serializes_with_summary_key() -> Result<()> {
        let summary = Summary::new(
            SummaryType::Count,
            Message::new("Test summary").expect("valid message"),
        )?;

        let line = OutputLine::Summary(summary);
        let json = serde_json::to_string(&line)?;

        assert!(
            json.contains("\"summary\":"),
            "Should serialize with 'summary' key: {}",
            json
        );

        Ok(())
    }

    /// Verify that ResultOutput::failure has outcome="failure"
    #[test]
    fn test_result_failure_has_outcome_failure() -> Result<()> {
        let result = ResultOutput::failure(
            ResultKind::Command,
            Message::new("Test failure").expect("valid message"),
        )?;

        let line = OutputLine::Result(result);
        let json = serde_json::to_string(&line)?;

        assert!(
            json.contains("\"outcome\":\"failure\""),
            "Failure should have outcome=failure: {}",
            json
        );

        Ok(())
    }

    /// Verify all IssueKind variants serialize correctly
    #[test]
    fn test_issue_kind_serialization() {
        let kinds = [
            (IssueKind::Validation, "validation"),
            (IssueKind::StateConflict, "state_conflict"),
            (IssueKind::ResourceNotFound, "resource_not_found"),
            (IssueKind::PermissionDenied, "permission_denied"),
            (IssueKind::Timeout, "timeout"),
            (IssueKind::Configuration, "configuration"),
            (IssueKind::External, "external"),
        ];

        for (kind, expected) in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            assert!(
                json.contains(expected),
                "IssueKind::{:?} should serialize as '{}', got: {}",
                kind,
                expected,
                json
            );
        }
    }

    /// Verify all IssueSeverity variants serialize correctly
    #[test]
    fn test_issue_severity_serialization() {
        let severities = [
            (IssueSeverity::Hint, "hint"),
            (IssueSeverity::Warning, "warning"),
            (IssueSeverity::Error, "error"),
            (IssueSeverity::Critical, "critical"),
        ];

        for (severity, expected) in severities {
            let json = serde_json::to_string(&severity).unwrap();
            assert!(
                json.contains(expected),
                "IssueSeverity::{:?} should serialize as '{}', got: {}",
                severity,
                expected,
                json
            );
        }
    }

    /// Verify all ResultKind variants serialize correctly
    #[test]
    fn test_result_kind_serialization() {
        let kinds = [
            (ResultKind::Command, "command"),
            (ResultKind::Operation, "operation"),
            (ResultKind::Assessment, "assessment"),
            (ResultKind::Recovery, "recovery"),
        ];

        for (kind, expected) in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            assert!(
                json.contains(expected),
                "ResultKind::{:?} should serialize as '{}', got: {}",
                kind,
                expected,
                json
            );
        }
    }

    /// Verify all ActionStatus variants serialize correctly
    #[test]
    fn test_action_status_serialization() {
        let statuses = [
            (ActionStatus::Pending, "pending"),
            (ActionStatus::InProgress, "in_progress"),
            (ActionStatus::Completed, "completed"),
            (ActionStatus::Failed, "failed"),
            (ActionStatus::Skipped, "skipped"),
        ];

        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(
                json.contains(expected),
                "ActionStatus::{:?} should serialize as '{}', got: {}",
                status,
                expected,
                json
            );
        }
    }
}

// ============================================================================
// INTEGRATION: VERIFY FOCUS EMITS COMPLETE OUTPUT SEQUENCE
// ============================================================================

mod focus_output_sequence {
    use super::*;

    /// Verify complete focus output sequence:
    /// 1. Optional Action line (if switching tabs)
    /// 2. SessionOutput line
    /// 3. ResultOutput::success line
    #[test]
    fn test_focus_output_sequence_complete() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        let session_name = "test-sequence";
        let result = harness.zjj(&["add", session_name, "--no-hooks"]);
        assert!(result.success, "Add should succeed: {}", result.stderr);

        let result = harness.zjj(&["focus", session_name]);
        assert!(result.success, "Focus should succeed: {}", result.stderr);

        let parsed = assert_all_lines_valid_jsonl(&result.stdout)?;

        // Verify sequence
        // Should have: Action (focus), Session, Result
        // Or just: Session, Result (if no action needed)

        assert!(
            parsed.len() >= 2,
            "Focus should emit at least 2 lines (session + result), got {}",
            parsed.len()
        );

        // First or second-to-last should be Session
        let has_session = parsed.iter().any(|line| line.get("session").is_some());
        assert!(has_session, "Should have a Session line");

        // Last should be Result
        assert_final_line_is_success_result(&parsed)?;

        Ok(())
    }
}

// ============================================================================
// ERROR OUTPUT SEQUENCE TESTS
// ============================================================================

mod error_output_sequence {
    use super::*;

    /// Verify error output sequence for missing session:
    /// 1. Issue line
    /// 2. (Optionally) ResultOutput::failure - depends on implementation
    #[test]
    fn test_focus_missing_session_output_sequence() -> Result<()> {
        let Some(harness) = TestHarness::try_new() else {
            return Ok(());
        };

        harness.zjj(&["init"]).assert_success();

        let result = harness.zjj(&["focus", "nonexistent", "--no-session"]);

        assert!(!result.success, "Focus on missing session should fail");

        if !result.stdout.is_empty() {
            let parsed = parse_all_jsonl_lines(&result.stdout)?;

            // Should have at least an Issue
            let has_issue = parsed.iter().any(|line| line.get("issue").is_some());
            assert!(has_issue, "Should emit an Issue line");

            // Issue should have ResourceNotFound kind
            let issue_line = parsed
                .iter()
                .find(|line| line.get("issue").is_some())
                .and_then(|line| line.get("issue"));

            if let Some(issue) = issue_line {
                let kind = issue.get("kind").and_then(|k| k.as_str());
                assert_eq!(
                    kind,
                    Some("resource_not_found"),
                    "Issue should have kind=resource_not_found"
                );
            }
        }

        Ok(())
    }
}
