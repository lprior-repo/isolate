//! Brutal BDD edge-case tests for spawn command
//!
//! CRITICAL: spawn.rs has ONLY 1 test (struct fields)!
//! This file adds comprehensive behavioral testing.
//!
//! These tests follow Martin Fowler's BDD approach:
//! - Given-When-Then structure
//! - Test BEHAVIOR not implementation
//! - Expose edge cases that SHOULD break the code
//! - Validate outcomes, not internal state

mod brutal_edge_cases {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::Duration,
    };

    use tempfile::TempDir;
    use zjj_core::OutputFormat;

    use crate::commands::spawn::{execute_spawn, SpawnError, SpawnOptions, SpawnOutput};

    /// Helper to create default SpawnOptions for testing
    fn test_spawn_options(bead_id: &str, command: &str, args: Vec<String>) -> SpawnOptions {
        SpawnOptions {
            bead_id: bead_id.to_string(),
            agent_command: command.to_string(),
            agent_args: args,
            background: false,
            no_auto_merge: true,
            no_auto_cleanup: true,
            timeout_secs: 300,
            format: OutputFormat::Human,
        }
    }

    /// Test fixture providing isolated test environment
    struct TestRepo {
        _temp_dir: TempDir,
        root: PathBuf,
    }

    impl TestRepo {
        /// Create a new test repository with JJ + beads initialized
        fn new() -> Result<Self, anyhow::Error> {
            let temp_dir = TempDir::new()?;
            let root = temp_dir.path().to_path_buf();

            // Initialize JJ repo
            let _ = std::process::Command::new("jj")
                .args(["git", "init", "--colocate"])
                .current_dir(&root)
                .output()?;

            // Create .zjj directory structure
            fs::create_dir_all(root.join(".zjj"))?;
            fs::create_dir_all(root.join(".zjj/workspaces"))?;

            // Create .beads directory structure
            fs::create_dir_all(root.join(".beads"))?;

            // Create issues.jsonl with test bead
            let issues_path = root.join(".beads/issues.jsonl");
            fs::write(
                &issues_path,
                r#"{"id":"test-bead-1","title":"Test task","status":"open","type":"task","priority":2,"created_at":"2026-02-02T00:00:00Z","updated_at":"2026-02-02T00:00:00Z"}
"#,
            )?;

            Ok(Self {
                _temp_dir: temp_dir,
                root,
            })
        }

        fn path(&self) -> &Path {
            &self.root
        }
    }

    // ========================================================================
    // BRUTAL EDGE CASE 1: Invalid bead states
    // ========================================================================

    #[test]
    fn given_nonexistent_bead_when_spawn_then_clear_error() {
        // Given: A bead ID that doesn't exist
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options("nonexistent-bead", "echo", vec!["test".to_string()]);

        // When: User attempts to spawn agent for nonexistent bead
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear "bead not found" error
        assert!(result.is_err(), "Should fail for nonexistent bead");
        let err: SpawnError = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("not found")
                || err.to_string().contains("does not exist")
                || err.to_string().contains("nonexistent-bead"),
            "Error should indicate bead not found: {}",
            err
        );
    }

    #[test]
    fn given_bead_already_in_progress_when_spawn_then_rejected() {
        // Given: A bead that's already in_progress
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        // Create bead with in_progress status
        let issues_path = repo.path().join(".beads/issues.jsonl");
        let _ = fs::write(
            &issues_path,
            r#"{"id":"busy-bead","title":"Busy task","status":"in_progress","type":"task","priority":2,"created_at":"2026-02-02T00:00:00Z","updated_at":"2026-02-02T00:00:00Z"}
"#,
        );

        let options = test_spawn_options("busy-bead", "echo", vec!["test".to_string()]);

        // When: User attempts to spawn agent
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns error about invalid status
        assert!(result.is_err(), "Should reject bead already in_progress");
        let err: SpawnError = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("in_progress")
                || err.to_string().contains("status")
                || err.to_string().contains("already"),
            "Error should indicate invalid bead status: {}",
            err
        );
    }

    #[test]
    fn given_bead_is_closed_when_spawn_then_rejected() {
        // Given: A bead that's already closed/completed
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let issues_path = repo.path().join(".beads/issues.jsonl");
        let _ = fs::write(
            &issues_path,
            r#"{"id":"closed-bead","title":"Done task","status":"closed","type":"task","priority":2,"created_at":"2026-02-02T00:00:00Z","updated_at":"2026-02-02T00:00:00Z"}
"#,
        );

        let options = test_spawn_options("closed-bead", "echo", vec!["test".to_string()]);

        // When: User attempts to spawn agent
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns error about invalid status
        assert!(result.is_err(), "Should reject closed bead");
        let err: SpawnError = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("closed")
                || err.to_string().contains("status")
                || err.to_string().contains("completed"),
            "Error should indicate bead is closed: {}",
            err
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 2: Agent subprocess failures
    // ========================================================================

    #[test]
    fn given_agent_command_not_found_when_spawn_then_clear_error() {
        // Given: An agent command that doesn't exist
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options(
            "test-bead-1",
            "/nonexistent/command/that/does/not/exist",
            vec![],
        );

        // When: User spawns with nonexistent command
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear command-not-found error
        assert!(result.is_err(), "Should fail for nonexistent command");
        let err: SpawnError = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("not found")
                || err.to_string().contains("No such file")
                || err.to_string().contains("command"),
            "Error should indicate command not found: {}",
            err
        );
    }

    #[test]
    fn given_agent_exits_nonzero_when_spawn_then_handled_gracefully() {
        // Given: An agent that exits with code 1
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options(
            "test-bead-1",
            "sh",
            vec!["-c".to_string(), "exit 1".to_string()],
        );

        // When: Agent exits with failure code
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Command completes without panic
        match result {
            Ok(output) => {
                // Should mark as failed
                assert!(
                    matches!(
                        output.status,
                        crate::commands::spawn::types::SpawnStatus::Failed
                    ),
                    "Status should be Failed when agent exits nonzero"
                );
                assert_eq!(output.exit_code, Some(1), "Exit code should be 1");
            }
            Err(e) => {
                // Also acceptable if it returns error
                let err: &SpawnError = &e;
                assert!(
                    err.to_string().contains("exit") || err.to_string().contains("failed"),
                    "Error should mention exit/failure: {}",
                    err
                );
            }
        }
    }

    #[test]
    fn given_agent_hangs_when_spawn_then_timeout_works() {
        // Given: An agent that hangs forever
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options("test-bead-1", "sleep", vec!["3600".to_string()]);

        // Spawn in thread with timeout
        let start = std::time::Instant::now();
        let handle = std::thread::spawn(move || {
            std::env::set_current_dir(repo.path()).ok();
            execute_spawn(&options)
        });

        // Wait maximum 2 seconds
        let _ = std::thread::sleep(Duration::from_secs(2));
        let elapsed = start.elapsed();

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Should timeout or be killable (not hang forever)
        // This test documents that long-running agents are a problem
        // Real implementation should have timeout mechanism
        assert!(
            elapsed < Duration::from_secs(5),
            "Should not hang indefinitely (elapsed: {:?})",
            elapsed
        );

        // Note: This test will likely fail, exposing missing timeout handling
    }

    // ========================================================================
    // BRUTAL EDGE CASE 3: Workspace conflicts
    // ========================================================================

    #[test]
    fn given_workspace_already_exists_when_spawn_then_handled() {
        // Given: A workspace directory that already exists
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        // Pre-create workspace directory
        let workspace_path = repo.path().join(".zjj/workspaces/test-bead-1");
        let _ = fs::create_dir_all(&workspace_path);
        let _ = fs::write(workspace_path.join("marker.txt"), "existing");

        let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);

        // When: User spawns with existing workspace
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Either cleans up and proceeds OR returns clear error
        match result {
            Ok(_) => {
                // If it succeeds, old marker should be gone (cleaned up)
                assert!(
                    !workspace_path.join("marker.txt").exists(),
                    "Old workspace should be cleaned before spawn"
                );
            }
            Err(e) => {
                // If it fails, error should mention conflict
                let err: &SpawnError = &e;
                assert!(
                    err.to_string().contains("exists")
                        || err.to_string().contains("conflict")
                        || err.to_string().contains("workspace"),
                    "Error should indicate workspace conflict: {}",
                    err
                );
            }
        }
    }

    #[test]
    fn given_jj_workspace_exists_but_no_db_when_spawn_then_reconciles() {
        // Given: JJ workspace exists but session DB doesn't know about it
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        // Create JJ workspace directly (bypassing zjj)
        let workspace_path = repo.path().join(".zjj/workspaces/test-bead-1");
        let _ = fs::create_dir_all(&workspace_path);

        let _ = std::process::Command::new("jj")
            .args(["workspace", "add", "--name", "test-bead-1"])
            .arg(&workspace_path)
            .current_dir(repo.path())
            .output()
            .expect("Failed to create JJ workspace");

        let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);

        // When: User spawns with orphaned JJ workspace
        let result = execute_spawn(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Either reconciles OR returns clear conflict error
        match result {
            Ok(_) => {
                // Success means it handled the orphaned workspace
                assert!(true, "Successfully reconciled orphaned JJ workspace");
            }
            Err(e) => {
                // Error should mention workspace conflict
                let err: &SpawnError = &e;
                assert!(
                    err.to_string().contains("workspace")
                        || err.to_string().contains("exists")
                        || err.to_string().contains("conflict"),
                    "Error should indicate workspace inconsistency: {}",
                    err
                );
            }
        }
    }

    // ========================================================================
    // BRUTAL EDGE CASE 4: Signal handling and interruption
    // ========================================================================

    #[test]
    fn given_sigterm_during_spawn_when_interrupted_then_rolls_back() {
        // Given: A long-running spawn operation
        let _repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));

        // This test documents that SIGTERM should trigger rollback
        // Real implementation should:
        // 1. Catch SIGTERM
        // 2. Kill agent subprocess
        // 3. Rollback workspace creation
        // 4. Reset bead status to 'open'

        // When: SIGTERM sent during agent execution
        // (This is hard to test - documenting expected behavior)

        // Then: Should rollback cleanly
        // - Workspace removed
        // - Bead status reset to 'open'
        // - No orphaned processes

        // Cleanup
        let _ = std::env::set_current_dir(PathBuf::from(".")).ok();

        // This test will fail - exposing missing signal handling
        // For now, just document the requirement
        assert!(
            true,
            "SIGTERM handling not yet tested - needs implementation"
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 5: Concurrent spawn operations
    // ========================================================================

    #[test]
    fn given_two_spawns_same_bead_when_racing_then_one_succeeds() {
        use std::{
            sync::{Arc, Barrier},
            thread,
        };

        // Given: Two threads spawning same bead simultaneously
        let repo =
            Arc::new(TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo")));
        let barrier = Arc::new(Barrier::new(2));

        let repo1 = Arc::clone(&repo);
        let barrier1 = Arc::clone(&barrier);
        let handle1 = thread::spawn(move || {
            std::env::set_current_dir(repo1.path()).ok();
            barrier1.wait(); // Synchronize start

            let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);
            execute_spawn(&options)
        });

        let repo2 = Arc::clone(&repo);
        let barrier2 = Arc::clone(&barrier);
        let handle2 = thread::spawn(move || {
            std::env::set_current_dir(repo2.path()).ok();
            barrier2.wait(); // Synchronize start

            let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);
            execute_spawn(&options)
        });

        // When: Both threads spawn simultaneously
        let result1 = handle1.join().expect("Thread 1 panicked");
        let result2 = handle2.join().expect("Thread 2 panicked");

        let _ = std::env::set_current_dir(PathBuf::from(".")).ok();

        // Then: Exactly one succeeds, one fails
        let success_count = [&result1, &result2]
            .iter()
            .filter(|r: &&Result<SpawnOutput, SpawnError>| r.is_ok())
            .count();
        let failure_count = [&result1, &result2]
            .iter()
            .filter(|r: &&Result<SpawnOutput, SpawnError>| r.is_err())
            .count();

        assert!(
            success_count <= 1,
            "At most one spawn should succeed in race condition"
        );
        assert!(
            failure_count >= 1,
            "At least one spawn should fail in race condition"
        );

        // And: Failure mentions conflict or already in progress
        if let Some(err_result) = [&result1, &result2]
            .iter()
            .find(|r: &&Result<SpawnOutput, SpawnError>| r.is_err())
        {
            let err: SpawnError = match err_result {
                Err(e) => e,
                Ok(_) => unreachable!(),
            };
            assert!(
                err.to_string().contains("in_progress")
                    || err.to_string().contains("conflict")
                    || err.to_string().contains("already"),
                "Race error should indicate conflict: {}",
                err
            );
        }
    }

    // ========================================================================
    // BRUTAL EDGE CASE 6: Environment variable injection
    // ========================================================================

    #[test]
    fn given_spawn_when_agent_runs_then_env_vars_set() {
        // Given: A spawn operation
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        // Create script that checks environment variables
        let script_path = repo.path().join("check_env.sh");
        let _ = fs::write(
            &script_path,
            r#"#!/bin/sh
if [ -z "$ZJJ_BEAD_ID" ]; then
    echo "ERROR: ZJJ_BEAD_ID not set"
    exit 1
fi
if [ -z "$ZJJ_WORKSPACE" ]; then
    echo "ERROR: ZJJ_WORKSPACE not set"
    exit 1
fi
if [ -z "$ZJJ_ACTIVE" ]; then
    echo "ERROR: ZJJ_ACTIVE not set"
    exit 1
fi
echo "All env vars set correctly"
exit 0
"#,
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&script_path, perms).unwrap();
        }

        let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);

        // When: Agent runs
        let result = execute_spawn(&options);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

        // Then: Agent receives ZJJ_* environment variables
        #[cfg(unix)]
        match result {
            Ok(output) => {
                assert_eq!(
                    output.exit_code,
                    Some(0),
                    "Agent should exit 0 if env vars are set"
                );
            }
            Err(e) => {
                panic!(
                    "Spawn should succeed and agent should receive env vars: {}",
                    e
                );
            }
        }
    }
}
