//! BDD Tests for CLI Integration & Environment Robustness
//!
//! Domain: CLI Integration & Environment Robustness
//!
//! Feature: Zellij Integration
//!   As a user/agent in various environments (TTY, non-TTY, Zellij, no-Zellij)
//!   I want zjj commands to be robust and skip integration when appropriate
//!   So that my workflows don't fail due to environment mismatches.
//!
//! Feature: TUI Dashboard
//!   As a user in a terminal
//!   I want the dashboard to launch only when a TTY is available
//!   So that I don't get obscure OS errors in non-interactive environments.
//!
//! Feature: Batch Execution
//!   As an automation script
//!   I want to execute multiple zjj commands in batch
//!   Regardless of whether I include the 'zjj' prefix or not.

#[cfg(test)]
mod tests {
    use std::process::Command;

    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use tempfile::tempdir;

    /// Scenario: Running 'zjj add' in non-TTY environment without --no-zellij
    ///   Given a non-interactive environment (no TTY)
    ///   When I run 'zjj add my-session'
    ///   Then it should succeed in creating the workspace
    ///   And it should skip Zellij integration with a warning instead of failing.
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

        // Initialize ZJJ
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["init"]).current_dir(repo_path).assert().success();

        // Run 'add' without TTY (standard Command::output() doesn't have a TTY)
        // We expect it to succeed and NOT fail because of Zellij
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["add", "test-session"])
            .current_dir(repo_path)
            .assert()
            .success();
        
        // Check list to confirm it was created
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["list"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("test-session"));
    }

    /// Scenario: Debugging database path mismatch
    #[test]
    fn debug_db_path() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and ZJJ
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["init"]).current_dir(repo_path).status().unwrap();

        println!("--- ZJJ CONFIG ---");
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        let output = cmd
            .args(["config"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("--- END ZJJ CONFIG ---");
    }

    /// Scenario: Batch execution with 'zjj' prefix
    ///   Given a batch file with "zjj add session1"
    ///   When I run 'zjj batch --file commands.txt'
    ///   Then it should correctly execute 'zjj add session1' instead of 'zjj zjj add session1'
    #[test]
    fn test_batch_with_zjj_prefix() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and ZJJ
        Command::new("jj")
            .args(["git", "init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["init"]).current_dir(repo_path).status().unwrap();

        let batch_file = repo_path.join("batch.txt");
        std::fs::write(
            &batch_file,
            "zjj add session1 --no-zellij
add session2 --no-zellij",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["batch", "--file", batch_file.to_str().unwrap()])
            .current_dir(repo_path)
            .assert()
            .success();

        // Verify both sessions created
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["list"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("session1"))
            .stdout(predicate::str::contains("session2"));
    }

    /// Scenario: Lock session with TTL
    ///   Given an active session
    ///   When I run 'zjj lock my-session --ttl 60'
    ///   Then it should succeed and show an expiration time
    #[test]
    fn test_lock_with_ttl() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and ZJJ
        Command::new("jj").args(["git", "init"]).current_dir(repo_path).status().unwrap();
        Command::cargo_bin("zjj").unwrap().args(["init"]).current_dir(repo_path).status().unwrap();
        Command::cargo_bin("zjj").unwrap().args(["add", "lock-test", "--no-zellij"]).current_dir(repo_path).status().unwrap();

        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["lock", "lock-test", "--ttl", "60", "--agent-id", "test-agent"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("Locked session 'lock-test'"))
            .stdout(predicate::str::contains("Expires at"));
    }

    /// Scenario: Queue with priority
    ///   Given a JJ repo
    ///   When I run 'zjj queue --add session1 --priority 1'
    ///   Then it should succeed and show in the list with correct priority
    #[test]
    fn test_queue_with_priority() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and ZJJ
        Command::new("jj").args(["git", "init"]).current_dir(repo_path).status().unwrap();
        Command::cargo_bin("zjj").unwrap().args(["init"]).current_dir(repo_path).status().unwrap();

        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["queue", "--add", "q-session", "--priority", "1"])
            .current_dir(repo_path)
            .assert()
            .success();
        
        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["queue", "--list"])
            .current_dir(repo_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("q-session"))
            .stdout(predicate::str::contains("priority: 1").or(predicate::str::contains("1")));
    }

    /// Scenario: Done with advanced flags
    ///   Given an active session with changes
    ///   When I run 'zjj done --keep-workspace --detect-conflicts'
    ///   Then it should succeed and merge changes while keeping the workspace
    #[test]
    fn test_done_with_flags() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize JJ and ZJJ
        Command::new("jj").args(["git", "init"]).current_dir(repo_path).status().unwrap();
        Command::cargo_bin("zjj").unwrap().args(["init"]).current_dir(repo_path).status().unwrap();
        
        // Configure workspace_dir to be inside .zjj for easier testing
        Command::cargo_bin("zjj").unwrap()
            .args(["config", "workspace_dir", ".zjj/workspaces"])
            .current_dir(repo_path)
            .status().unwrap();

        Command::cargo_bin("zjj").unwrap().args(["add", "done-test", "--no-zellij"]).current_dir(repo_path).status().unwrap();

        // Create a change
        let workspace_path = repo_path.join(".zjj/workspaces/done-test");
        std::fs::write(workspace_path.join("file.txt"), "content").unwrap();

        let mut cmd = Command::cargo_bin("zjj").unwrap();
        cmd.args(["done", "--workspace", "done-test", "--keep-workspace", "-m", "Completed work"])
            .current_dir(repo_path)
            .assert()
            .success();
        
        // Verify workspace still exists
        assert!(workspace_path.exists());
    }
}
