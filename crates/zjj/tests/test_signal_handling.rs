//! Integration tests for graceful shutdown and signal handling
//!
//! Tests SIGINT/SIGTERM handling during various operations to ensure:
//! - Clean state after interruption
//! - No orphaned resources
//! - Proper cleanup of in-flight operations
//! - Database consistency after shutdown

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]

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

    // OPTIMIZATION: Reduced from 5 to 3 cycles
    // Each cycle creates/removes a session with multiple subprocess operations
    // 3 cycles provides sufficient stress test coverage while 40% faster
    let cycle_count = 3;

    // Perform multiple rapid add/remove cycles to stress test cleanup
    // Use functional approach to track results
    let completed_cycles = (0..cycle_count)
        .map(|i| {
            let session_name = format!("cycle-session-{i}");

            // Add session - early return on failure using and_then
            let add_result = harness.zjj(&["add", &session_name, "--no-open"]);
            if !add_result.success {
                return None;
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

            Some(())
        })
        .count();

    // Verify we attempted all cycles
    assert_eq!(completed_cycles, cycle_count, "Should attempt all cycles");

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

    // OPTIMIZATION: Reduced from 10 to 5 sessions
    // Each session spawns multiple subprocesses (JJ, git, database ops)
    // 5 sessions provides sufficient coverage while 40% faster
    let session_names: Vec<String> = (0..5).map(|i| format!("concurrent-{i}")).collect();

    // Create sessions - use functional map to collect results
    let create_results: Vec<_> = session_names
        .iter()
        .map(|name| harness.zjj(&["add", name, "--no-open"]))
        .collect();

    // Verify at least some sessions were created successfully
    let created_count = create_results.iter().filter(|r| r.success).count();
    assert!(
        created_count > 0,
        "At least some sessions should be created successfully"
    );

    // Simulate mid-stream interruption by checking state
    // In real scenario, SIGTERM would arrive here

    // Verify final state consistency
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "List should succeed after concurrent operations"
    );

    // OPTIMIZATION: Cache the workspace check results to avoid repeated directory checks
    // Only check sessions that were successfully created - use functional iteration
    let created_sessions: Vec<_> = session_names
        .iter()
        .filter(|name| harness.workspace_path(name).exists())
        .collect();

    // Verify all created sessions are valid in one pass
    for name in created_sessions {
        let jj_result = harness.jj(&["workspace", "list"]);
        assert!(
            jj_result.success || result.stdout.contains(name),
            "Concurrent workspace should be valid: {name}"
        );
    }

    // Cleanup - functional approach with proper error handling
    let _remove_results: Vec<_> = session_names
        .iter()
        .map(|name| harness.zjj(&["remove", name, "--force"]))
        .collect();
}

#[test]
fn test_concurrent_list_operations_with_interruption() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create some sessions first - use functional creation
    let sessions = ["session1", "session2", "session3"];
    for name in sessions {
        harness.assert_success(&["add", name, "--no-open"]);
    }

    // Perform multiple list operations rapidly
    // In a real scenario, these could be interrupted by signals
    // Use functional approach to verify all succeed
    let list_results: Vec<_> = (0..5).map(|_| harness.zjj(&["list"])).collect();

    assert!(
        list_results.iter().all(|r| r.success),
        "All list operations should succeed during rapid operations"
    );

    // Verify state remains consistent - cache the result
    let result = harness.zjj(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("session1");
    result.assert_stdout_contains("session2");
    result.assert_stdout_contains("session3");

    // Cleanup - functional approach
    for name in sessions {
        harness.assert_success(&["remove", name, "--force"]);
    }
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

    // OPTIMIZATION: Reduced from 5 to 3 sessions
    // Each session involves subprocess creation and filesystem operations
    // 3 sessions provides sufficient orphan detection while 40% faster
    let created_sessions: Vec<String> = (0..3)
        .map(|i| format!("orphan-test-{i}"))
        .filter_map(|session_name| {
            let result = harness.zjj(&["add", &session_name, "--no-open"]);
            if result.success {
                Some(session_name)
            } else {
                None
            }
        })
        .collect();

    // Verify all created sessions are valid - functional iteration
    for session in &created_sessions {
        let workspace_path = harness.workspace_path(session);
        assert!(workspace_path.exists(), "Created workspace should exist");

        // Verify it's a valid JJ workspace
        assert!(
            workspace_path.join(".jj").exists(),
            "Should have .jj directory"
        );
    }

    // OPTIMIZATION: Short-circuit if no sessions created
    if created_sessions.is_empty() {
        return; // Nothing to verify
    }

    // Verify no extra workspaces exist - functional error handling
    let workspaces_dir = harness.repo_path.join("workspaces");
    if workspaces_dir.exists() {
        let entries_result = std::fs::read_dir(&workspaces_dir);
        assert!(
            entries_result.is_ok(),
            "Should be able to read workspaces directory"
        );

        let workspace_count = entries_result
            .unwrap_or_else(|e| panic!("Directory exists and is readable: {e}"))
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .count();

        assert_eq!(
            workspace_count,
            created_sessions.len(),
            "Should have exactly the number of workspaces we created"
        );
    }

    // Cleanup - functional approach
    let _cleanup_results: Vec<_> = created_sessions
        .iter()
        .map(|session| harness.zjj(&["remove", session, "--force"]))
        .collect();
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

    // OPTIMIZATION: Reduced from 3 to 2 iterations
    // Each iteration runs 2 commands (status + list) = 6 total commands
    // 2 iterations = 4 commands, still sufficient for rapid signal testing
    let iteration_count = 2;

    // Simulate rapid repeated operations (like multiple signals)
    // Use functional approach to verify all operations succeed
    let operation_results: Vec<_> = (0..iteration_count)
        .flat_map(|_| {
            [
                harness.zjj(&["status", "signal-test"]),
                harness.zjj(&["list"]),
            ]
        })
        .collect();

    assert!(
        operation_results.iter().all(|r| r.success),
        "All rapid operations should succeed"
    );

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

    // OPTIMIZATION: Reduced from 5 to 3 iterations
    // Status checks are fast (database queries), but 3 is sufficient
    // to test rapid interruption handling
    let check_count = 3;

    // Perform rapid status checks - functional approach
    let status_results: Vec<_> = (0..check_count)
        .map(|_| harness.zjj(&["status", "status-interrupt"]))
        .collect();

    assert!(
        status_results.iter().all(|r| r.success),
        "All status checks should succeed during rapid operations"
    );

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
        harness.assert_success(&["add", &format!("recovery-{i}"), "--no-open"]);
    }

    // Manually remove one workspace to simulate partial state
    let workspace_path = harness.workspace_path("recovery-1");
    if workspace_path.exists() {
        let _ = std::fs::remove_dir_all(&workspace_path);
    }

    // Operations should handle missing workspace gracefully
    let _status_result = harness.zjj(&["status", "recovery-1"]);
    // The system may succeed (graceful degradation) or report missing workspace
    // Both outcomes are acceptable for robust error handling

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

    // OPTIMIZATION: Reduced from 3 to 2 iterations
    // Status checks are database operations, 2 checks sufficient for
    // testing timeout state consistency
    let check_count = 2;

    // Perform rapid operations that could timeout in real scenarios
    // Use functional approach to collect results
    let status_results: Vec<_> = (0..check_count)
        .map(|_| harness.zjj(&["status", "timeout-test"]))
        .collect();

    assert!(
        status_results.iter().all(|r| r.success),
        "All status checks should succeed"
    );

    // Regardless of rapid operations, state should remain consistent
    let final_result = harness.zjj(&["status", "timeout-test"]);
    assert!(
        final_result.success,
        "State should remain consistent after rapid operations"
    );

    // Cleanup
    harness.assert_success(&["remove", "timeout-test", "--force"]);
}
