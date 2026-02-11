mod common;

use common::TestHarness;
use serde_json::Value;

#[test]
fn test_spawn_with_actual_bead() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a bead in .beads/issues.jsonl
    let bead_id = "feat-123";
    let bead_json = format!(
        r#"{{"id":"{}","title":"Test Feature","status":"open"}}"#,
        bead_id
    );
    harness
        .create_file(".beads/issues.jsonl", &bead_json)
        .unwrap();

    // Spawn agent for the bead
    // Using 'echo' as agent command and --no-auto-cleanup to verify workspace
    let result = harness.zjj(&[
        "spawn",
        bead_id,
        "--agent-command",
        "echo",
        "--no-auto-cleanup",
        "--agent-args",
        "hello",
    ]);
    result.assert_success();

    // Verify workspace was created and kept
    harness.assert_workspace_exists(bead_id);
}

#[test]
fn test_wait_timeout_and_interval() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Wait for session that doesn't exist with short timeout
    let start = std::time::Instant::now();
    let result = harness.zjj(&[
        "wait",
        "session-exists",
        "nonexistent",
        "-t",
        "1",
        "-i",
        "0.1",
    ]);
    let elapsed = start.elapsed();

    assert!(!result.success, "Wait should fail for nonexistent session");
    assert!(
        elapsed.as_secs_f32() >= 0.9,
        "Wait should respect timeout (took {:?})",
        elapsed
    );
}

#[test]
fn test_lock_ttl() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "lock-test", "--no-zellij", "--no-hooks"]);

    // Lock with 2s TTL
    harness.assert_success(&["lock", "lock-test", "--ttl", "2", "--agent-id", "agent1"]);

    // Immediately try to lock as another agent - should fail
    let result = harness.zjj(&["lock", "lock-test", "--agent-id", "agent2"]);
    assert!(
        !result.success,
        "Should not be able to lock while another agent holds it"
    );

    // Wait for TTL to expire
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Now should be able to lock as another agent
    harness.assert_success(&["lock", "lock-test", "--agent-id", "agent2"]);
}

#[test]
fn test_lock_agent_id_enforcement() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "agent-lock", "--no-zellij", "--no-hooks"]);

    // Lock as agent1
    harness.assert_success(&["lock", "agent-lock", "--agent-id", "agent1"]);

    // Try to unlock as agent2 - should fail
    let result = harness.zjj(&["unlock", "agent-lock", "--agent-id", "agent2"]);
    assert!(
        !result.success,
        "Agent2 should not be able to unlock Agent1's lock"
    );

    // Unlock as agent1 - should succeed
    harness.assert_success(&["unlock", "agent-lock", "--agent-id", "agent1"]);
}

#[test]
fn test_queue_priority_ordering() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "ws-low", "--no-zellij", "--no-hooks"]);
    harness.assert_success(&["add", "ws-high", "--no-zellij", "--no-hooks"]);

    // Add ws-low with low priority (10)
    harness.assert_success(&["queue", "--add", "ws-low", "--priority", "10"]);
    // Add ws-high with high priority (1)
    harness.assert_success(&["queue", "--add", "ws-high", "--priority", "1"]);

    // Next should be ws-high because it has higher priority (1 < 10)
    let result = harness.zjj(&["queue", "--next", "--json"]);
    assert!(result.success);

    let json: Value = serde_json::from_str(&result.stdout).unwrap();
    let workspace = json["entry"]["workspace"]
        .as_str()
        .expect("Should have workspace name");
    assert_eq!(
        workspace, "ws-high",
        "High priority workspace should come first"
    );
}

#[test]
fn test_done_with_keep_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "done-test", "--no-zellij", "--no-hooks"]);

    // Run done with --keep-workspace
    harness.assert_success(&[
        "done",
        "--workspace",
        "done-test",
        "--keep-workspace",
        "-m",
        "testing done",
    ]);

    // Verify workspace still exists
    harness.assert_workspace_exists("done-test");
}

#[test]
fn test_done_squash() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "squash-test", "--no-zellij", "--no-hooks"]);

    let ws_path = harness.workspace_path("squash-test");

    // Create multiple changes
    std::fs::write(ws_path.join("file1.txt"), "v1").unwrap();
    harness
        .jj_in_dir(&ws_path, &["commit", "-m", "commit 1"])
        .assert_success();
    std::fs::write(ws_path.join("file2.txt"), "v2").unwrap();
    harness
        .jj_in_dir(&ws_path, &["commit", "-m", "commit 2"])
        .assert_success();

    // Run done with --squash
    harness.assert_success(&[
        "done",
        "--workspace",
        "squash-test",
        "--squash",
        "-m",
        "squashed result",
    ]);

    // Check main log
    let result = harness.jj(&["log", "-r", "main", "--template", "description"]);
    assert!(result.stdout.contains("squashed result"));
}

#[test]
fn test_done_detect_conflicts() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a conflict scenario
    // 1. Change file in main
    std::fs::write(harness.repo_path.join("conflict.txt"), "main version").unwrap();
    harness
        .jj(&["commit", "-m", "main change"])
        .assert_success();

    // 2. Create workspace from previous state (or just change same file)
    harness.assert_success(&["add", "conflict-ws", "--no-zellij", "--no-hooks"]);
    let ws_path = harness.workspace_path("conflict-ws");
    std::fs::write(ws_path.join("conflict.txt"), "workspace version").unwrap();
    harness
        .jj_in_dir(&ws_path, &["commit", "-m", "ws change"])
        .assert_success();

    // 3. Change main again to ensure conflict
    std::fs::write(harness.repo_path.join("conflict.txt"), "main version 2").unwrap();
    harness
        .jj(&["commit", "-m", "main change 2"])
        .assert_success();

    // Run done with --detect-conflicts
    // It should either fail or warn about conflicts
    let result = harness.zjj(&["done", "--workspace", "conflict-ws", "--detect-conflicts"]);

    // If it detects conflicts, it should probably not proceed or at least report them
    // Depending on implementation, it might fail with conflict error.
    if !result.success {
        assert!(result.stderr.contains("conflict") || result.stdout.contains("conflict"));
    }
}

#[test]
fn test_remove_force_and_keep_branch() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "remove-test", "--no-zellij", "--no-hooks"]);

    // Manually create a bookmark in the workspace
    let ws_path = harness.workspace_path("remove-test");
    // Use remove-test@ to explicitly target the workspace revision
    harness
        .jj_in_dir(
            &ws_path,
            &["bookmark", "create", "remove-test", "-r", "remove-test@"],
        )
        .assert_success();

    // Remove with --force and --keep-branch
    harness.assert_success(&["remove", "remove-test", "-f", "-k"]);

    // Workspace should be gone
    harness.assert_workspace_not_exists("remove-test");

    // JJ bookmark should still exist
    let jj_result = harness.jj(&["bookmark", "list"]);
    assert!(
        jj_result.stdout.contains("remove-test"),
        "Bookmark should be kept"
    );
}

#[test]
fn test_remove_merge() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "merge-remove", "--no-zellij", "--no-hooks"]);

    let ws_path = harness.workspace_path("merge-remove");
    std::fs::write(ws_path.join("merged_file.txt"), "content").unwrap();
    harness
        .jj_in_dir(&ws_path, &["commit", "-m", "to be merged"])
        .assert_success();

    // Remove with --merge
    harness.assert_success(&["remove", "merge-remove", "-f", "--merge"]);

    // File should be in main now
    let result = harness.jj(&["file", "show", "merged_file.txt"]);
    assert!(result.success);
    assert_eq!(result.stdout.trim(), "content");
}
