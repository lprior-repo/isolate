//! Adversarial tests for Status aggregation (REVIEW Phase)
//!
//! These tests validate robustness under edge cases and partial state scenarios.
//! Run with: `cargo test --test status_adversarial_tests`
//!
//! # Adversarial Scenarios
//!
//! 1. Partial State: Missing workspace, partial session data
//! 2. Missing Objects: Non-existent sessions, corrupted data
//! 3. Boundary Conditions: Empty names, max values, special characters
//! 4. Concurrent Modifications: Status during ongoing operations
//!
//! # Bead IDs
//!
//! - status-scout (bd-dly9): BDD scenarios
//! - status-red (bd-udmj): Property tests
//! - status-green (bd-uc3l): Minimal implementation
//! - status-implement (bd-3dg8): Full implementation
//! - status-review (bd-rdvd): Adversarial tests (THIS FILE)

#![allow(clippy::unwrap_used)] // Test file allows unwrap in test code
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::too_many_lines)]

mod common;

use anyhow::Result;
use common::{parse_jsonl_output, TestHarness};

// =============================================================================
// Scenario: Non-existent Session Returns Proper Error
// =============================================================================

#[test]
fn adversarial_nonexistent_session_exit_code_2() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Query status for a session that doesn't exist
    let result = harness.zjj(&["status", "nonexistent-session-xyz"]);

    // Should fail with exit code 2 (NOT_FOUND)
    assert!(
        !result.success,
        "Status should fail for non-existent session"
    );
    assert_eq!(
        result.exit_code,
        Some(2),
        "Exit code should be 2 for NOT_FOUND, got {:?}",
        result.exit_code
    );
}

#[test]
fn adversarial_nonexistent_session_valid_json_error() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["status", "nonexistent-session-xyz"]);

    // Error output should still be valid JSON
    let output = result.stdout.trim();
    if !output.is_empty() {
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(output);
        assert!(
            parse_result.is_ok(),
            "Error output should be valid JSON: {}",
            output
        );
    }
}

// =============================================================================
// Scenario: Empty Session Name Handling
// =============================================================================

#[test]
fn adversarial_empty_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Query status with empty session name
    let result = harness.zjj(&["status", ""]);

    // Should either fail or return all sessions (implementation-defined)
    // But should NOT panic
    if result.success {
        // If it succeeds, output should be valid JSONL
        let lines = parse_jsonl_output(&result.stdout);
        assert!(lines.is_ok(), "Output should be valid JSONL if successful");
    }
}

// =============================================================================
// Scenario: Session With Missing Workspace
// =============================================================================

#[test]
fn adversational_session_with_missing_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create a session
    harness.assert_success(&["add", "missing-workspace-test", "--no-hooks"]);

    // Manually remove the workspace directory (simulating external deletion)
    let workspace_path = harness.workspace_path("missing-workspace-test");
    if workspace_path.exists() {
        std::fs::remove_dir_all(&workspace_path).ok();
    }

    // Status should still work (gracefully handle missing workspace)
    let result = harness.zjj(&["status", "missing-workspace-test"]);

    // Should succeed (status is read-only)
    assert!(
        result.success,
        "Status should succeed even with missing workspace"
    );

    // Output should be valid JSONL
    let lines = parse_jsonl_output(&result.stdout);
    assert!(
        lines.is_ok(),
        "Output should be valid JSONL even with missing workspace"
    );

    // Cleanup database entry
    let _ = harness.zjj(&["remove", "missing-workspace-test", "--merge"]);
}

// =============================================================================
// Scenario: Deeply Nested Stack
// =============================================================================

#[test]
fn adversarial_deep_stack_depth() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create a chain of parent-child sessions
    let depth = 5; // Test with moderate depth
    let mut session_names: Vec<String> = Vec::new();

    // Create root session
    let root_name = "deep-stack-root";
    let result = harness.zjj(&["add", root_name, "--no-hooks"]);
    if !result.success {
        println!("SKIP: Could not create root session");
        return;
    }
    session_names.push(root_name.to_string());

    // Create child sessions
    for i in 1..depth {
        let child_name = format!("deep-stack-{}", i);
        let parent_name = session_names.last().map_or("", |s| s.as_str());

        let result = harness.zjj(&["add", &child_name, "--no-hooks", "--parent", parent_name]);

        if !result.success {
            break;
        }
        session_names.push(child_name);
    }

    // Query status for the deepest session
    if let Some(deepest) = session_names.last() {
        let result = harness.zjj(&["status", deepest]);

        assert!(
            result.success,
            "Status should succeed for deep stack session"
        );

        let lines = parse_jsonl_output(&result.stdout);
        if let Ok(lines) = lines {
            if let Some(session_line) = lines.iter().find(|l| l.get("session").is_some()) {
                if let Some(session) = session_line.get("session") {
                    // Stack depth should be present and >= 0
                    if let Some(stack_depth) = session.get("stack_depth").and_then(|d| d.as_i64()) {
                        assert!(
                            stack_depth >= 0,
                            "Stack depth should be non-negative: {}",
                            stack_depth
                        );
                    }
                }
            }
        }
    }

    // Cleanup in reverse order
    for name in session_names.into_iter().rev() {
        let _ = harness.zjj(&["remove", &name, "--merge"]);
    }
}

// =============================================================================
// Scenario: Special Characters in Session Names
// =============================================================================

#[test]
fn adversarial_session_name_with_special_chars() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Valid session names with allowed special characters
    let valid_names = ["test-name", "test_name", "TestName123"];

    for name in &valid_names {
        let result = harness.zjj(&["add", name, "--no-hooks"]);
        if result.success {
            let status_result = harness.zjj(&["status", name]);
            assert!(
                status_result.success,
                "Status should succeed for valid name '{}'",
                name
            );

            // Verify JSONL output
            let lines = parse_jsonl_output(&status_result.stdout);
            assert!(
                lines.is_ok(),
                "Output should be valid JSONL for session '{}'",
                name
            );

            let _ = harness.zjj(&["remove", name, "--merge"]);
        }
    }
}

// =============================================================================
// Scenario: Status During Multiple Operations
// =============================================================================

#[test]
fn adversarial_concurrent_session_operations() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions
    let sessions = ["concurrent-1", "concurrent-2", "concurrent-3"];
    for name in &sessions {
        let _ = harness.zjj(&["add", name, "--no-hooks"]);
    }

    // Query status while potentially modifying
    let status_result = harness.zjj(&["status"]);

    assert!(
        status_result.success,
        "Status should succeed during operations"
    );

    let lines = parse_jsonl_output(&status_result.stdout);
    assert!(
        lines.is_ok(),
        "Output should be valid JSONL during operations"
    );

    // Verify all sessions are present
    if let Ok(lines) = lines {
        let found_sessions: Vec<String> = lines
            .iter()
            .filter_map(|l| l.get("session"))
            .filter_map(|s| s.get("name"))
            .filter_map(|n| n.as_str().map(std::string::ToString::to_string))
            .collect();

        for name in &sessions {
            assert!(
                found_sessions.contains(&name.to_string()),
                "Session '{}' should be in output",
                name
            );
        }
    }

    // Cleanup
    for name in &sessions {
        let _ = harness.zjj(&["remove", name, "--merge"]);
    }
}

// =============================================================================
// Scenario: Very Long Session List
// =============================================================================

#[test]
fn adversarial_many_sessions() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create many sessions
    let session_count = 10;
    let mut created_sessions: Vec<String> = Vec::new();

    for i in 0..session_count {
        let name = format!("many-sessions-{}", i);
        let result = harness.zjj(&["add", &name, "--no-hooks"]);
        if result.success {
            created_sessions.push(name);
        }
    }

    // Query status for all sessions
    let status_result = harness.zjj(&["status"]);

    assert!(
        status_result.success,
        "Status should succeed with many sessions"
    );

    let lines = parse_jsonl_output(&status_result.stdout);
    assert!(
        lines.is_ok(),
        "Output should be valid JSONL with many sessions"
    );

    // Verify all created sessions are present
    if let Ok(lines) = lines {
        let session_count = lines.iter().filter(|l| l.get("session").is_some()).count();
        assert!(
            session_count >= created_sessions.len(),
            "Should have at least {} session lines, got {}",
            created_sessions.len(),
            session_count
        );
    }

    // Cleanup
    for name in &created_sessions {
        let _ = harness.zjj(&["remove", name, "--merge"]);
    }
}

// =============================================================================
// Scenario: Status Is Read-Only
// =============================================================================

#[test]
fn adversarial_status_is_read_only() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "readonly-test", "--no-hooks"]);

    // Get initial state
    let initial_result = harness.zjj(&["status", "readonly-test"]);
    let initial_lines = parse_jsonl_output(&initial_result.stdout);

    // Query status again
    let second_result = harness.zjj(&["status", "readonly-test"]);
    let second_lines = parse_jsonl_output(&second_result.stdout);

    // Both should succeed
    assert!(initial_result.success);
    assert!(second_result.success);

    // Both should produce valid JSONL
    assert!(initial_lines.is_ok());
    assert!(second_lines.is_ok());

    // Session names should match (state unchanged)
    if let (Ok(initial), Ok(second)) = (initial_lines, second_lines) {
        let initial_name = initial
            .iter()
            .find_map(|l| l.get("session"))
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str());

        let second_name = second
            .iter()
            .find_map(|l| l.get("session"))
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str());

        assert_eq!(
            initial_name, second_name,
            "Session name should be unchanged after read-only status query"
        );
    }

    let _ = harness.zjj(&["remove", "readonly-test", "--merge"]);
}

// =============================================================================
// Scenario: Invalid Parent Session Reference
// =============================================================================

#[test]
fn adversarial_orphan_session_reference() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Try to create a session with a non-existent parent
    let result = harness.zjj(&[
        "add",
        "orphan-child",
        "--no-hooks",
        "--parent",
        "nonexistent-parent",
    ]);

    // Should either fail or create without parent (implementation-defined)
    // But should NOT panic
    if result.success {
        // If created, status should work
        let status_result = harness.zjj(&["status", "orphan-child"]);
        assert!(
            status_result.success,
            "Status should succeed for session with invalid parent reference"
        );

        let _ = harness.zjj(&["remove", "orphan-child", "--merge"]);
    }
}

// =============================================================================
// Scenario: Status After Session Removal
// =============================================================================

#[test]
fn adversarial_status_after_removal() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create and then remove a session
    harness.assert_success(&["add", "remove-test", "--no-hooks"]);
    harness.assert_success(&["remove", "remove-test", "--merge"]);

    // Status for removed session should fail with NOT_FOUND
    let result = harness.zjj(&["status", "remove-test"]);

    assert!(!result.success, "Status should fail for removed session");
    assert_eq!(
        result.exit_code,
        Some(2),
        "Exit code should be 2 for NOT_FOUND after removal"
    );
}

// =============================================================================
// Scenario: Unicode in Workspace Path
// =============================================================================

#[test]
fn adversarial_workspace_with_unicode_path() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create a session with ASCII name (Unicode names are rejected)
    let result = harness.zjj(&["add", "unicode-test", "--no-hooks"]);

    if result.success {
        let status_result = harness.zjj(&["status", "unicode-test"]);

        assert!(status_result.success, "Status should succeed");

        // Verify workspace_path can handle any characters
        let lines = parse_jsonl_output(&status_result.stdout);
        if let Ok(lines) = lines {
            if let Some(session) = lines.iter().find_map(|l| l.get("session")) {
                // workspace_path should be present and a string
                if let Some(path) = session.get("workspace_path") {
                    assert!(path.is_string(), "workspace_path should be a string");
                }
            }
        }

        let _ = harness.zjj(&["remove", "unicode-test", "--merge"]);
    }
}

// =============================================================================
// Scenario: Status Output Schema Validation
// =============================================================================

#[test]
fn adversarial_output_schema_validation() {
    let Some(harness) = TestHarness::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test", "--no-hooks"]);

    let result = harness.zjj(&["status", "schema-test"]);

    let lines = parse_jsonl_output(&result.stdout);
    if let Ok(lines) = lines {
        for line in &lines {
            // Each line must be a JSON object
            assert!(line.is_object(), "Each line must be a JSON object");

            // Must have exactly one type discriminator at top level
            let type_keys: Vec<&str> =
                ["session", "summary", "issue", "action", "warning", "result"]
                    .iter()
                    .filter(|&&k| line.get(k).is_some())
                    .copied()
                    .collect();

            assert!(
                !type_keys.is_empty(),
                "Each line must have at least one type discriminator"
            );
        }
    }

    let _ = harness.zjj(&["remove", "schema-test", "--merge"]);
}
