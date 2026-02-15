#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
use crate::common::TestHarness;

mod common;

#[tokio::test]
async fn test_remove_dry_run() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Run remove with dry-run
    let result = harness.zjj(&["remove", "test-session", "--dry-run"]);
    assert!(result.success);
    assert!(result
        .stdout
        .contains("Would remove session 'test-session'"));

    // Verify session still exists
    let list_result = harness.zjj(&["list"]);
    assert!(list_result.stdout.contains("test-session"));
}

#[tokio::test]
async fn test_init_dry_run() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    // Do NOT run init first

    // Run init with dry-run
    let result = harness.zjj(&["init", "--dry-run"]);
    assert!(result.success);
    assert!(result.stdout.contains("Would initialize ZJJ"));

    // Verify .zjj directory does NOT exist
    let zjj_dir = harness.zjj_dir();
    assert!(
        !zjj_dir.exists(),
        ".zjj directory should not exist after dry-run"
    );
}

#[tokio::test]
async fn test_sync_dry_run() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a session
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Run sync with dry-run
    let result = harness.zjj(&["sync", "test-session", "--dry-run"]);
    assert!(result.success);
    assert!(result.stdout.contains("Would sync workspace"));
}

#[tokio::test]
async fn test_spawn_dry_run() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Run spawn with dry-run
    let result = harness.zjj(&["spawn", "zjj-123", "--dry-run"]);
    assert!(result.success);
    assert!(result
        .stdout
        .contains("Would spawn session for bead 'zjj-123'"));
}

#[tokio::test]
async fn test_batch_dry_run() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Create a batch file
    let batch_file = harness.current_dir.join("commands.txt");
    tokio::fs::write(&batch_file, "add test-session --no-open")
        .await
        .unwrap();

    // Run batch with dry-run
    let result = harness.zjj(&["batch", "--file", batch_file.to_str().unwrap(), "--dry-run"]);
    assert!(result.success);
    // Adjust assertion based on what implementation decides to print
    assert!(result.stdout.contains("Dry run") || result.stdout.contains("Would execute"));

    // Verify session does NOT exist
    let list_result = harness.zjj(&["list"]);
    assert!(!list_result.stdout.contains("test-session"));
}
