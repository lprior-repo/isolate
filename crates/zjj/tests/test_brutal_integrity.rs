//! Brutal BDD tests for Integrity system
//!
//! Focus: Breaking the integrity validation and repair system.
//! - Deep corruption
//! - Race conditions during repair
//! - Resource exhaustion (file handles, disk space simulations)
//! - Concurrent validation/repair of same workspace

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use tempfile::TempDir;
use tokio::sync::Mutex;
use zjj_core::{
    workspace_integrity::{CorruptionType, IntegrityValidator, RepairExecutor, RepairStrategy},
    OutputFormat,
};

/// Global mutex to synchronize tests that change current directory or touch shared resources
static INTEGRITY_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

async fn get_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    INTEGRITY_TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await
}

struct IntegrityHarness {
    _temp_dir: TempDir,
    root: PathBuf,
    workspaces_root: PathBuf,
}

impl IntegrityHarness {
    async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let workspaces_root = root.join(".zjj/workspaces");
        fs::create_dir_all(&workspaces_root).unwrap();

        Self {
            _temp_dir: temp_dir,
            root,
            workspaces_root,
        }
    }

    fn create_workspace(&self, name: &str) -> PathBuf {
        let ws_path = self.workspaces_root.join(name);
        fs::create_dir_all(&ws_path).unwrap();

        // Valid JJ structure
        let jj_repo = ws_path.join(".jj/repo/op_store");
        fs::create_dir_all(&jj_repo).unwrap();
        fs::write(jj_repo.join("op1"), "data").unwrap();

        ws_path
    }

    fn validator(&self) -> IntegrityValidator {
        IntegrityValidator::new(&self.workspaces_root)
    }

    fn executor(&self) -> RepairExecutor {
        RepairExecutor::new().with_always_backup(true)
    }
}

// ========================================================================
// BDD SCENARIO 1: Deep Nested Corruption
// ========================================================================

#[tokio::test]
async fn scenario_deep_nested_corruption_detection() {
    let _lock = get_test_lock().await;
    // GIVEN: A workspace with deeply nested corruption (multiple issues)
    let harness = IntegrityHarness::new().await;
    let ws_name = "deep-corrupt";
    let ws_path = harness.create_workspace(ws_name);

    // Issue 1: Corrupt JJ directory (empty op_store)
    let op_store = ws_path.join(".jj/repo/op_store");
    for entry in fs::read_dir(&op_store).unwrap() {
        fs::remove_file(entry.unwrap().path()).unwrap();
    }

    // Issue 2: Stale lock file
    let lock_dir = ws_path.join(".jj/working_copy");
    fs::create_dir_all(&lock_dir).unwrap();
    let lock_file = lock_dir.join("lock");
    fs::write(&lock_file, "locked").unwrap();

    // Set lock time to 2 hours ago
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
    filetime::set_file_mtime(&lock_file, filetime::FileTime::from_system_time(past)).unwrap();

    let validator = harness.validator();

    // WHEN: Validation is executed
    let result = validator.validate(ws_name).await.unwrap();

    // THEN: It detects BOTH issues
    assert!(!result.is_valid);
    assert!(
        result.issues.len() >= 2,
        "Expected at least 2 issues, got {}",
        result.issues.len()
    );

    let has_corrupt_jj = result
        .issues
        .iter()
        .any(|i| i.corruption_type == CorruptionType::CorruptedJjDir);
    let has_stale_lock = result
        .issues
        .iter()
        .any(|i| i.corruption_type == CorruptionType::StaleLocks);

    assert!(has_corrupt_jj, "Should detect corrupted JJ dir");
    assert!(has_stale_lock, "Should detect stale lock");
}

// ========================================================================
// BDD SCENARIO 2: Concurrent Repair Race
// ========================================================================

#[tokio::test]
async fn scenario_concurrent_repair_safety() {
    let _lock = get_test_lock().await;
    // GIVEN: A corrupted workspace
    let harness = IntegrityHarness::new().await;
    let ws_name = "race-ws";
    let ws_path = harness.create_workspace(ws_name);

    // Create stale lock
    let lock_file = ws_path.join(".jj/working_copy/lock");
    fs::create_dir_all(lock_file.parent().unwrap()).unwrap();
    fs::write(&lock_file, "locked").unwrap();
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
    filetime::set_file_mtime(&lock_file, filetime::FileTime::from_system_time(past)).unwrap();

    let validator = Arc::new(harness.validator());
    let executor = Arc::new(harness.executor());

    // WHEN: Two repair operations are attempted simultaneously
    let v1 = Arc::clone(&validator);
    let e1 = Arc::clone(&executor);
    let h1 = tokio::spawn(async move {
        let val = v1.validate("race-ws").await.unwrap();
        e1.repair(&val).await
    });

    let v2 = Arc::clone(&validator);
    let e2 = Arc::clone(&executor);
    let h2 = tokio::spawn(async move {
        let val = v2.validate("race-ws").await.unwrap();
        e2.repair(&val).await
    });

    let r1 = h1.await.unwrap();
    let r2 = h2.await.unwrap();

    // THEN: Both should complete without crashing, and at least one should report success
    assert!(
        r1.is_ok() || r2.is_ok(),
        "At least one repair should attempt completion"
    );

    // Final state should be clean
    let final_val = validator.validate("race-ws").await.unwrap();
    assert!(
        final_val.is_valid,
        "Workspace should be valid after concurrent repairs"
    );
}

// ========================================================================
// BDD SCENARIO 3: Repair Failure During Process
// ========================================================================

#[tokio::test]
async fn scenario_repair_failure_roll_forward_protection() {
    let _lock = get_test_lock().await;
    // GIVEN: A workspace where repair will fail (simulated by making file immutable/unwritable)
    let harness = IntegrityHarness::new().await;
    let ws_name = "fail-repair";
    let ws_path = harness.create_workspace(ws_name);

    let lock_file = ws_path.join(".jj/working_copy/lock");
    fs::create_dir_all(lock_file.parent().unwrap()).unwrap();
    fs::write(&lock_file, "locked").unwrap();
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
    filetime::set_file_mtime(&lock_file, filetime::FileTime::from_system_time(past)).unwrap();

    // Make the lock file unremovable by removing write permissions from parent
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(lock_file.parent().unwrap())
            .unwrap()
            .permissions();
        perms.set_mode(0o555); // Read and execute only
        fs::set_permissions(lock_file.parent().unwrap(), perms).unwrap();
    }

    let validator = harness.validator();
    let executor = harness.executor();

    // WHEN: Repair is attempted
    let val = validator.validate(ws_name).await.unwrap();
    let result = executor.repair(&val).await;

    // THEN: It returns a graceful error instead of panicking
    assert!(result.is_err(), "Repair should fail due to permissions");

    // Cleanup permissions for test runner
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(lock_file.parent().unwrap())
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(lock_file.parent().unwrap(), perms).unwrap();
    }
}

// ========================================================================
// BDD SCENARIO 4: Mass Validation Performance & Stability
// ========================================================================

#[tokio::test]
async fn scenario_mass_validation_stability() {
    let _lock = get_test_lock().await;
    // GIVEN: 100 workspaces with mixed integrity states
    let harness = IntegrityHarness::new().await;
    let mut names = Vec::new();

    for i in 0..100 {
        let name = format!("ws-{}", i);
        harness.create_workspace(&name);

        // Every 5th workspace is corrupted
        if i % 5 == 0 {
            let ws_path = harness.workspaces_root.join(&name);
            fs::remove_dir_all(ws_path.join(".jj")).unwrap();
        }
        names.push(name);
    }

    let validator = harness.validator();

    // WHEN: All are validated in parallel
    let results = validator.validate_all(&names).await.unwrap();

    // THEN: All results are returned and accurately reflect state
    assert_eq!(results.len(), 100);
    let invalid_count = results.iter().filter(|r| !r.is_valid).count();
    assert_eq!(invalid_count, 20, "Expected 20 corrupted workspaces");
}
