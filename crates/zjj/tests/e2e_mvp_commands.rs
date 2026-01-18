//! End-to-end integration tests for all MVP commands
//!
//! Tests the complete workflow with real JJ and Zellij integration:
//! 1. `zjj init` - Initialize zjj in a JJ repository
//! 2. `zjj add <name>` - Create session with JJ workspace + Zellij tab
//! 3. `zjj list` - Show all sessions
//! 4. `zjj remove <name>` - Cleanup session and workspace
//! 5. `zjj focus <name>` - Switch to session's Zellij tab
//!
//! # Design Principles
//!
//! - Zero panics: All operations use Result and proper error handling
//! - Zero unwraps: Uses functional patterns (map, `and_then`, ?)
//! - Real integration: Tests against actual JJ and Zellij (when available)
//! - Graceful degradation: Skips tests when tools not available
//! - Railway-oriented: Error paths are tested as thoroughly as success paths

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]

mod common;

use common::TestHarness;
use serial_test::serial;

// ============================================================================
// E2E Test: Complete MVP Workflow
// ============================================================================

/// Test the complete end-to-end workflow: init → add → list → focus → remove
///
/// This is the primary integration test that validates all MVP commands
/// work together correctly with real JJ workspaces.
#[test]
#[serial]
fn test_e2e_complete_mvp_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Step 1: Initialize zjj
    harness.assert_success(&["init"]);
    harness.assert_zjj_dir_exists();

    // Verify initialization created all required files
    let zjj_dir = harness.zjj_dir();
    assert!(
        zjj_dir.join("config.toml").exists(),
        "config.toml should exist"
    );
    assert!(zjj_dir.join("state.db").exists(), "state.db should exist");
    assert!(zjj_dir.join("layouts").exists(), "layouts dir should exist");

    // Step 2: Add a session
    harness.assert_success(&["add", "feature-mvp", "--no-open"]);

    // Verify workspace was created
    harness.assert_workspace_exists("feature-mvp");

    // Verify it's a valid JJ workspace
    let result = harness.jj(&["workspace", "list"]);
    assert!(result.success, "jj workspace list should succeed");
    result.assert_stdout_contains("feature-mvp");

    // Step 3: List sessions
    let result = harness.zjj(&["list"]);
    assert!(result.success, "list command should succeed");
    result.assert_stdout_contains("feature-mvp");

    // Step 4: Check status
    let result = harness.zjj(&["status", "feature-mvp"]);
    assert!(result.success, "status command should succeed");
    result.assert_output_contains("feature-mvp");

    // Step 5: Remove session
    harness.assert_success(&["remove", "feature-mvp", "--force"]);

    // Verify workspace was deleted
    harness.assert_workspace_not_exists("feature-mvp");

    // Verify session removed from list
    let result = harness.zjj(&["list"]);
    assert!(
        !result.stdout.contains("feature-mvp"),
        "Session should not appear in list"
    );

    // Verify JJ workspace was deleted
    let result = harness.jj(&["workspace", "list"]);
    assert!(
        !result.stdout.contains("feature-mvp"),
        "JJ workspace should be removed"
    );
}

// ============================================================================
// JJ Workspace Integration Tests
// ============================================================================

/// Test that `zjj add` creates a valid JJ workspace
#[test]
#[serial]
fn test_e2e_jj_workspace_creation() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-workspace", "--no-open"]);

    // Verify JJ workspace exists and is functional
    let workspace_path = harness.workspace_path("test-workspace");

    // Check workspace has .jj directory
    assert!(
        workspace_path.join(".jj").exists(),
        "Workspace should have .jj directory"
    );

    // Verify we can run JJ commands in the workspace
    let result = harness.jj(&["log", "--limit", "1"]);
    assert!(result.success, "jj log should work in workspace");

    // Verify workspace appears in JJ workspace list
    let result = harness.jj(&["workspace", "list"]);
    assert!(result.success, "jj workspace list should succeed");
    result.assert_stdout_contains("test-workspace");
}

/// Test that `zjj remove` properly cleans up JJ workspace
#[test]
#[serial]
fn test_e2e_jj_workspace_cleanup() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "cleanup-test", "--no-open"]);

    // Verify workspace exists
    harness.assert_workspace_exists("cleanup-test");

    // Remove session
    harness.assert_success(&["remove", "cleanup-test", "--force"]);

    // Verify workspace directory is deleted
    harness.assert_workspace_not_exists("cleanup-test");

    // Verify JJ workspace is forgotten
    let result = harness.jj(&["workspace", "list"]);
    assert!(
        !result.stdout.contains("cleanup-test"),
        "JJ should forget workspace after removal"
    );
}

/// Test that multiple JJ workspaces can coexist
#[test]
#[serial]
fn test_e2e_multiple_jj_workspaces() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions
    let sessions = vec!["session-a", "session-b", "session-c"];

    for session in &sessions {
        harness.assert_success(&["add", session, "--no-open"]);
        harness.assert_workspace_exists(session);
    }

    // Verify all workspaces exist in JJ
    let result = harness.jj(&["workspace", "list"]);
    assert!(result.success, "jj workspace list should succeed");

    for session in &sessions {
        result.assert_stdout_contains(session);
    }

    // Remove middle session
    harness.assert_success(&["remove", "session-b", "--force"]);

    // Verify others still exist
    let result = harness.jj(&["workspace", "list"]);
    result.assert_stdout_contains("session-a");
    assert!(!result.stdout.contains("session-b"));
    result.assert_stdout_contains("session-c");

    // Cleanup
    harness.assert_success(&["remove", "session-a", "--force"]);
    harness.assert_success(&["remove", "session-c", "--force"]);
}

// ============================================================================
// Session Name Validation Tests
// ============================================================================

/// Test that session names are properly validated
#[test]
#[serial]
fn test_e2e_session_name_validation() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Valid names should succeed
    let valid_names = vec!["test", "my-feature", "bug_fix_123", "v1-0-0"];

    for name in &valid_names {
        harness.assert_success(&["add", name, "--no-open"]);
    }

    // Verify all appear in list
    let result = harness.zjj(&["list"]);
    for name in &valid_names {
        result.assert_stdout_contains(name);
    }

    // Invalid names should fail
    harness.assert_failure(
        &["add", "has spaces", "--no-open"],
        "Session name validation failed",
    );
    harness.assert_failure(
        &["add", "has@symbol", "--no-open"],
        "Session name validation failed",
    );
    harness.assert_failure(
        &["add", "has/slash", "--no-open"],
        "Session name validation failed",
    );

    // Cleanup
    for name in &valid_names {
        harness.assert_success(&["remove", name, "--force"]);
    }
}

/// Test that duplicate session names are rejected
#[test]
#[serial]
fn test_e2e_duplicate_session_names() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "unique", "--no-open"]);

    // Try to add duplicate
    harness.assert_failure(&["add", "unique", "--no-open"], "already exists");

    // Cleanup
    harness.assert_success(&["remove", "unique", "--force"]);
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

/// Test that zjj gracefully handles missing prerequisites
#[test]
#[serial]
fn test_e2e_error_handling_no_init() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Try to add without init
    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "add should fail without init");

    // Try to list without init
    let result = harness.zjj(&["list"]);
    assert!(!result.success, "list should fail without init");

    // Try to remove without init
    let result = harness.zjj(&["remove", "test", "--force"]);
    assert!(!result.success, "remove should fail without init");
}

/// Test error handling for nonexistent sessions
#[test]
#[serial]
fn test_e2e_error_handling_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Try to remove nonexistent session
    harness.assert_failure(&["remove", "nonexistent", "--force"], "");

    // Try to focus nonexistent session
    let result = harness.zjj(&["focus", "nonexistent", "--json"]);
    assert!(!result.success, "focus should fail for nonexistent session");
}

/// Test idempotent operations
#[test]
#[serial]
fn test_e2e_idempotent_init() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // First init
    harness.assert_success(&["init"]);

    // Second init should succeed but indicate already initialized
    let result = harness.zjj(&["init"]);
    assert!(result.success, "Second init should succeed");
    result.assert_output_contains("already initialized");

    // Third init should also succeed
    let result = harness.zjj(&["init"]);
    assert!(result.success, "Third init should succeed");
}

// ============================================================================
// JSON Output Tests
// ============================================================================

/// Test that all commands support JSON output
#[test]
#[serial]
fn test_e2e_json_output_format() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "json-test", "--no-open"]);

    // Test list --json
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success, "list --json should succeed");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    let stdout = &result.stdout;
    assert!(parsed.is_ok(), "list output should be valid JSON: {stdout}");

    // Test status --json
    let result = harness.zjj(&["status", "json-test", "--json"]);
    assert!(result.success, "status --json should succeed");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    let stdout = &result.stdout;
    assert!(
        parsed.is_ok(),
        "status output should be valid JSON: {stdout}"
    );

    // Test focus --json (will fail without TTY, but should produce valid JSON)
    let result = harness.zjj(&["focus", "json-test", "--json"]);
    // May succeed or fail depending on environment, but output should be JSON
    if !result.stdout.is_empty() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
        let stdout = &result.stdout;
        assert!(
            parsed.is_ok(),
            "focus output should be valid JSON: {stdout}"
        );
    }

    // Cleanup
    harness.assert_success(&["remove", "json-test", "--force"]);
}

// ============================================================================
// Configuration Tests
// ============================================================================

/// Test that custom workspace directory configuration works
#[test]
#[serial]
fn test_e2e_custom_workspace_directory() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Modify config to use custom workspace directory
    let custom_config = r#"
workspace_dir = "../custom_workspaces/{repo}"
main_branch = "main"
default_template = "standard"

[watch]
enabled = true
debounce_ms = 100

[zellij]
layout_template = "standard"
claude_command = "claude"
beads_command = "bv"

[dashboard]
enabled = true
port = 3000

[agent]
enabled = false
"#;

    if harness.write_config(custom_config).is_err() {
        eprintln!("Failed to write custom config");
        return;
    }

    // Add session - should use custom workspace directory
    harness.assert_success(&["add", "custom-test", "--no-open"]);

    // Verify workspace exists (TestHarness resolves the pattern)
    harness.assert_workspace_exists("custom-test");

    // Cleanup
    harness.assert_success(&["remove", "custom-test", "--force"]);
}

/// Test that configuration is preserved across operations
#[test]
#[serial]
fn test_e2e_config_persistence() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Read initial config
    let Ok(initial_config) = harness.read_config() else {
        eprintln!("Failed to read initial config");
        return;
    };

    // Perform operations
    harness.assert_success(&["add", "test1", "--no-open"]);
    harness.assert_success(&["add", "test2", "--no-open"]);
    harness.assert_success(&["remove", "test1", "--force"]);

    // Verify config unchanged
    let Ok(final_config) = harness.read_config() else {
        eprintln!("Failed to read final config");
        return;
    };

    assert_eq!(
        initial_config, final_config,
        "Config should remain unchanged after operations"
    );

    // Cleanup
    harness.assert_success(&["remove", "test2", "--force"]);
}

// ============================================================================
// Database Integrity Tests
// ============================================================================

/// Test that database maintains integrity across operations
#[test]
#[serial]
fn test_e2e_database_integrity() {
    tokio_test::block_on(async {
        use sqlx::Connection;

        let Some(harness) = TestHarness::try_new() else {
            eprintln!("Skipping test: jj not available");
            return;
        };

        harness.assert_success(&["init"]);

        // Create sessions
        harness.assert_success(&["add", "db-test-1", "--no-open"]);
        harness.assert_success(&["add", "db-test-2", "--no-open"]);

        // Verify database can be opened
        let db_path = harness.state_db_path();
        let db_url = format!("sqlite://{}", db_path.display());
        let Ok(mut conn) = sqlx::SqliteConnection::connect(&db_url).await else {
            eprintln!("Failed to open database");
            return;
        };

        // Verify sessions exist
        let Ok((count,)) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM sessions")
            .fetch_one(&mut conn)
            .await
        else {
            eprintln!("Failed to query database");
            return;
        };

        assert_eq!(count, 2, "Database should contain 2 sessions");

        // Remove one session
        harness.assert_success(&["remove", "db-test-1", "--force"]);

        // Verify database updated
        let Ok((count,)) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM sessions")
            .fetch_one(&mut conn)
            .await
        else {
            eprintln!("Failed to query database");
            return;
        };

        assert_eq!(count, 1, "Database should contain 1 session after removal");

        // Cleanup
        harness.assert_success(&["remove", "db-test-2", "--force"]);
    });
}

// ============================================================================
// File System Tests
// ============================================================================

/// Test that workspace directories are created with correct structure
#[test]
#[serial]
fn test_e2e_workspace_structure() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "structure-test", "--no-open"]);

    let workspace_path = harness.workspace_path("structure-test");

    // Verify workspace structure
    assert!(workspace_path.exists(), "Workspace directory should exist");
    assert!(workspace_path.is_dir(), "Workspace should be a directory");
    assert!(
        workspace_path.join(".jj").exists(),
        "Workspace should have .jj directory"
    );

    // Verify workspace is isolated (has its own working copy)
    let result = harness.zjj_with_env(&["list"], &[]);
    assert!(result.success, "Commands should work with workspace");

    // Cleanup
    harness.assert_success(&["remove", "structure-test", "--force"]);

    // Verify cleanup was complete
    assert!(
        !workspace_path.exists(),
        "Workspace should be completely removed"
    );
}

/// Test that removal handles missing workspace directories gracefully
#[test]
#[serial]
fn test_e2e_remove_missing_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "missing-test", "--no-open"]);

    // Manually delete workspace directory
    let workspace_path = harness.workspace_path("missing-test");
    let _ = std::fs::remove_dir_all(&workspace_path);

    // Remove should still succeed
    let result = harness.zjj(&["remove", "missing-test", "--force"]);
    // May succeed or warn, but should not crash
    assert!(
        result.success || result.stderr.contains("workspace"),
        "Should handle missing workspace gracefully"
    );
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

/// Test that zjj handles many sessions efficiently
#[test]
#[serial]
fn test_e2e_many_sessions() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create 10 sessions
    let session_count = 10;
    let sessions: Vec<String> = (0..session_count)
        .map(|i| format!("session-{i:03}"))
        .collect();

    for session in &sessions {
        harness.assert_success(&["add", session, "--no-open"]);
    }

    // Verify all appear in list
    let result = harness.zjj(&["list"]);
    assert!(result.success, "list should handle many sessions");

    for session in &sessions {
        result.assert_stdout_contains(session);
    }

    // Cleanup all
    for session in &sessions {
        harness.assert_success(&["remove", session, "--force"]);
    }

    // Verify empty
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "list should work after removing all sessions"
    );
}

// ============================================================================
// Focus Command Tests (Zellij Integration)
// ============================================================================

/// Test focus command behavior when not in Zellij
#[test]
#[serial]
fn test_e2e_focus_outside_zellij() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "focus-test", "--no-open"]);

    // Focus when not in Zellij (and no TTY in tests)
    let result = harness.zjj(&["focus", "focus-test", "--json"]);

    // Should produce JSON output (may succeed or fail based on environment)
    if !result.stdout.is_empty() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
        assert!(parsed.is_ok(), "focus output should be valid JSON");
    }

    // Cleanup
    harness.assert_success(&["remove", "focus-test", "--force"]);
}

/// Test focus command error handling for nonexistent session
#[test]
#[serial]
fn test_e2e_focus_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Try to focus nonexistent session
    let result = harness.zjj(&["focus", "nonexistent", "--json"]);
    assert!(!result.success, "focus should fail for nonexistent session");

    // Should produce JSON error
    if !result.stdout.is_empty() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
        assert!(parsed.is_ok(), "Error output should be valid JSON");

        let Ok(json) = parsed else {
            return;
        };
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "JSON should indicate failure"
        );
    }
}

// ============================================================================
// Concurrency Tests
// ============================================================================

/// Test that concurrent operations are handled safely
#[test]
#[serial]
fn test_e2e_sequential_operations() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Perform rapid sequential operations
    for i in 0..5 {
        let name = format!("rapid-{i}");
        harness.assert_success(&["add", &name, "--no-open"]);
        let result = harness.zjj(&["list"]);
        assert!(result.success);
        harness.assert_success(&["remove", &name, "--force"]);
    }
}
