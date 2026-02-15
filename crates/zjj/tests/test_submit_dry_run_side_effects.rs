#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

mod common;
use common::TestHarness;

#[test]
fn test_submit_dry_run_with_auto_commit_does_not_commit() {
    // GIVEN: An initialized ZJJ repository with uncommitted changes
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "dry-run-side-effect-test", "--no-open"]);

    let workspace_path = harness.workspace_path("dry-run-side-effect-test");
    std::fs::write(workspace_path.join("test.txt"), "initial content")
        .expect("Failed to write test file");

    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Initial"]);
    harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "dry-run-side-effect-test", "-r", "@"],
    );

    // Add uncommitted changes (dirty workspace)
    std::fs::write(workspace_path.join("uncommitted.txt"), "dirty changes")
        .expect("Failed to write uncommitted file");

    harness.current_dir = workspace_path.clone();

    // Verify workspace is dirty
    let status_before = harness.jj_in_dir(&workspace_path, &["status"]);
    println!("JJ STATUS BEFORE:\n{}", status_before.stdout);
    assert!(
        status_before.stdout.contains("Working copy changes:"),
        "Workspace should be dirty initially"
    );

    let bookmarks_before = harness.jj_in_dir(&workspace_path, &["bookmark", "list", "--all"]);
    println!("JJ BOOKMARKS BEFORE:\n{}", bookmarks_before.stdout);

    let change_id_before =
        harness.jj_in_dir(&workspace_path, &["log", "-r", "@", "-T", "change_id"]);
    println!("JJ CHANGE_ID BEFORE: {}", change_id_before.stdout);

    // WHEN: User runs submit with --dry-run AND --auto-commit

    let result = harness.zjj(&["submit", "--dry-run", "--auto-commit", "--json"]);

    // THEN: Command should succeed (or at least report what it WOULD do)
    assert!(result.success, "Submit dry-run should succeed even with dirty workspace if auto-commit is requested\nstdout: {}\nstderr: {}", result.stdout, result.stderr);

    // THEN: Workspace should STILL be dirty (no side effect)
    let status_after = harness.jj_in_dir(&workspace_path, &["status"]);
    assert!(
        status_after.stdout.contains("Working copy changes:"),
        "Workspace should still be dirty after dry-run"
    );
    assert!(
        !status_after
            .stdout
            .contains("wip: auto-commit before submit"),
        "Should not have auto-committed"
    );
}

#[test]
fn test_submit_auto_commit_works_when_not_dry_run() {
    // GIVEN: An initialized ZJJ repository with uncommitted changes
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "auto-commit-test", "--no-open"]);

    let workspace_path = harness.workspace_path("auto-commit-test");
    std::fs::write(workspace_path.join("test.txt"), "initial content")
        .expect("Failed to write test file");

    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Initial"]);
    harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "auto-commit-test", "-r", "@"],
    );

    // Add uncommitted changes (dirty workspace)
    std::fs::write(workspace_path.join("uncommitted.txt"), "dirty changes")
        .expect("Failed to write uncommitted file");

    harness.current_dir = workspace_path.clone();

    // WHEN: User runs submit WITHOUT --dry-run but WITH --auto-commit
    // Note: This might fail if there's no remote, but we care about the auto-commit side effect
    let _ = harness.zjj(&["submit", "--auto-commit", "--json"]);

    // THEN: Workspace should be clean now because it was auto-committed
    let status_after = harness.jj_in_dir(&workspace_path, &["status"]);
    assert!(
        !status_after.stdout.contains("Working copy changes:"),
        "Workspace should be clean after auto-commit"
    );

    // Check for the auto-commit message in log
    let log = harness.jj_in_dir(&workspace_path, &["log", "-r", "@-"]);
    assert!(
        log.stdout.contains("wip: auto-commit before submit"),
        "Should have the auto-commit message"
    );
}

#[test]
fn test_submit_dry_run_fails_if_dirty_without_auto_commit() {
    // GIVEN: An initialized ZJJ repository with uncommitted changes
    let Some(mut harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "dry-run-fail-test", "--no-open"]);

    let workspace_path = harness.workspace_path("dry-run-fail-test");
    std::fs::write(workspace_path.join("test.txt"), "initial content")
        .expect("Failed to write test file");

    harness.jj_in_dir(&workspace_path, &["commit", "-m", "Initial"]);
    harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "dry-run-fail-test", "-r", "@"],
    );

    // Add uncommitted changes (dirty workspace)
    std::fs::write(workspace_path.join("uncommitted.txt"), "dirty changes")
        .expect("Failed to write uncommitted file");

    harness.current_dir = workspace_path.clone();

    // WHEN: User runs submit with --dry-run but WITHOUT --auto-commit
    let result = harness.zjj(&["submit", "--dry-run", "--json"]);

    // THEN: Command should FAIL with DIRTY_WORKSPACE error (bd-34k fix validation)
    assert!(
        !result.success,
        "Submit dry-run should fail if workspace is dirty and auto-commit is not requested\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );
    assert!(
        result.stdout.contains("DIRTY_WORKSPACE"),
        "Output should contain DIRTY_WORKSPACE error code: {}",
        result.stdout
    );
    assert!(
        result.exit_code == Some(3),
        "Exit code should be 3 for precondition failure, got {:?}",
        result.exit_code
    );
}
