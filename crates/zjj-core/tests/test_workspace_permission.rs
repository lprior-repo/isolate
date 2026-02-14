#![cfg_attr(test, allow(clippy::unwrap_used, clippy::uninlined_format_args))]

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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

    // Build a deterministic permission-denied path inside test repo.
    // We create a read-only parent directory and attempt to create a workspace within it.
    let restricted_parent = temp_repo_root.join("restricted");
    tokio::fs::create_dir_all(&restricted_parent).await.unwrap();

    #[cfg(unix)]
    {
        std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o555))
            .unwrap();
    }

    let bad_path: PathBuf = restricted_parent.join(format!("test-workspace-{}", test_id));
    let workspace_name = format!("test-perm-denied-{}", test_id);
    let result = create_workspace_synced(&workspace_name, &bad_path, &temp_repo_root).await;

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

    // Cleanup: restore directory permissions before removing temp repo.
    #[cfg(unix)]
    {
        let _ =
            std::fs::set_permissions(&restricted_parent, std::fs::Permissions::from_mode(0o755));
    }

    let _ = tokio::fs::remove_dir_all(temp_repo_root).await;
}
