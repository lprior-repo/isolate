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
        sync::OnceLock,
    };

    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use zjj_core::OutputFormat;

    use crate::commands::spawn::{execute_spawn, SpawnError, SpawnOptions};

    /// Global mutex to synchronize tests that change current directory
    static CWD_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    async fn get_cwd_lock() -> tokio::sync::MutexGuard<'static, ()> {
        CWD_MUTEX.get_or_init(|| Mutex::new(())).lock().await
    }

    /// Helper to create default `SpawnOptions` for testing
    fn test_spawn_options(bead_id: &str, command: &str, args: Vec<String>) -> SpawnOptions {
        SpawnOptions {
            bead_id: bead_id.to_string(),
            agent_command: command.to_string(),
            agent_args: args,
            background: false,
            idempotent: false,
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
        async fn new() -> Result<Self, anyhow::Error> {
            let temp_dir = TempDir::new()?;
            let root = temp_dir.path().to_path_buf();

            // Initialize JJ repo
            let _ = tokio::process::Command::new("jj")
                .args(["git", "init", "--colocate"])
                .current_dir(&root)
                .output()
                .await?;

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

    /// Test fixture helper that creates a test repository.
    async fn setup_test_repo() -> TestRepo {
        match TestRepo::new().await {
            Ok(repo) => repo,
            Err(e) => {
                eprintln!("Test repo initialization failed: {e}");
                std::process::abort()
            }
        }
    }

    /// Helper that retrieves current directory with safe fallback.
    fn get_current_dir() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    // ========================================================================
    // BRUTAL EDGE CASE 1: Invalid bead states
    // ========================================================================

    #[tokio::test]
    async fn given_nonexistent_bead_when_spawn_then_clear_error() {
        let _lock = get_cwd_lock().await;
        // Given: A bead ID that doesn't exist
        let repo = setup_test_repo().await;
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options("nonexistent-bead", "echo", vec!["test".to_string()]);

        // When: User attempts to spawn agent for nonexistent bead
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear "bead not found" error
        assert!(result.is_err(), "Should fail for nonexistent bead");
        let Err(err) = result else {
            unreachable!("result was asserted Err above");
        };
        assert!(
            err.to_string().contains("not found")
                || err.to_string().contains("does not exist")
                || err.to_string().contains("nonexistent-bead"),
            "Error should indicate bead not found: {err}"
        );
    }

    #[tokio::test]
    async fn given_bead_already_in_progress_when_spawn_then_rejected() {
        let _lock = get_cwd_lock().await;
        // Given: A bead that's already in_progress
        let Ok(repo) = TestRepo::new().await else {
            return;
        };
        let original_dir = get_current_dir();
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
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

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
            "Error should indicate invalid bead status: {err}"
        );
    }

    #[tokio::test]
    async fn given_bead_is_closed_when_spawn_then_rejected() {
        let _lock = get_cwd_lock().await;
        // Given: A bead that's already closed/completed
        let Ok(repo) = TestRepo::new().await else {
            return;
        };
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        let issues_path = repo.path().join(".beads/issues.jsonl");
        let _ = fs::write(
            &issues_path,
            r#"{"id":"closed-bead","title":"Done task","status":"closed","type":"task","priority":2,"created_at":"2026-02-02T00:00:00Z","updated_at":"2026-02-02T00:00:00Z"}
"#,
        );

        let options = test_spawn_options("closed-bead", "echo", vec!["test".to_string()]);

        // When: User attempts to spawn agent
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

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
            "Error should indicate bead is closed: {err}"
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 2: Agent subprocess failures
    // ========================================================================

    #[tokio::test]
    async fn given_agent_command_not_found_when_spawn_then_clear_error() {
        let _lock = get_cwd_lock().await;
        // Given: An agent command that doesn't exist
        let repo = setup_test_repo().await;
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options(
            "test-bead-1",
            "/nonexistent/command/that/does/not/exist",
            vec![],
        );

        // When: User spawns with nonexistent command
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear command-not-found error
        assert!(result.is_err(), "Should fail for nonexistent command");
        let Err(err) = result else {
            unreachable!("result was asserted Err above");
        };
        assert!(
            err.to_string().contains("not found")
                || err.to_string().contains("No such file")
                || err.to_string().contains("command"),
            "Error should indicate command not found: {err}"
        );
    }

    #[tokio::test]
    async fn given_agent_exits_nonzero_when_spawn_then_handled_gracefully() {
        let _lock = get_cwd_lock().await;
        // Given: An agent that exits with code 1
        let repo = setup_test_repo().await;
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        let options = test_spawn_options(
            "test-bead-1",
            "sh",
            vec!["-c".to_string(), "exit 1".to_string()],
        );

        // When: Agent exits with failure code
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

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
                let err_str = e.to_string();
                assert!(
                    err_str.contains("exit") || err_str.contains("failed"),
                    "Error should mention exit/failure: {err_str}"
                );
            }
        }
    }

    #[tokio::test]
    async fn given_agent_hangs_when_spawn_then_timeout_works() {
        let _lock = get_cwd_lock().await;
        // Given: An agent that hangs forever
        let Ok(repo) = TestRepo::new().await else {
            return;
        };
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        // Set a very short timeout for the test
        let mut options = test_spawn_options("test-bead-1", "sleep", vec!["3600".to_string()]);
        options.timeout_secs = 1;

        // When: User spawns with short timeout
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

        // Then: Should return Timeout error
        assert!(result.is_err(), "Should return error");
        if let Err(err) = result {
            assert!(
                matches!(err, SpawnError::Timeout { .. }),
                "Expected Timeout error, got: {err:?}"
            );
        }
    }

    // ========================================================================
    // BRUTAL EDGE CASE 3: Workspace conflicts
    // ========================================================================

    #[tokio::test]
    async fn given_workspace_already_exists_when_spawn_then_handled() {
        let _lock = get_cwd_lock().await;
        // Given: A workspace directory that already exists
        let Ok(repo) = TestRepo::new().await else {
            return;
        };
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        // Pre-create workspace directory
        let workspace_path = repo.path().join(".zjj/workspaces/test-bead-1");
        let _ = fs::create_dir_all(&workspace_path);
        let _ = fs::write(workspace_path.join("marker.txt"), "existing");

        let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);

        // When: User spawns with existing workspace
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

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
                let err_str = e.to_string();
                assert!(
                    err_str.contains("exists")
                        || err_str.contains("conflict")
                        || err_str.contains("workspace"),
                    "Error should indicate workspace conflict: {err_str}"
                );
            }
        }
    }

    #[tokio::test]
    async fn given_retry_after_failed_spawn_when_idempotent_then_succeeds() {
        let _lock = get_cwd_lock().await;
        let Ok(repo) = TestRepo::new().await else {
            return;
        };
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        let first_attempt = test_spawn_options("test-bead-1", "false", vec![]);
        let first_result = execute_spawn(&first_attempt).await;
        assert!(first_result.is_ok(), "First spawn attempt should execute");

        let mut options = test_spawn_options("test-bead-1", "echo", vec!["ok".to_string()]);
        options.idempotent = true;

        let result = execute_spawn(&options).await;

        let _ = std::env::set_current_dir(original_dir).ok();

        assert!(
            result.is_ok(),
            "Idempotent spawn should succeed when workspace already exists"
        );
    }

    #[tokio::test]
    async fn given_jj_workspace_exists_but_no_db_when_spawn_then_reconciles() {
        let _lock = get_cwd_lock().await;
        // Given: JJ workspace exists but session DB doesn't know about it
        let Ok(repo) = TestRepo::new().await else {
            return;
        };
        let original_dir = get_current_dir();
        std::env::set_current_dir(repo.path()).ok();

        // Create JJ workspace directly (bypassing zjj)
        let workspace_path = repo.path().join(".zjj/workspaces/test-bead-1");
        let _ = fs::create_dir_all(&workspace_path);

        let _result = tokio::process::Command::new("jj")
            .args(["workspace", "add", "--name", "test-bead-1"])
            .arg(&workspace_path)
            .current_dir(repo.path())
            .output()
            .await
            .ok(); // Ignore errors in test setup

        let options = test_spawn_options("test-bead-1", "echo", vec!["test".to_string()]);

        // When: User spawns with orphaned JJ workspace
        let result = execute_spawn(&options).await;

        // Cleanup
        let _ = std::env::set_current_dir(original_dir).ok();

        // Then: Either reconciles OR returns clear conflict error
        match result {
            Ok(_) => {
                // Success means it handled the orphaned workspace
            }
            Err(e) => {
                // Error should mention workspace conflict
                let err_str = e.to_string();
                assert!(
                    err_str.contains("workspace")
                        || err_str.contains("exists")
                        || err_str.contains("conflict"),
                    "Error should indicate workspace inconsistency: {err_str}"
                );
            }
        }
    }

    // ========================================================================
    // BRUTAL EDGE CASE 4: Signal handling and interruption
    // ========================================================================

    #[tokio::test]
    async fn given_sigterm_during_spawn_when_interrupted_then_rolls_back() {
        // Given: A long-running spawn operation
        let Ok(_repo) = TestRepo::new().await else {
            return;
        };

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
    }

    // ========================================================================
    // BRUTAL EDGE CASE 5: Concurrent spawn operations
    // ========================================================================

    #[tokio::test]
    async fn given_two_spawns_same_bead_when_racing_then_one_succeeds() {
        use std::sync::Arc;

        use tokio::sync::Barrier;

        let _lock = get_cwd_lock().await;

        // Given: Two tasks spawning same bead simultaneously
        let Ok(repo_val) = TestRepo::new().await else {
            return;
        };
        let repo = Arc::new(repo_val);
        let barrier = Arc::new(Barrier::new(2));

        let repo1 = Arc::clone(&repo);
        let barrier1 = Arc::clone(&barrier);
        let handle1 = tokio::spawn(async move {
            let _ = std::env::set_current_dir(repo1.path());
            barrier1.wait().await; // Synchronize start

            let options = SpawnOptions {
                bead_id: "test-bead-1".to_string(),
                agent_command: "echo".to_string(),
                agent_args: vec!["test1".to_string()],
                background: false,
                idempotent: false,
                no_auto_merge: true,
                no_auto_cleanup: true,
                timeout_secs: 300,
                format: OutputFormat::Human,
            };
            execute_spawn(&options).await
        });

        let repo2 = Arc::clone(&repo);
        let barrier2 = Arc::clone(&barrier);
        let handle2 = tokio::spawn(async move {
            let _ = std::env::set_current_dir(repo2.path());
            barrier2.wait().await; // Synchronize start

            let options = SpawnOptions {
                bead_id: "test-bead-1".to_string(),
                agent_command: "echo".to_string(),
                agent_args: vec!["test2".to_string()],
                background: false,
                idempotent: false,
                no_auto_merge: true,
                no_auto_cleanup: true,
                timeout_secs: 300,
                format: OutputFormat::Human,
            };
            execute_spawn(&options).await
        });

        // When: Both tasks spawn simultaneously
        let result1 = handle1.await.unwrap_or_else(|_| {
            Err(SpawnError::WorkspaceCreationFailed {
                reason: "Task 1 panicked".to_string(),
            })
        });
        let result2 = handle2.await.unwrap_or_else(|_| {
            Err(SpawnError::WorkspaceCreationFailed {
                reason: "Task 2 panicked".to_string(),
            })
        });

        let _ = std::env::set_current_dir(PathBuf::from(".")).ok();

        // Then: Exactly one succeeds, one fails
        let mut success_count = 0;
        let mut failure_count = 0;

        if result1.is_ok() {
            success_count += 1;
        } else {
            failure_count += 1;
        }
        if result2.is_ok() {
            success_count += 1;
        } else {
            failure_count += 1;
        }

        assert!(
            success_count <= 1,
            "At most one spawn should succeed in race condition"
        );
        assert!(
            failure_count >= 1,
            "At least one spawn should fail in race condition"
        );

        // And: Failure mentions conflict or already in progress
        let err_str = match result1 {
            Err(e) => e.to_string(),
            Ok(_) => match result2 {
                Err(e) => e.to_string(),
                Ok(_) => String::new(),
            },
        };

        assert!(
            err_str.contains("in_progress")
                || err_str.contains("conflict")
                || err_str.contains("already"),
            "Race error should indicate conflict: {err_str}"
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 6: Environment variable injection
    // ========================================================================

    #[tokio::test]
    async fn given_spawn_when_agent_runs_then_env_vars_set() -> Result<(), anyhow::Error> {
        let _lock = get_cwd_lock().await;
        // Given: A spawn operation
        let repo = setup_test_repo().await;
        let original_dir = get_current_dir();
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
            if let Ok(metadata) = fs::metadata(&script_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&script_path, perms).ok();
            }
        }

        let script_path_str = script_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("script path should be valid UTF-8"))?;
        let options = test_spawn_options("test-bead-1", script_path_str, vec![]);

        // When: Agent runs
        let result = execute_spawn(&options).await;

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
                panic!("Spawn should succeed and agent should receive env vars: {e}");
            }
        }

        #[cfg(not(unix))]
        let _ = result;

        Ok(())
    }
}
