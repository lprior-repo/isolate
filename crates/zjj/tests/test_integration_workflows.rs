//! End-to-end integration tests for complete zjj workflows
//!
//! Tests full user workflows across multiple commands.
//! Focuses on real-world usage patterns and integration points.

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

mod common;

use common::{payload, TestHarness};

// ============================================================================
// Complete User Workflows
// ============================================================================

#[test]
fn test_new_user_onboarding_workflow() {
    // Workflow: User starts fresh, inits zjj, adds first session
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // User initializes zjj in their repo
    harness.assert_success(&["init", "--json"]);

    // Verify .zjj directory created
    assert!(harness.zjj_dir().exists());

    // User adds their first session
    harness.assert_success(&["add", "my-first-feature", "--no-open"]);

    // Verify session appears in list
    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("my-first-feature");

    // User checks status
    let result = harness.zjj(&["status", "my-first-feature", "--json"]);
    assert!(result.success);
    assert!(result.stdout.contains("my-first-feature"));

    // User is done with the feature
    harness.assert_success(&["remove", "my-first-feature", "--force"]);

    // Verify cleanup
    harness.assert_workspace_not_exists("my-first-feature");
}

#[test]
fn test_parallel_development_workflow() {
    // Workflow: Developer working on multiple features simultaneously
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions for different features
    let features = vec!["feature-auth", "feature-ui", "feature-api"];
    for feature in &features {
        harness.assert_success(&["add", feature, "--no-open"]);
    }

    // Verify all sessions are tracked
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);

    for feature in &features {
        result.assert_stdout_contains(feature);
    }

    // Simulate working on one feature (check status)
    let result = harness.zjj(&["status", "feature-auth"]);
    assert!(result.success);
    result.assert_output_contains("feature-auth");

    // Clean up one feature when done
    harness.assert_success(&["remove", "feature-ui", "--force"]);

    // Verify it's removed but others remain
    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("feature-auth");
    assert!(!result.stdout.contains("feature-ui"));
    result.assert_stdout_contains("feature-api");
}

#[test]
fn test_session_switching_workflow() {
    // Workflow: User switches between different contexts
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create sessions for different contexts
    harness.assert_success(&["add", "bugfix-123", "--no-open"]);
    harness.assert_success(&["add", "feature-456", "--no-open"]);
    harness.assert_success(&["add", "experiment-789", "--no-open"]);

    // Verify each session has correct workspace
    for session in &["bugfix-123", "feature-456", "experiment-789"] {
        harness.assert_workspace_exists(session);
        let result = harness.zjj(&["status", session]);
        assert!(result.success);
    }

    // Focus on different sessions (with --no-zellij to avoid Zellij deps)
    let result = harness.zjj(&["focus", "bugfix-123", "--no-zellij"]);
    assert!(result.success);
    result.assert_output_contains("bugfix-123");

    let result = harness.zjj(&["focus", "feature-456", "--no-zellij"]);
    assert!(result.success);
    result.assert_output_contains("feature-456");
}

#[test]
fn test_json_output_automation_workflow() {
    // Workflow: Automation scripts consuming zjj JSON output
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "automated-session", "--no-open"]);

    // Test init JSON output
    let result = harness.zjj(&["init", "--json"]);
    assert!(result.success);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "init --json should output valid JSON");

    // Test list JSON output
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "list --json should output valid JSON");

    // Test status JSON output
    let result = harness.zjj(&["status", "automated-session", "--json"]);
    assert!(result.success);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "status --json should output valid JSON");

    // Verify JSON structure includes session data
    if let Ok(json) = parsed {
        assert!(json.is_object());
        assert!(
            payload(&json).is_object(),
            "status --json payload should be an object"
        );
    }
}

// ============================================================================
// Error Handling and Edge Cases
// ============================================================================

#[test]
fn test_reinitialize_existing_zjj() {
    // Workflow: User runs init again (should be idempotent)
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Init again should succeed or warn, not fail
    let result = harness.zjj(&["init"]);
    assert!(result.success || result.stderr.contains("already"));
}

#[test]
fn test_session_name_with_numbers() {
    // Workflow: Feature branch names often include ticket numbers
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Valid session names with numbers
    harness.assert_success(&["add", "feature-123", "--no-open"]);
    harness.assert_success(&["add", "bug-456-fix", "--no-open"]);
    harness.assert_success(&["add", "task-789", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("feature-123");
    result.assert_stdout_contains("bug-456-fix");
    result.assert_stdout_contains("task-789");
}

#[test]
fn test_concurrent_session_operations() {
    // Workflow: Rapid operations on multiple sessions
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create sessions in quick succession
    for i in 0..5 {
        let name = format!("session-{i}");
        harness.assert_success(&["add", &name, "--no-open"]);
    }

    // Verify all exist
    let result = harness.zjj(&["list"]);
    for i in 0..5 {
        result.assert_stdout_contains(&format!("session-{i}"));
    }

    // Remove half of them
    harness.assert_success(&["remove", "session-0", "--force"]);
    harness.assert_success(&["remove", "session-2", "--force"]);
    harness.assert_success(&["remove", "session-4", "--force"]);

    // Verify correct sessions remain
    let result = harness.zjj(&["list"]);
    assert!(!result.stdout.contains("session-0"));
    result.assert_stdout_contains("session-1");
    assert!(!result.stdout.contains("session-2"));
    result.assert_stdout_contains("session-3");
    assert!(!result.stdout.contains("session-4"));
}

#[test]
fn test_workspace_cleanup_on_removal() {
    // Workflow: Verify workspace is properly cleaned up when session removed
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "cleanup-test", "--no-open"]);

    let workspace_path = harness.workspace_path("cleanup-test");
    assert!(workspace_path.exists());

    // Add a file in the workspace
    let test_file = workspace_path.join("test.txt");
    std::fs::write(&test_file, "test content").ok();
    assert!(test_file.exists());

    // Remove session
    harness.assert_success(&["remove", "cleanup-test", "--force"]);

    // Verify workspace is deleted (even with user files)
    assert!(!workspace_path.exists());
}

#[test]
fn test_status_after_multiple_operations() {
    // Workflow: Session status remains consistent across operations
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "persistent", "--no-open"]);

    // Check status multiple times
    for _ in 0..3 {
        let result = harness.zjj(&["status", "persistent"]);
        assert!(result.success);
        result.assert_output_contains("persistent");
    }

    // List should also show it consistently
    for _ in 0..3 {
        let result = harness.zjj(&["list"]);
        result.assert_stdout_contains("persistent");
    }
}

#[test]
fn test_session_with_underscores_and_hyphens() {
    // Workflow: Session names with different valid separators
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Various valid naming patterns
    harness.assert_success(&["add", "my_feature", "--no-open"]);
    harness.assert_success(&["add", "my-feature", "--no-open"]);
    harness.assert_success(&["add", "my_feature-123", "--no-open"]);
    harness.assert_success(&["add", "my_feature_123", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("my_feature");
    result.assert_stdout_contains("my-feature");
    result.assert_stdout_contains("my_feature-123");
    result.assert_stdout_contains("my_feature_123");
}

#[test]
fn test_empty_session_name_rejection() {
    // Workflow: User tries to create session with empty name
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Empty name should fail
    let result = harness.zjj(&["add", "", "--no-open"]);
    assert!(!result.success, "Empty session name should fail");
}

#[test]
fn test_very_long_session_name() {
    // Workflow: User tries to create session with very long name
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Very long name (should either work or fail gracefully)
    let long_name = "a".repeat(100);
    let result = harness.zjj(&["add", &long_name, "--no-open"]);

    // Either succeeds or fails with a reasonable error
    if !result.success {
        // If it fails, should have a clear error message
        assert!(!result.stderr.is_empty() || !result.stdout.is_empty());
    }
}

// ============================================================================
// Workspace Integration Tests
// ============================================================================

#[test]
fn test_workspace_is_jj_repository() {
    // Workflow: Verify created workspace is a valid JJ repo
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "jj-test", "--no-open"]);

    let workspace_path = harness.workspace_path("jj-test");

    // Verify .jj directory exists
    assert!(workspace_path.join(".jj").exists());

    // Verify jj commands work in workspace
    let result = harness.jj_in_dir(&workspace_path, &["workspace", "list"]);
    assert!(result.success, "jj should work in the workspace");
}

#[test]
fn test_multiple_sessions_independent_workspaces() {
    // Workflow: Each session has independent workspace
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "session-a", "--no-open"]);
    harness.assert_success(&["add", "session-b", "--no-open"]);

    let workspace_a = harness.workspace_path("session-a");
    let workspace_b = harness.workspace_path("session-b");

    // Verify both workspaces exist
    assert!(workspace_a.exists());
    assert!(workspace_b.exists());

    // Verify they're different directories
    assert_ne!(workspace_a, workspace_b);

    // Add different files to each
    std::fs::write(workspace_a.join("file-a.txt"), "content a").ok();
    std::fs::write(workspace_b.join("file-b.txt"), "content b").ok();

    // Verify independence
    assert!(workspace_a.join("file-a.txt").exists());
    assert!(!workspace_a.join("file-b.txt").exists());
    assert!(!workspace_b.join("file-a.txt").exists());
    assert!(workspace_b.join("file-b.txt").exists());
}

// ============================================================================
// Helper implementations for TestHarness
// ============================================================================

// Note: These helpers should be added to common/mod.rs if not already present
// For now, tests will use the existing zjj() method
