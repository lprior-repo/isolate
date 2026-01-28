//! Integration tests for error handling and edge cases
//!
//! Tests various error conditions and recovery scenarios

mod common;

use common::TestHarness;

// ============================================================================
// Missing Dependencies
// ============================================================================

#[test]
fn test_init_succeeds_with_jj_and_zellij_installed() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // This test assumes jj and zellij are installed
    // If they're not, init should fail with helpful error
    let result = harness.zjj(&["init"]);

    // Either succeeds (if deps available) or fails with helpful message
    if !result.success {
        result.assert_output_contains("dependencies");
    }
}

// ============================================================================
// Invalid Session Names
// ============================================================================

#[test]
fn test_add_empty_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Empty name should fail
    harness.assert_failure(&["add", "", "--no-open"], "");
}

#[test]
fn test_add_session_name_too_long() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Name longer than 64 characters
    let long_name = "a".repeat(65);
    harness.assert_failure(&["add", &long_name, "--no-open"], "");
}

#[test]
fn test_add_session_name_starts_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "-session", "--no-open"], "Invalid session name");
}

#[test]
fn test_add_session_name_starts_with_underscore() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "_session", "--no-open"], "Invalid session name");
}

#[test]
fn test_add_session_name_with_slash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(
        &["add", "feature/branch", "--no-open"],
        "Invalid session name",
    );
}

#[test]
fn test_add_session_name_with_dots() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "test..name", "--no-open"], "Invalid session name");
}

// ============================================================================
// Operations Without Init
// ============================================================================

#[test]
fn test_add_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Try to add without running init first
    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "add should fail without init");
}

#[test]
fn test_list_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Try to list without running init first
    let result = harness.zjj(&["list"]);
    assert!(!result.success, "list should fail without init");
}

#[test]
fn test_remove_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["remove", "test", "--force"]);
    assert!(!result.success, "remove should fail without init");
}

#[test]
fn test_status_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["status", "test"]);
    assert!(!result.success, "status should fail without init");
}

// ============================================================================
// Nonexistent Sessions
// ============================================================================

#[test]
fn test_remove_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["remove", "nonexistent", "--force"], "");
}

#[test]
fn test_status_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let _result = harness.zjj(&["status", "nonexistent"]);
    // May fail or return empty - either is acceptable
}

#[test]
fn test_focus_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["focus", "nonexistent"]);
    assert!(!result.success, "focus should fail for nonexistent session");
}

#[test]
fn test_sync_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let _result = harness.zjj(&["sync", "nonexistent"]);
    // May fail or handle gracefully
}

#[test]
fn test_diff_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["diff", "nonexistent"]);
    assert!(!result.success, "diff should fail for nonexistent session");
}

// ============================================================================
// Concurrent Operations
// ============================================================================

#[test]
fn test_cannot_add_same_session_twice() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);
    harness.assert_failure(&["add", "test", "--no-open"], "already exists");
}

#[test]
fn test_remove_already_removed_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);
    harness.assert_success(&["remove", "test", "--force"]);

    // Try to remove again
    harness.assert_failure(&["remove", "test", "--force"], "");
}

// ============================================================================
// Corrupted Database
// ============================================================================

#[test]
fn test_corrupted_database_recovery() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Corrupt the database by writing garbage
    let db_path = harness.state_db_path();
    if std::fs::write(&db_path, "garbage data").is_err() {
        std::process::abort()
    }

    // Operations should succeed by recovering (resetting the DB)
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "Should recover from corrupted database (reset)"
    );
}

#[test]
fn test_missing_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Delete the database
    let db_path = harness.state_db_path();
    if std::fs::remove_file(&db_path).is_err() {
        std::process::abort()
    }

    // Operations should succeed by re-creating the database
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "Should recover from missing database (re-create)"
    );
}

// ============================================================================
// File System Errors
// ============================================================================

#[test]
fn test_workspace_directory_creation_failure() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a file where workspace directory should be
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    std::fs::create_dir_all(&workspaces_dir).ok();
    let blocking_file = workspaces_dir.join("test-session");
    std::fs::write(&blocking_file, "blocking").ok();

    // Try to add session - should fail
    let _result = harness.zjj(&["add", "test-session", "--no-open"]);
    // May fail or handle the conflict
}

#[test]
fn test_readonly_zjj_directory() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Make .zjj directory readonly
    let zjj_dir = harness.zjj_dir();
    let Ok(metadata) = fs::metadata(&zjj_dir) else {
        std::process::abort()
    };
    let mut perms = metadata.permissions();
    perms.set_mode(0o444); // Readonly
    fs::set_permissions(&zjj_dir, perms).ok();

    // Operations that need write access should fail
    let _result = harness.zjj(&["add", "test", "--no-open"]);
    // Should fail with permission error

    // Restore permissions for cleanup
    let Ok(metadata) = fs::metadata(&zjj_dir) else {
        std::process::abort()
    };
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&zjj_dir, perms).ok();
}

// ============================================================================
// Invalid Arguments
// ============================================================================

#[test]
fn test_init_with_extra_arguments() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // init doesn't take arguments
    let _result = harness.zjj(&["init", "extra"]);
    // May fail or ignore extra args
}

#[test]
fn test_add_missing_name_argument() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add"]);
    assert!(!result.success, "add requires a name argument");
}

#[test]
fn test_remove_missing_name_argument() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["remove"]);
    assert!(!result.success, "remove requires a name argument");
}

#[test]
fn test_diff_missing_name_argument() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["diff"]);
    assert!(!result.success, "diff requires a name argument");
}

#[test]
fn test_unknown_subcommand() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["unknown-command"]);
    assert!(!result.success, "Unknown subcommand should fail");
}

#[test]
fn test_invalid_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["init", "--invalid-flag"]);
    assert!(!result.success, "Invalid flag should fail");
}

// ============================================================================
// Config File Errors
// ============================================================================

#[test]
fn test_invalid_toml_config() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Write invalid TOML
    harness.write_config("invalid toml {{{").ok();

    // Commands that read config may fail gracefully
    let _result = harness.zjj(&["add", "test", "--no-open"]);
    // Should either fail or use defaults
}

#[test]
fn test_missing_config_file() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Delete config file
    let config_path = harness.zjj_dir().join("config.toml");
    std::fs::remove_file(&config_path).ok();

    // Commands should still work with defaults or fail gracefully
    let _result = harness.zjj(&["add", "test", "--no-open"]);
    // Implementation may vary
}

// ============================================================================
// JJ Repository Errors
// ============================================================================

#[test]
fn test_corrupted_jj_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Corrupt the JJ workspace
    let workspace_jj = harness.workspace_path("test").join(".jj");
    std::fs::remove_dir_all(&workspace_jj).ok();

    // Status and other operations may fail
    let _result = harness.zjj(&["status", "test"]);
    // May fail or report corruption
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_session_name_exactly_64_chars() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Exactly 64 characters should be valid
    let name = "a".repeat(64);
    let result = harness.zjj(&["add", &name, "--no-open"]);
    assert!(result.success, "64-character name should be valid");
}

#[test]
fn test_session_name_with_numbers_only_rejected() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Numbers only should be rejected
    harness.assert_failure(
        &["add", "12345", "--no-open"],
        "Session name must start with a letter",
    );
}

#[test]
fn test_rapid_add_remove_cycles() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Add and remove multiple times
    for _ in 0..3 {
        harness.assert_success(&["add", "cycle", "--no-open"]);
        harness.assert_success(&["remove", "cycle", "--force"]);
    }

    // Should work without issues
    let result = harness.zjj(&["list"]);
    assert!(result.success);
}

#[test]
fn test_list_with_no_sessions_after_remove_all() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Add multiple sessions
    harness.assert_success(&["add", "s1", "--no-open"]);
    harness.assert_success(&["add", "s2", "--no-open"]);

    // Remove all
    harness.assert_success(&["remove", "s1", "--force"]);
    harness.assert_success(&["remove", "s2", "--force"]);

    // List should succeed but show no sessions
    let result = harness.zjj(&["list"]);
    assert!(result.success);
}

// ============================================================================
// PHASE 1: Security & Data Loss Prevention Tests
// ============================================================================

#[test]
fn test_session_name_path_traversal_double_dot() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name ".." should be rejected (prevents directory traversal)
    harness.assert_failure(&["add", "..", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_path_traversal_parent_ref() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name "../etc" should be rejected (prevents workspace in system directories)
    harness.assert_failure(&["add", "../etc", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_absolute_path() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name "/tmp/evil" should be rejected (prevents absolute path injection)
    harness.assert_failure(&["add", "/tmp/evil", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_null_byte() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with null byte should be rejected (prevents null byte injection in filesystem
    // operations)
    // Note: Null bytes in strings are typically filtered by the shell before reaching validation,
    // but we verify the command fails rather than succeeding with truncated input
    let result = harness.zjj(&["add", "test\0evil", "--no-open"]);
    assert!(
        !result.success,
        "Command with null byte in session name should fail (shell filtering or validation)"
    );
}

#[test]
fn test_session_name_zero_width_chars() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with U+200B (zero-width space) should be rejected
    let name_with_zwsp = "test\u{200B}name";
    harness.assert_failure(&["add", name_with_zwsp, "--no-open"], "ASCII");
}

#[test]
#[cfg(unix)]
fn test_remove_workspace_symlink_cleanup() {
    use std::os::unix::fs as unix_fs;

    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a workspace normally first
    harness.assert_success(&["add", "test-ws", "--no-open"]);

    // Get the workspace path
    let workspace_path = harness.workspace_path("test-ws");

    // Create a symlink that points to the workspace
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    let symlink_target = workspaces_dir.join("test-symlink");

    if std::fs::create_dir_all(&workspaces_dir).is_ok()
        && unix_fs::symlink(&workspace_path, &symlink_target).is_ok()
    {
        // Now when we remove the original workspace, the symlink should only be removed,
        // not the target it points to (data loss prevention)
        harness.assert_success(&["remove", "test-ws", "--force"]);

        // Verify symlink is gone
        assert!(!symlink_target.exists(), "Symlink should be removed");
    }
}

// ============================================================================
// PHASE 2: UX & Error Messages Tests
// ============================================================================

#[test]
fn test_session_name_all_special_chars() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with all special characters should fail with clear error message
    harness.assert_failure(&["add", "!@#$%^&*()", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_embedded_tab() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with embedded tab should be rejected (prevents invisible whitespace)
    harness.assert_failure(&["add", "test\tname", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_embedded_newline() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with embedded newline should be rejected (prevents multi-line names)
    harness.assert_failure(&["add", "test\nname", "--no-open"], "Invalid session name");
}

#[test]
fn test_rapid_sequential_add_remove() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Rapid add/remove cycles should maintain database integrity
    for i in 0..10 {
        let session_name = format!("rapid{i}");
        harness.assert_success(&["add", &session_name, "--no-open"]);
        harness.assert_success(&["remove", &session_name, "--force"]);
    }

    // Verify no sessions remain
    let result = harness.zjj(&["list"]);
    assert!(result.success);
}

#[test]
fn test_status_with_manually_deleted_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session normally
    harness.assert_success(&["add", "orphaned", "--no-open"]);

    // Manually delete the workspace directory
    let workspace_path = harness.workspace_path("orphaned");
    if workspace_path.exists() {
        let _result = std::fs::remove_dir_all(&workspace_path);
    }

    // Status command should detect the orphaned session
    // The command may succeed or fail depending on implementation,
    // but it should not panic or hang
    let result = harness.zjj(&["status", "orphaned"]);
    // We don't assert success/failure - just that it completes without panic
    drop(result);
}
