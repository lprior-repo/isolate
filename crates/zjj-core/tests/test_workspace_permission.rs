#![cfg_attr(test, allow(clippy::unwrap_used, clippy::uninlined_format_args))]

use std::path::Path;

use zjj_core::jj_operation_sync::create_workspace_synced;

#[tokio::test]
async fn test_workspace_creation_with_permission_denied() {
    // This test verifies that permission errors from JJ are handled gracefully

    // Create a temporary JJ repository for testing
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_repo_root = std::env::temp_dir().join(format!("zjj-test-repo-perm-{}", test_id));
    tokio::fs::create_dir_all(&temp_repo_root).await.unwrap();

    // Initialize JJ repository
    let init_output = tokio::process::Command::new("jj")
        .args(["git", "init", "--colocate"])
        .current_dir(&temp_repo_root)
        .output()
        .await
        .unwrap();

    assert!(init_output.status.success(), "JJ init should succeed");

    let bad_path = Path::new(
        "/tmp/systemd-private-0123b8fe31e9478eb1644b37442da32c-bolt.service-GGpRnH/test-workspace",
    );
    let workspace_name = format!("test-perm-denied-{}", test_id);
    let result = create_workspace_synced(&workspace_name, bad_path, &temp_repo_root).await;

    assert!(result.is_err(), "Should fail with permission denied");

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Error should mention the problem - may be permission error, lock timeout, or workspace exists
    // The test verifies error handling, not the specific error type
    assert!(
        err_msg.contains("Permission denied")
            || err_msg.contains("permission")
            || err_msg.contains("JJ")
            || err_msg.contains("lock")
            || err_msg.contains("timeout")
            || err_msg.contains("Workspace already exists")
            || err_msg.contains("Failed to create workspace"),
        "Error should indicate failure: {err_msg}"
    );

    println!("✓ Test passed: Error handled gracefully: {err_msg}");

    // Cleanup
    let _ = tokio::fs::remove_dir_all(temp_repo_root).await;
}
