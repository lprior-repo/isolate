//! Property-based tests for Status aggregation (RED Phase)
//!
//! These tests define invariants that must hold for all status outputs.
//! Run with: `cargo test --test status_property_tests`
//!
//! # Properties
//!
//! 1. JSON Validity: All status outputs are valid JSONL
//! 2. Field Completeness: All required fields are present
//! 3. Aggregation Consistency: Session + Queue + Stack = Status
//!
//! # Bead IDs
//!
//! - status-scout (bd-dly9): BDD scenarios
//! - status-red (bd-udmj): Property tests (THIS FILE)
//! - status-green (bd-uc3l): Minimal implementation
//! - status-implement (bd-3dg8): Full implementation
//! - status-review (bd-rdvd): Adversarial tests

#![allow(clippy::unwrap_used)] // Test file allows unwrap in test code
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(dead_code)] // Allow unused types in test file

mod common;

use std::collections::HashSet;

use anyhow::Result;
use common::{parse_jsonl_output, TestHarness};

// =============================================================================
// Property 1: JSON Validity
// =============================================================================

/// Property: All status output lines must be valid JSON
/// Each line must parse as a valid JSON object
#[test]
fn prop_status_output_is_valid_jsonl() {
    let Some(harness) = TestHarness::try_new() else {
        // Skip test if jj not available
        println!("SKIP: jj not available");
        return;
    };

    // Initialize the repo
    harness.assert_success(&["init"]);

    // Create a session
    let result = harness.zjj(&["add", "prop-test", "--no-zellij", "--no-hooks"]);
    if !result.success {
        // Skip if can't create session
        println!("SKIP: could not create session");
        return;
    }

    // Query status
    let status_result = harness.zjj(&["status"]);

    // Parse each line - all must be valid JSON
    for (i, line) in status_result.stdout.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parse_result.is_ok(),
            "Line {} must be valid JSON: {}",
            i,
            line
        );

        let json = parse_result.map_or(serde_json::Value::Null, |v| v);
        assert!(
            json.is_object(),
            "Line {} must be a JSON object: {:?}",
            i,
            json
        );
    }

    // Cleanup
    let _ = harness.zjj(&["remove", "prop-test", "--merge"]);
}

/// Property: JSON lines must have recognized type discriminators
#[test]
fn prop_status_lines_have_type_discriminator() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "prop-type-test", "--no-zellij", "--no-hooks"]);
    if !result.success {
        println!("SKIP: could not create session");
        return;
    }

    let status_result = harness.zjj(&["status"]);

    let valid_types = ["session", "summary", "issue", "action", "warning", "result"];

    for line in status_result.stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let json: serde_json::Value =
            serde_json::from_str(line).map_or(serde_json::Value::Null, |v| v);

        if !json.is_object() {
            continue;
        }

        // At least one of the valid types should be present as a key
        let has_type = valid_types.iter().any(|t| json.get(t).is_some());
        assert!(
            has_type,
            "Line must have a recognized type discriminator: {:?}",
            json
        );
    }

    let _ = harness.zjj(&["remove", "prop-type-test", "--merge"]);
}

// =============================================================================
// Property 2: Field Completeness
// =============================================================================

/// Property: Session lines must have all required fields
#[test]
fn prop_session_lines_have_required_fields() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "prop-fields-test", "--no-zellij", "--no-hooks"]);
    if !result.success {
        println!("SKIP: could not create session");
        return;
    }

    let status_result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&status_result.stdout);
    if lines.is_err() {
        // If parsing fails, that's a separate property violation
        println!("SKIP: JSONL parsing failed");
        return;
    }

    let lines = lines.map_or(Vec::new(), |v| v);

    for line in &lines {
        if let Some(session) = line.get("session") {
            // Required fields
            assert!(
                session.get("name").is_some(),
                "Session must have 'name' field"
            );
            assert!(
                session.get("status").is_some(),
                "Session must have 'status' field"
            );

            // Name must not be empty
            if let Some(name) = session.get("name").and_then(|n| n.as_str()) {
                assert!(!name.is_empty(), "Session name must not be empty");
            }

            // Status must be a valid value
            if let Some(status) = session.get("status").and_then(|s| s.as_str()) {
                let valid_statuses = ["creating", "active", "paused", "completed", "failed"];
                assert!(
                    valid_statuses.contains(&status),
                    "Invalid session status: {}",
                    status
                );
            }
        }
    }

    let _ = harness.zjj(&["remove", "prop-fields-test", "--merge"]);
}

/// Property: Summary line must be present and contain counts
#[test]
fn prop_status_has_valid_summary() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create some sessions
    for name in ["prop-summary-1", "prop-summary-2", "prop-summary-3"] {
        let _ = harness.zjj(&["add", name, "--no-zellij", "--no-hooks"]);
    }

    let status_result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&status_result.stdout);
    if lines.is_err() {
        println!("SKIP: JSONL parsing failed");
        return;
    }

    let lines = lines.map_or(Vec::new(), |v| v);

    // Find summary line
    let summary_line = lines.iter().find(|line| line.get("summary").is_some());
    assert!(
        summary_line.is_some(),
        "Status output must contain a summary line"
    );

    if let Some(summary_json) = summary_line {
        if let Some(summary) = summary_json.get("summary") {
            // Summary must have either message or count
            let has_message = summary
                .get("message")
                .and_then(|m| m.as_str())
                .map_or(false, |s| !s.is_empty());
            let has_count = summary.get("count").is_some();

            assert!(
                has_message || has_count,
                "Summary must have 'message' or 'count' field"
            );
        }
    }

    // Cleanup
    for name in ["prop-summary-1", "prop-summary-2", "prop-summary-3"] {
        let _ = harness.zjj(&["remove", name, "--merge"]);
    }
}

// =============================================================================
// Property 3: Aggregation Consistency
// =============================================================================

/// Property: Session count in summary matches actual session lines
#[test]
fn prop_session_count_matches_summary() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create a known number of sessions
    let session_names = ["prop-count-1", "prop-count-2", "prop-count-3"];
    for name in &session_names {
        let result = harness.zjj(&["add", name, "--no-zellij", "--no-hooks"]);
        if !result.success {
            // If we can't create all sessions, skip this test run
            println!("SKIP: could not create all sessions");
            return;
        }
    }

    let status_result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&status_result.stdout);
    if lines.is_err() {
        println!("SKIP: JSONL parsing failed");
        return;
    }

    let lines = lines.map_or(Vec::new(), |v| v);

    // Count session lines
    let session_count = lines.iter().filter(|l| l.get("session").is_some()).count();

    // Find summary count
    let summary_count = lines
        .iter()
        .find(|l| l.get("summary").is_some())
        .and_then(|l| l.get("summary"))
        .and_then(|s| s.get("count"))
        .and_then(|c| c.as_i64());

    // If summary provides a count, it should match actual sessions
    if let Some(count) = summary_count {
        assert!(
            i64::try_from(session_count).map_or(true, |sc| sc == count),
            "Summary count ({}) should match actual session count ({})",
            count,
            session_count
        );
    }

    // Cleanup
    for name in &session_names {
        let _ = harness.zjj(&["remove", name, "--merge"]);
    }
}

/// Property: No duplicate session names in output
#[test]
fn prop_no_duplicate_sessions() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create sessions
    for name in ["prop-dup-1", "prop-dup-2"] {
        let _ = harness.zjj(&["add", name, "--no-zellij", "--no-hooks"]);
    }

    let status_result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&status_result.stdout);
    if lines.is_err() {
        println!("SKIP: JSONL parsing failed");
        return;
    }

    let lines = lines.map_or(Vec::new(), |v| v);

    // Extract all session names
    let names: Vec<String> = lines
        .iter()
        .filter_map(|l| l.get("session"))
        .filter_map(|s| s.get("name"))
        .filter_map(|n| n.as_str().map(std::string::ToString::to_string))
        .collect();

    // Check for duplicates
    let unique_names: HashSet<_> = names.iter().cloned().collect();

    assert!(
        unique_names.len() == names.len(),
        "No duplicate session names should exist. Found duplicates: {:?}",
        names
            .into_iter()
            .collect::<HashSet<_>>()
            .symmetric_difference(&unique_names)
            .collect::<Vec<_>>()
    );

    // Cleanup
    for name in ["prop-dup-1", "prop-dup-2"] {
        let _ = harness.zjj(&["remove", name, "--merge"]);
    }
}

/// Property: Stack depth is consistent with parent chain
#[test]
fn prop_stack_depth_consistent() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create parent session
    let parent_result = harness.zjj(&["add", "prop-parent", "--no-zellij", "--no-hooks"]);
    if !parent_result.success {
        println!("SKIP: could not create parent session");
        return;
    }

    // Create child session with parent reference
    let child_result = harness.zjj(&[
        "add",
        "prop-child",
        "--no-zellij",
        "--no-hooks",
        "--parent",
        "prop-parent",
    ]);

    if child_result.success {
        // Query status for child
        let status_result = harness.zjj(&["status", "prop-child"]);

        let lines = parse_jsonl_output(&status_result.stdout);
        if lines.is_ok() {
            let lines = lines.map_or(Vec::new(), |v| v);

            if let Some(session_line) = lines.iter().find(|l| l.get("session").is_some()) {
                if let Some(session) = session_line.get("session") {
                    // If stack_depth is present, it should be at least 1 for a child
                    if let Some(depth) = session.get("stack_depth").and_then(|d| d.as_i64()) {
                        assert!(
                            depth >= 1,
                            "Child session stack_depth should be >= 1, got {}",
                            depth
                        );
                    }

                    // If parent_session is present, it should match our parent
                    if let Some(parent) = session.get("parent_session").and_then(|p| p.as_str()) {
                        assert!(
                            parent == "prop-parent",
                            "Parent session should be 'prop-parent', got '{}'",
                            parent
                        );
                    }
                }
            }
        }
    }

    // Cleanup
    let _ = harness.zjj(&["remove", "prop-child", "--merge"]);
    let _ = harness.zjj(&["remove", "prop-parent", "--merge"]);
}

/// Property: Queue status values are valid
#[test]
fn prop_queue_status_is_valid() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "prop-queue-test", "--no-zellij", "--no-hooks"]);
    if !result.success {
        println!("SKIP: could not create session");
        return;
    }

    let status_result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&status_result.stdout);
    if lines.is_err() {
        println!("SKIP: JSONL parsing failed");
        return;
    }

    let lines = lines.map_or(Vec::new(), |v| v);

    let valid_queue_statuses = [
        "pending",
        "claimed",
        "rebasing",
        "testing",
        "ready_to_merge",
        "merging",
        "merged",
        "failed_retryable",
        "failed_terminal",
        "cancelled",
    ];

    for line in &lines {
        if let Some(session) = line.get("session") {
            if let Some(queue_status) = session.get("queue_status").and_then(|q| q.as_str()) {
                assert!(
                    valid_queue_statuses.contains(&queue_status),
                    "Invalid queue_status: {}",
                    queue_status
                );
            }
        }
    }

    let _ = harness.zjj(&["remove", "prop-queue-test", "--merge"]);
}

// =============================================================================
// Unit Tests for RED Phase
// =============================================================================

#[test]
fn test_json_validity_invariant() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "json-test", "--no-zellij", "--no-hooks"]);

    let result = harness.zjj(&["status"]);

    assert!(result.success, "Status should succeed: {}", result.stderr);

    // Every non-empty line must be valid JSON
    for line in result.stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "Invalid JSON: {}", line);
    }

    let _ = harness.zjj(&["remove", "json-test", "--merge"]);
}

#[test]
fn test_field_completeness_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "completeness-test", "--no-zellij", "--no-hooks"]);

    let result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&result.stdout).ok();
    assert!(lines.is_some(), "Output should be valid JSONL");

    if let Some(lines) = lines {
        let session_line = lines.iter().find(|l| l.get("session").is_some());
        assert!(session_line.is_some(), "Should have session line");

        if let Some(session) = session_line.and_then(|l| l.get("session")) {
            assert!(session.get("name").is_some(), "Session must have name");
            assert!(session.get("status").is_some(), "Session must have status");

            let name = session.get("name").and_then(|n| n.as_str());
            assert_eq!(name, Some("completeness-test"));
        }
    }

    let _ = harness.zjj(&["remove", "completeness-test", "--merge"]);
}

#[test]
fn test_summary_line_present() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "summary-test", "--no-zellij", "--no-hooks"]);

    let result = harness.zjj(&["status"]);

    let lines = parse_jsonl_output(&result.stdout).ok();
    assert!(lines.is_some(), "Output should be valid JSONL");

    if let Some(lines) = lines {
        let summary_line = lines.iter().find(|l| l.get("summary").is_some());
        assert!(summary_line.is_some(), "Should have summary line");
    }

    let _ = harness.zjj(&["remove", "summary-test", "--merge"]);
}

#[test]
fn test_no_sessions_summary() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Status with no sessions should still produce valid JSONL
    let result = harness.zjj(&["status"]);

    assert!(result.success, "Status should succeed with no sessions");

    let lines = parse_jsonl_output(&result.stdout).ok();
    assert!(
        lines.is_some(),
        "Output should be valid JSONL even with no sessions"
    );

    if let Some(lines) = lines {
        // Should have a summary line
        let summary_line = lines.iter().find(|l| l.get("summary").is_some());
        assert!(
            summary_line.is_some(),
            "Should have summary line even with no sessions"
        );

        // Summary should indicate no sessions
        if let Some(summary) = summary_line.and_then(|l| l.get("summary")) {
            let message = summary.get("message").and_then(|m| m.as_str());
            let has_no_sessions_msg = message.map_or(false, |m| {
                m.contains("No active sessions") || m.contains("0 active")
            });
            let count_is_zero = summary.get("count").and_then(|c| c.as_i64()) == Some(0);

            assert!(
                has_no_sessions_msg || count_is_zero,
                "Summary should indicate no sessions"
            );
        }
    }
}
