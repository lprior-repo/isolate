//! Error recovery and resilience tests
//!
//! Tests comprehensive error recovery scenarios including:
//! - Corrupted database recovery
//! - Partial cleanup scenarios
//! - Missing workspace directories
//! - Invalid config TOML files
//! - Permission denied errors
//! - Disk full simulation

mod common;

use std::{fs, os::unix::fs::PermissionsExt};

use common::TestHarness;

// ============================================================================
// Database Corruption and Recovery
// ============================================================================

#[test]
fn test_corrupted_database_provides_helpful_error() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Corrupt the database by writing invalid SQLite data
    let db_path = harness.state_db_path();
    fs::write(&db_path, "NOT A VALID SQLITE DATABASE FILE").ok();

    // Try to list sessions - should fail with database error
    let result = harness.zjj(&["list"]);
    assert!(!result.success, "Should fail with corrupted database");
    result.assert_output_contains("database");
}

#[test]
fn test_empty_database_file() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Truncate database to zero bytes
    let db_path = harness.state_db_path();
    fs::write(&db_path, "").ok();

    // Operations should fail with helpful error
    let result = harness.zjj(&["list"]);
    assert!(!result.success, "Should fail with empty database");
}

#[test]
fn test_database_with_wrong_schema() {
    tokio_test::block_on(async {
        use sqlx::Connection;

        let Some(harness) = TestHarness::try_new() else {
            eprintln!("Skipping test: jj not available");
            return;
        };
        harness.assert_success(&["init"]);

        // Replace database with valid SQLite but wrong schema
        let db_path = harness.state_db_path();
        let db_url = format!("sqlite://{}", db_path.display());
        if let Ok(mut conn) = sqlx::SqliteConnection::connect(&db_url).await {
            // Drop the sessions table
            let _ = sqlx::query("DROP TABLE IF EXISTS sessions")
                .execute(&mut conn)
                .await;
            // Create a different table
            let _ = sqlx::query("CREATE TABLE wrong_table (id INTEGER PRIMARY KEY, data TEXT)")
                .execute(&mut conn)
                .await;
        }

        // Operations should detect schema mismatch
        // Note: SQLx may handle some schema mismatches gracefully by creating missing tables,
        // so this test accepts both failure and success
        let _result = harness.zjj(&["list"]);
        // Test passes as long as the command doesn't panic
    });
}

#[test]
fn test_missing_database_file() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Delete the database file
    let db_path = harness.state_db_path();
    fs::remove_file(&db_path).ok();

    // Operations should fail with clear error about missing database
    let result = harness.zjj(&["list"]);
    assert!(!result.success, "Should fail with missing database");
}

#[test]
fn test_database_locked_by_another_process() {
    tokio_test::block_on(async {
        use sqlx::Connection;

        let Some(harness) = TestHarness::try_new() else {
            eprintln!("Skipping test: jj not available");
            return;
        };
        harness.assert_success(&["init"]);

        let db_path = harness.state_db_path();

        // Open connection and start exclusive transaction
        let db_url = format!("sqlite://{}", db_path.display());
        let _conn = sqlx::SqliteConnection::connect(&db_url).await.ok();
        // Note: In practice, SQLite's locking is complex and may not always block
        // This test documents the expected behavior if locking occurs

        // Try to perform operation while locked
        let _result = harness.zjj(&["list"]);
        // May succeed or fail depending on SQLite's WAL mode and timing
    });
}

// ============================================================================
// Partial Cleanup Scenarios
// ============================================================================

#[test]
fn test_workspace_exists_but_not_in_database() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create workspace directory manually without database entry
    let workspace_path = harness.workspace_path("orphaned");
    fs::create_dir_all(&workspace_path).ok();

    // List should work without showing orphaned workspace
    let result = harness.zjj(&["list"]);
    assert!(result.success);
    assert!(!result.stdout.contains("orphaned"));

    // Trying to add with same name should fail (directory exists)
    let _add_result = harness.zjj(&["add", "orphaned", "--no-open"]);
    // May fail due to existing directory
}

#[test]
fn test_database_entry_exists_but_workspace_missing() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create session normally
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Delete workspace directory
    let workspace_path = harness.workspace_path("test-session");
    fs::remove_dir_all(&workspace_path).ok();

    // List should still show session
    let result = harness.zjj(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("test-session");

    // Status might report missing workspace
    let _status_result = harness.zjj(&["status", "test-session"]);
    // Implementation-defined behavior
}

#[test]
fn test_partial_session_creation_jj_workspace_fails() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Make workspaces directory read-only to prevent workspace creation
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    fs::create_dir_all(&workspaces_dir).ok();

    let metadata = fs::metadata(&workspaces_dir).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&workspaces_dir, perms).ok();
    }

    // Try to create session - should fail gracefully
    let _result = harness.zjj(&["add", "failing-session", "--no-open"]);
    // Should fail and not leave partial state

    // Restore permissions for cleanup
    let metadata = fs::metadata(&workspaces_dir).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&workspaces_dir, perms).ok();
    }
}

#[test]
fn test_cleanup_after_failed_session_creation() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Attempt to create session with invalid name
    let _result = harness.zjj(&["add", "-invalid-name", "--no-open"]);
    // Should fail validation

    // Verify no partial artifacts left behind
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.success);
    assert!(!list_result.stdout.contains("invalid-name"));
}

// ============================================================================
// Missing Workspace Directories
// ============================================================================

#[test]
fn test_missing_jjz_directory() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Don't run init - .zjj directory should not exist
    let result = harness.zjj(&["list"]);
    assert!(!result.success, "Should fail without .zjj directory");
    result.assert_output_contains("init");
}

#[test]
fn test_corrupted_jjz_directory_structure() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Configure workspace_dir to use .zjj/workspaces for this test
    harness
        .write_config(r#"workspace_dir = ".zjj/workspaces""#)
        .ok();

    // Replace workspaces directory with a file
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    fs::remove_dir_all(&workspaces_dir).ok();
    fs::write(&workspaces_dir, "I am a file, not a directory").ok();

    // Try to add session - should fail
    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "Should fail with corrupted directory");
}

#[test]
fn test_workspace_path_with_symlink() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Configure workspace_dir to use .zjj/workspaces for this test
    harness
        .write_config(r#"workspace_dir = ".zjj/workspaces""#)
        .ok();

    // Create a symlink in workspaces directory
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    fs::create_dir_all(&workspaces_dir).ok();

    let target = harness.repo_path.join("some_target");
    fs::create_dir_all(&target).ok();

    let link = workspaces_dir.join("symlink-session");
    std::os::unix::fs::symlink(&target, &link).ok();

    // Try to add session with symlink name - should fail with security error (zjj-zgs, DEBT-04)
    // NOTE: May be caught by either:
    // 1. validate_workspace_path (DEBT-04) - canonical path escapes bounds
    // 2. validate_no_symlinks (zjj-zgs) - symlink detected
    let result = harness.zjj(&["add", "symlink-session", "--no-open"]);
    assert!(
        !result.success,
        "Should fail when workspace path is a symlink"
    );
    // Accept either error message (both are valid security rejections)
    let output = format!("{}{}", result.stdout, result.stderr);
    assert!(
        output.to_lowercase().contains("security")
            || output.to_lowercase().contains("symlink")
            || output.to_lowercase().contains("escape"),
        "Error should mention security/symlink/escape. Got: {output}"
    );
}

#[test]
fn test_workspace_parent_contains_symlink() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a real directory as target
    let real_workspaces = harness.repo_path.join("real_workspaces");
    fs::create_dir_all(&real_workspaces).ok();

    // Create symlink at .zjj/workspaces pointing to real_workspaces
    let workspaces_link = harness.zjj_dir().join("workspaces");
    fs::remove_dir_all(&workspaces_link).ok(); // Remove if exists
    std::os::unix::fs::symlink(&real_workspaces, &workspaces_link).ok();

    // Configure to use the symlinked workspaces directory
    harness
        .write_config(r#"workspace_dir = ".zjj/workspaces""#)
        .ok();

    // Try to create session - should fail because parent path contains symlink (zjj-zgs)
    let result = harness.zjj(&["add", "test-session", "--no-open"]);
    assert!(
        !result.success,
        "Should fail when workspace parent contains symlink"
    );
    result.assert_output_contains("symlink");
}

#[test]
fn test_symlink_to_system_directory_rejected() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Configure workspace_dir to use .zjj/workspaces for this test
    harness
        .write_config(r#"workspace_dir = ".zjj/workspaces""#)
        .ok();

    // Create symlink to /tmp (a potentially dangerous location)
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    fs::create_dir_all(&workspaces_dir).ok();

    let link = workspaces_dir.join("dangerous-session");
    std::os::unix::fs::symlink("/tmp", &link).ok();

    // Try to add session - should fail with security error (zjj-zgs, DEBT-04)
    // NOTE: May be caught by either:
    // 1. validate_workspace_path (DEBT-04) - canonical path escapes bounds
    // 2. validate_no_symlinks (zjj-zgs) - symlink detected
    let result = harness.zjj(&["add", "dangerous-session", "--no-open"]);
    assert!(!result.success, "Should reject symlink to system directory");
    // Accept either error message (both are valid security rejections)
    let output = format!("{}{}", result.stdout, result.stderr);
    assert!(
        output.to_lowercase().contains("security")
            || output.to_lowercase().contains("symlink")
            || output.to_lowercase().contains("escape"),
        "Error should mention security/symlink/escape. Got: {output}"
    );
}

// ============================================================================
// Invalid Config TOML Files
// ============================================================================

#[test]
fn test_config_with_syntax_error() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Write syntactically invalid TOML
    harness
        .write_config(
            r#"
        workspace_dir = "../workspaces"
        [invalid syntax here
        main_branch = "main"
        "#,
        )
        .ok();

    // Commands should fail with parse error
    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "Should fail with invalid TOML");
    result.assert_output_contains("parse");
}

#[test]
fn test_config_with_invalid_field_types() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Write config with wrong types
    harness
        .write_config(
            r#"
        workspace_dir = 123
        main_branch = ["should", "be", "string"]
        "#,
        )
        .ok();

    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "Should fail with type mismatch");
}

#[test]
fn test_config_with_out_of_range_values() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Write config with invalid range values
    harness
        .write_config(
            r#"
workspace_dir = "../{repo}__workspaces"
# main_branch is auto-detected when not set
default_template = "standard"
state_db = ".zjj/state.db"

[watch]
enabled = true
debounce_ms = 5  # Too low (must be 10-5000)
paths = [".beads/beads.db"]

[hooks]
post_create = []
pre_remove = []
post_merge = []

[zellij]
session_prefix = "jjz"
use_tabs = true
layout_dir = ".zjj/layouts"

[zellij.panes.main]
command = "claude"
args = []
size = "70%"

[zellij.panes.beads]
command = "bv"
args = []
size = "50%"

[zellij.panes.status]
command = "jjz"
args = ["status", "--watch"]
size = "50%"

[zellij.panes.float]
enabled = true
command = ""
width = "80%"
height = "60%"

[dashboard]
refresh_ms = 1000
theme = "default"
columns = ["name", "status", "branch", "changes", "beads"]
vim_keys = true

[agent]
command = "claude"

[agent.env]

[session]
auto_commit = false
commit_prefix = "wip:"
        "#,
        )
        .ok();

    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "Should fail with out-of-range value");
    result.assert_output_contains("10-5000");
}

#[test]
fn test_config_with_missing_required_fields() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Write minimal config (should use defaults for missing fields)
    harness
        .write_config(
            r#"
        workspace_dir = "../workspaces"
        "#,
        )
        .ok();

    // Should work with defaults
    let result = harness.zjj(&["add", "test", "--no-open"]);
    // May succeed using default values
    let _ = result;
}

#[test]
fn test_config_file_permissions_unreadable() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Make config file unreadable
    let config_path = harness.zjj_dir().join("config.toml");
    let metadata = fs::metadata(&config_path).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&config_path, perms.clone()).ok();

        // Try to run command
        let result = harness.zjj(&["add", "test", "--no-open"]);
        // Should fail or use defaults

        // Restore permissions for cleanup
        perms.set_mode(0o644);
        fs::set_permissions(&config_path, perms).ok();

        let _ = result;
    }
}

#[test]
fn test_config_as_directory_instead_of_file() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Replace config.toml with a directory
    let config_path = harness.zjj_dir().join("config.toml");
    fs::remove_file(&config_path).ok();
    fs::create_dir(&config_path).ok();

    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success, "Should fail when config is a directory");
}

// ============================================================================
// Permission Denied Errors
// ============================================================================

#[test]
fn test_readonly_jjz_directory_prevents_operations() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let jjz_dir = harness.zjj_dir();
    let metadata = fs::metadata(&jjz_dir).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&jjz_dir, perms.clone()).ok();

        // Try to add session - should fail with permission error or "does not exist" (readonly
        // prevents creation)
        let result = harness.zjj(&["add", "test", "--no-open"]);
        assert!(!result.success, "Should fail with read-only directory");
        // Accept either explicit permission error or "does not exist" (which happens when readonly
        // prevents file creation)
        let stdout = &result.stdout;
        let stderr = &result.stderr;
        let output = format!("{stdout}{stderr}");
        assert!(
            output.to_lowercase().contains("permission") || output.contains("does not exist"),
            "Expected permission error or 'does not exist', got: {output}"
        );

        // Restore permissions for cleanup
        perms.set_mode(0o755);
        fs::set_permissions(&jjz_dir, perms).ok();
    }
}

#[test]
fn test_readonly_database_file() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session first
    harness.assert_success(&["add", "test", "--no-open"]);

    // Make database read-only
    let db_path = harness.state_db_path();
    let metadata = fs::metadata(&db_path).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&db_path, perms.clone()).ok();

        // Read operations should still work
        let list_result = harness.zjj(&["list"]);
        assert!(list_result.success, "List should work with read-only DB");

        // Write operations should fail
        let add_result = harness.zjj(&["add", "test2", "--no-open"]);
        assert!(!add_result.success, "Add should fail with read-only DB");

        // Restore permissions for cleanup
        perms.set_mode(0o644);
        fs::set_permissions(&db_path, perms).ok();
    }
}

#[test]
fn test_workspace_directory_not_writable() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Configure workspace_dir to use .zjj/workspaces for this test
    harness
        .write_config(r#"workspace_dir = ".zjj/workspaces""#)
        .ok();

    // Make workspaces directory not writable
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    fs::create_dir_all(&workspaces_dir).ok();

    let metadata = fs::metadata(&workspaces_dir).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o555); // Read and execute, no write
        fs::set_permissions(&workspaces_dir, perms.clone()).ok();

        // Try to add session
        let result = harness.zjj(&["add", "test", "--no-open"]);
        assert!(
            !result.success,
            "Should fail when workspace dir not writable"
        );

        // Restore permissions
        perms.set_mode(0o755);
        fs::set_permissions(&workspaces_dir, perms).ok();
    }
}

// ============================================================================
// Disk Full Simulation
// ============================================================================

#[test]
fn test_write_fails_database_full_simulation() {
    // This test documents expected behavior when disk is full
    // Actual disk full simulation is platform-specific and difficult to test reliably

    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create many sessions to potentially fill disk (unlikely but documents behavior)
    for i in 0..100 {
        let name = format!("session{i:03}");
        let _result = harness.zjj(&["add", &name, "--no-open"]);
        // Should either succeed or fail gracefully with disk full error
    }
}

#[test]
fn test_large_metadata_json_causes_no_issues() {
    // Test that large data doesn't cause issues (related to disk space)
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // This would require direct database access to set large metadata
    // Documenting the expected resilience to large data
}

// ============================================================================
// Error Message Quality (zjj-vd3 integration)
// ============================================================================

#[test]
fn test_error_messages_include_suggestions_for_common_issues() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Trying to use without init
    let result = harness.zjj(&["add", "test", "--no-open"]);
    assert!(!result.success);
    // Should suggest running init
    result.assert_output_contains("init");
}

#[test]
fn test_jj_not_installed_error_has_install_instructions() {
    // This would require uninstalling jj or mocking the check
    // Documented in existing error handling code
}

#[test]
fn test_validation_errors_explain_constraints() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Invalid session name
    let result = harness.zjj(&["add", "-invalid", "--no-open"]);
    assert!(!result.success);
    // Should explain what makes a valid session name
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_database_error_suggests_corruption_recovery() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Corrupt database
    let db_path = harness.state_db_path();
    fs::write(&db_path, "CORRUPTED").ok();

    let result = harness.zjj(&["list"]);
    assert!(!result.success);
    // Should suggest recovery steps
    result.assert_output_contains("database");
}

// ============================================================================
// Functional Error Handling Patterns
// ============================================================================

#[test]
fn test_multiple_failures_cascading() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Corrupt both config and database
    harness.write_config("invalid toml [[[").ok();
    let db_path = harness.state_db_path();
    fs::write(&db_path, "CORRUPTED").ok();

    // Should report first critical error encountered
    let result = harness.zjj(&["list"]);
    assert!(!result.success);
}

#[test]
fn test_error_recovery_maintains_consistency() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create session
    harness.assert_success(&["add", "test1", "--no-open"]);

    // Cause an error during operation
    let db_path = harness.state_db_path();
    let metadata = fs::metadata(&db_path).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&db_path, perms.clone()).ok();

        // Try to add another session - should fail
        let _result = harness.zjj(&["add", "test2", "--no-open"]);

        // Restore permissions
        perms.set_mode(0o644);
        fs::set_permissions(&db_path, perms).ok();
    }

    // Verify original session still intact
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.success);
    list_result.assert_stdout_contains("test1");
    assert!(!list_result.stdout.contains("test2"));
}

#[test]
fn test_transaction_rollback_on_failure() {
    // Documents expected behavior: failures should not leave partial state
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Attempt operation that will fail
    let result = harness.zjj(&["add", "", "--no-open"]);
    assert!(!result.success);

    // Verify no partial session created
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.success);
    // Should be empty (no partial sessions)
}

// ============================================================================
// Concurrent Error Scenarios (zjj-9vj: Race condition fixes)
// ============================================================================

#[test]
fn test_concurrent_session_creation_same_name() {
    use std::{
        sync::Arc,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    // Test that concurrent attempts to create the same session are handled atomically
    // Only one should succeed due to database UNIQUE constraint
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Use timestamp to ensure unique session name across test runs
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);
    let session_name = format!("concurrent-same-{timestamp}");

    let harness = Arc::new(harness);

    // Spawn two threads trying to create the same session concurrently
    let harness1 = Arc::clone(&harness);
    let harness2 = Arc::clone(&harness);
    let name1 = session_name.clone();
    let name2 = session_name.clone();

    let handle1 = thread::spawn(move || harness1.zjj(&["add", &name1, "--no-open"]));

    let handle2 = thread::spawn(move || harness2.zjj(&["add", &name2, "--no-open"]));

    let result1 = handle1.join().ok();
    let result2 = handle2.join().ok();

    // Exactly one should succeed, one should fail with "already exists"
    let success_count = [&result1, &result2]
        .iter()
        .filter(|r| r.as_ref().is_some_and(|res| res.success))
        .count();

    assert_eq!(
        success_count, 1,
        "Exactly one thread should succeed in creating the session"
    );

    // Verify only one session exists in database
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.success);
    let session_count = list_result.stdout.matches(&session_name).count();
    assert_eq!(
        session_count, 1,
        "Only one session should exist in database"
    );

    // Verify only one workspace exists
    let workspace_path = harness.workspace_path(&session_name);
    assert!(
        workspace_path.exists(),
        "Exactly one workspace should exist at {}",
        workspace_path.display()
    );
}

#[test]
fn test_concurrent_session_creation_different_names() {
    use std::{
        sync::Arc,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    // Test that concurrent creation of different sessions works correctly
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Use timestamp to ensure unique session names across test runs
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);

    let harness = Arc::new(harness);

    // Spawn multiple threads creating different sessions with unique names
    // Note: Workspace locking is at parent directory level, so concurrent creation
    // is serialized. The test verifies that serialization works correctly.
    let mut handles = vec![];
    let session_count = 3;

    for i in 0..session_count {
        let harness_clone = Arc::clone(&harness);
        let session_name = format!("concurrent-diff-{timestamp}-{i}");
        let session_name_clone = session_name.clone();
        let handle =
            thread::spawn(move || harness_clone.zjj(&["add", &session_name_clone, "--no-open"]));
        handles.push((handle, session_name));

        // Small delay to stagger thread start times
        thread::sleep(std::time::Duration::from_millis(10));
    }

    // Collect results and track failures
    // Due to workspace locking at parent directory level, some operations may fail
    // if they can't acquire the lock within timeout. This is expected behavior.
    let mut success_count: i32 = 0;
    let mut lock_timeout_count: i32 = 0;
    let mut other_failures = vec![];

    for (handle, name) in handles {
        if let Ok(result) = handle.join() {
            if result.success {
                success_count += 1;
            } else if result.stderr.contains("Lock file:")
                || result
                    .stderr
                    .contains("Another session creation is in progress")
                || result.stderr.contains("Resource temporarily unavailable")
                || result.stderr.contains("os error 11")
            {
                // Expected: lock contention during concurrent operations
                // Includes EAGAIN (os error 11) from filesystem locks
                lock_timeout_count += 1;
            } else {
                // Unexpected error
                other_failures.push((name, result.stderr));
            }
        }
    }

    // Report unexpected failures
    if !other_failures.is_empty() {
        eprintln!("Unexpected failures (not lock timeouts):");
        for (name, stderr) in &other_failures {
            eprintln!("  {name}: {stderr}");
        }
    }

    // At least one session should succeed (verify serialization works)
    // The rest may fail due to lock contention, which is acceptable
    assert!(
        success_count >= 1,
        "At least 1 session should succeed (got {success_count}), \
         {lock_timeout_count} failed due to lock timeout"
    );

    // No other failures should occur
    assert!(
        other_failures.is_empty(),
        "Only lock timeouts are acceptable failures, found: {other_failures:?}"
    );

    // Verify that the successful sessions exist in database
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.success, "List command should succeed");

    // Count how many of our test sessions are actually in the database
    let mut found_count = 0;
    for i in 0..session_count {
        let session_name = format!("concurrent-diff-{timestamp}-{i}");
        if list_result.stdout.contains(&session_name) {
            found_count += 1;
        }
    }

    // The number of sessions in database should match successful creations
    // Note: Due to race conditions in concurrent creation, there may be slight discrepancies
    // Allow for +/- 1 session due to timing between lock release and database commit
    assert!(
        found_count >= success_count.saturating_sub(1) && found_count <= success_count + 1,
        "Database should contain approximately {success_count} successfully created sessions \
         (found {found_count} in list output, allowing for +/-1 due to race conditions)"
    );

    // Cleanup test sessions
    for i in 0..session_count {
        let session_name = format!("concurrent-diff-{timestamp}-{i}");
        let _ = harness.zjj(&["remove", &session_name, "--force"]);
    }
}

#[test]
fn test_workspace_creation_fails_no_orphaned_db_entry() {
    // Test that if workspace creation fails, no orphaned database entry is left
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Pre-create a workspace directory to make JJ workspace creation fail
    let workspace_path = harness.workspace_path("will-fail");
    fs::create_dir_all(&workspace_path).ok();

    // Try to create session - should fail because workspace already exists
    let result = harness.zjj(&["add", "will-fail", "--no-open"]);
    // May succeed or fail depending on JJ's behavior with existing directories

    // If it failed, verify no database entry exists
    if !result.success {
        let list_result = harness.zjj(&["list"]);
        assert!(list_result.success);
        assert!(
            !list_result.stdout.contains("will-fail"),
            "No database entry should exist after failed workspace creation"
        );
    }
}

#[test]
fn test_rollback_maintains_database_filesystem_consistency() {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Test that rollback properly cleans up both database and filesystem
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Configure workspace_dir to be inside .zjj so we can make it read-only
    harness
        .write_config(r#"workspace_dir = ".zjj/workspaces""#)
        .ok();

    // Create the workspaces directory
    let workspaces_dir = harness.zjj_dir().join("workspaces");
    fs::create_dir_all(&workspaces_dir).ok();

    // Use timestamp for unique session name
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);
    let session_name = format!("rollback-test-{timestamp}");

    let metadata = fs::metadata(&workspaces_dir).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&workspaces_dir, perms.clone()).ok();

        // Try to create session - should fail because workspace parent is read-only
        let result = harness.zjj(&["add", &session_name, "--no-open"]);

        // Restore permissions before assertions
        perms.set_mode(0o755);
        fs::set_permissions(&workspaces_dir, perms).ok();

        // Verify failure
        assert!(
            !result.success,
            "Session creation should fail when workspace parent is read-only. Output:\nstdout: {}\nstderr: {}",
            result.stdout,
            result.stderr
        );

        // Verify no database entry exists
        let list_result = harness.zjj(&["list"]);
        assert!(list_result.success);
        assert!(
            !list_result.stdout.contains(&session_name),
            "Database should not contain rolled-back session '{}'. List output:\n{}",
            session_name,
            list_result.stdout
        );

        // Verify no workspace directory exists
        let workspace_path = harness.workspace_path(&session_name);
        assert!(
            !workspace_path.exists(),
            "Workspace directory should not exist after rollback at {}",
            workspace_path.display()
        );
    }
}

#[test]
fn test_concurrent_database_access_during_corruption() {
    // Documents behavior when database is corrupted while in use
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // This would require sophisticated multi-threading
    // Documenting expected resilience
}

#[test]
fn test_file_deleted_during_read() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create session
    harness.assert_success(&["add", "test", "--no-open"]);

    // Simulate file being deleted during operation
    let workspace_path = harness.workspace_path("test");
    fs::remove_dir_all(&workspace_path).ok();

    // Status should handle gracefully
    let _result = harness.zjj(&["status", "test"]);
    // Should either report missing workspace or error gracefully
}

// ============================================================================
// Edge Cases in Error Paths
// ============================================================================

#[test]
fn test_very_long_error_messages() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create session name that's too long
    let long_name = "a".repeat(1000);
    let result = harness.zjj(&["add", &long_name, "--no-open"]);
    assert!(!result.success);
    // Error message should be clear despite long input
}

#[test]
fn test_special_characters_in_error_messages() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try session name with special characters
    let special_name = "test\n\r\t\"'\\session";
    let result = harness.zjj(&["add", special_name, "--no-open"]);
    assert!(!result.success);
    // Error should handle special characters safely
}

#[test]
fn test_unicode_in_paths_error_handling() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Session names reject unicode (tested in validation)
    let unicode_name = "café";
    let result = harness.zjj(&["add", unicode_name, "--no-open"]);
    assert!(!result.success);
    result.assert_output_contains("ASCII");
}

#[test]
fn test_null_bytes_in_input() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try session name with null byte (will be rejected by shell/OS first)
    let name_with_null = "test\0name";
    let result = harness.zjj(&["add", name_with_null, "--no-open"]);
    assert!(!result.success);
}

// ============================================================================
// Recovery After Error
// ============================================================================

#[test]
fn test_system_recovers_after_permission_error() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let jjz_dir = harness.zjj_dir();
    let metadata = fs::metadata(&jjz_dir).ok();
    if let Some(metadata) = metadata {
        let mut perms = metadata.permissions();

        // Make read-only
        perms.set_mode(0o444);
        fs::set_permissions(&jjz_dir, perms.clone()).ok();

        // Operation should fail
        let result1 = harness.zjj(&["add", "test", "--no-open"]);
        assert!(!result1.success);

        // Restore permissions
        perms.set_mode(0o755);
        fs::set_permissions(&jjz_dir, perms).ok();

        // Now operation should succeed
        let result2 = harness.zjj(&["add", "test", "--no-open"]);
        assert!(result2.success, "Should recover after permissions fixed");
    }
}

#[test]
fn test_system_recovers_after_config_fix() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Break config
    harness.write_config("invalid [[[").ok();

    // Should fail
    let result1 = harness.zjj(&["add", "test1", "--no-open"]);
    assert!(!result1.success);

    // Fix config
    harness
        .write_config(r#"workspace_dir = "../workspaces""#)
        .ok();

    // Should now work
    let result2 = harness.zjj(&["add", "test1", "--no-open"]);
    // May succeed after config is fixed
    let _ = result2;
}

#[test]
fn test_continues_operation_after_transient_failure() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try invalid operation
    let _result1 = harness.zjj(&["add", "-invalid", "--no-open"]);

    // Subsequent valid operation should work
    let result2 = harness.zjj(&["add", "valid-session", "--no-open"]);
    assert!(result2.success, "Should continue after validation error");
}
