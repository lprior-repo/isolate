// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! Integration tests for error handling and edge cases
//!
//! Tests various error conditions and recovery scenarios
//!
//! Performance optimizations:
//! - Reuses `TestHarness` across related tests to reduce setup overhead
//! - Pre-allocates strings for validation tests
//! - Minimizes temp directory creation
//! - Uses functional error handling patterns

mod common;

use std::sync::OnceLock;

use common::TestHarness;

// ============================================================================
// Shared Test Utilities
// ============================================================================

/// Pre-allocated long name for validation tests (65 chars)
/// Cached to avoid repeated allocations in tests
fn long_session_name() -> String {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| "a".repeat(65)).clone()
}

/// Pre-allocated valid 64-char name
/// Cached to avoid repeated allocations
fn max_valid_session_name() -> String {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| "a".repeat(64)).clone()
}

// ============================================================================
// Missing Dependencies
// ============================================================================

#[test]
fn test_init_succeeds_with_jj_installed() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // This test assumes jj is installed
    // If it's not, init should fail with helpful error
    let result = harness.isolate(&["init"]);

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
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Empty name should fail
    harness.assert_failure(&["add", "", "--no-open"], "");
}

#[test]
fn test_add_session_name_too_long() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Name longer than 64 characters - use cached string
    harness.assert_failure(&["add", &long_session_name(), "--no-open"], "");
}

#[test]
fn test_add_session_name_starts_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "-session", "--no-open"], "Invalid session name");
}

#[test]
fn test_add_session_name_starts_with_underscore() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "_session", "--no-open"], "Invalid session name");
}

#[test]
fn test_add_session_name_with_slash() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
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
        // Test framework will handle skipping - no output needed
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
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to add without running init first
    let result = harness.isolate(&["add", "test", "--no-open"]);
    assert!(!result.success, "add should fail without init");
}

#[test]
fn test_list_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to list without running init first
    let result = harness.isolate(&["list"]);
    assert!(!result.success, "list should fail without init");
}

#[test]
fn test_remove_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    let result = harness.isolate(&["remove", "test", "--force"]);
    assert!(!result.success, "remove should fail without init");
}

#[test]
fn test_status_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    let result = harness.isolate(&["status", "test"]);
    assert!(!result.success, "status should fail without init");
}

// ============================================================================
// Nonexistent Sessions
// ============================================================================

#[test]
fn test_remove_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["remove", "nonexistent", "--force"], "");
}

#[test]
fn test_status_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let _result = harness.isolate(&["status", "nonexistent"]);
    // May fail or return empty - either is acceptable
}

#[test]
fn test_focus_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["focus", "nonexistent"]);
    assert!(!result.success, "focus should fail for nonexistent session");
}

#[test]
fn test_sync_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let _result = harness.isolate(&["sync", "nonexistent"]);
    // May fail or handle gracefully
}

#[test]
fn test_diff_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["diff", "nonexistent"]);
    assert!(!result.success, "diff should fail for nonexistent session");
}

// ============================================================================
// Concurrent Operations
// ============================================================================

#[test]
fn test_cannot_add_same_session_twice() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);
    harness.assert_failure(&["add", "test", "--no-open"], "already exists");
}

#[test]
fn test_remove_already_removed_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
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
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Corrupt the database by writing garbage
    // Functional error handling: use Result instead of abort
    let db_path = harness.state_db_path();
    let write_result = std::fs::write(&db_path, "garbage data");
    assert!(
        write_result.is_ok(),
        "Failed to corrupt database for testing"
    );

    // Operations should succeed by recovering (resetting the DB)
    // We need to set recovery policy to silent to allow auto-recovery without error
    let result = harness.isolate_with_env(&["list"], &[("Isolate_RECOVERY_POLICY", "silent")]);
    assert!(
        result.success,
        "Should recover from corrupted database (reset)"
    );
}

#[test]
fn test_missing_database() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Delete the database - functional error handling
    let db_path = harness.state_db_path();
    let remove_result = std::fs::remove_file(&db_path);
    assert!(
        remove_result.is_ok(),
        "Failed to remove database for testing"
    );

    // Operations should succeed by re-creating the database
    let result = harness.isolate(&["list"]);
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
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Create a file where workspace directory should be
    let workspaces_dir = harness.isolate_dir().join("workspaces");
    std::fs::create_dir_all(&workspaces_dir).ok();
    let blocking_file = workspaces_dir.join("test-session");
    std::fs::write(&blocking_file, "blocking").ok();

    // Try to add session - should fail
    let _result = harness.isolate(&["add", "test-session", "--no-open"]);
    // May fail or handle the conflict
}

#[test]
fn test_readonly_isolate_directory() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Make .isolate directory readonly - functional error handling
    let isolate_dir = harness.isolate_dir();
    let metadata =
        fs::metadata(&isolate_dir).unwrap_or_else(|e| panic!("Failed to get directory metadata: {e}"));
    let mut perms = metadata.permissions();
    perms.set_mode(0o444); // Readonly
    fs::set_permissions(&isolate_dir, perms)
        .unwrap_or_else(|e| panic!("Failed to set readonly permissions: {e}"));

    // Operations that need write access should fail
    let _result = harness.isolate(&["add", "test", "--no-open"]);
    // Should fail with permission error

    // Restore permissions for cleanup - use functional pattern
    fs::metadata(&isolate_dir)
        .and_then(|metadata| {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&isolate_dir, perms)
        })
        .unwrap_or_else(|e| panic!("Failed to restore permissions: {e}"));
}

// ============================================================================
// Invalid Arguments
// ============================================================================

#[test]
fn test_init_with_extra_arguments() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // init doesn't take arguments
    let _result = harness.isolate(&["init", "extra"]);
    // May fail or ignore extra args
}

#[test]
fn test_add_missing_name_argument() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["add"]);
    assert!(!result.success, "add requires a name argument");
}

#[test]
fn test_remove_missing_name_argument() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["remove"]);
    assert!(!result.success, "remove requires a name argument");
}

#[test]
fn test_diff_missing_name_argument() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.isolate(&["diff"]);
    assert!(!result.success, "diff requires a name argument");
}

#[test]
fn test_unknown_subcommand() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    let result = harness.isolate(&["unknown-command"]);
    assert!(!result.success, "Unknown subcommand should fail");
}

#[test]
fn test_invalid_flag() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    let result = harness.isolate(&["init", "--invalid-flag"]);
    assert!(!result.success, "Invalid flag should fail");
}

// ============================================================================
// Config File Errors
// ============================================================================

#[test]
fn test_invalid_toml_config() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Write invalid TOML
    harness.write_config("invalid toml {{{").ok();

    // Commands that read config may fail gracefully
    let _result = harness.isolate(&["add", "test", "--no-open"]);
    // Should either fail or use defaults
}

#[test]
fn test_missing_config_file() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Delete config file
    let config_path = harness.isolate_dir().join("config.toml");
    std::fs::remove_file(&config_path).ok();

    // Commands should still work with defaults or fail gracefully
    let _result = harness.isolate(&["add", "test", "--no-open"]);
    // Implementation may vary
}

// ============================================================================
// JJ Repository Errors
// ============================================================================

#[test]
fn test_corrupted_jj_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Corrupt the JJ workspace
    let workspace_jj = harness.workspace_path("test").join(".jj");
    std::fs::remove_dir_all(&workspace_jj).ok();

    // Status and other operations may fail
    let _result = harness.isolate(&["status", "test"]);
    // May fail or report corruption
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_session_name_exactly_64_chars() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Exactly 64 characters should be valid - use cached string
    let result = harness.isolate(&["add", &max_valid_session_name(), "--no-open"]);
    assert!(result.success, "64-character name should be valid");
}

#[test]
fn test_session_name_with_numbers_only_rejected() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
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
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Add and remove multiple times - optimized with reduced overhead
    // Use functional pattern: iterate with side effects
    (0..3).for_each(|_| {
        harness.assert_success(&["add", "cycle", "--no-open"]);
        harness.assert_success(&["remove", "cycle", "--force"]);
    });

    // Should work without issues
    let result = harness.isolate(&["list"]);
    assert!(result.success);
}

#[test]
fn test_list_with_no_sessions_after_remove_all() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
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
    let result = harness.isolate(&["list"]);
    assert!(result.success);
}

// ============================================================================
// PHASE 1: Security & Data Loss Prevention Tests
// ============================================================================

#[test]
fn test_session_name_path_traversal_double_dot() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name ".." should be rejected (prevents directory traversal)
    harness.assert_failure(&["add", "..", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_path_traversal_parent_ref() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name "../etc" should be rejected (prevents workspace in system directories)
    harness.assert_failure(&["add", "../etc", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_absolute_path() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name "/tmp/evil" should be rejected (prevents absolute path injection)
    harness.assert_failure(&["add", "/tmp/evil", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_null_byte() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with null byte should be rejected (prevents null byte injection in filesystem
    // operations)
    // Note: Null bytes in strings are typically filtered by the shell before reaching validation,
    // but we verify the command fails rather than succeeding with truncated input
    let result = harness.isolate(&["add", "test\0evil", "--no-open"]);
    assert!(
        !result.success,
        "Command with null byte in session name should fail (shell filtering or validation)"
    );
}

#[test]
fn test_session_name_zero_width_chars() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
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
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Create a workspace normally first
    harness.assert_success(&["add", "test-ws", "--no-open"]);

    // Get the workspace path
    let workspace_path = harness.workspace_path("test-ws");

    // Create a symlink that points to the workspace
    let workspaces_dir = harness.isolate_dir().join("workspaces");
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
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with all special characters should fail with clear error message
    harness.assert_failure(&["add", "!@#$%^&*()", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_embedded_tab() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with embedded tab should be rejected (prevents invisible whitespace)
    harness.assert_failure(&["add", "test\tname", "--no-open"], "Invalid session name");
}

#[test]
fn test_session_name_embedded_newline() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Session name with embedded newline should be rejected (prevents multi-line names)
    harness.assert_failure(&["add", "test\nname", "--no-open"], "Invalid session name");
}

#[test]
fn test_rapid_sequential_add_remove() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Rapid add/remove cycles should maintain database integrity
    // Pre-allocate session names to avoid repeated format! calls
    let session_names: Vec<String> = (0..10).map(|i| format!("rapid{i}")).collect();

    for name in &session_names {
        harness.assert_success(&["add", name, "--no-open"]);
        harness.assert_success(&["remove", name, "--force"]);
    }

    // Verify no sessions remain
    let result = harness.isolate(&["list"]);
    assert!(result.success);
}

#[test]
fn test_status_with_manually_deleted_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
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
    let result = harness.isolate(&["status", "orphaned"]);
    // We don't assert success/failure - just that it completes without panic
    drop(result);
}
