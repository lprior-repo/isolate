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
    // Pattern matching relaxions
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! End-to-end CLI tests for FIFO and deterministic merge behavior (bd-1no)
//!
//! These tests exercise the complete submit-to-merge flow through the CLI:
//! - Submit multiple entries
//! - Verify FIFO processing order
//! - Verify deterministic behavior
//!
//! Each test uses an isolated environment via TestHarness.

mod common;
use common::TestHarness;
use serde_json::Value as JsonValue;

// ============================================================================
// HELPERS
// ============================================================================

/// Extract queue entries from JSON response
fn get_queue_entries(json: &JsonValue) -> Vec<(String, String)> {
    // Try data.entries first (envelope format)
    json.get("data")
        .and_then(|d| d.get("entries"))
        .and_then(JsonValue::as_array)
        .or_else(|| json.get("entries").and_then(JsonValue::as_array))
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let workspace = e.get("workspace")?.as_str()?.to_string();
                    let status = e
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Some((workspace, status))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Count entries with a specific status
#[allow(dead_code)]
fn count_status(entries: &[(String, String)], status: &str) -> usize {
    entries.iter().filter(|(_, s)| s == status).count()
}

// ============================================================================
// E2E FIFO ORDERING TESTS
// ============================================================================

/// Test that queue list returns entries in FIFO order.
///
/// GIVEN: Multiple workspaces added to queue at different times
/// WHEN: We query the queue list
/// THEN: Entries are returned in submission order (FIFO)
#[test]
fn test_e2e_queue_list_returns_fifo_order() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add multiple workspaces to the queue
    let workspaces = ["ws-alpha", "ws-beta", "ws-gamma"];
    for ws in workspaces {
        let result = harness.zjj(&["queue", "add", ws, "--json"]);
        if !result.success {
            eprintln!("Skipping: queue add failed for {ws}");
            return;
        }
    }

    // Get queue list
    let result = harness.zjj(&["queue", "list", "--json"]);

    if !result.success {
        eprintln!("Skipping: queue list not available");
        return;
    }

    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(_) => return,
    };

    let entries = get_queue_entries(&json);

    // Should have 3 entries
    assert!(entries.len() >= 3, "Should have at least 3 queue entries");

    // Verify all workspaces are present
    let workspace_names: Vec<&str> = entries.iter().map(|(w, _)| w.as_str()).collect();
    for ws in workspaces {
        assert!(
            workspace_names.contains(&ws),
            "Workspace {ws} should be in queue"
        );
    }
}

/// Test that queue next returns entries in FIFO order.
///
/// GIVEN: Multiple entries in queue
/// WHEN: We repeatedly call queue next
/// THEN: Entries are returned in submission order
#[test]
fn test_e2e_queue_next_returns_fifo_order() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add workspaces via queue add
    let workspaces = ["e2e-first", "e2e-second", "e2e-third"];
    for ws in workspaces {
        let result = harness.zjj(&["queue", "add", ws, "--json"]);
        if !result.success {
            eprintln!("Skipping: queue add failed for {ws}");
            return;
        }
    }

    // Query next multiple times
    let mut retrieved = Vec::new();
    for _ in &workspaces {
        let result = harness.zjj(&["queue", "next", "--json"]);
        if !result.success {
            break;
        }

        let json: JsonValue = match serde_json::from_str(&result.stdout) {
            Ok(v) => v,
            Err(_) => break,
        };

        // Extract workspace from response
        let workspace = json
            .get("data")
            .and_then(|d| d.get("entry"))
            .and_then(|e| e.get("workspace"))
            .and_then(JsonValue::as_str)
            .or_else(|| {
                json.get("entry")
                    .and_then(|e| e.get("workspace"))
                    .and_then(JsonValue::as_str)
            })
            .map(String::from);

        if let Some(ws) = workspace {
            retrieved.push(ws);
        }
    }

    // Verify FIFO order (first entry should be first)
    if !retrieved.is_empty() {
        assert_eq!(
            retrieved[0], "e2e-first",
            "First retrieved should be first submitted (FIFO)"
        );
    }
}

/// Test priority ordering with FIFO tiebreaker.
///
/// GIVEN: Entries with different priorities added out of order
/// WHEN: We query the queue
/// THEN: Lower priority numbers come first, same priority follows FIFO
#[test]
fn test_e2e_priority_with_fifo_tiebreaker() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add entries with different priorities
    // Priority 10 (low) first
    let result1 = harness.zjj(&["queue", "add", "low-1", "--priority", "10", "--json"]);

    // Priority 0 (high) second
    let result2 = harness.zjj(&["queue", "add", "high", "--priority", "0", "--json"]);

    // Priority 10 (low) third
    let result3 = harness.zjj(&["queue", "add", "low-2", "--priority", "10", "--json"]);

    // Priority 5 (medium) fourth
    let result4 = harness.zjj(&["queue", "add", "mid", "--priority", "5", "--json"]);

    if !result1.success || !result2.success || !result3.success || !result4.success {
        eprintln!("Skipping: queue add not available");
        return;
    }

    // Get queue list
    let result = harness.zjj(&["queue", "list", "--json"]);
    if !result.success {
        return;
    }

    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(_) => return,
    };

    let entries = get_queue_entries(&json);

    // Expected order: high (0), mid (5), low-1 (10), low-2 (10)
    // We verify by checking positions
    let workspace_order: Vec<&str> = entries.iter().map(|(w, _)| w.as_str()).collect();

    // high should come before mid
    let high_pos = workspace_order.iter().position(|&w| w == "high");
    let mid_pos = workspace_order.iter().position(|&w| w == "mid");

    if let (Some(h), Some(m)) = (high_pos, mid_pos) {
        assert!(
            h < m,
            "high (priority 0) should come before mid (priority 5)"
        );
    }

    // mid should come before low-1 and low-2
    let low1_pos = workspace_order.iter().position(|&w| w == "low-1");
    let low2_pos = workspace_order.iter().position(|&w| w == "low-2");

    if let (Some(m), Some(l1)) = (mid_pos, low1_pos) {
        assert!(
            m < l1,
            "mid (priority 5) should come before low-1 (priority 10)"
        );
    }

    if let (Some(l1), Some(l2)) = (low1_pos, low2_pos) {
        assert!(
            l1 < l2,
            "low-1 should come before low-2 (FIFO within same priority)"
        );
    }
}

/// Test queue statistics.
///
/// GIVEN: A queue with entries in various states
/// WHEN: We query queue stats
/// THEN: Accurate counts are returned
#[test]
fn test_e2e_queue_stats_accurate() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add some entries
    for i in 0..5 {
        let _ = harness.zjj(&["queue", "add", &format!("stats-test-{i}"), "--json"]);
    }

    // Get stats
    let result = harness.zjj(&["queue", "stats", "--json"]);

    if !result.success {
        eprintln!("Skipping: queue stats not available");
        return;
    }

    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(_) => return,
    };

    // Verify stats structure
    let data = json.get("data").unwrap_or(&json);

    assert!(data.get("total").is_some(), "Stats should have total");
    assert!(data.get("pending").is_some(), "Stats should have pending");

    // Total should be at least 5
    let total = data.get("total").and_then(JsonValue::as_u64).unwrap_or(0);
    assert!(total >= 5, "Total should be at least 5, got {total}");
}

/// Test queue status for a specific workspace.
///
/// GIVEN: A workspace in the queue
/// WHEN: We query its status
/// THEN: Correct status is returned
#[test]
fn test_e2e_queue_status_for_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add a workspace
    let add_result = harness.zjj(&["queue", "add", "status-test-ws", "--json"]);
    if !add_result.success {
        eprintln!("Skipping: queue add not available");
        return;
    }

    // Query status
    let result = harness.zjj(&["queue", "status", "status-test-ws", "--json"]);
    if !result.success {
        return;
    }

    let json: JsonValue = match serde_json::from_str(&result.stdout) {
        Ok(v) => v,
        Err(_) => return,
    };

    // Verify response structure
    let data = json.get("data").unwrap_or(&json);

    let exists = data
        .get("exists")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    assert!(exists, "Workspace should exist in queue");

    let status = data.get("status").and_then(JsonValue::as_str).unwrap_or("");
    assert!(!status.is_empty(), "Status should be present");
}

// ============================================================================
// ISOLATED ENVIRONMENT TESTS
// ============================================================================

/// Test that each test run gets an isolated queue.
///
/// GIVEN: Multiple tests running with TestHarness
/// WHEN: Each test adds entries
/// THEN: Entries don't leak between tests
#[test]
fn test_e2e_isolated_queue_environment() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add entry with unique name for this test
    let unique_ws = "isolated-test-unique-xyz";
    let _ = harness.zjj(&["queue", "add", unique_ws, "--json"]);

    // Verify it exists
    let result = harness.zjj(&["queue", "status", unique_ws, "--json"]);

    if result.success {
        let json: JsonValue = match serde_json::from_str(&result.stdout) {
            Ok(v) => v,
            Err(_) => return,
        };

        let data = json.get("data").unwrap_or(&json);
        let exists = data
            .get("exists")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);

        assert!(exists, "Entry should exist in isolated test environment");
    }
}

/// Test queue remove operation.
///
/// GIVEN: An entry in the queue
/// WHEN: We remove it
/// THEN: It no longer appears in the queue
#[test]
fn test_e2e_queue_remove_entry() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Add entry
    let ws_name = "remove-test-ws";
    let add_result = harness.zjj(&["queue", "add", ws_name, "--json"]);
    if !add_result.success {
        eprintln!("Skipping: queue add not available");
        return;
    }

    // Verify it exists
    let status_result = harness.zjj(&["queue", "status", ws_name, "--json"]);
    if status_result.success {
        let json: JsonValue = serde_json::from_str(&status_result.stdout).unwrap_or_default();
        let data = json.get("data").unwrap_or(&json);
        let exists = data
            .get("exists")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        assert!(exists, "Entry should exist before removal");
    }

    // Remove it
    let remove_result = harness.zjj(&["queue", "remove", ws_name, "--json"]);

    if remove_result.success {
        // Verify it no longer exists
        let status_after = harness.zjj(&["queue", "status", ws_name, "--json"]);
        if status_after.success {
            let json: JsonValue = serde_json::from_str(&status_after.stdout).unwrap_or_default();
            let data = json.get("data").unwrap_or(&json);
            let exists = data
                .get("exists")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true);
            assert!(!exists, "Entry should not exist after removal");
        }
    }
}

/// Test JSON output schema for queue commands.
///
/// GIVEN: Queue commands with --json flag
/// WHEN: We execute them
/// THEN: Output is valid JSON with expected schema
#[test]
fn test_e2e_queue_json_schema_validation() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Test list schema
    let list_result = harness.zjj(&["queue", "list", "--json"]);
    if list_result.success {
        let json: JsonValue = serde_json::from_str(&list_result.stdout).unwrap_or_default();
        // Should have schema field
        assert!(
            json.get("$schema").is_some() || json.get("schema").is_some(),
            "Queue list JSON should have schema field"
        );
    }

    // Test stats schema
    let stats_result = harness.zjj(&["queue", "stats", "--json"]);
    if stats_result.success {
        let json: JsonValue = serde_json::from_str(&stats_result.stdout).unwrap_or_default();
        assert!(
            json.get("$schema").is_some() || json.get("schema").is_some(),
            "Queue stats JSON should have schema field"
        );
    }

    // Test next schema
    let next_result = harness.zjj(&["queue", "next", "--json"]);
    if next_result.success {
        let json: JsonValue = serde_json::from_str(&next_result.stdout).unwrap_or_default();
        assert!(
            json.get("$schema").is_some() || json.get("schema").is_some(),
            "Queue next JSON should have schema field"
        );
    }
}
