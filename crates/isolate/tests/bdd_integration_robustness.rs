//! ! BDD Tests for CLI Integration & Environment Robustness
//! !
//! ! Domain: CLI Integration & Environment Robustness
//! !
//! ! Feature: Zellij Integration
//! !   As a user/agent in various environments (TTY, non-TTY, Zellij, no-Zellij)
//! !   I want isolate commands to be robust and skip integration when appropriate
//! !   So that my workflows don't fail due to environment mismatches.
//! !
//! ! Feature: Batch Execution
//! !   As an automation script
//! !   I want to execute multiple isolate commands in batch
//! !   Regardless of whether I include the 'isolate' prefix or not.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::process::Command;

    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use tempfile::tempdir;

    /// Scenario: Running 'isolate add' in non-TTY environment
    ///   Given a non-interactive environment (no TTY)
    ///   When I run 'isolate add my-session'
    ///   Then it should succeed in creating the workspace
    #[test]
    fn test_add_robustness_in_non_tty() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a JJ repo
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Initialize Isolate
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["init"]).current_dir(repo_path).assert().success();

        // Run 'add' without TTY (standard Command::output() doesn't have a TTY)
        // We expect it to succeed and NOT fail because of Zellij
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["add", "test-session"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Check list to confirm it was created
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["list"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("test-session"));
    }

    /// Scenario: Database path resolution in config output
    ///   Given a freshly initialized isolate repository
    ///   When I run 'isolate config'
    ///   Then it should show the `state_db` path
    ///   And the path should be relative to .isolate directory
    ///   And the config output should be valid
    #[test]
    fn test_db_path_resolution_in_config() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .assert()
            .success();

        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Get config output
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["config"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("state_db"))
            .stdout(predicate::str::contains(".isolate"));
    }

    /// Scenario: Database file actually exists at resolved path
    ///   Given an initialized isolate repository
    ///   When I run operations that use the database
    ///   Then the state.db file should exist at .isolate/state.db
    ///   And it should be a valid `SQLite` database
    #[test]
    fn test_db_file_exists_at_expected_path() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .assert()
            .success();

        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Create a session to ensure DB is used
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["add", "test-session"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Verify DB file exists
        let db_path = repo_path.join(".isolate").join("state.db");
        assert!(
            db_path.exists(),
            "Database should exist at .isolate/state.db"
        );

        // Verify it's a valid SQLite file (starts with SQLite format header)
        let db_content = std::fs::read(&db_path).unwrap();
        assert!(
            db_content.starts_with(b"SQLite format 3"),
            "Database should be valid SQLite format"
        );
    }

    /// Scenario: Config shows correct `workspace_dir` path
    ///   Given an initialized isolate repository
    ///   When I run 'isolate config `workspace_dir`'
    ///   Then it should show a valid path relative to repo
    ///   And the path should match the default pattern
    #[test]
    fn test_workspace_dir_resolution() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .assert()
            .success();

        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Get workspace_dir config
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["config", "workspace_dir"])
            .current_dir(repo_path)
            .assert()
            .success()
            // Should show a path (either default or configured)
            .stdout(predicate::str::is_empty().not());
    }

    /// Scenario: Batch execution with 'isolate' prefix
    ///   Given a batch file with "isolate add session1"
    ///   When I run 'isolate batch --file commands.txt'
    ///   Then it should correctly execute 'isolate add session1' instead of 'isolate isolate add
    /// session1'
    #[test]
    fn test_batch_with_isolate_prefix() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["init"]).current_dir(repo_path).status().unwrap();

        let batch_file = repo_path.join("batch.txt");
        std::fs::write(
            &batch_file,
            "isolate add session1
add session2",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["batch", "--file", batch_file.to_str().unwrap()])
            .current_dir(repo_path)
            .assert()
            .success();

        // Verify both sessions created
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["list"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("session1"))
            .stdout(predicate::str::contains("session2"));
    }

    /// Scenario: Lock session with TTL
    ///   Given an active session
    ///   When I run 'isolate lock my-session --ttl 60'
    ///   Then it should succeed and show an expiration time
    #[test]
    fn test_lock_with_ttl() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Keep workspace inside repo for deterministic path assertion
        let config_status = Command::cargo_bin("isolate")
            .unwrap()
            .args(["config", "workspace_dir", ".isolate/workspaces"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        assert!(config_status.success(), "failed to set workspace_dir");

        // Configure workspace_dir to be inside .isolate for deterministic assertion
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["config", "workspace_dir", ".isolate/workspaces"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["add", "lock-test"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args([
            "lock",
            "lock-test",
            "--ttl",
            "60",
            "--agent-id",
            "test-agent",
        ])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"session\": \"lock-test\""))
        .stdout(predicate::str::contains("\"locked\": true"))
        .stdout(predicate::str::contains("\"ttl_seconds\": 60"));
    }

    // Scenario: Queue session
    //   Given a JJ repo
    //   When I run 'isolate queue enqueue session1'
    //   Then it should succeed and show in the list
    // #[test]
    // fn test_queue_with_priority() {
    // let temp_dir = tempdir().unwrap();
    // let repo_path = temp_dir.path();
    //
    // Initialize JJ and Isolate
    // Command::new("jj")
    // .args(["git", "init"])
    // .current_dir(repo_path)
    // .status()
    // .unwrap();
    // Command::cargo_bin("isolate")
    // .unwrap()
    // .args(["init"])
    // .current_dir(repo_path)
    // .status()
    // .unwrap();
    //
    // let mut cmd = Command::cargo_bin("isolate").unwrap();
    // cmd.args(["queue", "enqueue", "q-session"])
    // .current_dir(repo_path)
    // .assert()
    // .success();
    //
    // let mut cmd = Command::cargo_bin("isolate").unwrap();
    // cmd.args(["queue", "list"])
    // .current_dir(repo_path)
    // .assert()
    // .success();
    // }

    /// Scenario: Done with advanced flags
    ///   Given an active session with changes
    ///   When I run 'isolate done --keep-workspace --detect-conflicts'
    ///   Then it should succeed and merge changes while keeping the workspace
    #[test]
    fn test_done_with_flags() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Configure workspace_dir to be inside .isolate for easier testing
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["config", "workspace_dir", ".isolate/workspaces"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        Command::cargo_bin("isolate")
            .unwrap()
            .args(["add", "done-test"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Create a change
        let workspace_path = repo_path.join(".isolate/workspaces/done-test");
        std::fs::write(workspace_path.join("file.txt"), "content").unwrap();

        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args([
            "done",
            "--workspace",
            "done-test",
            "--keep-workspace",
            "-m",
            "Completed work",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

        // Verify workspace still exists
        assert!(workspace_path.exists());
    }

    /// Scenario: Spawn with bead ID (failure case to keep workspace)
    #[test]
    fn test_spawn_with_bead_failure_keep() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Add a bead via JSONL
        let beads_dir = repo_path.join(".beads");
        std::fs::create_dir_all(&beads_dir).unwrap();
        let issues_file = beads_dir.join("issues.jsonl");
        let bead_json = r#"{"id": "isolate-123", "title": "Test Bead", "status": "open"}"#;
        std::fs::write(&issues_file, format!("{bead_json}\n")).unwrap();

        // Now spawn with a command that fails
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args([
            "spawn",
            "isolate-123",
            "--no-auto-cleanup",
            "--agent-command",
            "false",
        ])
        .current_dir(repo_path)
        .assert()
        .failure(); // Spawn should return failure because the agent failed

        // Verify workspace still exists because of --no-auto-cleanup
        let workspace_path = repo_path.join(".isolate/workspaces/isolate-123");
        let sibling_workspace_path = repo_path
            .parent()
            .map(|parent| {
                let repo_name = repo_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("repo");
                parent.join(format!("{repo_name}__workspaces/isolate-123"))
            })
            .unwrap_or_else(|| repo_path.join("__missing_parent__/isolate-123"));
        assert!(workspace_path.exists() || sibling_workspace_path.exists());
    }

    /// Scenario: Pause and Resume session
    #[test]
    fn test_pause_resume() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and Isolate
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        Command::cargo_bin("isolate")
            .unwrap()
            .args(["add", "pause-test"])
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Pause
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["pause", "pause-test"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Verify status is paused
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["status", "pause-test", "--json"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("paused"));

        // Resume
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["resume", "pause-test"])
            .current_dir(repo_path)
            .assert()
            .success();

        // Verify status is active
        let mut cmd = Command::cargo_bin("isolate").unwrap();
        cmd.args(["status", "pause-test", "--json"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("active"));
    }
}
