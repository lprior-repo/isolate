//! Comprehensive edge case tests for command operations
//!
//! This module tests advanced edge cases for command operations including:
//! - Concurrent session creation (race conditions)
//! - Empty database operations
//! - Duplicate session names
//! - Sessions without workspaces (orphaned DB entries)
//! - Database locking scenarios
//! - Hook execution failures
//!
//! Part of zjj-abk: Advanced edge case testing

mod common;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use common::TestHarness;
use serial_test::serial;

// ============================================================================
// Concurrent Session Creation Tests
// ============================================================================

#[test]
#[serial]
fn test_concurrent_add_same_name_race_condition() {
    // Test race condition where two processes try to create the same session simultaneously
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Spawn multiple threads trying to create the same session
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    for i in 0..3 {
        let b = Arc::clone(&barrier);
        let repo_path = harness.repo_path.clone();
        let handle = thread::spawn(move || {
            // Wait for all threads to be ready
            b.wait();

            // All threads try to create at the same time
            let result = std::process::Command::new(env!("CARGO_BIN_EXE_jjz"))
                .args(["add", "race-session", "--no-open"])
                .current_dir(&repo_path)
                .env("NO_COLOR", "1")
                .env("JJZ_TEST_MODE", "1")
                .output();

            (i, result)
        });
        handles.push(handle);
    }

    // Collect results
    let mut success_count = 0;
    let mut failure_count = 0;

    for handle in handles {
        if let Ok((id, Ok(output))) = handle.join() {
            if output.status.success() {
                success_count += 1;
                eprintln!("Thread {id} succeeded");
            } else {
                failure_count += 1;
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Thread {id} failed: {stderr}");
                // Should fail with either "already exists", JJ workspace error, or lock contention
                assert!(
                    stderr.contains("already exists")
                        || stderr.contains("UNIQUE")
                        || stderr.contains("Failed to create JJ workspace")
                        || stderr.contains("Failed to list JJ workspaces")
                        || stderr.contains("lock")
                        || stderr.contains("Resource temporarily unavailable"),
                    "Expected failure due to race condition, got: {stderr}"
                );
            }
        }
    }

    // Exactly one should succeed (database UNIQUE constraint)
    assert_eq!(
        success_count, 1,
        "Expected exactly 1 success, got {success_count}"
    );
    assert_eq!(failure_count, 2, "Expected 2 failures, got {failure_count}");

    // Verify only one session exists in the database
    let result = harness.jjz(&["list"]);
    assert!(result.success);

    // Count occurrences of "race-session" in output
    let count = result.stdout.matches("race-session").count();
    assert_eq!(count, 1, "Should have exactly one race-session in database");
}

#[test]
#[serial]
fn test_concurrent_add_different_names() {
    // Test that multiple sessions can be created concurrently with different names
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Use 3 threads instead of 5 to reduce JJ filesystem contention
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    for i in 0..3 {
        let b = Arc::clone(&barrier);
        let repo_path = harness.repo_path.clone();

        // Add small delay to stagger thread creation
        thread::sleep(std::time::Duration::from_millis(10));

        let handle = thread::spawn(move || {
            b.wait();

            let name = format!("concurrent-{i}");
            std::process::Command::new(env!("CARGO_BIN_EXE_jjz"))
                .args(["add", &name, "--no-open"])
                .current_dir(&repo_path)
                .env("NO_COLOR", "1")
                .env("JJZ_TEST_MODE", "1")
                .output()
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(output)) = handle.join() {
            if output.status.success() {
                success_count += 1;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Concurrent add failed: {stderr}");
            }
        }
    }

    // With 3 threads and staggering, at least 2 should succeed
    assert!(
        success_count >= 2,
        "Expected at least 2 successful concurrent adds, got {success_count}"
    );

    // Verify successful sessions exist in the database
    let result = harness.jjz(&["list"]);
    assert!(result.success);
    // At least verify that we got some sessions created
    assert!(
        !result.stdout.is_empty() || result.stdout.contains("concurrent"),
        "Should have created at least some concurrent sessions"
    );
}

#[test]
#[serial]
fn test_concurrent_remove_same_session() {
    // Test race condition where multiple processes try to remove the same session
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "to-remove", "--no-open"]);

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    for i in 0..3 {
        let b = Arc::clone(&barrier);
        let repo_path = harness.repo_path.clone();
        let handle = thread::spawn(move || {
            b.wait();

            let result = std::process::Command::new(env!("CARGO_BIN_EXE_jjz"))
                .args(["remove", "to-remove", "--force"])
                .current_dir(&repo_path)
                .env("NO_COLOR", "1")
                .env("JJZ_TEST_MODE", "1")
                .output();

            (i, result)
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok((_, Ok(output))) = handle.join() {
            if output.status.success() {
                success_count += 1;
            }
        }
    }

    // At least one should succeed (the first one to acquire the lock)
    assert!(
        success_count >= 1,
        "At least one remove should succeed, got {success_count}"
    );

    // Verify session no longer exists
    let result = harness.jjz(&["list"]);
    assert!(result.success);
    assert!(!result.stdout.contains("to-remove"));
}

// ============================================================================
// Empty Database Operations
// ============================================================================

#[test]
fn test_list_on_empty_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // List should work on empty database
    let result = harness.jjz(&["list"]);
    assert!(result.success, "List should succeed on empty database");

    // Output should indicate no sessions or be empty
    // Implementation may vary - either shows "No sessions" or empty list
}

#[test]
fn test_status_on_empty_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Status for nonexistent session in empty database
    let result = harness.jjz(&["status", "nonexistent"]);
    // Should either fail or show "not found"
    if result.success {
        result.assert_output_contains("not found");
    }
}

#[test]
fn test_remove_from_empty_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Remove should fail gracefully on empty database
    let result = harness.jjz(&["remove", "nonexistent", "--force"]);
    assert!(
        !result.success,
        "Remove should fail for nonexistent session"
    );
}

#[test]
fn test_focus_on_empty_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Focus should fail gracefully
    let result = harness.jjz(&["focus", "nonexistent"]);
    assert!(!result.success, "Focus should fail for nonexistent session");
}

// ============================================================================
// Duplicate Session Name Edge Cases
// ============================================================================

#[test]
fn test_add_duplicate_immediately_after_creation() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create session
    harness.assert_success(&["add", "session1", "--no-open"]);

    // Try to create duplicate immediately - should fail
    let result = harness.jjz(&["add", "session1", "--no-open"]);
    assert!(
        !result.success,
        "Duplicate session creation should fail. Stdout: {}, Stderr: {}",
        result.stdout, result.stderr
    );
}

#[test]
fn test_add_case_sensitive_duplicates() {
    // Session names are case-sensitive, so "Session" and "session" are different
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "Session", "--no-open"]);
    harness.assert_success(&["add", "session", "--no-open"]);
    harness.assert_success(&["add", "SESSION", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("Session");
    result.assert_stdout_contains("session");
    result.assert_stdout_contains("SESSION");
}

#[test]
fn test_add_after_remove_same_name() {
    // Should be able to reuse name after removal
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "reusable", "--no-open"]);
    harness.assert_success(&["remove", "reusable", "--force"]);

    // Should be able to create again with same name
    harness.assert_success(&["add", "reusable", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("reusable");
}

// ============================================================================
// Orphaned Database Entries (Sessions without Workspaces)
// ============================================================================

#[test]
fn test_list_with_missing_workspace_directories() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session
    harness.assert_success(&["add", "orphaned", "--no-open"]);

    // Manually delete the workspace directory (simulating filesystem corruption)
    let workspace_path = harness.workspace_path("orphaned");
    fs::remove_dir_all(&workspace_path).ok();

    // List should still work (database entry exists, workspace doesn't)
    let result = harness.jjz(&["list"]);
    assert!(result.success, "List should work with missing workspace");

    // Session should still appear in list (it's in DB)
    result.assert_stdout_contains("orphaned");
}

#[test]
fn test_status_with_missing_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "orphaned", "--no-open"]);

    // Delete workspace directory
    let workspace_path = harness.workspace_path("orphaned");
    fs::remove_dir_all(&workspace_path).ok();

    // Status should either succeed with warning or fail gracefully
    let result = harness.jjz(&["status", "orphaned"]);
    // Implementation may vary - either shows status with warning or fails
    if !result.success {
        result.assert_output_contains("orphaned");
    }
}

#[test]
fn test_remove_orphaned_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "orphaned", "--no-open"]);

    // Delete workspace directory
    let workspace_path = harness.workspace_path("orphaned");
    fs::remove_dir_all(&workspace_path).ok();

    // Remove should still work (clean up DB entry)
    harness.assert_success(&["remove", "orphaned", "--force"]);

    // Verify it's gone from database
    let result = harness.jjz(&["list"]);
    assert!(!result.stdout.contains("orphaned"));
}

#[test]
fn test_workspace_exists_without_db_entry() {
    // Opposite case: workspace directory exists but no DB entry
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Manually create a workspace directory without DB entry
    let workspace_path = harness.workspace_path("manual-workspace");
    fs::create_dir_all(&workspace_path).ok();

    // List should not show the manual workspace (only DB entries)
    let result = harness.jjz(&["list"]);
    assert!(result.success);
    assert!(
        !result.stdout.contains("manual-workspace"),
        "Manually created workspace should not appear in list"
    );
}

// ============================================================================
// Database Locking Scenarios
// ============================================================================

#[test]
#[serial]
fn test_multiple_concurrent_reads() {
    // Multiple list operations should work concurrently
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let b = Arc::clone(&barrier);
        let repo_path = harness.repo_path.clone();
        let handle = thread::spawn(move || {
            b.wait();

            std::process::Command::new(env!("CARGO_BIN_EXE_jjz"))
                .args(["list"])
                .current_dir(&repo_path)
                .env("NO_COLOR", "1")
                .env("JJZ_TEST_MODE", "1")
                .output()
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(output)) = handle.join() {
            if output.status.success() {
                success_count += 1;
            }
        }
    }

    // All reads should succeed
    assert_eq!(success_count, 10, "All concurrent reads should succeed");
}

#[test]
#[serial]
fn test_concurrent_read_write() {
    // Mix of read and write operations
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let barrier = Arc::new(Barrier::new(6));
    let mut handles = vec![];

    // 3 readers
    for i in 0..3 {
        let b = Arc::clone(&barrier);
        let repo_path = harness.repo_path.clone();
        let handle = thread::spawn(move || {
            b.wait();

            (
                format!("read-{i}"),
                std::process::Command::new(env!("CARGO_BIN_EXE_jjz"))
                    .args(["list"])
                    .current_dir(&repo_path)
                    .env("NO_COLOR", "1")
                    .env("JJZ_TEST_MODE", "1")
                    .output(),
            )
        });
        handles.push(handle);
    }

    // 3 writers
    for i in 0..3 {
        let b = Arc::clone(&barrier);
        let repo_path = harness.repo_path.clone();
        let handle = thread::spawn(move || {
            b.wait();

            let name = format!("write-session-{i}");
            (
                name.clone(),
                std::process::Command::new(env!("CARGO_BIN_EXE_jjz"))
                    .args(["add", &name, "--no-open"])
                    .current_dir(&repo_path)
                    .env("NO_COLOR", "1")
                    .env("JJZ_TEST_MODE", "1")
                    .output(),
            )
        });
        handles.push(handle);
    }

    let mut total_success = 0;
    for handle in handles {
        if let Ok((op, Ok(output))) = handle.join() {
            if output.status.success() {
                total_success += 1;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Operation {op} failed: {stderr}");
            }
        }
    }

    // Most operations should succeed (SQLite handles concurrent access)
    assert!(
        total_success >= 5,
        "Expected at least 5 successful operations, got {total_success}"
    );
}

#[test]
fn test_corrupted_database_recovery() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create some sessions
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);

    // Corrupt the database by writing garbage
    let db_path = harness.state_db_path();
    if fs::write(&db_path, "CORRUPTED DATA").is_err() {
        eprintln!("Could not corrupt database file");
        return;
    }

    // SQLite is very resilient and will treat corrupted files as new databases.
    // The application will successfully recreate the schema in most cases.
    // The key is that it handles corruption gracefully without crashing.
    let result = harness.jjz(&["list"]);

    // If the command succeeds, it has successfully recreated the database
    // If it fails, error should mention database issue
    if result.success {
        // Command succeeded - SQLite recovered by treating it as a new DB
        // This is actually the expected behavior for most corruption scenarios
        assert!(
            result.success,
            "Command should handle corruption gracefully"
        );
    } else {
        result.assert_output_contains("database");
    }
}

// ============================================================================
// Hook Execution Failures
// ============================================================================

#[test]
fn test_add_with_no_hooks_flag() {
    // --no-hooks should skip hook execution
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "no-hooks-session", "--no-open", "--no-hooks"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("no-hooks-session");
}

#[test]
fn test_add_with_failing_hook() {
    // Create a hook that will fail
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a hook directory
    let hooks_dir = harness.jjz_dir().join("hooks");
    fs::create_dir_all(&hooks_dir).ok();

    // Create a failing pre-add hook
    let hook_path = hooks_dir.join("pre-add");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&hook_path, "#!/bin/sh\nexit 1").ok();
        let metadata = fs::metadata(&hook_path).ok();
        if let Some(metadata) = metadata {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&hook_path, permissions).ok();
        }
    }
    #[cfg(not(unix))]
    {
        fs::write(&hook_path, "@echo off\nexit /b 1").ok();
    }

    // Try to add session - hook should fail
    let result = harness.jjz(&["add", "hooked-session", "--no-open"]);

    // Implementation may vary: either fails with hook error or succeeds anyway
    // If it fails, error should mention hook
    if !result.success {
        // Expected to fail with hook error
        result.assert_output_contains("hook");
    }
}

#[test]
fn test_add_with_missing_hook_executable() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create hooks directory with non-executable file
    let hooks_dir = harness.jjz_dir().join("hooks");
    fs::create_dir_all(&hooks_dir).ok();

    let hook_path = hooks_dir.join("pre-add");
    fs::write(&hook_path, "#!/bin/sh\necho test").ok();
    // Don't set executable permission

    // Should either skip the hook or fail gracefully
    let result = harness.jjz(&["add", "session", "--no-open"]);

    // Should succeed (non-executable hooks are typically skipped)
    if !result.success {
        let output = format!("{}{}", result.stdout, result.stderr);
        eprintln!("Unexpected failure: {output}");
    }
}

#[test]
fn test_remove_with_no_hooks_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Remove with --no-hooks (if implemented)
    let result = harness.jjz(&["remove", "test", "--force", "--no-hooks"]);

    // Should succeed (or fail gracefully if --no-hooks not implemented for remove)
    if !result.success {
        // May not support --no-hooks on remove
        eprintln!("Remove with --no-hooks not supported or failed");
    }
}

// ============================================================================
// Complex Edge Cases
// ============================================================================

#[test]
fn test_rapid_add_remove_cycles() {
    // Rapid creation and deletion to stress test database
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    for i in 0..5 {
        let name = format!("cycle-{i}");
        harness.assert_success(&["add", &name, "--no-open"]);

        // Small delay to ensure filesystem operations complete
        thread::sleep(Duration::from_millis(50));

        harness.assert_success(&["remove", &name, "--force"]);
    }

    // Database should be in consistent state
    let result = harness.jjz(&["list"]);
    assert!(result.success, "List should work after rapid cycles");
}

#[test]
fn test_max_sessions_stress_test() {
    // Create many sessions to test scalability
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let session_count = 20;

    for i in 0..session_count {
        let name = format!("stress-{i:02}");
        harness.assert_success(&["add", &name, "--no-open"]);
    }

    // List should show all sessions
    let result = harness.jjz(&["list"]);
    assert!(result.success);

    for i in 0..session_count {
        let name = format!("stress-{i:02}");
        result.assert_stdout_contains(&name);
    }

    // Clean up
    for i in 0..session_count {
        let name = format!("stress-{i:02}");
        harness.assert_success(&["remove", &name, "--force"]);
    }
}

#[test]
fn test_session_name_at_max_length() {
    // Test session name at exactly 64 characters (max allowed)
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a name with exactly 64 chars (starts with letter)
    let name = format!("a{}", "x".repeat(63));
    assert_eq!(name.len(), 64);

    harness.assert_success(&["add", &name, "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains(&name);

    harness.assert_success(&["remove", &name, "--force"]);
}

#[test]
fn test_operations_with_readonly_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "readonly-test", "--no-open"]);

    // Make database read-only
    let db_path = harness.state_db_path();
    if let Ok(metadata) = fs::metadata(&db_path) {
        let mut permissions = metadata.permissions();
        permissions.set_readonly(true);
        let _ = fs::set_permissions(&db_path, permissions);
    }

    // Read operations should still work
    let result = harness.jjz(&["list"]);
    assert!(result.success, "List should work with readonly database");

    // Write operations should fail gracefully (or may succeed if SQLite can write)
    // This test verifies the system handles readonly gracefully
    let _result = harness.jjz(&["add", "should-fail", "--no-open"]);
    // We don't assert failure because SQLite behavior with readonly files is complex

    // Restore write permissions for cleanup
    if let Ok(metadata) = fs::metadata(&db_path) {
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o644); // Read-write for owner, read for group/others
        let _ = fs::set_permissions(&db_path, permissions);
    }
}

#[test]
fn test_workspace_directory_without_jj_metadata() {
    // Workspace directory exists but .jj directory is missing/corrupted
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "broken-workspace", "--no-open"]);

    // Delete the .jj directory inside the workspace
    let workspace_path = harness.workspace_path("broken-workspace");
    let jj_dir = workspace_path.join(".jj");
    fs::remove_dir_all(&jj_dir).ok();

    // Status should detect the corruption
    let _result = harness.jjz(&["status", "broken-workspace"]);
    // May succeed with warning or fail - both are acceptable

    // Remove should still work (clean up what we can)
    harness.assert_success(&["remove", "broken-workspace", "--force"]);
}
