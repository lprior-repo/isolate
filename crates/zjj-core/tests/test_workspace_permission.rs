#![cfg_attr(test, allow(clippy::unwrap_used, clippy::uninlined_format_args))]

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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
    let repo_root = temp.path().join("repo");

    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|e| panic!("failed to get test timestamp: {e}"))
        .as_nanos();

    // Build a deterministic permission-denied path inside the test repo.
    let restricted_parent = repo_root.join("restricted");
    tokio::fs::create_dir_all(&restricted_parent)
        .await
        .unwrap_or_else(|e| panic!("failed to create restricted parent: {e}"));

    #[cfg(unix)]
    {
        std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o555))
            .unwrap_or_else(|e| panic!("failed to set read-only permissions: {e}"));
    }

    let bad_path: PathBuf = restricted_parent.join(format!("test-workspace-{test_id}"));
    let workspace_name = format!("test-perm-denied-{test_id}");
    let result = create_workspace_synced(&workspace_name, &bad_path, &repo_root).await;

    assert!(result.is_err(), "Should fail with permission denied");

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Error should specifically indicate a permission/access failure.
    let err_lower = err_msg.to_lowercase();
    assert!(
        err_lower.contains("permission denied")
            || err_lower.contains("operation not permitted")
            || err_lower.contains("access is denied"),
        "Error should indicate permission failure, got: {err_msg}"
    );

    println!("✓ Test passed: Error handled gracefully: {err_msg}");

    // Cleanup: restore directory permissions before temp dir drop.
    #[cfg(unix)]
    {
        let _ =
            std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o755));
    }
}
