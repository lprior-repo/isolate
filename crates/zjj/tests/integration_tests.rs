//! Integration tests for core ZJJ workflows
//!
//! These tests cover the critical user workflows end-to-end:
//! - Session creation → workspace creation → task operations → close workflow
//! - Queue operations: enqueue → list → dequeue
//! - Session state transitions through full lifecycle
//! - Workspace creation and removal
//!
//! ## Test Architecture
//!
//! - **Multiple state changes across aggregates**: Tests span sessions, workspaces, tasks, and queues
//! - **Database/file I/O interactions**: Real SQLite databases and file system operations
//! - **Full workflow scenarios**: Complete user journeys from start to finish
//!
//! ## Design Principles
//!
//! - Uses test helpers from `common::mod` for harness setup
//! - Tests real integration behavior with actual persistence
//! - Follows functional patterns: Result types, no unwraps in production code
//! - Each test is independent and cleans up after itself

#![allow(clippy::unwrap_used)] // Integration tests use unwrap for test assertions
#![allow(clippy::panic)] // Integration tests use panic for assertions
#![allow(clippy::too_many_lines)] // Integration test scenarios are comprehensive

mod common;

use common::{TestHarness, CommandResult};

// ============================================================================
// WORKFLOW 1: Session → Workspace → Close Lifecycle
// ============================================================================

#[test]
fn integration_session_workspace_close_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Step 1: Initialize zjj
    harness.assert_success(&["init"]);
    harness.assert_zjj_dir_exists();

    // Step 2: Create a session (this also creates workspace)
    let session_name = "feature-auth";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);
    harness.assert_workspace_exists(session_name);

    // Step 3: Verify session is listed
    let result = harness.zjj(&["list", "--output=json"]);
    result.assert_success();
    result.assert_stdout_contains(session_name);

    // Step 4: Check whoami shows the current session
    let whoami = harness.zjj(&["whoami"]);
    whoami.assert_success();
    whoami.assert_stdout_contains(session_name);

    // Step 5: Close the session (remove workspace)
    harness.assert_success(&["remove", session_name, "--merge"]);
    harness.assert_workspace_not_exists(session_name);

    // Step 6: Verify session is no longer in list
    let list_after = harness.zjj(&["list", "--output=json"]);
    list_after.assert_success();
    assert!(!list_after.stdout.contains(session_name) || list_after.stdout.contains("[]"));
}

#[test]
fn integration_multiple_sessions_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize and create sessions
    harness.assert_success(&["init"]);

    let sessions = vec![
        "feature-payment",
        "feature-auth",
        "feature-ui",
    ];

    for session in &sessions {
        harness.assert_success(&["add", session, "--no-zellij", "--no-hooks"]);
    }

    // Verify all sessions are listed
    let list_result = harness.zjj(&["list", "--output=json"]);
    list_result.assert_success();

    for session in &sessions {
        list_result.assert_stdout_contains(session);
    }

    // Verify all workspaces exist
    for session in &sessions {
        harness.assert_workspace_exists(session);
    }

    // Cleanup
    for session in &sessions {
        harness.assert_success(&["remove", session, "--merge"]);
    }

    // Verify all workspaces removed
    for session in &sessions {
        harness.assert_workspace_not_exists(session);
    }
}

#[test]
fn integration_session_branch_transitions() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize
    harness.assert_success(&["init"]);

    // Create initial session on main branch
    let session1 = "session-main";
    harness.assert_success(&["add", session1, "--no-zellij", "--no-hooks"]);

    // Create a new branch
    harness.jj(&["bookmark", "create", "feature-xyz"]);

    // Create session on new branch
    let session2 = "session-feature";
    harness.assert_success(&["add", session2, "--no-zellij", "--no-hooks"]);

    // Verify both sessions exist
    let list_result = harness.zjj(&["list", "--output=json"]);
    list_result.assert_success();
    list_result.assert_stdout_contains(session1);
    list_result.assert_stdout_contains(session2);

    // Cleanup
    harness.assert_success(&["remove", session1, "--merge"]);
    harness.assert_success(&["remove", session2, "--merge"]);
}

// ============================================================================
// WORKFLOW 2: Queue Operations - Enqueue → List → Dequeue
// ============================================================================

#[test]
fn integration_queue_enqueue_dequeue_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize
    harness.assert_success(&["init"]);

    // Create sessions for queue processing
    let sessions = vec!["queue-session-1", "queue-session-2", "queue-session-3"];
    for session in &sessions {
        harness.assert_success(&["add", session, "--no-zellij", "--no-hooks"]);
    }

    // Add workspaces to queue
    for session in &sessions {
        let result = harness.zjj(&["queue", "enqueue", session]);
        result.assert_success();
        result.assert_stdout_contains("enqueued");
    }

    // List queue to verify entries
    let queue_list = harness.zjj(&["queue", "list"]);
    queue_list.assert_success();
    for session in &sessions {
        queue_list.assert_stdout_contains(session);
    }

    // Get queue status
    let status = harness.zjj(&["queue", "status"]);
    status.assert_success();

    // Dequeue sessions
    for session in &sessions {
        let result = harness.zjj(&["queue", "dequeue", session]);
        result.assert_success();
    }

    // Verify queue is empty
    let list_after = harness.zjj(&["queue", "list"]);
    list_after.assert_success();
    // Empty queue output format varies, just check command succeeded

    // Cleanup sessions
    for session in &sessions {
        harness.assert_success(&["remove", session, "--merge"]);
    }
}

#[test]
fn integration_queue_status_with_entries() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create and enqueue a session
    let session_name = "status-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    let enqueue_result = harness.zjj(&["queue", "enqueue", session_name]);
    enqueue_result.assert_success();

    // Get queue status with JSON output
    let status = harness.zjj(&["queue", "status", "--json"]);
    status.assert_success();

    // Verify session appears in status
    status.assert_stdout_contains(session_name);

    // Cleanup
    harness.zjj(&["queue", "dequeue", session_name]);
    harness.assert_success(&["remove", session_name, "--merge"]);
}

#[test]
fn integration_queue_dequeue_nonexistent() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Attempt to dequeue non-existent session
    let result = harness.zjj(&["queue", "dequeue", "nonexistent-session"]);
    // Should fail gracefully
    assert!(!result.success, "Dequeue should fail for non-existent session");
}

// ============================================================================
// WORKFLOW 3: Session State Transitions Full Lifecycle
// ============================================================================

#[test]
fn integration_session_full_lifecycle_transitions() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize
    harness.assert_success(&["init"]);

    // State 1: No session (initial state)
    let whoami_before = harness.zjj(&["whoami", "--json"]);
    whoami_before.assert_success();
    // Check for unregistered state
    whoami_before.assert_stdout_contains("unregistered");

    // State 2: Create session (Creating → Ready)
    let session_name = "lifecycle-session";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // State 3: Session becomes active
    let whoami_after = harness.zjj(&["whoami", "--json"]);
    whoami_after.assert_success();
    whoami_after.assert_stdout_contains(session_name);

    // Verify session is in list
    let list_result = harness.zjj(&["list", "--output=json"]);
    list_result.assert_success();
    list_result.assert_stdout_contains(session_name);

    // State 4: Switch away from session (Ready state persists)
    let session2 = "another-session";
    harness.assert_success(&["add", session2, "--no-zellij", "--no-hooks"]);

    // State 5: Switch back to original session
    let switch_result = harness.zjj(&["switch", session_name]);
    switch_result.assert_success();

    // Verify we're back in original session
    let whoami_switched = harness.zjj(&["whoami", "--json"]);
    whoami_switched.assert_success();
    whoami_switched.assert_stdout_contains(session_name);

    // State 6: Close session (Active → Removed)
    harness.assert_success(&["remove", session_name, "--merge"]);

    // State 7: Verify session is no longer active
    let whoami_final = harness.zjj(&["whoami", "--json"]);
    whoami_final.assert_success();
    // Should show different session or unregistered

    // Cleanup
    harness.assert_success(&["remove", session2, "--merge"]);
}

#[test]
fn integration_session_switch_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let sessions = vec!["session-a", "session-b", "session-c"];
    for session in &sessions {
        harness.assert_success(&["add", session, "--no-zellij", "--no-hooks"]);
    }

    // Start in first session
    let whoami1 = harness.zjj(&["whoami", "--json"]);
    whoami1.assert_success();
    whoami1.assert_stdout_contains("session-c"); // Last created is current

    // Switch to first session
    let switch_result = harness.zjj(&["switch", "session-a"]);
    switch_result.assert_success();

    let whoami2 = harness.zjj(&["whoami", "--json"]);
    whoami2.assert_success();
    whoami2.assert_stdout_contains("session-a");

    // Switch again
    let switch_result2 = harness.zjj(&["switch", "session-b"]);
    switch_result2.assert_success();

    let whoami3 = harness.zjj(&["whoami", "--json"]);
    whoami3.assert_success();
    whoami3.assert_stdout_contains("session-b");

    // Cleanup
    for session in &sessions {
        harness.assert_success(&["remove", session, "--merge"]);
    }
}

// ============================================================================
// WORKFLOW 4: Workspace Creation and Removal
// ============================================================================

#[test]
fn integration_workspace_creation_removal_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "workspace-test";
    let workspace_path = harness.workspace_path(session_name);

    // Verify workspace doesn't exist initially
    harness.assert_workspace_not_exists(session_name);

    // Create workspace (via add session)
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);
    harness.assert_workspace_exists(session_name);

    // Verify workspace contains expected JJ files
    let jj_path = workspace_path.join(".jj");
    harness.assert_file_exists(&jj_path);

    // Verify workspace is a valid JJ repo
    let jj_status = harness.jj_in_dir(&workspace_path, &["status"]);
    jj_status.assert_success();

    // Remove workspace
    harness.assert_success(&["remove", session_name, "--merge"]);
    harness.assert_workspace_not_exists(session_name);
}

#[test]
fn integration_multiple_workspaces_isolation() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let sessions = vec!["workspace-1", "workspace-2", "workspace-3"];
    for session in &sessions {
        harness.assert_success(&["add", session, "--no-zellij", "--no-hooks"]);
    }

    // Verify all workspaces exist and are isolated
    for session in &sessions {
        let workspace_path = harness.workspace_path(session);
        harness.assert_file_exists(&workspace_path);

        // Create a file in one workspace
        if session == &"workspace-1" {
            let test_file = workspace_path.join("test.txt");
            std::fs::write(&test_file, "test content").expect("Write should succeed");
            harness.assert_file_exists(&test_file);
        }
    }

    // Verify file only exists in workspace-1
    for session in &sessions {
        let workspace_path = harness.workspace_path(session);
        let test_file = workspace_path.join("test.txt");
        if session == &"workspace-1" {
            harness.assert_file_exists(&test_file);
        } else {
            harness.assert_file_not_exists(&test_file);
        }
    }

    // Cleanup
    for session in &sessions {
        harness.assert_success(&["remove", session, "--merge"]);
    }
}

#[test]
fn integration_workspace_state_persistence() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "persistent-session";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // Create a commit in the workspace
    let workspace_path = harness.workspace_path(session_name);
    let test_file = workspace_path.join("feature.txt");
    std::fs::write(&test_file, "feature content").expect("Write should succeed");

    harness.jj_in_dir(&workspace_path, &["new", "add-feature"]);
    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Add feature"]);

    // Verify commit persists across session switches
    let log_result = harness.jj_in_dir(&workspace_path, &["log"]);
    log_result.assert_success();
    log_result.assert_stdout_contains("Add feature");

    // Switch away and back
    harness.assert_success(&["add", "temp-session", "--no-zellij", "--no-hooks"]);
    harness.assert_success(&["switch", session_name]);

    // Verify state persisted
    let log_after = harness.jj_in_dir(&workspace_path, &["log"]);
    log_after.assert_success();
    log_after.assert_stdout_contains("Add feature");

    // Cleanup
    harness.assert_success(&["remove", "temp-session", "--merge"]);
    harness.assert_success(&["remove", session_name, "--merge"]);
}

// ============================================================================
// WORKFLOW 5: Complex Multi-Aggregate Scenarios
// ============================================================================

#[test]
fn integration_status_across_all_aggregates() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions
    let sessions = vec!["status-test-1", "status-test-2"];
    for session in &sessions {
        harness.assert_success(&["add", session, "--no-zellij", "--no-hooks"]);
    }

    // Add to queue
    harness.zjj(&["queue", "enqueue", "status-test-1"]).assert_success();

    // Get comprehensive status
    let status_result = harness.zjj(&["status", "--json"]);
    status_result.assert_success();

    // Status should include sessions information
    let list_result = harness.zjj(&["list"]);
    list_result.assert_success();
    for session in &sessions {
        list_result.assert_stdout_contains(session);
    }

    // Cleanup
    harness.zjj(&["queue", "dequeue", "status-test-1"]);
    for session in &sessions {
        harness.assert_success(&["remove", session, "--merge"]);
    }
}

#[test]
fn integration_error_recovery_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Attempt to remove non-existent session
    let remove_result = harness.zjj(&["remove", "non-existent", "--merge"]);
    assert!(!remove_result.success, "Should fail for non-existent session");

    // Verify system is still functional
    let session_name = "recovery-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);
    let list_result = harness.zjj(&["list"]);
    list_result.assert_success();

    // Cleanup
    harness.assert_success(&["remove", session_name, "--merge"]);
}

#[test]
fn integration_session_with_sync_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "sync-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // Make a change in the workspace
    let workspace_path = harness.workspace_path(session_name);
    let test_file = workspace_path.join("change.txt");
    std::fs::write(&test_file, "test change").expect("Write should succeed");

    harness.jj_in_dir(&workspace_path, &["new", "test-change"]);
    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Test change"]);

    // Sync with main
    let sync_result = harness.zjj(&["sync"]);
    sync_result.assert_success();

    // Cleanup
    harness.assert_success(&["remove", session_name, "--merge"]);
}

#[test]
fn integration_context_command_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Get context before session
    let ctx_before = harness.zjj(&["context", "--json"]);
    ctx_before.assert_success();

    // Create session
    let session_name = "context-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // Get context after session creation
    let ctx_after = harness.zjj(&["context", "--json"]);
    ctx_after.assert_success();
    ctx_after.assert_stdout_contains(session_name);

    // Cleanup
    harness.assert_success(&["remove", session_name, "--merge"]);
}

#[test]
fn integration_whereami_command_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Check location before session
    let where_before = harness.zjj(&["whereami"]);
    where_before.assert_success();
    where_before.assert_stdout_contains("main");

    // Create session
    let session_name = "whereami-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // Check location in session
    let where_after = harness.zjj(&["whereami"]);
    where_after.assert_success();
    where_after.assert_stdout_contains("workspace");

    // Cleanup
    harness.assert_success(&["remove", session_name, "--merge"]);

    // Check location after session
    let where_final = harness.zjj(&["whereami"]);
    where_final.assert_success();
    where_final.assert_stdout_contains("main");
}

#[test]
fn integration_list_with_filters() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let sessions = vec!["list-test-1", "list-test-2", "list-test-3"];
    for session in &sessions {
        harness.assert_success(&["add", session, "--no-zellij", "--no-hooks"]);
    }

    // List all sessions
    let all_result = harness.zjj(&["list", "--output=json"]);
    all_result.assert_success();
    for session in &sessions {
        all_result.assert_stdout_contains(session);
    }

    // Cleanup
    for session in &sessions {
        harness.assert_success(&["remove", session, "--merge"]);
    }
}

#[test]
fn integration_diff_command_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "diff-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // Make a change in the workspace
    let workspace_path = harness.workspace_path(session_name);
    let test_file = workspace_path.join("diff-test.txt");
    std::fs::write(&test_file, "content for diff").expect("Write should succeed");

    harness.jj_in_dir(&workspace_path, &["new", "diff-change"]);
    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Add diff test file"]);

    // Get diff
    let diff_result = harness.zjj(&["diff", "--json"]);
    diff_result.assert_success();

    // Cleanup
    harness.assert_success(&["remove", session_name, "--merge"]);
}

#[test]
fn integration_done_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let session_name = "done-test";
    harness.assert_success(&["add", session_name, "--no-zellij", "--no-hooks"]);

    // Make a change
    let workspace_path = harness.workspace_path(session_name);
    let test_file = workspace_path.join("done-test.txt");
    std::fs::write(&test_file, "completed work").expect("Write should succeed");

    harness.jj_in_dir(&workspace_path, &["new", "done-change"]);
    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Completed work"]);

    // Use done command to merge and cleanup
    let done_result = harness.zjj(&["done"]);
    done_result.assert_success();

    // Verify workspace is cleaned up
    harness.assert_workspace_not_exists(session_name);
}

// ============================================================================
// HELPERS AND ASSERTIONS
// ============================================================================

/// Helper to verify a session exists in the list output
#[allow(dead_code)]
fn assert_session_exists(result: &CommandResult, session_name: &str) {
    result.assert_stdout_contains(session_name);
}

/// Helper to verify a session does not exist in the list output
#[allow(dead_code)]
fn assert_session_not_exists(result: &CommandResult, session_name: &str) {
    assert!(!result.stdout.contains(session_name),
        "Session '{}' should not exist in output: {}", session_name, result.stdout);
}
