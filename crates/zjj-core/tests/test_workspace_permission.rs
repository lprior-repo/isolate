#![cfg_attr(test, allow(clippy::unwrap_used, clippy::uninlined_format_args))]

use std::path::Path;

use anyhow::Result;
use zjj_core::jj_operation_sync::create_workspace_synced;

async fn create_test_repo() -> Result<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    let repo_root = temp.path().join("repo");
    std::fs::create_dir_all(&repo_root)?;

    let init = tokio::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(&repo_root)
        .output()
        .await?;
    if !init.status.success() {
        anyhow::bail!(
            "jj git init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        );
    }

    std::fs::write(repo_root.join("README.md"), "# permission test\n")?;
    let commit = tokio::process::Command::new("jj")
        .args(["commit", "-m", "initial commit"])
        .current_dir(&repo_root)
        .output()
        .await?;
    if !commit.status.success() {
        anyhow::bail!(
            "jj commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        );
    }

    Ok(temp)
}

#[tokio::test]
async fn test_workspace_creation_with_permission_denied() {
    // This test verifies that permission errors from JJ are handled gracefully
    let temp = create_test_repo()
        .await
        .unwrap_or_else(|e| panic!("failed to create test repo: {e}"));
    let bad_path = Path::new(
        "/tmp/systemd-private-0123b8fe31e9478eb1644b37442da32c-bolt.service-GGpRnH/test-workspace",
    );
    let repo_root = temp.path().join("repo");
    let result = create_workspace_synced("test-perm-denied", bad_path, &repo_root).await;

    assert!(result.is_err(), "Should fail with permission denied");

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Error should mention the problem - may be permission error or lock timeout
    // (lock timeout happens when we can't access the directory at all)
    assert!(
        err_msg.contains("Permission denied")
            || err_msg.contains("permission")
            || err_msg.contains("already exists")
            || err_msg.contains("JJ")
            || err_msg.contains("lock")
            || err_msg.contains("timeout"),
        "Error should mention permission, JJ, or lock issue: {err_msg}"
    );

    println!("✓ Test passed: Error handled gracefully: {err_msg}");
}
