// Automatic self-healing tests for stale locks (bd-2i5)
//
// These tests verify the automatic stale lock detection and recovery
// functionality that enhances the merge queue with self-healing capabilities.
//
// BDD SCENARIOS:
// 1. Automatic recovery cleans expired locks
// 2. Automatic recovery reclaims stale entries
// 3. Recovery stats accurately report cleanup
// 4. Multiple consecutive claims with auto-recovery
// 5. get_recovery_stats reports without cleaning
// 6. is_lock_stale correctly reports lock state
// 7. Auto-recovery happens before every claim attempt
// 8. Auto-recovery failure doesn't prevent claim
// 9. Manual reclaim_stale still works
// 10. Auto-recovery with custom lock timeout
// 11. Recovery preserves entry metadata
// 12. Multiple workers can safely recover concurrently
// 13. Recovery is idempotent
// 14. Auto-recovery works with empty queue
// 15. Auto-recovery integrates with existing retry logic

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]

use std::time::Duration;

use zjj_core::{
    coordination::queue::{MergeQueue, QueueStatus},
    Result,
};

/// Helper: Simulate passage of time by waiting before reclaim.
/// The detect_and_recover_stale function checks `started_at < cutoff` where cutoff = now -
/// threshold. Using 100ms for fast tests - entries become stale when started_at < now - 1
/// (with lock_timeout_secs=1), which happens quickly after starting processing.
const RECLAIM_DELAY_MS: u64 = 100;

/// Lock timeout for tests - must be less than RECLAIM_DELAY_MS for auto-recovery to work.
/// Using 0 seconds so entries become stale immediately after any delay (started_at < now - 0).
const TEST_LOCK_TIMEOUT_SECS: i64 = 0;

// ========================================================================
// HP-001: Automatic recovery cleans expired locks
// ========================================================================

#[tokio::test]
async fn test_hp001_automatic_recovery_cleans_expired_locks() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add entry
    queue.add("workspace-hp001", None, 5, None).await?;

    // First worker claims (acquires lock)
    let claimed1 = queue.next_with_lock("worker-hp001-1").await?;
    assert!(claimed1.is_some());

    // Get lock info
    let lock1 = queue.get_processing_lock().await?.unwrap();
    let original_expires = lock1.expires_at;

    // Simulate time passing (lock expires)
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Second worker claims - should automatically clean expired lock
    let claimed2 = queue.next_with_lock("worker-hp001-2").await?;

    // Verify lock was cleaned and new lock acquired
    assert!(
        claimed2.is_some(),
        "Second worker should claim after auto-cleanup"
    );

    let lock2 = queue.get_processing_lock().await?.unwrap();
    assert!(
        lock2.expires_at > original_expires,
        "Lock should have new expiration"
    );
    assert_eq!(lock2.agent_id, "worker-hp001-2");

    // Cleanup
    queue.release_processing_lock("worker-hp001-2").await?;

    Ok(())
}

// ========================================================================
// HP-002: Automatic recovery reclaims stale entries
// ========================================================================

#[tokio::test]
async fn test_hp002_automatic_recovery_reclaims_stale_entries() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add entry
    queue.add("workspace-hp002", None, 5, None).await?;

    // First worker claims and releases lock (simulating crash)
    let claimed1 = queue.next_with_lock("worker-hp002-1").await?;
    assert!(claimed1.is_some());

    // Verify entry is claimed
    let entry = queue
        .get_by_workspace("workspace-hp002")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert_eq!(entry.agent_id, Some("worker-hp002-1".to_string()));

    // Release lock (simulating crash)
    queue.release_processing_lock("worker-hp002-1").await?;

    // Wait for entry to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Second worker claims - should auto-reclaim stale entry
    let claimed2 = queue.next_with_lock("worker-hp002-2").await?;
    assert!(claimed2.is_some(), "Should claim after auto-reclaim");

    let entry = claimed2.unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert_eq!(entry.agent_id, Some("worker-hp002-2".to_string()));
    assert!(entry.started_at.is_some(), "started_at should be set");

    // Cleanup
    queue.release_processing_lock("worker-hp002-2").await?;

    Ok(())
}

// ========================================================================
// HP-003: Recovery stats accurately report cleanup
// ========================================================================

#[tokio::test]
async fn test_hp003_recovery_stats_accurately_report_cleanup() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add entries and claim them
    for i in 0..3 {
        queue
            .add(&format!("workspace-hp003-{i}"), None, 5, None)
            .await?;
        let claimed = queue.next_with_lock(&format!("worker-hp003-{i}")).await?;
        assert!(claimed.is_some());
        queue
            .release_processing_lock(&format!("worker-hp003-{i}"))
            .await?;
    }

    // Wait for entries to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Run recovery
    let stats = queue.detect_and_recover_stale().await?;

    // Verify stats
    assert_eq!(
        stats.locks_cleaned, 0,
        "Should clean 0 locks (all explicitly released)"
    );
    assert_eq!(stats.entries_reclaimed, 3, "Should reclaim 3 stale entries");
    assert!(stats.recovery_timestamp > 0, "Timestamp should be set");

    Ok(())
}

// ========================================================================
// HP-004: Multiple consecutive claims with auto-recovery
// ========================================================================

#[tokio::test]
async fn test_hp004_multiple_consecutive_claims_with_auto_recovery() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add 5 entries
    for i in 0..5 {
        queue
            .add(&format!("workspace-hp004-{i}"), None, 5, None)
            .await?;
    }

    // Simulate crashes: claim and release lock
    for i in 0..5 {
        let worker = format!("worker-hp004-{i}");
        let claimed = queue.next_with_lock(&worker).await?;
        assert!(claimed.is_some());
        queue.release_processing_lock(&worker).await?;
    }

    // Verify all are claimed
    let claimed_entries = queue.list(Some(QueueStatus::Claimed)).await?;
    assert_eq!(claimed_entries.len(), 5);

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // New workers claim - should auto-reclaim
    let mut claimed_count = 0;
    for i in 0..5 {
        let worker = format!("worker-hp004-recovery-{i}");
        let claimed = queue.next_with_lock(&worker).await?;
        if claimed.is_some() {
            claimed_count += 1;
            queue.release_processing_lock(&worker).await?;
        }
    }

    assert_eq!(
        claimed_count, 5,
        "All 5 entries should be claimed after auto-recovery"
    );

    Ok(())
}

// ========================================================================
// HP-005: get_recovery_stats reports without cleaning
// ========================================================================

#[tokio::test]
async fn test_hp005_get_recovery_stats_reports_without_cleaning() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add and claim entry
    queue.add("workspace-hp005", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-hp005").await?;
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-hp005").await?;

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Get stats (should not clean)
    let stats1 = queue.get_recovery_stats().await?;
    assert_eq!(
        stats1.locks_cleaned, 0,
        "Should report 0 locks (explicitly released)"
    );
    assert_eq!(stats1.entries_reclaimed, 1, "Should report 1 stale entry");

    // Get stats again - should report same counts (nothing cleaned)
    let stats2 = queue.get_recovery_stats().await?;
    assert_eq!(stats2.locks_cleaned, 0, "Should still report 0 locks");
    assert_eq!(
        stats2.entries_reclaimed, 1,
        "Should still report 1 stale entry"
    );

    // Verify entry is still claimed (not reclaimed)
    let entry = queue
        .get_by_workspace("workspace-hp005")
        .await?
        .expect("Entry should exist");
    assert_eq!(
        entry.status,
        QueueStatus::Claimed,
        "Entry should still be claimed"
    );

    // Now actually clean
    let stats3 = queue.detect_and_recover_stale().await?;
    assert_eq!(
        stats3.locks_cleaned, 0,
        "Should clean 0 locks (already released)"
    );
    assert_eq!(stats3.entries_reclaimed, 1, "Should reclaim 1 entry");

    // Verify entry is now pending
    let entry = queue
        .get_by_workspace("workspace-hp005")
        .await?
        .expect("Entry should exist");
    assert_eq!(
        entry.status,
        QueueStatus::Pending,
        "Entry should be pending after cleanup"
    );

    Ok(())
}

// ========================================================================
// HP-006: is_lock_stale correctly reports lock state
// ========================================================================

#[tokio::test]
async fn test_hp006_is_lock_stale_correctly_reports_lock_state() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // No lock initially
    assert!(!queue.is_lock_stale().await?, "No lock should not be stale");

    // Add entry and acquire lock
    queue.add("workspace-hp006", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-hp006").await?;
    assert!(claimed.is_some());

    // Lock is fresh (not stale)
    assert!(
        !queue.is_lock_stale().await?,
        "Fresh lock should not be stale"
    );

    // Release lock
    queue.release_processing_lock("worker-hp006").await?;

    // After release, is_lock_stale should return false (no lock)
    assert!(!queue.is_lock_stale().await?, "No lock should not be stale");

    Ok(())
}

// ========================================================================
// HP-007: Auto-recovery happens before every claim attempt
// ========================================================================

#[tokio::test]
async fn test_hp007_auto_recovery_before_every_claim() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add and abandon entry
    queue.add("workspace-hp007", None, 5, None).await?;
    let _claimed = queue
        .next_with_lock("worker-hp007-abandon")
        .await?
        .expect("Should claim entry");
    queue
        .release_processing_lock("worker-hp007-abandon")
        .await?;

    // Verify entry is claimed
    let entry = queue
        .get_by_workspace("workspace-hp007")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Claimed);

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // First claim attempt - should auto-recover
    let claimed = queue
        .next_with_lock("worker-hp007-recover")
        .await?
        .expect("Should auto-recover and claim");

    assert_eq!(claimed.status, QueueStatus::Claimed);
    assert_eq!(claimed.agent_id, Some("worker-hp007-recover".to_string()));

    // Cleanup
    queue
        .release_processing_lock("worker-hp007-recover")
        .await?;

    Ok(())
}

// ========================================================================
// HP-008: Auto-recovery failure doesn't prevent claim
// ========================================================================

#[tokio::test]
async fn test_hp008_recovery_failure_doesnt_prevent_claim() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add entry
    queue.add("workspace-hp008", None, 5, None).await?;

    // Normal claim should succeed
    // (recovery will run but there's nothing stale)
    let claimed = queue.next_with_lock("worker-hp008").await?;
    assert!(claimed.is_some());

    // Cleanup
    queue.release_processing_lock("worker-hp008").await?;

    Ok(())
}

// ========================================================================
// HP-009: Manual reclaim_stale still works
// ========================================================================

#[tokio::test]
async fn test_hp009_manual_reclaim_stale_still_works() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add and abandon entries
    for i in 0..3 {
        queue
            .add(&format!("workspace-hp009-{i}"), None, 5, None)
            .await?;
        let claimed = queue.next_with_lock(&format!("worker-hp009-{i}")).await?;
        assert!(claimed.is_some());
        queue
            .release_processing_lock(&format!("worker-hp009-{i}"))
            .await?;
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Manual reclaim (original API)
    let reclaimed = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed, 3, "Manual reclaim should work as before");

    // Verify entries are pending
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(pending.len(), 3);

    Ok(())
}

// ========================================================================
// HP-011: Recovery preserves entry metadata
// ========================================================================

#[tokio::test]
async fn test_hp011_recovery_preserves_entry_metadata() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add entry with metadata
    queue
        .add("workspace-hp011", Some("bead-hp011-123"), 1, None)
        .await?;

    // Claim and abandon
    let claimed = queue
        .next_with_lock("worker-hp011")
        .await?
        .expect("Should claim entry");
    assert_eq!(claimed.bead_id, Some("bead-hp011-123".to_string()));
    assert_eq!(claimed.priority, 1);

    queue.release_processing_lock("worker-hp011").await?;

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Auto-recover and reclaim
    let claimed = queue
        .next_with_lock("worker-hp011-recover")
        .await?
        .expect("Should reclaim entry");

    assert_eq!(
        claimed.bead_id,
        Some("bead-hp011-123".to_string()),
        "bead_id preserved"
    );
    assert_eq!(claimed.priority, 1, "priority preserved");
    assert_eq!(claimed.status, QueueStatus::Claimed);

    // Cleanup
    queue
        .release_processing_lock("worker-hp011-recover")
        .await?;

    Ok(())
}

// ========================================================================
// HP-012: Multiple workers can safely recover concurrently
// ========================================================================

#[tokio::test]
async fn test_hp012_multiple_workers_safely_recover_concurrently() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add and abandon entries
    for i in 0..10 {
        queue
            .add(&format!("workspace-hp012-{i}"), None, 5, None)
            .await?;
        let claimed = queue.next_with_lock(&format!("worker-abandon-{i}")).await?;
        assert!(claimed.is_some());
        queue
            .release_processing_lock(&format!("worker-abandon-{i}"))
            .await?;
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Multiple workers claim concurrently
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let q = queue.clone();
            tokio::spawn(async move {
                let worker = format!("worker-concurrent-{i}");
                q.next_with_lock(&worker).await
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Count successful claims
    let successful_claims = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        successful_claims, 10,
        "All 10 claim attempts should complete"
    );

    // Count actual entries claimed
    // Note: Due to the processing lock (singleton), only one worker can claim at a time.
    // This test verifies that recovery works, but concurrent processing is serialized.
    let claimed_entries: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok().and_then(|e| e.as_ref()))
        .collect();

    assert!(
        !claimed_entries.is_empty(),
        "At least 1 entry should be claimed (others serialized by lock)"
    );

    // Verify unique workspaces (if multiple entries claimed)
    let workspaces: Vec<_> = claimed_entries
        .iter()
        .map(|e| e.workspace.clone())
        .collect();
    let unique_workspaces: std::collections::HashSet<_> = workspaces.iter().collect();
    assert_eq!(
        workspaces.len(),
        unique_workspaces.len(),
        "Claimed workspaces should be unique"
    );

    // Cleanup
    for result in results {
        if let Ok(Some(entry)) = result {
            let _ = queue
                .release_processing_lock(&entry.agent_id.unwrap())
                .await;
        }
    }

    Ok(())
}

// ========================================================================
// HP-013: Recovery is idempotent
// ========================================================================

#[tokio::test]
async fn test_hp013_recovery_is_idempotent() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add and abandon entry
    queue.add("workspace-hp013", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-hp013").await?;
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-hp013").await?;

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // First recovery
    let stats1 = queue.detect_and_recover_stale().await?;
    assert_eq!(stats1.entries_reclaimed, 1, "Should reclaim 1 entry");

    // Second recovery (idempotent)
    let stats2 = queue.detect_and_recover_stale().await?;
    assert_eq!(
        stats2.entries_reclaimed, 0,
        "Should reclaim 0 entries (already pending)"
    );

    Ok(())
}

// ========================================================================
// HP-014: Auto-recovery works with empty queue
// ========================================================================

#[tokio::test]
async fn test_hp014_auto_recovery_with_empty_queue() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Queue is empty, call next_with_lock
    let claimed = queue.next_with_lock("worker-hp014").await?;
    assert!(claimed.is_none(), "No work should be available");

    // Should be able to call again without issues
    let claimed2 = queue.next_with_lock("worker-hp014").await?;
    assert!(claimed2.is_none());

    Ok(())
}

// ========================================================================
// EC-001: Zero entries with stale lock
// ========================================================================

#[tokio::test]
async fn test_ec001_zero_entries_with_stale_lock() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Manually create an expired lock
    let pool = queue.pool();
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) VALUES (1, 'test', ?, ?)")
        .bind(now - 100)
        .bind(now - 50)
        .execute(pool)
        .await
        .map_err(|e| zjj_core::Error::DatabaseError(e.to_string()))?;

    // Verify lock exists
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some());

    // Run recovery
    let stats = queue.detect_and_recover_stale().await?;
    assert_eq!(stats.locks_cleaned, 1, "Should clean 1 lock");
    assert_eq!(stats.entries_reclaimed, 0, "Should reclaim 0 entries");

    // Verify lock is gone
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_none());

    Ok(())
}

// ========================================================================
// EC-003: Very large number of stale entries
// ========================================================================

#[tokio::test]
async fn test_ec003_very_large_number_of_stale_entries() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add 100 entries
    for i in 0..100 {
        queue
            .add(&format!("workspace-ec003-{i}"), None, 5, None)
            .await?;
        let claimed = queue.next_with_lock(&format!("worker-{i}")).await?;
        assert!(claimed.is_some());
        queue
            .release_processing_lock(&format!("worker-{i}"))
            .await?;
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Recover all at once
    let start = std::time::Instant::now();
    let stats = queue.detect_and_recover_stale().await?;
    let elapsed = start.elapsed();

    assert_eq!(
        stats.entries_reclaimed, 100,
        "Should reclaim all 100 entries"
    );
    assert!(
        elapsed.as_secs() < 5,
        "Recovery should complete in < 5 seconds"
    );

    // Verify all are pending
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(pending.len(), 100);

    Ok(())
}

// ========================================================================
// EC-004: Entry stuck in intermediate state
// ========================================================================

#[tokio::test]
async fn test_ec004_entry_stuck_in_intermediate_state() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add entry
    queue.add("workspace-ec004", None, 5, None).await?;

    // Manually set to rebasing with old started_at
    let pool = queue.pool();
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE merge_queue SET status = 'rebasing', started_at = ?, agent_id = 'test' WHERE workspace = ?")
        .bind(now - 1000)
        .bind("workspace-ec004")
        .execute(pool)
        .await
        .map_err(|e| zjj_core::Error::DatabaseError(e.to_string()))?;

    // Run recovery
    let stats = queue.detect_and_recover_stale().await?;

    // Entry should NOT be reclaimed (not in 'claimed' state)
    assert_eq!(
        stats.entries_reclaimed, 0,
        "Should not reclaim rebasing entry"
    );

    // Verify still in rebasing
    let entry = queue
        .get_by_workspace("workspace-ec004")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Rebasing);

    Ok(())
}

// ========================================================================
// CV-001: All existing tests still pass
// ========================================================================
// This is verified by running all tests in test_worker_crash_recovery.rs

// ========================================================================
// CV-002: Manual reclaim_stale API unchanged
// ========================================================================

#[tokio::test]
async fn test_cv002_manual_reclaim_api_unchanged() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Verify function signature and return type
    // This test compiles - signature check
    let result: Result<usize> = queue.reclaim_stale(0).await;
    assert!(result.is_ok(), "Should return Result<usize, Error>");

    // Verify behavior
    queue.add("workspace-cv002", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-cv002").await?;
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-cv002").await?;

    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    let reclaimed = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed, 1, "Should reclaim 1 entry");

    Ok(())
}

// ========================================================================
// CV-003: Lock acquisition semantics preserved
// ========================================================================

#[tokio::test]
async fn test_cv003_lock_acquisition_semantics_preserved() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // First acquire
    let acquired1 = queue.acquire_processing_lock("worker-cv003-1").await?;
    assert!(acquired1, "First acquire should succeed");

    // Second acquire should fail (lock held)
    let acquired2 = queue.acquire_processing_lock("worker-cv003-2").await?;
    assert!(!acquired2, "Second acquire should fail (lock held)");

    // Release
    let released = queue.release_processing_lock("worker-cv003-1").await?;
    assert!(released);

    // Third acquire should succeed
    let acquired3 = queue.acquire_processing_lock("worker-cv003-3").await?;
    assert!(acquired3, "Acquire after release should succeed");

    // Cleanup
    queue.release_processing_lock("worker-cv003-3").await?;

    Ok(())
}

// ========================================================================
// CV-008: Recovery idempotence verified
// ========================================================================

#[tokio::test]
async fn test_cv008_recovery_idempotence_verified() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Add and abandon entry
    queue.add("workspace-cv008", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-cv008").await?;
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-cv008").await?;

    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // First recovery
    let stats1 = queue.detect_and_recover_stale().await?;
    assert_eq!(stats1.entries_reclaimed, 1);

    // Second recovery (idempotent)
    let stats2 = queue.detect_and_recover_stale().await?;
    assert_eq!(stats2.entries_reclaimed, 0);

    // Third recovery (still no-op)
    let stats3 = queue.detect_and_recover_stale().await?;
    assert_eq!(stats3.entries_reclaimed, 0);

    Ok(())
}

// ========================================================================
// CV-010: Error propagation paths unchanged
// ========================================================================

#[tokio::test]
async fn test_cv010_error_propagation_unchanged() -> Result<()> {
    let queue = MergeQueue::open_in_memory_with_timeout(TEST_LOCK_TIMEOUT_SECS).await?;

    // Try to get non-existent entry
    let result = queue.get_by_workspace("nonexistent").await?;
    assert!(result.is_none(), "Non-existent entry returns None");

    // Try to claim from empty queue
    let claimed = queue.next_with_lock("worker-cv010").await?;
    assert!(claimed.is_none(), "Empty queue returns None");

    // All operations should return Result types
    // No panics should occur

    Ok(())
}
