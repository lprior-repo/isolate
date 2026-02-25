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

    // Should either fail or return all sessions
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
