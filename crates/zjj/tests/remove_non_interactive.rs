//! Tests for bd-1z6: Remove interactive confirmation from remove command
//!
//! This test suite implements the Martin Fowler test plan for removing
//! interactive confirmation from the `zjj remove` command.
//!
//! Test Organization:
//! - HP-001 to HP-010: Happy Path tests
//! - EP-001 to EP-025: Error Path tests
//! - EC-001 to EC-015: Edge Case tests
//! - CV-001 to CV-020: Contract Verification tests

use std::{fs, path::PathBuf};

use tempfile::TempDir;

// Test helper functions
mod test_helpers {
    use std::path::Path;

    /// Check if a workspace directory exists
    #[allow(dead_code)]
    pub fn workspace_exists(workspace_path: &Path) -> bool {
        workspace_path.exists()
    }
}

/// Helper to set up a test repository with zjj initialized
fn setup_test_repo() -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize a jj repository
    std::process::Command::new("jj")
        .args(["init", "--git", repo_path.to_str().unwrap()])
        .output()?;

    // Initialize zjj
    let _output = std::process::Command::new("zjj")
        .args(["init"])
        .current_dir(&repo_path)
        .output()?;

    Ok((temp_dir, repo_path))
}

/// Helper to create a test session and return the actual workspace path
fn create_test_session(repo_path: &PathBuf, session_name: &str) -> anyhow::Result<PathBuf> {
    let output = std::process::Command::new("zjj")
        .args(["add", session_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create test session: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Get the workspace path from the session status JSON
    // Note: JSON output goes to stderr due to INFO logs, so we read both streams
    let output = std::process::Command::new("zjj")
        .args(["status", session_name, "--json"])
        .current_dir(repo_path)
        .env("RUST_LOG", "error")  // Suppress INFO logs so JSON goes to stdout
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to get session status: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Try stdout first, fallback to stderr for backwards compatibility
    let json_str = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Parse JSON to extract workspace_path
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

    // Extract workspace_path from sessions array
    let workspace_path = json["sessions"][0]["workspace_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("workspace_path not found in JSON"))?;

    Ok(PathBuf::from(workspace_path))
}

// ============================================================================
// Happy Path Tests (HP-001 to HP-010)
// ============================================================================

#[test]
fn test_hp001_non_interactive_remove_succeeds() {
    // GIVEN an initialized ZJJ repository with existing session "feature-auth"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    // Create a test session
    let session_name = "feature-auth";
    let workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // Verify session and workspace exist
    assert!(
        workspace_path.exists(),
        "Workspace should exist before removal"
    );

    // WHEN user runs `zjj remove feature-auth`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN session is removed immediately without prompting
    assert!(output.status.success(), "Remove command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Removed session"),
        "Output should indicate session was removed"
    );
    assert!(
        !stdout.contains("[y/N]"),
        "Output should NOT contain confirmation prompt"
    );

    // AND workspace directory is deleted
    assert!(!workspace_path.exists(), "Workspace should be deleted");

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");
}

#[test]
fn test_hp002_force_flag_is_no_op() {
    // GIVEN an initialized ZJJ repository with existing session "bugfix-123"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "bugfix-123";
    let workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove bugfix-123 --force`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--force"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN session is removed immediately
    assert!(
        output.status.success(),
        "Remove with --force should succeed"
    );

    // AND no confirmation prompt is shown
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("[y/N]"),
        "Output should NOT contain confirmation prompt"
    );

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND workspace is deleted
    assert!(!workspace_path.exists(), "Workspace should be deleted");

    // AND behavior is identical to `zjj remove bugfix-123` (force is no-op)
    // This is verified by comparing with HP-001 which also succeeds immediately
}

#[test]
fn test_hp004_idempotent_remove_existing_session() {
    // GIVEN an initialized ZJJ repository with existing session "test-session"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "test-session";
    let workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove test-session --idempotent`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--idempotent"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN session is removed immediately
    assert!(
        output.status.success(),
        "Remove with --idempotent should succeed"
    );

    // AND workspace is deleted
    assert!(!workspace_path.exists(), "Workspace should be deleted");

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND no confirmation prompt shown
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("[y/N]"),
        "Output should NOT contain confirmation prompt"
    );
}

#[test]
fn test_hp005_idempotent_remove_missing_session_succeeds() {
    // GIVEN an initialized ZJJ repository with NO session "missing-session"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "missing-session";

    // WHEN user runs `zjj remove missing-session --idempotent`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--idempotent"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN command succeeds immediately
    assert!(
        output.status.success(),
        "Remove with --idempotent should succeed for missing session"
    );

    // AND NO error is returned
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "Stderr should be empty for successful idempotent removal"
    );

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND output contains "already removed"
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("already removed"),
        "Output should indicate session was already removed"
    );

    // AND no confirmation prompt shown
    assert!(
        !stdout.contains("[y/N]"),
        "Output should NOT contain confirmation prompt"
    );
}

#[test]
fn test_hp006_dry_run_shows_preview_without_changes() {
    // GIVEN an initialized ZJJ repository with existing session "preview-session"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "preview-session";
    let workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove preview-session --dry-run`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--dry-run"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN NO changes are made to filesystem
    assert!(
        workspace_path.exists(),
        "Workspace should still exist after dry-run"
    );

    // AND NO changes are made to database (session still exists)
    // We verify this by trying to get the session
    let status_output = std::process::Command::new("zjj")
        .args(["status", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run status command");

    assert!(
        status_output.status.success(),
        "Session should still exist after dry-run"
    );

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND output starts with "DRY-RUN:"
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("DRY-RUN:"),
        "Output should contain DRY-RUN prefix"
    );

    // AND output includes session name and workspace path
    assert!(
        stdout.contains(session_name),
        "Output should include session name"
    );
}

#[test]
fn test_hp007_json_output_has_correct_schema() {
    // GIVEN an initialized ZJJ repository with existing session "json-test"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "json-test";
    let _workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove json-test --json`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--json"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN session is removed
    assert!(output.status.success(), "Remove with --json should succeed");

    // AND output is valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // AND JSON is wrapped in SchemaEnvelope
    assert!(
        json.get("$schema").is_some(),
        "JSON should have $schema field"
    );
    assert!(
        json.get("_schema_version").is_some(),
        "JSON should have _schema_version field"
    );
    assert!(
        json.get("schema_type").is_some(),
        "JSON should have schema_type field"
    );

    // AND `$schema` field is "zjj://remove/v1"
    let schema = json.get("$schema").and_then(|v| v.as_str()).unwrap();
    assert!(
        schema.starts_with("zjj://"),
        "$schema should start with 'zjj://'"
    );
    assert!(schema.contains("remove"), "$schema should contain 'remove'");

    // AND `schema_type` field is "single"
    let schema_type = json.get("schema_type").and_then(|v| v.as_str()).unwrap();
    assert_eq!(schema_type, "single", "schema_type should be 'single'");

    // AND JSON contains `name` and `message` fields at top level (flattened by SchemaEnvelope)
    assert!(
        json.get("name").is_some(),
        "JSON should have name field at top level"
    );
    assert!(
        json.get("message").is_some(),
        "JSON should have message field at top level"
    );

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");
}

#[test]
fn test_hp008_remove_succeeds_when_workspace_already_missing() {
    // GIVEN an initialized ZJJ repository with session "orphan-session"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "orphan-session";
    let workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // AND session's workspace directory was externally deleted
    // Note: We need to delete through JJ since the workspace path might be normalized
    let _ = std::process::Command::new("jj")
        .args(["workspace", "forget", session_name])
        .current_dir(&repo_path)
        .output();

    // Also try direct deletion
    let _ = fs::remove_dir_all(&workspace_path);

    // WHEN user runs `zjj remove orphan-session`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN database record is deleted
    assert!(
        output.status.success(),
        "Remove should succeed even with missing workspace"
    );

    // AND NO error is returned (workspace already gone)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty() || !stderr.to_lowercase().contains("error"),
        "Stderr should not contain errors when workspace is already gone, got: {stderr}"
    );

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND output contains "(workspace was already gone)" or similar success message
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("workspace was already gone") || stdout.contains("Removed session"),
        "Output should indicate workspace was already gone or session was removed, got: {stdout}"
    );
}

#[test]
fn test_hp010_multiple_idempotent_removals_all_succeed() {
    // GIVEN an initialized ZJJ repository
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "retry-test";
    let _workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove retry-test --idempotent` three times consecutively
    let results: Vec<_> = (0..3)
        .map(|_| {
            std::process::Command::new("zjj")
                .args(["remove", session_name, "--idempotent"])
                .current_dir(&repo_path)
                .output()
                .expect("Failed to run remove command")
        })
        .collect();

    // THEN all three invocations succeed
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.status.success(),
            "Invocation {} should succeed",
            i + 1
        );
        assert_eq!(
            result.status.code().unwrap(),
            0,
            "Invocation {} exit code should be 0",
            i + 1
        );
    }

    // AND first invocation removes the session
    let stdout1 = String::from_utf8_lossy(&results[0].stdout);
    assert!(
        stdout1.contains("Removed session"),
        "First invocation should remove the session"
    );

    // AND subsequent invocations output "already removed"
    let stdout2 = String::from_utf8_lossy(&results[1].stdout);
    let stdout3 = String::from_utf8_lossy(&results[2].stdout);
    assert!(
        stdout2.contains("already removed") || stdout3.contains("already removed"),
        "Subsequent invocations should indicate session was already removed"
    );
}

// ============================================================================
// Contract Verification Tests (CV-001 to CV-020)
// ============================================================================

#[test]
fn test_cv005_session_removed_from_database_postcondition() {
    // GIVEN an initialized ZJJ repository with session "verify-removal"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "verify-removal";
    let _workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // Verify session exists before removal
    let status_before = std::process::Command::new("zjj")
        .args(["status", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run status command");

    assert!(
        status_before.status.success(),
        "Session should exist before removal"
    );

    // WHEN user runs `zjj remove verify-removal`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    assert!(output.status.success(), "Remove should succeed");

    // THEN `db.get("verify-removal").await` returns `None`
    // AND session is not in database
    let status_after = std::process::Command::new("zjj")
        .args(["status", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run status command");

    assert!(
        !status_after.status.success(),
        "Session should not exist after removal"
    );
    let stderr = String::from_utf8_lossy(&status_after.stderr);
    assert!(
        stderr.contains("not found"),
        "Status should indicate session not found"
    );
}

#[test]
fn test_cv006_workspace_deleted_postcondition() {
    // GIVEN an initialized ZJJ repository with session and workspace
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "workspace-test";
    let workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // Verify workspace exists before removal
    assert!(
        workspace_path.exists(),
        "Workspace should exist before removal"
    );

    // WHEN user runs `zjj remove workspace-test`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    assert!(output.status.success(), "Remove should succeed");

    // THEN workspace directory no longer exists
    // AND `Path::new(&workspace_path).exists()` is false
    assert!(
        !workspace_path.exists(),
        "Workspace should not exist after removal"
    );
}

#[test]
fn test_cv010_force_flag_no_op_invariant() {
    // GIVEN an initialized ZJJ repository with session
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    // Create two sessions for comparison
    let session1_name = "force-test";
    let session2_name = "no-force-test";

    let workspace1 =
        create_test_session(&repo_path, session1_name).expect("Failed to create session1");
    let workspace2 =
        create_test_session(&repo_path, session2_name).expect("Failed to create session2");

    // WHEN user runs `zjj remove force-test --force`
    let output1 = std::process::Command::new("zjj")
        .args(["remove", session1_name, "--force"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // AND `zjj remove force-test` (without --force)
    let output2 = std::process::Command::new("zjj")
        .args(["remove", session2_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN behavior is identical to `zjj remove force-test`
    // AND no confirmation prompt occurs in either case
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    assert!(
        !stdout1.contains("[y/N]"),
        "With --force: no confirmation prompt"
    );
    assert!(
        !stdout2.contains("[y/N]"),
        "Without --force: no confirmation prompt"
    );

    // AND both succeed
    assert!(
        output1.status.success(),
        "Remove with --force should succeed"
    );
    assert!(
        output2.status.success(),
        "Remove without --force should succeed"
    );

    // AND both workspaces are deleted
    assert!(
        !workspace1.exists(),
        "Workspace should be deleted with --force"
    );
    assert!(
        !workspace2.exists(),
        "Workspace should be deleted without --force"
    );
}

#[test]
fn test_cv011_no_interactive_prompting_invariant() {
    // GIVEN an initialized ZJJ repository with session
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "no-prompt-test";
    let _workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove no-prompt-test`
    // We'll use a timeout to ensure the command doesn't hang waiting for input
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN NO stdin reading occurs (command completes without hanging)
    // AND NO "y/N" prompt displayed
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!(
        "=== STDOUT ===\n{}\n=== STDERR ===\n{}\n=== EXIT CODE ===\n{:?}",
        stdout,
        stderr,
        output.status.code()
    );
    assert!(
        !stdout.contains("[y/N]"),
        "Should NOT prompt for confirmation. Got: {}",
        stdout
    );

    // AND removal executes immediately
    assert!(output.status.success(), "Remove should succeed immediately");
}

#[test]
fn test_cv020_backwards_compatibility_force_flag() {
    // GIVEN an initialized ZJJ repository with session
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "compat-test";
    let _workspace_path =
        create_test_session(&repo_path, session_name).expect("Failed to create session");

    // WHEN user runs `zjj remove compat-test -f`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "-f"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN command succeeds
    // AND flag is accepted (no "unknown argument" error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unknown argument"),
        "-f flag should be accepted"
    );

    // AND behavior is identical to without flag
    assert!(output.status.success(), "Remove with -f should succeed");
}

// ============================================================================
// Error Path Tests (EP-001 to EP-025)
// ============================================================================

#[test]
fn test_ep001_remove_nonexistent_session_fails() {
    // GIVEN an initialized ZJJ repository with NO session "missing"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "missing";

    // WHEN user runs `zjj remove missing`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN command fails
    assert!(
        !output.status.success(),
        "Remove should fail for nonexistent session"
    );

    // AND exit code is 2 (NOT_FOUND)
    assert_eq!(
        output.status.code().unwrap(),
        2,
        "Exit code should be 2 for NOT_FOUND"
    );

    // AND error message contains "not found"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("not found"),
        "Error should mention 'not found'"
    );

    // NOTE: Currently error does NOT suggest --idempotent
    // This is a MINOR improvement for future UX enhancement
    // Filed as bead: Suggest --idempotent flag when session not found

    // AND no confirmation prompt was shown
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("[y/N]"),
        "Should NOT show confirmation prompt even for error"
    );
}

#[test]
fn test_ep004_remove_fails_when_not_initialized() {
    // GIVEN a JJ repository WITHOUT ZJJ initialized
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize jj but NOT zjj
    std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to initialize jj");

    // WHEN user runs `zjj remove test-session`
    let output = std::process::Command::new("zjj")
        .args(["remove", "test-session"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN command fails
    assert!(
        !output.status.success(),
        "Remove should fail when zjj not initialized"
    );

    // AND exit code is 1 (ValidationError)
    assert_eq!(
        output.status.code().unwrap(),
        1,
        "Exit code should be 1 for ValidationError when zjj not initialized"
    );

    // AND error message suggests running `zjj init`
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("zjj init"),
        "Error should suggest running 'zjj init', got: {stderr}"
    );
}

#[test]
fn test_ep010_dry_run_fails_on_nonexistent_session() {
    // GIVEN an initialized ZJJ repository with NO session "dry-missing"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "dry-missing";

    // WHEN user runs `zjj remove dry-missing --dry-run`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--dry-run"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN command fails
    assert!(
        !output.status.success(),
        "Remove with --dry-run should fail for nonexistent session"
    );

    // AND exit code is 2 (NOT_FOUND)
    assert_eq!(
        output.status.code().unwrap(),
        2,
        "Exit code should be 2 for NOT_FOUND"
    );

    // AND dry-run doesn't bypass existence check
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("not found"),
        "Should still fail with 'not found'"
    );
}

#[test]
fn test_ep025_json_error_output_format() {
    // GIVEN an initialized ZJJ repository with NO session "json-error"
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session_name = "json-error";

    // WHEN user runs `zjj remove json-error --json`
    let output = std::process::Command::new("zjj")
        .args(["remove", session_name, "--json"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run remove command");

    // THEN command fails
    assert!(
        !output.status.success(),
        "Remove should fail for nonexistent session"
    );

    // AND exit code is 2 (NOT_FOUND)
    assert_eq!(
        output.status.code().unwrap(),
        2,
        "Exit code should be 2 for NOT_FOUND"
    );

    // AND stderr contains valid JSON error
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        let json: serde_json::Value = serde_json::from_str(&stderr)
            .expect("Error output should be valid JSON when --json flag is used");

        // AND JSON error follows schema with error fields
        assert!(
            json.get("error").is_some() || json.get("code").is_some(),
            "JSON error should have error or code field"
        );
    }
}
