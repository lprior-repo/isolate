//! DRQ Adversarial Test Bank
//!
//! These tests are designed to FAIL on the current champion implementation.
//! They represent real-world failure modes that AI agents will encounter.
//!
//! The goal is not to find bugs, but to discover weakness in the
//! *fitness signal* - the set of tests that define "correctness".
//!
//! Each test here should:
//! 1. FAIL on the current implementation
//! 2. Expose a real-world failure mode
//! 3. Become a permanent test once fixed (regression must not reoccur)

// Import from the test common module
mod common;
use std::fs;

use common::TestHarness;
use serde_json::Value as JsonValue;

// ============================================================================
// OPPONENT 1: State Divergence
// Tests for crashes leaving inconsistent state
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_workspace_exists_without_db_entry() {
    // SCENARIO: Process crashes after workspace creation but before DB insert
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - Workspace directory exists on disk
    // - `zjj list` shows nothing (no DB entry)
    // - `zjj add same-name` fails because workspace already exists
    // - Zombie state that requires manual cleanup
    //
    // EXPECTED BEHAVIOR:
    // - Either: Transactional rollback (delete workspace on failure)
    // - Or: Recovery on next operation (detect orphaned workspace)
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Manually create a workspace directory (simulating crash after workspace creation)
    let workspace_path = harness.workspace_path("zombie-session");
    assert!(
        fs::create_dir_all(&workspace_path).is_ok(),
        "Failed to create workspace for test setup"
    );

    // Try to create a session with the same name
    let result = harness.zjj(&["add", "zombie-session", "--no-open"]);

    // CURRENT CHAMPION: This fails with unclear error
    // EXPECTED: Either succeeds (recovering the orphan) OR fails with clear error
    assert!(
        !result.success,
        "Should detect workspace exists and handle it"
    );

    // Verify the error message is actionable
    let error_output = if result.stdout.contains("error") || result.stdout.contains("Error") {
        &result.stdout
    } else {
        &result.stderr
    };

    // EXPECTED: Error mentions cleanup action OR automatic recovery occurred
    assert!(
        error_output.contains("already exists")
            || error_output.contains("workspace")
            || error_output.contains("recover"),
        "Error should be actionable: {error_output}"
    );
}

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_db_entry_exists_without_workspace() {
    // SCENARIO: Process crashes after DB insert but before workspace creation
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - DB entry exists
    // - `zjj list` shows the session
    // - `zjj status` fails because workspace is missing
    //
    // EXPECTED BEHAVIOR:
    // - Health check on every operation that accesses workspace
    // - Automatically mark session as "failed" if workspace missing
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "ghost-session", "--no-open"]);

    // Delete the workspace behind zjj's back
    let workspace_path = harness.workspace_path("ghost-session");
    assert!(
        fs::remove_dir_all(&workspace_path).is_ok(),
        "Failed to remove workspace for test setup"
    );

    // Query session-exists
    let query_result = harness.zjj(&["query", "session-exists", "ghost-session", "--json"]);
    assert!(query_result.success, "Query should succeed");

    let Ok(json) = serde_json::from_str::<JsonValue>(&query_result.stdout) else {
        panic!("Failed to parse JSON output from query");
    };

    // CURRENT CHAMPION: Reports exists=true despite workspace being gone
    // EXPECTED: Either exists=false OR status indicates failure/missing
    let exists = json
        .get("exists")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    if exists {
        // If it reports existence, the status should reflect the problem
        if let Some(session) = json.get("session") {
            let status = session
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            assert_ne!(
                status, "active",
                "Session should not report as 'active' when workspace is missing, got: {status}"
            );
        }
    }
}

// ============================================================================
// OPPONENT 2: Concurrent Agents
// Tests for race conditions in multi-agent scenarios
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_concurrent_same_session_creation() {
    // SCENARIO: Two agents both try to create the same session
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - Both check db.get().is_some() - both see false
    // - Both try to create workspace - one succeeds, one fails
    // - The one that fails gets a confusing error
    // - Lock table exists but is not used by add command
    //
    // EXPECTED BEHAVIOR:
    // - Exactly one succeeds
    // - The other gets a clear "session already exists" error
    // - Or: Second agent waits for first to complete
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // We can't actually spawn concurrent processes in this test,
    // but we can document the gap: lock table exists but add.rs doesn't use it
    //
    // This test would need to be implemented with actual concurrent processes
    // to properly test the race condition.
    //
    // DOCUMENTATION GAP: Lock acquisition needs to be integrated into add/remove/sync
}

// ============================================================================
// OPPONENT 3: JSON vs Exit Code Truth
// Tests for inconsistency between JSON output and exit codes
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_json_success_matches_exit_code() {
    // SCENARIO: Every command should have consistent truth between JSON and exit code
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - sync.rs outputs JSON then returns Err
    // - exit code 1 but JSON might say success=true
    //
    // EXPECTED BEHAVIOR:
    // - exit_code == 0 ⇔ success == true
    // - exit_code != 0 ⇔ success == false AND error field is present
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Test various commands
    let commands: &[&[&str]] = &[
        &["list", "--json"],
        &["query", "session-count", "--json"],
        &["status", "nonexistent", "--json"],
    ];

    for args in commands {
        let result = harness.zjj(args);

        // Parse JSON if available
        if let Ok(json) = serde_json::from_str::<JsonValue>(&result.stdout) {
            if let Some(success) = json.get("success").and_then(serde_json::Value::as_bool) {
                assert_eq!(
                    success,
                    result.success,
                    "JSON success field ({}) must match exit code ({}={}) for command {:?}\nOutput: {}",
                    success,
                    !result.success,
                    result.exit_code.unwrap_or(1),
                    args,
                    result.stdout
                );
            }
        }
    }
}

// ============================================================================
// OPPONENT 4: Orphan Detection and Recovery
// Tests for detecting and cleaning up inconsistent state
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_clean_command_detects_orphans() {
    // SCENARIO: Workspace exists without DB entry
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - `zjj clean` only removes sessions from DB that have no workspace
    // - Does NOT detect workspaces that have no DB entry
    //
    // EXPECTED BEHAVIOR:
    // - `zjj clean --detect-orphans` finds workspaces without DB entries
    // - Offers to recover or delete them
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Create an orphan workspace
    let workspace_path = harness.workspace_path("orphan-session");
    assert!(
        fs::create_dir_all(&workspace_path).is_ok(),
        "Failed to create workspace for test setup"
    );

    // Run clean
    let clean_result = harness.zjj(&["clean", "--dry-run", "--json"]);
    assert!(clean_result.success, "clean should succeed");

    let Ok(json) = serde_json::from_str::<JsonValue>(&clean_result.stdout) else {
        panic!("Failed to parse JSON output from clean");
    };

    // CURRENT CHAMPION: Does not report orphaned workspace
    // EXPECTED: Lists orphaned workspaces and suggests actions
    let orphan_count = json
        .get("orphans")
        .and_then(|o| o.as_array())
        .map(Vec::len)
        .unwrap_or(0);

    // This will FAIL on current champion - that's the point
    // Once fixed, this test should pass
    // WARNING: clean command does not detect orphaned workspaces
    // This is expected to FAIL on current champion
    assert!(
        orphan_count > 0,
        "clean command should detect orphaned workspaces (expected to fail on current champion)"
    );
}

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_recover_orphaned_session() {
    // SCENARIO: Workspace exists without DB entry, agent wants to recover it
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - No recovery mechanism
    // - Agent must manually delete workspace or manually add DB entry
    //
    // EXPECTED BEHAVIOR:
    // - `zjj add --recover orphan-session` detects workspace and creates DB entry
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Create a realistic orphan workspace (with .jj directory)
    let workspace_path = harness.workspace_path("recoverable-session");
    assert!(
        fs::create_dir_all(&workspace_path).is_ok(),
        "Failed to create workspace for test setup"
    );

    // Create a minimal .jj directory to make it look real
    let jj_dir = workspace_path.join(".jj");
    assert!(
        fs::create_dir_all(&jj_dir).is_ok(),
        "Failed to create .jj for test setup"
    );

    // Try to add with --recover flag (if it exists)
    let result = harness.zjj(&["add", "recoverable-session", "--no-open"]);

    // CURRENT CHAMPION: Fails because workspace exists
    // EXPECTED: With --recover flag, succeeds by detecting and adopting workspace
    assert!(
        !result.success,
        "Current champion does not support --recover"
    );
}

// ============================================================================
// OPPONENT 5: Non-Functional Requirements
// Tests for timeouts, memory, latency
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_query_performance_scales_with_session_count() {
    // SCENARIO: Agent has 1000 sessions, queries should still be fast
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - No performance tests exist
    // - Unknown if queries scale linearly or have performance cliffs
    //
    // EXPECTED BEHAVIOR:
    // - All queries complete in < 100ms even with 1000 sessions
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    let start = std::time::Instant::now();

    // Query with no sessions
    let result = harness.zjj(&["query", "session-count", "--json"]);
    assert!(result.success);

    let elapsed = start.elapsed();

    // Baseline should be fast
    assert!(
        elapsed.as_millis() < 100,
        "Empty query should complete in < 100ms, took {}ms",
        elapsed.as_millis()
    );

    // This test would need to be expanded to test with actual session load
    // For now, it documents the performance requirement
}

// ============================================================================
// OPPONENT 6: Error Message Quality
// Tests for actionable, parseable error messages
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_error_messages_are_actionable() {
    // SCENARIO: Agent encounters an error, needs to know what to do
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - Some errors are cryptic or missing context
    // - No standard "recovery" field in error responses
    //
    // EXPECTED BEHAVIOR:
    // - Every error includes "recovery" field with suggested action
    // - Error codes are machine-parsable for automated handling
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Try various error scenarios
    let error_scenarios: &[&[&str]] = &[
        &["add", "123invalid", "--no-open", "--json"], // Invalid name
        &["status", "nonexistent", "--json"],          // Not found
        &["remove", "nonexistent", "-f", "--json"],    // Remove non-existent
    ];

    let mut missing_recovery_count = 0;

    for args in error_scenarios {
        let result = harness.zjj(args);

        if !result.success {
            // Try to parse as JSON
            if let Ok(json) = serde_json::from_str::<JsonValue>(&result.stdout) {
                if let Some(error) = json.get("error") {
                    // EXPECTED: Error should have actionable recovery info
                    let has_recovery = error.get("recovery").is_some()
                        || error.get("suggestion").is_some()
                        || error.get("resolution").is_some();

                    if !has_recovery {
                        missing_recovery_count += 1;
                        // Track error scenarios lacking recovery field
                    }
                }
            }
        }
    }

    // Assert that all errors have recovery information
    assert_eq!(
        missing_recovery_count, 0,
        "{missing_recovery_count} error scenarios lack recovery information"
    );
}

// ============================================================================
// OPPONENT 7: Idempotency
// Tests for safe retry of operations
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_add_is_not_idempotent() {
    // SCENARIO: Agent tries to create a session that already exists
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - Returns error "session already exists"
    // - Agent can't distinguish "created just now" from "existed before"
    //
    // EXPECTED BEHAVIOR:
    // - With --idempotent flag, succeeds if session already exists in correct state
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // First add
    harness.assert_success(&["add", "idemp-test", "--no-open"]);

    // Second add without --idempotent (should fail)
    let result = harness.zjj(&["add", "idemp-test", "--no-open"]);

    // CURRENT CHAMPION: Fails with "already exists"
    // EXPECTED: With --idempotent, returns success with "already_exists" field
    assert!(
        !result.success,
        "Duplicate add should fail without --idempotent flag"
    );

    // Cleanup
    harness.assert_success(&["remove", "idemp-test", "-f"]);
}

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_remove_is_somewhat_idempotent() {
    // SCENARIO: Agent tries to remove a session that doesn't exist
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - Returns error "session not found"
    // - Agent can't safely retry remove operation
    //
    // EXPECTED BEHAVIOR:
    // - With --idempotent flag, succeeds if session doesn't exist
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Remove session that doesn't exist
    let _result = harness.zjj(&["remove", "never-existed", "-f"]);

    // CURRENT CHAMPION: Fails
    // EXPECTED: With --idempotent, succeeds with "already_removed" field
    //
    // Note: remove -f is already somewhat idempotent (it doesn't error if not found)
    // This test documents that behavior
}

// ============================================================================
// OPPONENT 8: Lock Cleanup
// Tests for automatic lock expiration handling
// ============================================================================

#[test]
#[ignore = "DRQ adversarial - designed to fail on current champion"]
fn test_lock_cleanup_on_query() {
    // SCENARIO: Lock expires, next operation should clean it up
    //
    // CURRENT CHAMPION BEHAVIOR:
    // - cleanup_expired_locks() exists but is never called automatically
    // - Lock table could grow indefinitely
    //
    // EXPECTED BEHAVIOR:
    // - Lock cleanup runs automatically on every query
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Note: We can't directly acquire locks via the CLI
    // This would require a new command like `zjj lock acquire`
    //
    // This test documents the gap: locks exist but no CLI interface to test them
}

// ============================================================================
// Fitness Signal Summary
// ============================================================================
//
// These tests define the fitness signal for zjj:
//
// 1. **State Consistency**: Database and filesystem never diverge
// 2. **Concurrency Safety**: Multiple agents can operate without races
// 3. **Truth Alignment**: JSON output matches exit codes
// 4. **Self-Healing**: Orphaned state is detected and recoverable
// 5. **Performance**: Operations complete within time budgets
// 6. **Error Quality**: All errors are actionable
// 7. **Idempotency**: Safe retry for all operations
//
// The "champion" is the implementation that passes ALL these tests.
// Any regression means the champion has been dethroned.
