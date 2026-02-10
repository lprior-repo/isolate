#![cfg_attr(test, allow(clippy::unwrap_used, clippy::uninlined_format_args))]

use std::path::Path;

use zjj_core::jj_operation_sync::create_workspace_synced;

#[tokio::test]
async fn test_workspace_creation_with_permission_denied() {
    // This test verifies that permission errors from JJ are handled gracefully
    let bad_path = Path::new(
        "/tmp/systemd-private-0123b8fe31e9478eb1644b37442da32c-bolt.service-GGpRnH/test-workspace",
    );
    let repo_root = std::env::current_dir().unwrap();
    let result = create_workspace_synced("test-perm-denied", bad_path, &repo_root).await;

    assert!(result.is_err(), "Should fail with permission denied");

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Error should mention the problem
    assert!(
        err_msg.contains("Permission denied")
            || err_msg.contains("permission")
            || err_msg.contains("JJ"),
        "Error should mention permission or JJ: {err_msg}"
    );

    println!("✓ Test passed: Error handled gracefully: {err_msg}");
}
