use std::{path::PathBuf, time::Duration};

use tokio::time::timeout;
use zjj_core::commands::init::InitLock;

#[tokio::test]
async fn test_init_lock_stale_file_removed() {
    // Create a lock file in a location that doesn't exist
    // This simulates the scenario where a lock file exists but the directory doesn't
    let lock_path = PathBuf::from("/tmp/test_init_lock_location.lock");

    // Create a lock file with a very old modification time
    std::fs::write(&lock_path, "stale lock").expect("Failed to create lock file");
    let mut file = std::fs::File::open(&lock_path).expect("Failed to open lock file");
    file.set_modified(std::time::SystemTime::UNIX_EPOCH)
        .expect("Failed to set modification time");

    // Try to acquire the lock - the stale lock should be removed automatically
    let result = timeout(Duration::from_secs(5), InitLock::acquire(lock_path.clone())).await;

    // The lock should have been removed
    let lock_removed = !std::fs::metadata(&lock_path)
        .ok()
        .map(|m| m.is_file())
        .unwrap_or(true);

    assert!(
        lock_removed,
        "Stale lock file should have been removed but still exists at {}",
        lock_path.display()
    );

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}

#[tokio::test]
async fn test_init_lock_with_missing_directory() {
    // Create a lock file in a directory that doesn't exist
    let lock_path = PathBuf::from("/tmp/nonexistent_dir/.test.lock");

    // Create a lock file
    std::fs::write(&lock_path, "test lock").expect("Failed to create lock file");

    // Try to acquire the lock - should fail because directory doesn't exist
    let result: Result<InitLock, anyhow::Error> =
        timeout(Duration::from_secs(2), InitLock::acquire(lock_path.clone())).await;

    // Should fail with an error
    assert!(
        result.is_err(),
        "Lock acquisition should fail when directory doesn't exist"
    );

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}

#[tokio::test]
async fn test_init_lock_dir_created_before_lock() {
    // This test verifies that the directory is created before the lock is acquired
    // This is the critical flow that the bead is about
    use std::fs;

    let temp_dir = std::env::temp_dir();
    let lock_path = temp_dir.join(".test_init_dir.lock");

    // First, create a lock file
    fs::write(&lock_path, "initial lock").expect("Failed to create lock file");

    // Now simulate the init flow:
    // 1. Create the directory
    fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

    // 2. Try to acquire the lock
    let result: Result<InitLock, anyhow::Error> =
        timeout(Duration::from_secs(2), InitLock::acquire(lock_path.clone())).await;

    // Should succeed because directory now exists
    assert!(
        result.is_ok(),
        "Lock acquisition should succeed when directory exists"
    );

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}
