//! Security tests for path validation (DEBT-04)
//!
//! These tests verify that workspace path validation prevents:
//! - Directory traversal attacks using `..` components
//! - Absolute path injection
//! - Symlink-based path escapes
//! - Deep nesting traversal attempts
//! - TOCTOU race conditions
//!
//! All tests use the `TestHarness` to create isolated environments
//! and test the actual CLI integration (not unit tests of private functions).

mod common;

use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;

use common::TestHarness;

/// Test that session names with parent directory components are rejected
/// by session name validation (first line of defense)
#[test]
fn test_reject_parent_directory_in_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Attempt 1: Session name with ../ (rejected by session name validator)
    let result = harness.jjz(&["add", "../../etc", "--no-open"]);
    assert!(
        !result.success,
        "Should reject session name with ../ components"
    );
    assert!(
        result.stderr.contains("Invalid session name")
            || result.stderr.contains("can only contain"),
        "Error should mention invalid session name. Got: {}",
        result.stderr
    );

    // Attempt 2: Session name with single .. (rejected by session name validator)
    let result = harness.jjz(&["add", "..", "--no-open"]);
    assert!(
        !result.success,
        "Should reject session name with .. components"
    );
    assert!(
        result.stderr.contains("Invalid session name")
            || result.stderr.contains("can only contain"),
        "Error should mention invalid session name. Got: {}",
        result.stderr
    );

    // Attempt 3: Session name with / (rejected by session name validator)
    let result = harness.jjz(&["add", "../evil", "--no-open"]);
    assert!(
        !result.success,
        "Should reject session name with / separator"
    );
    assert!(
        result.stderr.contains("Invalid session name")
            || result.stderr.contains("can only contain"),
        "Error should mention invalid session name. Got: {}",
        result.stderr
    );
}

/// Test that `workspace_dir` config with escaping `..` is rejected
/// by path validation (second line of defense - DEBT-04)
#[test]
fn test_reject_workspace_dir_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create malicious config with workspace_dir escaping repo bounds
    // This attempts to create workspaces in /tmp or similar
    let malicious_config = r#"
workspace_dir = "../../../../../../../tmp/evil_workspaces"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#;

    harness.write_config(malicious_config)?;

    // Attempt to create session - should be blocked by validate_workspace_path
    let result = harness.jjz(&["add", "test-session", "--no-open"]);
    assert!(
        !result.success,
        "Should reject workspace_dir with excessive path traversal"
    );
    assert!(
        result.stderr.contains("Security") || result.stderr.contains("escape"),
        "Error should mention security or escape. Got: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("DEBT-04"),
        "Error should cite DEBT-04 requirement. Got: {}",
        result.stderr
    );

    // Verify workspace was NOT created in /tmp
    let tmp_path = std::path::Path::new("/tmp/evil_workspaces/test-session");
    assert!(
        !tmp_path.exists(),
        "Workspace should NOT have been created in /tmp"
    );
    Ok(())
}

/// Test that absolute path injection is rejected
#[test]
fn test_reject_absolute_path_injection() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create malicious config with absolute workspace_dir
    let malicious_config = r#"
workspace_dir = "/tmp/evil_absolute"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#;

    harness.write_config(malicious_config)?;

    // Attempt to create session - should be blocked
    let result = harness.jjz(&["add", "test-session", "--no-open"]);
    assert!(
        !result.success,
        "Should reject absolute path in workspace_dir"
    );
    assert!(
        result.stderr.contains("Security") || result.stderr.contains("escape"),
        "Error should mention security or escape. Got: {}",
        result.stderr
    );

    // Verify workspace was NOT created in /tmp
    let tmp_path = std::path::Path::new("/tmp/evil_absolute/test-session");
    assert!(
        !tmp_path.exists(),
        "Workspace should NOT have been created with absolute path"
    );
    Ok(())
}

/// Test that symlinks in workspace path are detected
#[cfg(unix)]
#[test]
fn test_canonicalization_resolves_symlinks() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create a directory structure with symlink
    let jjz_dir = harness.jjz_dir();
    let real_dir = jjz_dir.join("real_workspaces");
    fs::create_dir_all(&real_dir)?;

    let link_dir = jjz_dir.join("link_workspaces");
    unix_fs::symlink(&real_dir, &link_dir)?;

    // Create config pointing to symlinked directory
    let config_with_symlink = r#"
workspace_dir = ".jjz/link_workspaces"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#;

    harness.write_config(config_with_symlink)?;

    // Attempt to create session - should be blocked by validate_no_symlinks
    let result = harness.jjz(&["add", "test-session", "--no-open"]);
    assert!(
        !result.success,
        "Should reject workspace path containing symlinks"
    );
    assert!(
        result.stderr.contains("symlink"),
        "Error should mention symlink. Got: {}",
        result.stderr
    );
    Ok(())
}

/// Test that valid relative paths with `..` are allowed when they stay within bounds
#[test]
fn test_valid_relative_paths_allowed() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Initialize jjz (uses default config with `../{repo}__workspaces`)
    harness.assert_success(&["init"]);

    // The default config intentionally places workspaces outside the repo
    // using a pattern like "../{repo}__workspaces"
    // This should be ALLOWED because it doesn't escape the parent directory bounds

    // Create a session - should succeed with default config
    let result = harness.jjz(&["add", "valid-session", "--no-open"]);
    assert!(
        result.success,
        "Should allow default config with controlled .. usage. Stderr: {}",
        result.stderr
    );

    // Verify workspace was created in the expected location
    harness.assert_workspace_exists("valid-session");
}

/// Test that deeply nested path traversal is blocked
#[test]
fn test_deeply_nested_traversal_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create config with many ../../../ to try to escape far up the tree
    let deep_traversal_config = r#"
workspace_dir = "../../../../../../../../../../../../../../../../tmp/deep_evil"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#;

    harness.write_config(deep_traversal_config)?;

    // Attempt to create session - should be blocked
    let result = harness.jjz(&["add", "test-session", "--no-open"]);
    assert!(
        !result.success,
        "Should reject deeply nested path traversal"
    );
    assert!(
        result.stderr.contains("Security") || result.stderr.contains("escape"),
        "Error should mention security or escape. Got: {}",
        result.stderr
    );

    // Verify workspace was NOT created in /tmp
    let tmp_path = std::path::Path::new("/tmp/deep_evil/test-session");
    assert!(
        !tmp_path.exists(),
        "Workspace should NOT have been created with deep traversal"
    );
    Ok(())
}

/// Test that TOCTOU attacks are prevented by locking + symlink validation
#[cfg(unix)]
#[test]
fn test_boundary_check_prevents_toctou() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Configure to use .jjz/workspaces
    harness.write_config(r#"workspace_dir = ".jjz/workspaces""#)?;

    // Create workspace_dir as a regular directory first
    let jjz_dir = harness.jjz_dir();
    let workspace_dir = jjz_dir.join("workspaces");
    fs::create_dir_all(&workspace_dir)?;

    // Create a target directory outside repo
    let temp_dir = tempfile::TempDir::new()?;
    let evil_target = temp_dir.path().join("evil");
    fs::create_dir_all(&evil_target)?;

    // Replace workspace_dir with symlink (simulating TOCTOU attack)
    fs::remove_dir(&workspace_dir)?;
    unix_fs::symlink(&evil_target, &workspace_dir)?;

    // Attempt to create session - should be blocked by validate_no_symlinks
    // The locking + validation order prevents TOCTOU
    let result = harness.jjz(&["add", "test-session", "--no-open"]);
    assert!(
        !result.success,
        "Should detect symlink replacement (TOCTOU attack)"
    );
    assert!(
        result.stderr.contains("symlink"),
        "Error should mention symlink. Got: {}",
        result.stderr
    );

    // Verify workspace was NOT created in evil_target
    let evil_workspace = evil_target.join("test-session");
    assert!(
        !evil_workspace.exists(),
        "Workspace should NOT have been created via symlink TOCTOU"
    );
    Ok(())
}

/// Test that `workspace_dir` pointing to repo root is allowed
#[test]
fn test_workspace_dir_at_repo_root_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create config with workspace_dir at repo root (no escaping)
    let root_config = r#"
workspace_dir = "workspaces"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#;

    harness.write_config(root_config)?;

    // Create session - should succeed
    let result = harness.jjz(&["add", "root-session", "--no-open"]);
    assert!(
        result.success,
        "Should allow workspace_dir at repo root. Stderr: {}",
        result.stderr
    );

    // Verify workspace was created at correct location
    let workspace_path = harness.repo_path.join("workspaces/root-session");
    assert!(
        workspace_path.exists(),
        "Workspace should exist at repo root. Path: {}",
        workspace_path.display()
    );
    Ok(())
}

/// Test that `workspace_dir` with single parent escape but staying in bounds is allowed
#[test]
fn test_single_parent_escape_in_bounds_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create config with one level up (this is the default pattern)
    // This goes to parent dir but doesn't escape further
    let parent_config = format!(
        r#"
workspace_dir = "../{}_workspaces"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#,
        harness
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo")
    );

    harness.write_config(&parent_config)?;

    // Create session - should succeed (this is the default behavior)
    let result = harness.jjz(&["add", "parent-session", "--no-open"]);
    assert!(
        result.success,
        "Should allow single parent escape within bounds. Stderr: {}",
        result.stderr
    );

    // Verify workspace was created
    harness.assert_workspace_exists("parent-session");
    Ok(())
}

/// Test error message quality for path traversal attempts
#[test]
fn test_error_message_quality() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return Ok(());
    };

    // Initialize jjz
    harness.assert_success(&["init"]);

    // Create malicious config
    let malicious_config = r#"
workspace_dir = "../../../tmp/evil"

[hooks]
post_create = []

[zellij]
layout_dir = ""
default_template = "standard"
"#;

    harness.write_config(malicious_config)?;

    // Attempt to create session
    let result = harness.jjz(&["add", "test-session", "--no-open"]);
    assert!(!result.success, "Should reject malicious config");

    // Verify error message quality
    let error = result.stderr;
    assert!(
        error.contains("Security"),
        "Error should start with 'Security:'"
    );
    assert!(
        error.contains("DEBT-04"),
        "Error should cite DEBT-04 requirement"
    );
    assert!(
        error.contains("escape")
            || error.contains("excessive")
            || error.contains("parent directory"),
        "Error should mention escape, excessive, or parent directory"
    );
    assert!(
        error.contains("directory traversal") || error.contains("../") || error.contains(".."),
        "Error should mention directory traversal or '..' components"
    );
    assert!(
        error.contains("Suggestions:"),
        "Error should provide actionable suggestions"
    );
    assert!(
        error.contains("workspace_dir"),
        "Error should mention workspace_dir config"
    );
    Ok(())
}
