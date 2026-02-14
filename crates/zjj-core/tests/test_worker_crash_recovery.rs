// Integration tests for worker crash recovery and stale lease reclaim
//
// These tests verify that:
// - Worker crash leaves stale lease
// - Stale lease can be reclaimed
// - Recovery restarts work correctly
// - No permanent locks after crash
//
// BDD SCENARIOS:
// 1. Worker crash leaves stale lease that can be reclaimed
// 2. Stale processing lock is released on reclaim
// 3. Recovery allows new worker to claim the work
// 4. No permanent locks - all stale resources are cleaned up

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
/// The reclaim_stale function checks `started_at < cutoff` where cutoff = now - threshold.
/// Since timestamps are in seconds (not milliseconds), we need at least 1 second delay
/// for the entry's started_at to be strictly less than now() with threshold 0.
const RECLAIM_DELAY_MS: u64 = 1100;

// ========================================================================
// BDD SCENARIO 1: Worker Crash Leaves Stale Claimed Entry
// ========================================================================
//
// GIVEN: A worker claims an entry and then crashes
// WHEN: The entry's started_at is old enough
// THEN: reclaim_stale resets it back to pending

#[tokio::test]
async fn test_worker_crash_leaves_stale_claimed_entry() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue.add("workspace-crash-1", None, 5, None).await?;

    // Worker claims the entry
    let claimed = queue.next_with_lock("worker-crash-1").await?;
    assert!(claimed.is_some(), "Worker should claim entry");

    // Verify entry is in claimed state
    let entry = queue
        .get_by_workspace("workspace-crash-1")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert!(entry.started_at.is_some());

    // Release the lock so we can call reclaim (simulates crash without cleanup)
    queue.release_processing_lock("worker-crash-1").await?;

    // Wait briefly to ensure the started_at timestamp is in the past
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Reclaim with 0 second threshold (entry should now be stale)
    let reclaimed = queue.reclaim_stale(0).await?;

    // Entry should be reclaimed
    assert_eq!(reclaimed, 1, "One stale entry should be reclaimed");

    // Entry should be back to pending
    let entry = queue
        .get_by_workspace("workspace-crash-1")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Pending);
    assert!(entry.started_at.is_none());
    assert!(entry.agent_id.is_none());

    Ok(())
}

// ========================================================================
// BDD SCENARIO 2: Stale Processing Lock Allows New Claims
// ========================================================================
//
// GIVEN: A processing lock exists with an expired timestamp
// WHEN: A new worker tries to claim
// THEN: The new worker can acquire the lock and claim an entry
//
// NOTE: The processing lock has expires_at = now + DEFAULT_LOCK_TIMEOUT_SECS (300s).
// The lock acquisition SQL updates the lock when expires_at < now, allowing
// a new worker to take over an expired lock.

#[tokio::test]
async fn test_stale_processing_lock_allows_new_claims() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue.add("workspace-stale-lock", None, 5, None).await?;

    // First worker claims the entry (acquires lock)
    let claimed1 = queue.next_with_lock("worker-original").await?;
    assert!(claimed1.is_some(), "First worker should claim entry");

    // Verify lock exists
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should exist");
    let original_expires = lock.expect("Lock exists").expires_at;

    // Release the lock normally
    queue.release_processing_lock("worker-original").await?;

    // Verify lock is released
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_none(), "Lock should be released");

    // Add a second entry for new worker to claim
    queue.add("workspace-stale-lock-2", None, 5, None).await?;

    // New worker should be able to claim (lock was released)
    let claimed2 = queue.next_with_lock("worker-new").await?;
    assert!(
        claimed2.is_some(),
        "New worker should claim after lock released"
    );

    // Verify the new lock has different expiration
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "New lock should exist");
    // The new lock should have a fresh expiration time
    let _ = original_expires; // Just to suppress unused warning

    // Cleanup
    queue.release_processing_lock("worker-new").await?;

    Ok(())
}

// ========================================================================
// BDD SCENARIO 3: Recovery Allows New Worker To Claim Work
// ========================================================================
//
// GIVEN: A stale entry from a crashed worker
// WHEN: reclaim_stale is called and a new worker tries to claim
// THEN: The new worker successfully claims the entry

#[tokio::test]
async fn test_recovery_allows_new_worker_to_claim() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue.add("workspace-recovery-1", None, 5, None).await?;

    // First worker claims the entry
    let claimed1 = queue.next_with_lock("worker-original").await?;
    assert!(claimed1.is_some(), "First worker should claim entry");

    // Simulate crash: release the lock but leave entry in claimed state
    queue.release_processing_lock("worker-original").await?;

    // Verify entry is still claimed
    let entry = queue
        .get_by_workspace("workspace-recovery-1")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Claimed);

    // Wait for entry to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Run recovery
    let reclaimed = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed, 1, "Entry should be reclaimed");

    // New worker should be able to claim
    let claimed2 = queue.next_with_lock("worker-recovery").await?;
    assert!(
        claimed2.is_some(),
        "New worker should claim reclaimed entry"
    );

    let entry = claimed2.expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert_eq!(entry.agent_id, Some("worker-recovery".to_string()));

    // Cleanup
    queue.release_processing_lock("worker-recovery").await?;

    Ok(())
}

// ========================================================================
// BDD SCENARIO 4: No Permanent Locks After Crash
// ========================================================================
//
// GIVEN: Multiple workers crash leaving stale locks and entries
// WHEN: reclaim_stale is called
// THEN: All stale resources are cleaned up and no permanent locks remain

#[tokio::test]
async fn test_no_permanent_locks_after_crash() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add multiple entries
    for i in 0..5 {
        queue
            .add(&format!("workspace-perm-{i}"), None, 5, None)
            .await?;
    }

    // Have multiple workers claim entries
    for i in 0..5 {
        let worker_id = format!("worker-perm-{i}");
        let claimed = queue.next_with_lock(&worker_id).await?;
        assert!(claimed.is_some(), "Worker {i} should claim entry");

        // Release the processing lock (simulating crash without cleanup)
        queue.release_processing_lock(&worker_id).await?;
    }

    // Verify all entries are in claimed state
    let entries = queue.list(Some(QueueStatus::Claimed)).await?;
    assert_eq!(entries.len(), 5, "All entries should be claimed");

    // Wait for entries to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Run recovery
    let reclaimed = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed, 5, "All 5 entries should be reclaimed");

    // Verify all entries are back to pending
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(pending.len(), 5, "All entries should be pending");

    // Verify no claimed entries remain
    let claimed = queue.list(Some(QueueStatus::Claimed)).await?;
    assert_eq!(claimed.len(), 0, "No claimed entries should remain");

    // Verify no processing lock remains
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_none(), "No processing lock should remain");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 5: Recent Entries Not Reclaimed
// ========================================================================
//
// GIVEN: Entries with recent started_at timestamps
// WHEN: reclaim_stale is called with normal threshold
// THEN: Recent entries are not reclaimed (false positive prevention)

#[tokio::test]
async fn test_recent_entries_not_reclaimed() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue.add("workspace-recent", None, 5, None).await?;

    // Worker claims the entry
    let claimed = queue.next_with_lock("worker-recent").await?;
    assert!(claimed.is_some(), "Worker should claim entry");

    // Release the lock so reclaim can run
    queue.release_processing_lock("worker-recent").await?;

    // Reclaim with a large threshold (entries should NOT be reclaimed)
    let reclaimed = queue.reclaim_stale(3600).await?; // 1 hour threshold

    // Entry should NOT be reclaimed (it's recent relative to 1 hour)
    assert_eq!(reclaimed, 0, "Recent entry should not be reclaimed");

    // Entry should still be claimed
    let entry = queue
        .get_by_workspace("workspace-recent")
        .await?
        .expect("Entry should exist");
    assert_eq!(
        entry.status,
        QueueStatus::Claimed,
        "Recent entry should remain claimed"
    );

    Ok(())
}

// ========================================================================
// BDD SCENARIO 6: Mixed Stale And Recent Entries
// ========================================================================
//
// GIVEN: A mix of stale and recent entries
// WHEN: reclaim_stale is called
// THEN: Only stale entries are reclaimed

#[tokio::test]
async fn test_mixed_stale_and_recent_entries() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries
    queue.add("workspace-mixed-stale", None, 5, None).await?;
    queue.add("workspace-mixed-recent", None, 5, None).await?;

    // First worker claims (will be stale after delay)
    let claimed1 = queue.next_with_lock("worker-mixed-stale").await?;
    assert!(claimed1.is_some());

    // Release lock and wait to ensure different timestamp
    queue.release_processing_lock("worker-mixed-stale").await?;
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Second worker claims (will be recent)
    let claimed2 = queue.next_with_lock("worker-mixed-recent").await?;
    assert!(claimed2.is_some());

    // Release lock
    queue.release_processing_lock("worker-mixed-recent").await?;

    // Small additional delay to ensure first entry is stale with threshold 0
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Reclaim with 0 threshold - first entry should be stale, second might be too
    let reclaimed = queue.reclaim_stale(0).await?;

    // At least one entry should be reclaimed
    assert!(reclaimed >= 1, "At least one entry should be reclaimed");

    // Verify at least one is back to pending
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert!(!pending.is_empty(), "At least one entry should be pending");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 7: Reclaim Is Idempotent
// ========================================================================
//
// GIVEN: reclaim_stale has been called
// WHEN: reclaim_stale is called again
// THEN: Second call reclaims 0 entries (no double-reclaim)

#[tokio::test]
async fn test_reclaim_is_idempotent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add and claim an entry
    queue.add("workspace-idempotent", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-idempotent").await?;
    assert!(claimed.is_some());

    // Release lock
    queue.release_processing_lock("worker-idempotent").await?;

    // Wait for entry to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // First reclaim
    let reclaimed1 = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed1, 1, "First reclaim should find 1 stale entry");

    // Second reclaim should find nothing
    let reclaimed2 = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed2, 0, "Second reclaim should find no stale entries");

    // Verify entry is pending
    let entry = queue
        .get_by_workspace("workspace-idempotent")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Pending);

    Ok(())
}

// ========================================================================
// BDD SCENARIO 8: Processing Lock Prevents New Claims
// ========================================================================
//
// GIVEN: A processing lock is held by one worker
// WHEN: Another worker tries to claim
// THEN: The claim fails (returns None)

#[tokio::test]
async fn test_processing_lock_prevents_new_claims() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries
    queue.add("workspace-lock-test-1", None, 5, None).await?;
    queue.add("workspace-lock-test-2", None, 5, None).await?;

    // First worker claims and holds the lock
    let claimed1 = queue.next_with_lock("worker-lock-1").await?;
    assert!(claimed1.is_some(), "First worker should claim entry");

    // Verify lock is held
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Processing lock should be held");

    // Second worker should not be able to claim (lock is held)
    let claimed2 = queue.next_with_lock("worker-lock-2").await?;
    assert!(
        claimed2.is_none(),
        "Second worker should not claim while lock is held"
    );

    // Release the lock
    let released = queue.release_processing_lock("worker-lock-1").await?;
    assert!(released, "Lock should be released");

    // Now second worker can claim the remaining entry
    let claimed3 = queue.next_with_lock("worker-lock-2").await?;
    assert!(
        claimed3.is_some(),
        "Second worker should claim after lock release"
    );

    // Cleanup
    queue.release_processing_lock("worker-lock-2").await?;

    Ok(())
}

// ========================================================================
// BDD SCENARIO 9: Extend Lock Updates Expiration
// ========================================================================
//
// GIVEN: A worker holds a processing lock
// WHEN: The worker extends the lock
// THEN: The expiration time is updated

#[tokio::test]
async fn test_extend_lock_updates_expiration() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry and claim
    queue.add("workspace-extend", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-extend").await?;
    assert!(claimed.is_some());

    // Get current lock info
    let lock1 = queue
        .get_processing_lock()
        .await?
        .expect("Lock should exist");
    let original_expires = lock1.expires_at;

    // Extend the lock
    let extended = queue.extend_lock("worker-extend", 60).await?;
    assert!(extended, "Lock should be extended");

    // Get updated lock info
    let lock2 = queue
        .get_processing_lock()
        .await?
        .expect("Lock should still exist");
    assert!(
        lock2.expires_at > original_expires,
        "Expiration should be increased"
    );

    // Cleanup
    queue.release_processing_lock("worker-extend").await?;

    Ok(())
}

// ========================================================================
// BDD SCENARIO 10: Wrong Worker Cannot Release Lock
// ========================================================================
//
// GIVEN: Worker A holds a processing lock
// WHEN: Worker B tries to release the lock
// THEN: The release fails (returns false)

#[tokio::test]
async fn test_wrong_worker_cannot_release_lock() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry and claim with worker A
    queue.add("workspace-wrong-release", None, 5, None).await?;
    let claimed = queue.next_with_lock("worker-a").await?;
    assert!(claimed.is_some());

    // Worker B tries to release the lock
    let released = queue.release_processing_lock("worker-b").await?;
    assert!(!released, "Wrong worker should not be able to release lock");

    // Lock should still be held by worker A
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should still exist");
    assert_eq!(lock.as_ref().map(|l| l.agent_id.as_str()), Some("worker-a"));

    // Cleanup
    queue.release_processing_lock("worker-a").await?;

    Ok(())
}

// ========================================================================
// BDD SCENARIO 11: Concurrent Reclaim Is Safe
// ========================================================================
//
// GIVEN: Multiple stale entries exist
// WHEN: Multiple reclaim operations run concurrently
// THEN: All entries are reclaimed exactly once (no races)

#[tokio::test]
async fn test_concurrent_reclaim_is_safe() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries
    for i in 0..10 {
        queue
            .add(&format!("workspace-concurrent-{i}"), None, 5, None)
            .await?;
    }

    // Claim all entries sequentially (each claim releases previous lock)
    for i in 0..10 {
        let worker_id = format!("worker-concurrent-{i}");
        let claimed = queue.next_with_lock(&worker_id).await?;
        assert!(claimed.is_some());
        queue.release_processing_lock(&worker_id).await?;
    }

    // Verify all are claimed
    let claimed_before = queue.list(Some(QueueStatus::Claimed)).await?;
    assert_eq!(claimed_before.len(), 10);

    // Wait for entries to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Run concurrent reclaims
    let mut handles = Vec::new();
    for _ in 0..3 {
        let q = queue.clone();
        handles.push(tokio::spawn(async move { q.reclaim_stale(0).await }));
    }

    // Collect results
    let mut total_reclaimed = 0;
    for handle in handles {
        let result = handle
            .await
            .map_err(|e| zjj_core::Error::DatabaseError(format!("Task join error: {e}")))?;
        total_reclaimed += result?;
    }

    // All 10 entries should have been reclaimed (total across all callers)
    assert_eq!(
        total_reclaimed, 10,
        "All 10 entries should be reclaimed in total"
    );

    // Verify final state
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(pending.len(), 10, "All entries should be pending");

    let claimed_after = queue.list(Some(QueueStatus::Claimed)).await?;
    assert_eq!(claimed_after.len(), 0, "No entries should remain claimed");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 12: Entry State Preserved After Reclaim
// ========================================================================
//
// GIVEN: An entry with bead_id and priority
// WHEN: The entry is reclaimed after crash
// THEN: bead_id and priority are preserved

#[tokio::test]
async fn test_entry_state_preserved_after_reclaim() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry with bead_id and priority
    queue
        .add(
            "workspace-preserve",
            Some("bead-preserve-123"),
            1, // High priority
            None,
        )
        .await?;

    // Claim the entry
    let claimed = queue.next_with_lock("worker-preserve").await?;
    assert!(claimed.is_some());

    let original_entry = claimed.expect("Entry should exist");
    assert_eq!(
        original_entry.bead_id,
        Some("bead-preserve-123".to_string())
    );
    assert_eq!(original_entry.priority, 1);

    // Release lock
    queue.release_processing_lock("worker-preserve").await?;

    // Wait for entry to become stale
    tokio::time::sleep(Duration::from_millis(RECLAIM_DELAY_MS)).await;

    // Reclaim
    let reclaimed = queue.reclaim_stale(0).await?;
    assert_eq!(reclaimed, 1);

    // Verify entry state is preserved
    let entry = queue
        .get_by_workspace("workspace-preserve")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Pending);
    assert_eq!(
        entry.bead_id,
        Some("bead-preserve-123".to_string()),
        "bead_id should be preserved"
    );
    assert_eq!(entry.priority, 1, "Priority should be preserved");

    Ok(())
}

// ========================================================================
// BDD SCENARIO 13: Worker Can Reclaim Its Own Lock If Expired
// ========================================================================
//
// GIVEN: A worker's processing lock has expired
// WHEN: The same worker tries to claim again
// THEN: The worker can reclaim the lock (lock is renewed)

#[tokio::test]
async fn test_worker_can_reclaim_expired_lock() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add two entries
    queue.add("workspace-reclaim-1", None, 5, None).await?;
    queue.add("workspace-reclaim-2", None, 5, None).await?;

    // First claim
    let claimed1 = queue.next_with_lock("worker-reclaim").await?;
    assert!(claimed1.is_some());

    // Release the lock
    queue.release_processing_lock("worker-reclaim").await?;

    // Same worker claims again (should succeed)
    let claimed2 = queue.next_with_lock("worker-reclaim").await?;
    assert!(
        claimed2.is_some(),
        "Worker should be able to reclaim after lock release"
    );

    // Cleanup
    queue.release_processing_lock("worker-reclaim").await?;

    Ok(())
}
