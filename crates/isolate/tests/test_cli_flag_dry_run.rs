#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

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
    let result = harness.isolate(&["remove", "test-session", "--dry-run"]);
    assert!(result.success);
    assert!(result
        .stdout
        .contains("Would remove session 'test-session'"));

    // Verify session still exists
    let list_result = harness.isolate(&["list"]);
    assert!(list_result.stdout.contains("test-session"));
}

#[tokio::test]
async fn test_init_dry_run() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    // Do NOT run init first

    // Run init with dry-run
    let result = harness.isolate(&["init", "--dry-run"]);
    assert!(result.success);
    assert!(result.stdout.contains("Would initialize Isolate"));

    // Verify .isolate directory does NOT exist
    let isolate_dir = harness.isolate_dir();
    assert!(
        !isolate_dir.exists(),
        ".isolate directory should not exist after dry-run"
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
    let result = harness.isolate(&["sync", "test-session", "--dry-run"]);
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
    let result = harness.isolate(&["spawn", "isolate-123", "--dry-run"]);
    assert!(result.success);
    assert!(
        result
            .stdout
            .contains("Would spawn session for bead 'isolate-123'")
            || result.stdout.contains("\"dry_run\": true")
            || result.stdout.contains("\"would_spawn\": true")
            || result.stdout.contains("\"bead_id\": \"isolate-123\"")
    );
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
    let result = harness.isolate(&["batch", "--file", batch_file.to_str().unwrap(), "--dry-run"]);
    assert!(result.success);
    // Adjust assertion based on what implementation decides to print
    assert!(result.stdout.contains("Dry run") || result.stdout.contains("Would execute"));

    // Verify session does NOT exist
    let list_result = harness.isolate(&["list"]);
    assert!(!list_result.stdout.contains("test-session"));
}
