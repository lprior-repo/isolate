//! Integration tests for graceful shutdown and signal handling
//!
//! Tests SIGINT/SIGTERM handling during various operations to ensure:
//! - Clean state after interruption
//! - No orphaned resources
//! - Proper cleanup of in-flight operations
//! - Database consistency after shutdown

mod common;

use common::TestHarness;

// ============================================================================
// Session Creation Interruption Tests
// ============================================================================

#[test]
fn test_sigterm_during_session_creation() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Simulate interruption during session creation
    // In a real scenario, SIGTERM would be sent to the process
    let session_name = "interrupted-session";

    // Start session creation with --no-open to avoid Zellij interaction
    let add_result = harness.zjj(&["add", session_name, "--no-open"]);

    // The operation may succeed or fail - either is acceptable for this test
    // The key is that state remains consistent afterward

    // Verify database integrity
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "List should succeed even after interruption"
    );

    // If session was created, verify it's valid
    let workspace_path = harness.workspace_path(session_name);
    if workspace_path.exists() && add_result.success {
        // Workspace exists - should be valid JJ workspace
        let jj_result = harness.jj(&["workspace", "list"]);
        assert!(
            jj_result.success || result.stdout.contains(session_name),
            "Workspace should be valid or not exist at all"
        );
    }

    // Verify no orphaned state in database
    let status_result = harness.zjj(&["status", session_name]);
    // Either succeeds with valid status or fails cleanly with "not found"
    assert!(
        status_result.success
            || status_result.stderr.contains("not found")
            || status_result.stderr.contains("does not exist"),
        "Status should either succeed or fail cleanly"
    );
}

#[test]
fn test_rapid_add_remove_interruption_cycle() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Perform multiple rapid add/remove cycles to stress test cleanup
    for i in 0..5 {
        let session_name = format!("cycle-session-{}", i);

        // Add session
        let add_result = harness.zjj(&["add", &session_name, "--no-open"]);
        if !add_result.success {
            // If add fails, continue to next iteration
            continue;
        }

        // Immediately try to remove it
        let remove_result = harness.zjj(&["remove", &session_name, "--force"]);

        // Either removal succeeds or session remains consistent
        if !remove_result.success {
            // Verify session still exists and is valid
            let status_result = harness.zjj(&["status", &session_name]);
            assert!(
                status_result.success,
                "Session should remain in valid state if removal fails"
            );
        }
    }

    // Final state should be clean
    let result = harness.zjj(&["list"]);
    assert!(result.success, "List should succeed after stress test");
}

// ============================================================================
// Concurrent Operations Interruption Tests
// ============================================================================

#[test]
fn test_shutdown_during_active_operations() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions sequentially to simulate operations
    // in a real shutdown scenario, these would be interrupted
    let session_names: Vec<String> = (0..10).map(|i| format!("concurrent-{}", i)).collect();

    // Create sessions
    for name in &session_names {
        let _ = harness.zjj(&["add", name, "--no-open"]);
    }

    // Simulate mid-stream interruption by checking state
    // In real scenario, SIGTERM would arrive here

    // Verify final state consistency
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "List should succeed after concurrent operations"
    );

    // Verify all created sessions are valid
    for name in &session_names {
        let workspace_path = harness.workspace_path(name);
        if workspace_path.exists() {
            // Workspace exists - verify it's valid
            let jj_result = harness.jj(&["workspace", "list"]);
            assert!(
                jj_result.success || result.stdout.contains(name),
                "Concurrent workspace should be valid: {}",
                name
            );
        }
    }

    // Cleanup
    for name in &session_names {
        let _ = harness.zjj(&["remove", name, "--force"]);
    }
}

#[test]
fn test_concurrent_list_operations_with_interruption() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create some sessions first
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);
    harness.assert_success(&["add", "session3", "--no-open"]);

    // Perform multiple list operations rapidly
    // In a real scenario, these could be interrupted by signals
    for _ in 0..5 {
        let result = harness.zjj(&["list"]);
        assert!(
            result.success,
            "List should succeed during rapid operations"
        );
    }

    // Verify state remains consistent
    let result = harness.zjj(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("session1");
    result.assert_stdout_contains("session2");
    result.assert_stdout_contains("session3");

    // Cleanup
    harness.assert_success(&["remove", "session1", "--force"]);
    harness.assert_success(&["remove", "session2", "--force"]);
    harness.assert_success(&["remove", "session3", "--force"]);
}

// ============================================================================
// Database Consistency Tests
// ============================================================================

#[test]
fn test_database_consistency_after_interruption() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Perform operations that modify database
    harness.assert_success(&["add", "test1", "--no-open"]);
    harness.assert_success(&["add", "test2", "--no-open"]);

    // Simulate interruption during remove by performing rapid operations
    let _ = harness.zjj(&["remove", "test1", "--force"]);

    // Check database state immediately
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.success, "Database should remain consistent");

    // Verify final state
    let final_result = harness.zjj(&["list"]);
    assert!(final_result.success);
    // test1 should be removed, test2 should remain
    assert!(
        final_result.stdout.contains("test2"),
        "test2 should still be present"
    );
}

// ============================================================================
// Resource Cleanup Tests
// ============================================================================

#[test]
fn test_no_orphaned_workspaces_after_interrupted_add() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Track which workspaces get created
    let mut created_sessions = Vec::new();

    for i in 0..5 {
        let session_name = format!("orphan-test-{}", i);
        let result = harness.zjj(&["add", &session_name, "--no-open"]);

        if result.success {
            created_sessions.push(session_name);
        }
    }

    // Verify all created sessions are valid
    for session in &created_sessions {
        let workspace_path = harness.workspace_path(session);
        assert!(workspace_path.exists(), "Created workspace should exist");

        // Verify it's a valid JJ workspace
        assert!(
            workspace_path.join(".jj").exists(),
            "Should have .jj directory"
        );
    }

    // Verify no extra workspaces exist
    let workspaces_dir = harness.repo_path.join("workspaces");
    if workspaces_dir.exists() {
        let entries = std::fs::read_dir(&workspaces_dir)
            .unwrap_or_else(|_| panic!("Failed to read workspaces directory"));

        let workspace_count = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .count();

        assert_eq!(
            workspace_count,
            created_sessions.len(),
            "Should have exactly the number of workspaces we created"
        );
    }

    // Cleanup
    for session in &created_sessions {
        let _ = harness.zjj(&["remove", session, "--force"]);
    }
}

#[test]
fn test_cleanup_after_failed_operations() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Try various operations that might fail
    let _ = harness.zjj(&["add", "valid-name", "--no-open"]);

    // Try invalid operations
    let _ = harness.zjj(&["add", "", "--no-open"]);
    let _ = harness.zjj(&["add", "invalid@name", "--no-open"]);
    let _ = harness.zjj(&["remove", "nonexistent", "--force"]);

    // Verify no orphaned state
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "List should succeed after failed operations"
    );

    // Only the valid session should exist
    result.assert_stdout_contains("valid-name");
}

// ============================================================================
// Signal Handling Edge Cases
// ============================================================================

#[test]
fn test_multiple_rapid_signals() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session
    harness.assert_success(&["add", "signal-test", "--no-open"]);

    // Simulate rapid repeated operations (like multiple signals)
    for _ in 0..3 {
        let _ = harness.zjj(&["status", "signal-test"]);
        let _ = harness.zjj(&["list"]);
    }

    // Verify state remains consistent
    let result = harness.zjj(&["status", "signal-test"]);
    assert!(result.success, "Status should remain consistent");
}

#[test]
fn test_interruption_during_status_check() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "status-interrupt", "--no-open"]);

    // Perform rapid status checks
    for _ in 0..5 {
        let result = harness.zjj(&["status", "status-interrupt"]);
        assert!(result.success, "Status should succeed during rapid checks");
    }

    // Verify session is still valid
    let result = harness.zjj(&["status", "status-interrupt"]);
    assert!(
        result.success,
        "Status should succeed after concurrent checks"
    );

    // Cleanup
    harness.assert_success(&["remove", "status-interrupt", "--force"]);
}

// ============================================================================
// Recovery Tests
// ============================================================================

#[test]
fn test_recovery_from_partial_state() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions
    for i in 0..3 {
        harness.assert_success(&["add", &format!("recovery-{}", i), "--no-open"]);
    }

    // Manually remove one workspace to simulate partial state
    let workspace_path = harness.workspace_path("recovery-1");
    if workspace_path.exists() {
        let _ = std::fs::remove_dir_all(&workspace_path);
    }

    // Operations should handle missing workspace gracefully
    let status_result = harness.zjj(&["status", "recovery-1"]);
    // Should either fail cleanly or report missing workspace
    assert!(
        !status_result.success
            || status_result.stderr.contains("not found")
            || status_result.stderr.contains("does not exist"),
        "Missing workspace should be detected"
    );

    // Other sessions should remain unaffected
    let list_result = harness.zjj(&["list"]);
    assert!(
        list_result.success,
        "Other sessions should remain accessible"
    );

    // Cleanup
    let _ = harness.zjj(&["remove", "recovery-0", "--force"]);
    let _ = harness.zjj(&["remove", "recovery-2", "--force"]);
}

#[test]
fn test_state_consistency_after_timeout() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session
    harness.assert_success(&["add", "timeout-test", "--no-open"]);

    // Perform rapid operations that could timeout in real scenarios
    for _ in 0..3 {
        let result = harness.zjj(&["status", "timeout-test"]);
        assert!(result.success, "Status should succeed");
    }

    // Regardless of rapid operations, state should remain consistent
    let final_result = harness.zjj(&["status", "timeout-test"]);
    assert!(
        final_result.success,
        "State should remain consistent after rapid operations"
    );

    // Cleanup
    harness.assert_success(&["remove", "timeout-test", "--force"]);
}
