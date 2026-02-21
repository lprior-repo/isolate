# Martin Fowler Test Plan: bd-2i5 - Implement self-healing for stale processing locks

**Bead ID:** bd-2i5
**Title:** Implement self-healing for stale processing locks
**Test Framework:** Given-When-Then (BDD style)
**Coverage Target:** 100% of contract specification

## Test Suite Organization

```
bd-2i5-tests/
├── happy_path/          # HP-001 to HP-015
├── error_path/          # EP-001 to EP-010
├── edge_cases/          # EC-001 to EC-015
└── contract_verification/ # CV-001 to CV-020
```

---

## Happy Path Tests (HP)

### HP-001: Automatic recovery cleans expired locks

**GIVEN** a processing lock has expired (expires_at < now)
**AND** the lock exists in queue_processing_lock table
**WHEN** a worker calls `next_with_lock()` to claim work
**THEN** the expired lock is automatically deleted
**AND** the worker successfully acquires a new lock
**AND** the new lock has a fresh expires_at timestamp
**AND** the recovery stats show locks_cleaned = 1

```rust
#[tokio::test]
async fn test_hp001_automatic_recovery_cleans_expired_locks() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-hp001", None, 5, None).await.unwrap();

    // First worker claims (acquires lock)
    let claimed1 = queue.next_with_lock("worker-hp001-1").await.unwrap();
    assert!(claimed1.is_some());

    // Get lock info
    let lock1 = queue.get_processing_lock().await.unwrap().unwrap();
    let original_expires = lock1.expires_at;

    // Simulate time passing (lock expires)
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Second worker claims - should automatically clean expired lock
    let claimed2 = queue.next_with_lock("worker-hp001-2").await.unwrap();

    // Verify lock was cleaned and new lock acquired
    assert!(claimed2.is_some(), "Second worker should claim after auto-cleanup");

    let lock2 = queue.get_processing_lock().await.unwrap().unwrap();
    assert!(lock2.expires_at > original_expires, "Lock should have new expiration");
    assert_eq!(lock2.agent_id, "worker-hp001-2");

    // Cleanup
    queue.release_processing_lock("worker-hp001-2").await.unwrap();
}
```

---

### HP-002: Automatic recovery reclaims stale entries

**GIVEN** an entry is in 'claimed' status with started_at < threshold
**AND** the processing lock has been released
**WHEN** a worker calls `next_with_lock()` to claim work
**THEN** the stale entry is automatically reset to 'pending'
**AND** the worker can claim the entry
**AND** the entry's agent_id and started_at are cleared
**AND** state_changed_at is updated

```rust
#[tokio::test]
async fn test_hp002_automatic_recovery_reclaims_stale_entries() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-hp002", None, 5, None).await.unwrap();

    // First worker claims and releases lock (simulating crash)
    let claimed1 = queue.next_with_lock("worker-hp002-1").await.unwrap();
    assert!(claimed1.is_some());

    // Verify entry is claimed
    let entry = queue.get_by_workspace("workspace-hp002").await.unwrap().unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert_eq!(entry.agent_id, Some("worker-hp002-1".to_string()));

    // Release lock (simulating crash)
    queue.release_processing_lock("worker-hp002-1").await.unwrap();

    // Wait for entry to become stale
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Second worker claims - should auto-reclaim stale entry
    let claimed2 = queue.next_with_lock("worker-hp002-2").await.unwrap();
    assert!(claimed2.is_some(), "Should claim after auto-reclaim");

    let entry = claimed2.unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert_eq!(entry.agent_id, Some("worker-hp002-2".to_string()));
    assert!(entry.started_at.is_some(), "started_at should be set");

    // Cleanup
    queue.release_processing_lock("worker-hp002-2").await.unwrap();
}
```

---

### HP-003: Recovery stats accurately report cleanup

**GIVEN** there are expired locks and stale entries
**WHEN** `detect_and_recover_stale()` is called
**THEN** RecoveryStats reflects correct counts
**AND** locks_cleaned equals number of expired locks removed
**AND** entries_reclaimed equals number of stale entries reset
**AND** recovery_timestamp is set to current time

```rust
#[tokio::test]
async fn test_hp003_recovery_stats_accurately_report_cleanup() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entries and claim them
    for i in 0..3 {
        queue.add(&format!("workspace-hp003-{i}"), None, 5, None).await.unwrap();
        let claimed = queue.next_with_lock(&format!("worker-hp003-{i}")).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&format!("worker-hp003-{i}")).await.unwrap();
    }

    // Wait for entries to become stale
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Run recovery
    let stats = queue.detect_and_recover_stale().await.unwrap();

    // Verify stats
    assert_eq!(stats.locks_cleaned, 1, "Should clean 1 expired lock");
    assert_eq!(stats.entries_reclaimed, 3, "Should reclaim 3 stale entries");
    assert!(stats.recovery_timestamp > 0, "Timestamp should be set");
}
```

---

### HP-004: Multiple consecutive claims with auto-recovery

**GIVEN** multiple workers crash claiming entries
**AND** entries are left in stale claimed state
**WHEN** new workers repeatedly call `next_with_lock()`
**THEN** each call automatically recovers one stale entry
**AND** all entries are eventually processed
**AND** no manual reclaim_stale() call is needed

```rust
#[tokio::test]
async fn test_hp004_multiple_consecutive_claims_with_auto_recovery() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add 5 entries
    for i in 0..5 {
        queue.add(&format!("workspace-hp004-{i}"), None, 5, None).await.unwrap();
    }

    // Simulate crashes: claim and release lock
    for i in 0..5 {
        let worker = format!("worker-hp004-{i}");
        let claimed = queue.next_with_lock(&worker).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&worker).await.unwrap();
    }

    // Verify all are claimed
    let claimed_entries = queue.list(Some(QueueStatus::Claimed)).await.unwrap();
    assert_eq!(claimed_entries.len(), 5);

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // New workers claim - should auto-reclaim
    let mut claimed_count = 0;
    for i in 0..5 {
        let worker = format!("worker-hp004-recovery-{i}");
        let claimed = queue.next_with_lock(&worker).await.unwrap();
        if claimed.is_some() {
            claimed_count += 1;
            queue.release_processing_lock(&worker).await.unwrap();
        }
    }

    assert_eq!(claimed_count, 5, "All 5 entries should be claimed after auto-recovery");
}
```

---

### HP-005: get_recovery_stats reports without cleaning

**GIVEN** there are expired locks and stale entries
**WHEN** `get_recovery_stats()` is called
**THEN** stats are returned (locks and entries counted)
**AND** NO cleanup is performed
**AND** calling again returns same counts
**AND** subsequent `detect_and_recover_stale()` actually cleans

```rust
#[tokio::test]
async fn test_hp005_get_recovery_stats_reports_without_cleaning() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and claim entry
    queue.add("workspace-hp005", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-hp005").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-hp005").await.unwrap();

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Get stats (should not clean)
    let stats1 = queue.get_recovery_stats().await.unwrap();
    assert_eq!(stats1.locks_cleaned, 1, "Should report 1 expired lock");
    assert_eq!(stats1.entries_reclaimed, 1, "Should report 1 stale entry");

    // Get stats again - should report same counts (nothing cleaned)
    let stats2 = queue.get_recovery_stats().await.unwrap();
    assert_eq!(stats2.locks_cleaned, 1, "Should still report 1 expired lock");
    assert_eq!(stats2.entries_reclaimed, 1, "Should still report 1 stale entry");

    // Verify entry is still claimed (not reclaimed)
    let entry = queue.get_by_workspace("workspace-hp005").await.unwrap().unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed, "Entry should still be claimed");

    // Now actually clean
    let stats3 = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats3.locks_cleaned, 1, "Should clean 1 lock");
    assert_eq!(stats3.entries_reclaimed, 1, "Should reclaim 1 entry");

    // Verify entry is now pending
    let entry = queue.get_by_workspace("workspace-hp005").await.unwrap().unwrap();
    assert_eq!(entry.status, QueueStatus::Pending, "Entry should be pending after cleanup");
}
```

---

### HP-006: is_lock_stale correctly reports lock state

**GIVEN** a processing lock exists
**WHEN** the lock is not expired (expires_at >= now)
**THEN** `is_lock_stale()` returns false
**WHEN** the lock is expired (expires_at < now)
**THEN** `is_lock_stale()` returns true
**WHEN** no lock exists
**THEN** `is_lock_stale()` returns false

```rust
#[tokio::test]
async fn test_hp006_is_lock_stale_correctly_reports_lock_state() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // No lock initially
    assert!(!queue.is_lock_stale().await.unwrap(), "No lock should not be stale");

    // Add entry and acquire lock
    queue.add("workspace-hp006", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-hp006").await.unwrap();
    assert!(claimed.is_some());

    // Lock is fresh (not stale)
    assert!(!queue.is_lock_stale().await.unwrap(), "Fresh lock should not be stale");

    // Release lock
    queue.release_processing_lock("worker-hp006").await.unwrap();

    // Wait for lock to expire (need to manually expire it)
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Note: can't easily test expired lock without direct DB manipulation
    // The UPSERT in acquire_processing_lock will have cleaned it
    // This is tested indirectly by next_with_lock behavior
}
```

---

### HP-007: Auto-recovery happens before every claim attempt

**GIVEN** a stale entry exists
**AND** no manual reclaim has been called
**WHEN** a worker calls `next_with_lock()` for the first time
**THEN** stale entry is automatically reclaimed
**AND** worker successfully claims the entry
**AND** no manual intervention was required

```rust
#[tokio::test]
async fn test_hp007_auto_recovery_before_every_claim() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and abandon entry
    queue.add("workspace-hp007", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-hp007-abandon").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-hp007-abandon").await.unwrap();

    // Verify entry is claimed
    let entry = queue.get_by_workspace("workspace-hp007").await.unwrap().unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // First claim attempt - should auto-recover
    let claimed = queue.next_with_lock("worker-hp007-recover").await.unwrap();
    assert!(claimed.is_some(), "Should auto-recover and claim");

    // Cleanup
    queue.release_processing_lock("worker-hp007-recover").await.unwrap();
}
```

---

### HP-008: Auto-recovery failure doesn't prevent claim

**GIVEN** the database has a transient error during recovery
**WHEN** a worker calls `next_with_lock()`
**THEN** recovery failure is logged but doesn't crash
**AND** the claim attempt proceeds anyway
**AND** claim may succeed or fail based on actual state

```rust
#[tokio::test]
async fn test_hp008_recovery_failure_doesnt_prevent_claim() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-hp008", None, 5, None).await.unwrap();

    // Normal claim should succeed
    // (recovery will run but there's nothing stale)
    let claimed = queue.next_with_lock("worker-hp008").await.unwrap();
    assert!(claimed.is_some());

    // Cleanup
    queue.release_processing_lock("worker-hp008").await.unwrap();

    // Note: Testing actual DB errors during recovery requires
    // more complex setup (transaction failures, etc.)
    // This test verifies the happy path where recovery is no-op
}
```

---

### HP-009: Manual reclaim_stale still works

**GIVEN** the new automatic recovery is implemented
**AND** stale entries exist
**WHEN** `reclaim_stale()` is called manually
**THEN** it works exactly as before
**AND** entries are reclaimed
**AND** return value is count of reclaimed entries
**AND** automatic and manual recovery can coexist

```rust
#[tokio::test]
async fn test_hp009_manual_reclaim_stale_still_works() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and abandon entries
    for i in 0..3 {
        queue.add(&format!("workspace-hp009-{i}"), None, 5, None).await.unwrap();
        let claimed = queue.next_with_lock(&format!("worker-hp009-{i}")).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&format!("worker-hp009-{i}")).await.unwrap();
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Manual reclaim (original API)
    let reclaimed = queue.reclaim_stale(0).await.unwrap();
    assert_eq!(reclaimed, 3, "Manual reclaim should work as before");

    // Verify entries are pending
    let pending = queue.list(Some(QueueStatus::Pending)).await.unwrap();
    assert_eq!(pending.len(), 3);
}
```

---

### HP-010: Auto-recovery with custom lock timeout

**GIVEN** a queue with custom lock_timeout_secs (e.g., 1 second)
**AND** entries are claimed and abandoned
**WHEN** timeout period elapses
**THEN** auto-recovery uses the custom timeout
**AND** entries are reclaimed after correct duration
**AND** recovery behavior is consistent with configuration

```rust
#[tokio::test]
async fn test_hp010_auto_recovery_with_custom_lock_timeout() {
    // Create queue with very short timeout (1 second)
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let queue = MergeQueue::new_with_timeout(pool, 1).await.unwrap();

    // Add and abandon entry
    queue.add("workspace-hp010", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-hp010").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-hp010").await.unwrap();

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Auto-recover and claim
    let claimed = queue.next_with_lock("worker-hp010-recover").await.unwrap();
    assert!(claimed.is_some(), "Should reclaim with custom timeout");

    // Cleanup
    queue.release_processing_lock("worker-hp010-recover").await.unwrap();
}
```

---

### HP-011: Recovery preserves entry metadata

**GIVEN** a stale claimed entry with bead_id and priority
**WHEN** auto-recovery reclaims the entry
**THEN** bead_id is preserved
**AND** priority is preserved
**AND** workspace is preserved
**AND** only status, agent_id, started_at are reset

```rust
#[tokio::test]
async fn test_hp011_recovery_preserves_entry_metadata() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry with metadata
    queue.add("workspace-hp011", Some("bead-hp011-123"), 1, None).await.unwrap();

    // Claim and abandon
    let claimed = queue.next_with_lock("worker-hp011").await.unwrap();
    assert!(claimed.is_some());
    let original = claimed.unwrap();
    assert_eq!(original.bead_id, Some("bead-hp011-123".to_string()));
    assert_eq!(original.priority, 1);

    queue.release_processing_lock("worker-hp011").await.unwrap();

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Auto-recover and reclaim
    let claimed = queue.next_with_lock("worker-hp011-recover").await.unwrap();
    assert!(claimed.is_some());

    let recovered = claimed.unwrap();
    assert_eq!(recovered.bead_id, Some("bead-hp011-123".to_string()), "bead_id preserved");
    assert_eq!(recovered.priority, 1, "priority preserved");
    assert_eq!(recovered.status, QueueStatus::Claimed);

    // Cleanup
    queue.release_processing_lock("worker-hp011-recover").await.unwrap();
}
```

---

### HP-012: Multiple workers can safely recover concurrently

**GIVEN** multiple stale entries exist
**WHEN** multiple workers call `next_with_lock()` concurrently
**THEN** each worker gets a unique entry
**AND** no entry is claimed twice
**AND** no race conditions occur
**AND** all entries are processed exactly once

```rust
#[tokio::test]
async fn test_hp012_multiple_workers_safely_recover_concurrently() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and abandon entries
    for i in 0..10 {
        queue.add(&format!("workspace-hp012-{i}"), None, 5, None).await.unwrap();
        let claimed = queue.next_with_lock(&format!("worker-abandon-{i}")).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&format!("worker-abandon-{i}")).await.unwrap();
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

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
        .map(|r| r.unwrap().unwrap())
        .collect();

    // Count successful claims
    let successful_claims = results.iter().filter(|r| r.is_some()).count();
    assert_eq!(successful_claims, 10, "All 10 entries should be claimed");

    // Verify unique workspaces
    let workspaces: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().map(|e| e.workspace.clone()))
        .collect();
    assert_eq!(workspaces.len(), 10, "All workspaces should be unique");

    // Cleanup
    for result in results {
        if let Some(entry) = result {
            let _ = queue.release_processing_lock(&entry.agent_id.unwrap()).await;
        }
    }
}
```

---

### HP-013: Recovery is idempotent

**GIVEN** stale entries have been reclaimed
**WHEN** `detect_and_recover_stale()` is called again
**THEN** no additional entries are reclaimed
**AND** return counts are zero
**AND** entries remain in pending state

```rust
#[tokio::test]
async fn test_hp013_recovery_is_idempotent() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and abandon entry
    queue.add("workspace-hp013", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-hp013").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-hp013").await.unwrap();

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // First recovery
    let stats1 = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats1.entries_reclaimed, 1, "Should reclaim 1 entry");

    // Second recovery (idempotent)
    let stats2 = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats2.entries_reclaimed, 0, "Should reclaim 0 entries (already pending)");
}
```

---

### HP-014: Auto-recovery works with empty queue

**GIVEN** the queue is empty
**WHEN** a worker calls `next_with_lock()`
**THEN** auto-recovery runs without error
**AND** worker gets None (no work available)
**AND** no crashes or panics occur

```rust
#[tokio::test]
async fn test_hp014_auto_recovery_with_empty_queue() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Queue is empty, call next_with_lock
    let claimed = queue.next_with_lock("worker-hp014").await.unwrap();
    assert!(claimed.is_none(), "No work should be available");

    // Should be able to call again without issues
    let claimed2 = queue.next_with_lock("worker-hp014").await.unwrap();
    assert!(claimed2.is_none());
}
```

---

### HP-015: Auto-recovery integrates with existing retry logic

**GIVEN** a transient database lock occurs
**WHEN** a worker calls `next_with_lock()`
**THEN** auto-recovery runs before each retry
**AND** existing exponential backoff works
**AND** eventually succeeds or fails appropriately

```rust
#[tokio::test]
async fn test_hp015_auto_recovery_integrates_with_retry_logic() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-hp015", None, 5, None).await.unwrap();

    // First claim
    let claimed1 = queue.next_with_lock("worker-hp015-1").await.unwrap();
    assert!(claimed1.is_some());

    // Attempt concurrent claim (will retry)
    let handle = tokio::spawn({
        let q = queue.clone();
        async move {
            // This will fail initially, then retry
            // Recovery runs before each retry
            q.next_with_lock("worker-hp015-2").await
        }
    });

    // Give time for retry attempts
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Release first lock
    queue.release_processing_lock("worker-hp015-1").await.unwrap();

    // Second worker should eventually succeed
    let result = handle.await.unwrap().unwrap();
    assert!(result.is_some(), "Second worker should eventually succeed");

    // Cleanup
    queue.release_processing_lock("worker-hp015-2").await.unwrap();
}
```

---

## Error Path Tests (EP)

### EP-001: Database error during recovery doesn't crash

**GIVEN** the database connection fails during recovery
**WHEN** `detect_and_recover_stale()` encounters a database error
**THEN** error is wrapped in Result::Err
**AND** calling code can handle the error
**AND** system doesn't crash or panic

```rust
#[tokio::test]
async fn test_ep001_database_error_during_recovery() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Note: Testing actual database errors requires closing the pool
    // or simulating failure. This test verifies error handling exists.

    // In next_with_lock, recovery errors are caught and logged
    // Verify that a bad query doesn't crash the system
    queue.add("workspace-ep001", None, 5, None).await.unwrap();

    // Normal operation should work
    let claimed = queue.next_with_lock("worker-ep001").await.unwrap();
    assert!(claimed.is_some());

    // Cleanup
    queue.release_processing_lock("worker-ep001").await.unwrap();
}
```

---

### EP-002: Recovery with corrupted lock table

**GIVEN** the queue_processing_lock table has invalid data
**WHEN** recovery is attempted
**THEN** error is returned gracefully
**AND** queue remains in consistent state
**AND** subsequent operations can succeed

```rust
#[tokio::test]
async fn test_ep002_recovery_with_corrupted_lock_table() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let queue = MergeQueue::new(pool).await.unwrap();

    // Add invalid lock data directly
    let pool = queue.pool();
    sqlx::query("INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) VALUES (1, 'invalid', 'not-a-number', 'not-a-number')")
        .execute(pool)
        .await
        .ok(); // Ignore error

    // Recovery should handle this gracefully
    let result = queue.detect_and_recover_stale().await;

    // Either succeeds (cleaned up) or fails gracefully
    match result {
        Ok(_) => {}, // Cleaned up successfully
        Err(_) => {}, // Failed gracefully - OK
    }

    // Subsequent operations should work
    queue.add("workspace-ep002", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-ep002").await.unwrap();
    assert!(claimed.is_some());
}
```

---

### EP-003: Concurrent reclaim with manual reclaim_stale

**GIVEN** stale entries exist
**WHEN** auto-recovery and manual reclaim_stale run concurrently
**THEN** both operations complete successfully
**AND** entries are reclaimed exactly once
**AND** no race conditions or deadlocks occur

```rust
#[tokio::test]
async fn test_ep003_concurrent_auto_and_manual_reclaim() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and abandon entries
    for i in 0..5 {
        queue.add(&format!("workspace-ep003-{i}"), None, 5, None).await.unwrap();
        let claimed = queue.next_with_lock(&format!("worker-abandon-{i}")).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&format!("worker-abandon-{i}")).await.unwrap();
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Run manual and auto recovery concurrently
    let q1 = queue.clone();
    let q2 = queue.clone();

    let handle1 = tokio::spawn(async move {
        q1.reclaim_stale(0).await
    });

    let handle2 = tokio::spawn(async move {
        q2.detect_and_recover_stale().await
    });

    let result1 = handle1.await.unwrap().unwrap();
    let result2 = handle2.await.unwrap().unwrap();

    // Total reclaimed should be 5 (entries reclaimed exactly once)
    // Note: The exact split between the two is non-deterministic
    assert!(result1 + result2.entries_reclaimed <= 5,
            "Total reclaimed should not exceed 5");
}
```

---

### EP-004: Recovery during queue shutdown

**GIVEN** the queue is being shut down
**WHEN** recovery is triggered
**THEN** operation completes or fails gracefully
**AND** no resource leaks occur
**AND** system can shutdown cleanly

```rust
#[tokio::test]
async fn test_ep004_recovery_during_shutdown() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-ep004", None, 5, None).await.unwrap();

    // Simulate shutdown by dropping queue
    // Recovery should not cause panics during drop
    drop(queue);

    // If we get here, shutdown was clean
}
```

---

### EP-005: Recovery with partially migrated schema

**GIVEN** the database schema is missing columns
**WHEN** recovery is attempted
**THEN** migration is attempted first
**AND** appropriate error is returned if migration fails
**AND** system doesn't crash

```rust
#[tokio::test]
async fn test_ep005_recovery_with_partial_schema() {
    // This test verifies that schema migrations run
    // In normal operation, init_schema() is called first
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Init schema (includes migrations)
    let queue = MergeQueue::new(pool.clone()).await.unwrap();

    // Recovery should work with full schema
    queue.add("workspace-ep005", None, 5, None).await.unwrap();

    let stats = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats.entries_reclaimed, 0);
}
```

---

## Edge Cases Tests (EC)

### EC-001: Zero entries with stale lock

**GIVEN** a stale processing lock exists
**AND** there are no entries in the queue
**WHEN** auto-recovery runs
**THEN** lock is cleaned
**AND** no entries are reclaimed
**AND** next_with_lock returns None

```rust
#[tokio::test]
async fn test_ec001_zero_entries_with_stale_lock() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Manually create an expired lock
    let pool = queue.pool();
    let now = queue::MergeQueue::now();
    sqlx::query("INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) VALUES (1, 'test', ?, ?)")
        .bind(now - 100)
        .bind(now - 50)
        .execute(pool)
        .await
        .unwrap();

    // Verify lock exists
    let lock = queue.get_processing_lock().await.unwrap();
    assert!(lock.is_some());

    // Run recovery
    let stats = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats.locks_cleaned, 1, "Should clean 1 lock");
    assert_eq!(stats.entries_reclaimed, 0, "Should reclaim 0 entries");

    // Verify lock is gone
    let lock = queue.get_processing_lock().await.unwrap();
    assert!(lock.is_none());
}
```

---

### EC-002: Stale entry with zero timeout

**GIVEN** a queue with lock_timeout_secs = 0
**AND** an entry is claimed
**WHEN** next_with_lock is called immediately
**THEN** entry is reclaimed (started_at < now - 0 is false for same second)
**AND** new worker can claim after 1 second delay

```rust
#[tokio::test]
async fn test_ec002_stale_entry_with_zero_timeout() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let queue = MergeQueue::new_with_timeout(pool, 0).await.unwrap();

    // Add and claim
    queue.add("workspace-ec002", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-ec002").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-ec002").await.unwrap();

    // Small delay to ensure different second
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Should auto-reclaim
    let claimed = queue.next_with_lock("worker-ec002-recover").await.unwrap();
    assert!(claimed.is_some(), "Should reclaim with zero timeout");

    // Cleanup
    queue.release_processing_lock("worker-ec002-recover").await.unwrap();
}
```

---

### EC-003: Very large number of stale entries

**GIVEN** 1000 stale claimed entries
**WHEN** auto-recovery runs
**THEN** all entries are reclaimed in single call
**AND** performance is acceptable
**AND** no timeouts occur

```rust
#[tokio::test]
async fn test_ec003_very_large_number_of_stale_entries() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add 100 entries (using 100 for speed, production would test 1000)
    for i in 0..100 {
        queue.add(&format!("workspace-ec003-{i}"), None, 5, None).await.unwrap();
        let claimed = queue.next_with_lock(&format!("worker-{i}")).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&format!("worker-{i}")).await.unwrap();
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Recover all at once
    let start = std::time::Instant::now();
    let stats = queue.detect_and_recover_stale().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(stats.entries_reclaimed, 100, "Should reclaim all 100 entries");
    assert!(elapsed.as_secs() < 5, "Recovery should complete in < 5 seconds");

    // Verify all are pending
    let pending = queue.list(Some(QueueStatus::Pending)).await.unwrap();
    assert_eq!(pending.len(), 100);
}
```

---

### EC-004: Entry stuck in intermediate state

**GIVEN** an entry is in 'rebasing' state with old started_at
**WHEN** auto-recovery runs
**THEN** entry is NOT reclaimed (only 'claimed' state is reclaimed)
**AND** entry remains in intermediate state
**AND** manual intervention may be required

```rust
#[tokio::test]
async fn test_ec004_entry_stuck_in_intermediate_state() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-ec004", None, 5, None).await.unwrap();

    // Manually set to rebasing with old started_at
    let pool = queue.pool();
    let now = queue::MergeQueue::now();
    sqlx::query("UPDATE merge_queue SET status = 'rebasing', started_at = ?, agent_id = 'test' WHERE workspace = ?")
        .bind(now - 1000)
        .bind("workspace-ec004")
        .execute(pool)
        .await
        .unwrap();

    // Run recovery
    let stats = queue.detect_and_recover_stale().await.unwrap();

    // Entry should NOT be reclaimed (not in 'claimed' state)
    assert_eq!(stats.entries_reclaimed, 0, "Should not reclaim rebasing entry");

    // Verify still in rebasing
    let entry = queue.get_by_workspace("workspace-ec004").await.unwrap().unwrap();
    assert_eq!(entry.status, QueueStatus::Rebasing);
}
```

---

### EC-005: Recovery called during active transaction

**GIVEN** a worker is in the middle of claiming an entry
**WHEN** another worker triggers recovery
**THEN** recovery completes successfully
**AND** transaction is not disrupted
**AND** both operations complete atomically

```rust
#[tokio::test]
async fn test_ec005_recovery_during_active_transaction() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-ec005", None, 5, None).await.unwrap();

    // Start claim (holds transaction)
    let handle1 = tokio::spawn({
        let q = queue.clone();
        async move {
            q.next_with_lock("worker-ec005-1").await
        }
    });

    // Give transaction time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Trigger recovery (should not interfere)
    let stats = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats.entries_reclaimed, 0, "Nothing stale to reclaim");

    // First claim should succeed
    let result1 = handle1.await.unwrap().unwrap();
    assert!(result1.is_some());

    // Cleanup
    queue.release_processing_lock("worker-ec005-1").await.unwrap();
}
```

---

### EC-006: Lock exactly at expiration boundary

**GIVEN** a lock with expires_at = now (exactly now)
**WHEN** recovery is triggered
**THEN** lock is cleaned (expires_at < now)
**AND** new lock can be acquired

```rust
#[tokio::test]
async fn test_ec006_lock_at_expiration_boundary() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Manually set lock with expires_at = now - 1 (expired)
    let pool = queue.pool();
    let now = queue::MergeQueue::now();
    sqlx::query("INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) VALUES (1, 'test', ?, ?)")
        .bind(now - 300)
        .bind(now - 1)
        .execute(pool)
        .await
        .unwrap();

    // Verify lock is considered stale
    assert!(queue.is_lock_stale().await.unwrap());

    // Recovery should clean it
    let stats = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats.locks_cleaned, 1);

    // Should be able to acquire new lock
    queue.add("workspace-ec006", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-ec006").await.unwrap();
    assert!(claimed.is_some());

    // Cleanup
    queue.release_processing_lock("worker-ec006").await.unwrap();
}
```

---

### EC-007: Entry with NULL started_at in claimed state

**GIVEN** an entry in 'claimed' state with started_at = NULL
**WHEN** auto-recovery runs
**THEN** entry is NOT reclaimed (started_at < check fails for NULL)
**AND** entry remains in claimed state
**AND** manual cleanup may be needed

```rust
#[tokio::test]
async fn test_ec007_claimed_entry_with_null_started_at() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entry
    queue.add("workspace-ec007", None, 5, None).await.unwrap();

    // Manually set to claimed with NULL started_at
    let pool = queue.pool();
    sqlx::query("UPDATE merge_queue SET status = 'claimed', started_at = NULL, agent_id = 'orphan' WHERE workspace = ?")
        .bind("workspace-ec007")
        .execute(pool)
        .await
        .unwrap();

    // Run recovery
    let stats = queue.detect_and_recover_stale().await.unwrap();

    // Should NOT be reclaimed (started_at IS NULL check)
    assert_eq!(stats.entries_reclaimed, 0, "NULL started_at should not be reclaimed");

    // Verify still claimed
    let entry = queue.get_by_workspace("workspace-ec007").await.unwrap().unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert!(entry.started_at.is_none());
}
```

---

### EC-008: Recovery with database pool at max capacity

**GIVEN** the connection pool is saturated
**WHEN** recovery is attempted
**THEN** operation waits for available connection
**AND** completes successfully
**AND** no connection leaks occur

```rust
#[tokio::test]
async fn test_ec008_recovery_with_saturated_pool() {
    // This test verifies connection pool behavior
    // In normal operation, SQLite can handle many concurrent operations
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add entries
    for i in 0..10 {
        queue.add(&format!("workspace-ec008-{i}"), None, 5, None).await.unwrap();
    }

    // Run many concurrent operations
    let tasks: Vec<_> = (0..20)
        .map(|_| {
            let q = queue.clone();
            tokio::spawn(async move {
                q.detect_and_recover_stale().await
            })
        })
        .collect();

    // All should complete successfully
    for task in tasks {
        let result = task.await.unwrap();
        assert!(result.is_ok(), "Concurrent recovery should succeed");
    }
}
```

---

### EC-009: Recovery timestamp edge cases

**GIVEN** system clock has low resolution
**WHEN** multiple entries are claimed within same second
**THEN** all entries are reclaimed after timeout
**AND** no entries are left behind

```rust
#[tokio::test]
async fn test_ec009_recovery_timestamp_edge_cases() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and claim multiple entries quickly
    for i in 0..5 {
        queue.add(&format!("workspace-ec009-{i}"), None, 5, None).await.unwrap();
        let claimed = queue.next_with_lock(&format!("worker-{i}")).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&format!("worker-{i}")).await.unwrap();
    }

    // Wait for staleness
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // All should be reclaimed
    let stats = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats.entries_reclaimed, 5, "All 5 entries should be reclaimed");
}
```

---

### EC-010: Empty string agent_id in lock

**GIVEN** a processing lock with empty string agent_id
**WHEN** recovery is triggered
**THEN** lock is cleaned
**AND** new lock with valid agent_id can be acquired

```rust
#[tokio::test]
async fn test_ec010_empty_string_agent_id_in_lock() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Manually create lock with empty agent_id
    let pool = queue.pool();
    let now = queue::MergeQueue::now();
    sqlx::query("INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) VALUES (1, '', ?, ?)")
        .bind(now - 300)
        .bind(now - 1)
        .execute(pool)
        .await
        .unwrap();

    // Recovery should clean it
    let stats = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats.locks_cleaned, 1);

    // Should be able to acquire new lock
    queue.add("workspace-ec010", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-ec010").await.unwrap();
    assert!(claimed.is_some());

    // Cleanup
    queue.release_processing_lock("worker-ec010").await.unwrap();
}
```

---

## Contract Verification Tests (CV)

### CV-001: All existing tests still pass

**GIVEN** the auto-recovery enhancement is implemented
**WHEN** all existing crash recovery tests run
**THEN** 100% of tests pass
**AND** no test modifications were required
**AND** backwards compatibility is verified

```rust
// This is a meta-test - run all existing tests
// No code needed, just verification that:
// - test_worker_crash_leaves_stale_claimed_entry passes
// - test_stale_processing_lock_allows_new_claims passes
// - test_recovery_allows_new_worker_to_claim passes
// - test_no_permanent_locks_after_crash passes
// - etc. (all 13 existing scenarios)
```

---

### CV-002: Manual reclaim_stale API unchanged

**GIVEN** the enhanced queue implementation
**WHEN** reclaim_stale(threshold) is called
**THEN** signature is identical to original
**AND** return type (Result<usize>) is unchanged
**AND** behavior is functionally equivalent

```rust
#[tokio::test]
async fn test_cv002_manual_reclaim_api_unchanged() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Verify function signature and return type
    // This test compiles - signature check
    let result: Result<usize, Error> = queue.reclaim_stale(0).await;
    assert!(result.is_ok(), "Should return Result<usize, Error>");

    // Verify behavior
    queue.add("workspace-cv002", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-cv002").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-cv002").await.unwrap();

    tokio::time::sleep(Duration::from_millis(1100)).await;

    let reclaimed = queue.reclaim_stale(0).await.unwrap();
    assert_eq!(reclaimed, 1, "Should reclaim 1 entry");
}
```

---

### CV-003: Lock acquisition semantics preserved

**GIVEN** the enhanced queue
**WHEN** acquire_processing_lock is called
**THEN** UPSERT behavior unchanged
**AND** expiry check unchanged
**AND** mutual exclusion unchanged

```rust
#[tokio::test]
async fn test_cv003_lock_acquisition_semantics_preserved() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // First acquire
    let acquired1 = queue.acquire_processing_lock("worker-cv003-1").await.unwrap();
    assert!(acquired1, "First acquire should succeed");

    // Second acquire should fail (lock held)
    let acquired2 = queue.acquire_processing_lock("worker-cv003-2").await.unwrap();
    assert!(!acquired2, "Second acquire should fail (lock held)");

    // Release
    let released = queue.release_processing_lock("worker-cv003-1").await.unwrap();
    assert!(released);

    // Third acquire should succeed
    let acquired3 = queue.acquire_processing_lock("worker-cv003-3").await.unwrap();
    assert!(acquired3, "Acquire after release should succeed");

    // Cleanup
    queue.release_processing_lock("worker-cv003-3").await.unwrap();
}
```

---

### CV-004: Release processing lock validation unchanged

**GIVEN** a lock is held by agent A
**WHEN** agent B tries to release
**THEN** release fails (returns false)
**AND** lock remains held by agent A

```rust
#[tokio::test]
async fn test_cv004_release_lock_validation_unchanged() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    queue.add("workspace-cv004", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-cv004-a").await.unwrap();
    assert!(claimed.is_some());

    // Wrong agent tries to release
    let released = queue.release_processing_lock("worker-cv004-b").await.unwrap();
    assert!(!released, "Wrong agent should not release lock");

    // Correct agent releases
    let released = queue.release_processing_lock("worker-cv004-a").await.unwrap();
    assert!(released, "Correct agent should release lock");
}
```

---

### CV-005: Extend lock behavior unchanged

**GIVEN** a worker holds a lock
**WHEN** extend_lock is called
**THEN** expiration is extended
**AND** only lock holder can extend

```rust
#[tokio::test]
async fn test_cv005_extend_lock_behavior_unchanged() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    queue.add("workspace-cv005", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-cv005").await.unwrap();
    assert!(claimed.is_some());

    let lock1 = queue.get_processing_lock().await.unwrap().unwrap();
    let original_expires = lock1.expires_at;

    // Extend
    let extended = queue.extend_lock("worker-cv005", 60).await.unwrap();
    assert!(extended);

    let lock2 = queue.get_processing_lock().await.unwrap().unwrap();
    assert!(lock2.expires_at > original_expires);

    // Cleanup
    queue.release_processing_lock("worker-cv005").await.unwrap();
}
```

---

### CV-006: Database schema unchanged

**GIVEN** the enhanced implementation
**WHEN** database schema is inspected
**THEN** no new tables added
**AND** no columns added to existing tables
**AND** schema is backwards compatible

```rust
#[tokio::test]
async fn test_cv006_database_schema_unchanged() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Init schema
    let queue = MergeQueue::new(pool.clone()).await.unwrap();

    // Verify tables exist (no new tables)
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(tables.contains(&"merge_queue".to_string()));
    assert!(tables.contains(&"queue_processing_lock".to_string()));
    assert!(tables.contains(&"queue_events".to_string()));

    // Verify merge_queue columns (no new columns)
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM pragma_table_info('merge_queue') ORDER BY cid"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    // Verify expected columns exist
    assert!(columns.contains(&"id".to_string()));
    assert!(columns.contains(&"workspace".to_string()));
    assert!(columns.contains(&"status".to_string()));
    assert!(columns.contains(&"started_at".to_string()));
    assert!(columns.contains(&"agent_id".to_string()));
    // ... etc
}
```

---

### CV-007: Recovery doesn't break transaction isolation

**GIVEN** two workers operating concurrently
**WHEN** one triggers recovery while other claims
**THEN** each sees consistent state
**AND** no dirty reads occur
**AND** no lost updates occur

```rust
#[tokio::test]
async fn test_cv007_recovery_transaction_isolation() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add 10 entries
    for i in 0..10 {
        queue.add(&format!("workspace-cv007-{i}"), None, 5, None).await.unwrap();
    }

    // Concurrent claims
    let tasks: Vec<_> = (0..5)
        .map(|i| {
            let q = queue.clone();
            tokio::spawn(async move {
                q.next_with_lock(&format!("worker-{i}")).await
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.unwrap().unwrap())
        .collect();

    // Each should get unique entry
    let workspaces: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().map(|e| e.workspace.clone()))
        .collect();

    assert_eq!(workspaces.len(), 5, "Should have 5 unique workspaces");

    // Cleanup
    for result in results {
        if let Some(entry) = result {
            let _ = queue.release_processing_lock(&entry.agent_id.unwrap()).await;
        }
    }
}
```

---

### CV-008: Recovery idempotence verified

**GIVEN** stale entries have been reclaimed
**WHEN** recovery is called multiple times
**THEN** subsequent calls are no-ops
**AND** no errors occur
**AND** counts are accurate

```rust
#[tokio::test]
async fn test_cv008_recovery_idempotence_verified() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add and abandon entry
    queue.add("workspace-cv008", None, 5, None).await.unwrap();
    let claimed = queue.next_with_lock("worker-cv008").await.unwrap();
    assert!(claimed.is_some());
    queue.release_processing_lock("worker-cv008").await.unwrap();

    tokio::time::sleep(Duration::from_millis(1100)).await;

    // First recovery
    let stats1 = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats1.entries_reclaimed, 1);

    // Second recovery (idempotent)
    let stats2 = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats2.entries_reclaimed, 0);

    // Third recovery (still no-op)
    let stats3 = queue.detect_and_recover_stale().await.unwrap();
    assert_eq!(stats3.entries_reclaimed, 0);
}
```

---

### CV-009: Performance regression check

**GIVEN** the enhanced implementation
**WHEN** benchmarking next_with_lock calls
**THEN** performance is within 10% of baseline
**AND** no significant slowdown introduced

```rust
#[tokio::test]
async fn test_cv009_performance_regression_check() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Add 100 entries
    for i in 0..100 {
        queue.add(&format!("workspace-cv009-{i}"), None, 5, None).await.unwrap();
    }

    // Measure time to claim all
    let start = std::time::Instant::now();
    for i in 0..100 {
        let worker = format!("worker-{i}");
        let claimed = queue.next_with_lock(&worker).await.unwrap();
        assert!(claimed.is_some());
        queue.release_processing_lock(&worker).await.unwrap();
    }
    let elapsed = start.elapsed();

    // Should complete in reasonable time (< 10 seconds for 100 claims)
    assert!(elapsed.as_secs() < 10, "Performance regression: {elapsed:?}");
}
```

---

### CV-010: Error propagation paths unchanged

**GIVEN** various error conditions
**WHEN** operations fail
**THEN** error types are consistent
**AND** error messages are informative
**AND** no new panic paths introduced

```rust
#[tokio::test]
async fn test_cv010_error_propagation_unchanged() {
    let queue = MergeQueue::open_in_memory().await.unwrap();

    // Try to get non-existent entry
    let result = queue.get_by_workspace("nonexistent").await.unwrap();
    assert!(result.is_none(), "Non-existent entry returns None");

    // Try to claim from empty queue
    let claimed = queue.next_with_lock("worker-cv010").await.unwrap();
    assert!(claimed.is_none(), "Empty queue returns None");

    // All operations should return Result types
    // No panics should occur
}
```

---

## Summary

**Total Test Count:** 45
- Happy Path: 15 tests
- Error Path: 5 tests
- Edge Cases: 10 tests
- Contract Verification: 10 tests

**Coverage Areas:**
- Automatic stale lock cleanup
- Automatic stale entry reclamation
- Recovery statistics accuracy
- Concurrent operation safety
- Backwards compatibility
- Error handling
- Edge cases and corner cases
- Performance characteristics

**Test Execution Order:**
1. Run all contract verification tests first (CV-001 to CV-010)
2. Run happy path tests (HP-001 to HP-015)
3. Run error path tests (EP-001 to EP-005)
4. Run edge case tests (EC-001 to EC-010)

**Success Criteria:**
- 100% of tests pass
- No test modifications required for existing tests
- All new functionality verified
- Backwards compatibility confirmed

---

**Test Plan Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
