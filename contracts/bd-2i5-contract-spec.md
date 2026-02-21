# Contract Specification: bd-2i5 - Implement self-healing for stale processing locks

**Bead ID:** bd-2i5
**Title:** Implement self-healing for stale processing locks
**Status:** Design Contract
**Version:** 1.0

## Overview

This contract defines the implementation of automatic self-healing mechanisms for stale processing locks in the merge queue system. The current implementation requires manual intervention via `reclaim_stale()` calls to recover from worker crashes. This enhancement introduces automatic stale lock detection and recovery.

### Current State Analysis

**Existing Infrastructure:**
- `queue_processing_lock` table with `expires_at` timestamp (300s default TTL)
- `acquire_processing_lock()` - uses UPSERT with expiry check
- `release_processing_lock()` - deletes lock by agent_id
- `get_processing_lock()` - retrieves current lock
- `extend_lock()` - refreshes lock expiry
- `reclaim_stale()` - manual cleanup of stale entries and expired locks
- Worker crash recovery tests in `test_worker_crash_recovery.rs`

**Current Limitations:**
1. **No automatic stale lock detection** - locks expire but aren't automatically cleaned
2. **No automatic entry reclamation** - claimed entries stay claimed after crash
3. **Manual recovery required** - depends on external caller to invoke `reclaim_stale()`
4. **Race condition window** - expired locks exist until next acquire attempt
5. **No monitoring/visibility** - stale locks aren't logged or reported

**Failure Scenarios:**
1. Worker crashes while holding processing lock
2. Worker process killed (SIGKILL) without cleanup
3. Network partition prevents lock release
4. Machine failure during processing
5. Lock TTL expires but entry remains "claimed" indefinitely

### Scope of Changes

**Affected Components:**
- `crates/zjj-core/src/coordination/queue.rs` - Add self-healing methods
- `crates/zjj-core/src/coordination/queue_repository.rs` - Add repository trait methods
- `crates/zjj-core/tests/test_worker_crash_recovery.rs` - Add automated recovery tests

**NOT Affected:**
- `crates/zjj-core/src/coordination/locks.rs` - Session locks (separate system)
- `crates/zjj/src/commands/queue_worker.rs` - Worker logic unchanged
- Database schema - No new tables required

### Key Behavioral Changes

**Before:** Stale locks require manual `reclaim_stale()` invocation
**After:** Stale locks are automatically detected and cleaned on lock acquisition

## Preconditions

### Global Preconditions (MUST hold before execution)

1. **Queue Schema Exists**
   - `merge_queue` table with `status`, `started_at`, `agent_id` columns
   - `queue_processing_lock` table with single row (id=1)
   - Lock timeout configured (default: 300 seconds)
   - **Violation:** Schema migration required

2. **Existing Lock Infrastructure**
   - `acquire_processing_lock()` uses UPSERT with expiry check
   - `release_processing_lock()` validates agent_id before deletion
   - `get_processing_lock()` returns current lock state
   - `Status:** VERIFIED - Implementation exists

3. **Reclaim Implementation**
   - `reclaim_stale()` resets entries older than threshold
   - Deletes expired locks from `queue_processing_lock`
   - Updates `state_changed_at` on reclaim
   - **Status:** VERIFIED - Manual reclaim works

4. **Worker Lifecycle Integration**
   - Workers call `next_with_lock()` to claim work
   - Workers call `release_processing_lock()` on completion
   - Workers may crash without cleanup
   - **Status:** VERIFIED - Worker patterns identified

### Mode-Specific Preconditions

5. **No In-Flight Processing During Migration**
   - No active workers claiming entries
   - No locked entries in processing state
   - **Violation:** Coordinated deployment required

## Postconditions

### Success Postconditions (MUST hold after successful execution)

1. **Automatic Stale Lock Detection**
   - `next_with_lock()` checks for expired locks before acquisition
   - Expired locks are deleted automatically
   - Log entry created when stale lock detected
   - Detection threshold = `lock_timeout_secs` (default 300s)

2. **Automatic Entry Reclamation**
   - `next_with_lock()` checks for stale claimed entries
   - Entries with `started_at < now - lock_timeout_secs` reset to pending
   - `agent_id`, `started_at` cleared on reclaim
   - `state_changed_at` updated on reclaim

3. **Lock Acquisition Enhanced**
   - Existing `acquire_processing_lock()` behavior preserved
   - Additional automatic cleanup before acquisition
   - Retry logic handles transient contention
   - UPSERT with expiry check remains atomic

4. **Backwards Compatibility**
   - `reclaim_stale()` method still works (manual reclaim still supported)
   - Existing worker code unchanged
   - Lock timeout configuration unchanged
   - Database schema unchanged

5. **Observability Improvements**
   - New method: `detect_and_recover_stale()` - explicit recovery call
   - Returns recovery statistics (locks cleaned, entries reclaimed)
   - Audit log entries for self-healing actions
   - No breaking changes to existing API

### Special Case Postconditions

6. **Concurrent Safety**
   - Multiple workers can safely detect and recover
   - No duplicate reclamation of same entry
   - No race conditions between detection and claim
   - Transaction boundaries prevent lost updates

7. **Error Handling**
   - Database errors during recovery don't prevent lock acquisition
   - Recovery failures are logged but don't fail the operation
   - Existing error propagation patterns preserved

## Invariants

### Always True (during and after execution)

1. **Lock Expiry Semantics**
   - Lock expires at `expires_at` timestamp (not before)
   - Expired locks can be safely acquired by any worker
   - Active locks cannot be stolen (UPSERT guard)

2. **Entry State Consistency**
   - `status = 'claimed'` implies `started_at IS NOT NULL`
   - `started_at` always <= `state_changed_at`
   - `agent_id` present iff `status IN ('claimed', 'rebasing', 'testing', ...)`
   - Reclaim preserves `bead_id`, `priority`, `workspace`

3. **Mutual Exclusion**
   - Only one worker holds processing lock at a time
   - Only one `next_with_lock()` succeeds per entry
   - Transaction boundaries prevent double-claim

4. **Idempotence**
   - Calling recovery multiple times is safe
   - Reclaiming already-reclaimed entry is no-op
   - Releasing already-released lock returns false (no error)

5. **Backwards Compatibility**
   - All existing tests continue to pass
   - Manual `reclaim_stale()` still works
   - Worker lifecycle unchanged
   - No changes to public API signatures

## Error Taxonomy

### Exhaustive Error Variants

```rust
// Existing errors preserved - no new error types introduced

use crate::Error;

// Recovery-specific error cases use existing Error variants:
impl From<RecoveryError> for Error {
    fn from(err: RecoveryError) -> Self {
        match err {
            RecoveryError::DatabaseError(msg) =>
                Error::DatabaseError(format!("Recovery failed: {msg}")),
            RecoveryError::TransactionError(msg) =>
                Error::DatabaseError(format!("Recovery transaction failed: {msg}")),
        }
    }
}

// Recovery statistics (return type, not error):
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    pub locks_cleaned: usize,
    pub entries_reclaimed: usize,
    pub recovery_timestamp: i64,
}
```

### Error Propagation Mapping

```rust
// All existing error paths preserved:
// - DatabaseError for SQL failures
// - NotFound for missing entries
// - SessionLocked for lock contention

// New recovery path failures are non-fatal:
impl MergeQueue {
    pub async fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
        // 1. Attempt automatic recovery (best-effort, logged on failure)
        let _ = self.detect_and_recover_stale().await;

        // 2. Proceed with normal lock acquisition
        // ... existing logic unchanged ...
    }
}
```

## Function Signatures

### New Functions

```rust
/// Detect and automatically recover stale locks and entries
///
/// This is called automatically by `next_with_lock()` before attempting
/// to claim work. It can also be called explicitly for monitoring purposes.
///
/// # Returns
/// Statistics about recovery actions performed
pub async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
    let now = Self::now();

    // 1. Delete expired processing locks
    let locks_cleaned = sqlx::query(
        "DELETE FROM queue_processing_lock WHERE expires_at < ?1"
    )
    .bind(now)
    .execute(&self.pool)
    .await?
    .rows_affected();

    // 2. Reset stale claimed entries to pending
    let cutoff = now - self.lock_timeout_secs;
    let entries_reclaimed = sqlx::query(
        "UPDATE merge_queue
         SET status = 'pending',
             started_at = NULL,
             agent_id = NULL,
             state_changed_at = ?1
         WHERE status = 'claimed'
           AND started_at IS NOT NULL
           AND started_at < ?2"
    )
    .bind(now)
    .bind(cutoff)
    .execute(&self.pool)
    .await?
    .rows_affected();

    Ok(RecoveryStats {
        locks_cleaned: locks_cleaned as usize,
        entries_reclaimed: entries_reclaimed as usize,
        recovery_timestamp: now,
    })
}

/// Get recovery statistics without performing recovery
pub async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
    // Count expired locks and stale entries
    // Return stats without performing cleanup
}

/// Check if a specific lock is stale
pub async fn is_lock_stale(&self) -> Result<bool> {
    let lock = self.get_processing_lock().await?;
    let now = Self::now();

    Ok(match lock {
        Some(l) => l.expires_at < now,
        None => false, // No lock = not stale
    })
}
```

### Modified Functions

```rust
/// enhanced with automatic recovery call
pub async fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
    const MAX_RETRIES: u32 = 5;
    const INITIAL_BACKOFF_MS: u64 = 50;

    let mut attempt = 0;
    let mut backoff_ms = INITIAL_BACKOFF_MS;

    loop {
        attempt += 1;

        // NEW: Automatic recovery before claim attempt
        // Best-effort - failures logged but don't prevent claim
        if let Err(e) = self.detect_and_recover_stale().await {
            eprintln!("Warning: Automatic recovery failed: {e}");
            // Continue anyway - lock acquisition may still succeed
        }

        // Existing logic unchanged
        let result = self.try_claim_next_entry(agent_id).await;

        match &result {
            Ok(_entry) => return result,
            Err(e) => {
                // Existing retry logic unchanged
                let error_str = e.to_string();
                let is_retryable = error_str.contains("UNIQUE constraint")
                    || error_str.contains("database is locked");

                if is_retryable && attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = backoff_ms.saturating_mul(2);
                    continue;
                }
                return result;
            }
        }
    }
}

/// Repository trait extended
#[async_trait]
impl QueueRepository for MergeQueue {
    // Existing methods unchanged...

    // NEW: Recovery methods
    async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
        self.detect_and_recover_stale().await
    }

    async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
        self.get_recovery_stats().await
    }
}
```

### Unchanged Functions

```rust
// These functions remain EXACTLY as they are:
pub async fn acquire_processing_lock(&self, agent_id: &str) -> Result<bool>;
pub async fn release_processing_lock(&self, agent_id: &str) -> Result<bool>;
pub async fn get_processing_lock(&self) -> Result<Option<ProcessingLock>>;
pub async fn extend_lock(&self, agent_id: &str, extra_secs: i64) -> Result<bool>;
pub async fn reclaim_stale(&self, stale_threshold_secs: i64) -> Result<usize>;
```

## Behavioral Change Summary

### New Behavior

1. **Automatic Stale Lock Cleanup**
   - `next_with_lock()` calls `detect_and_recover_stale()` before claiming
   - Expired locks deleted automatically
   - Stale claimed entries reset to pending automatically
   - No manual intervention required

2. **Recovery Statistics**
   - `RecoveryStats` struct tracks recovery actions
   - Counts locks cleaned and entries reclaimed
   - Timestamp of recovery operation
   - Useful for monitoring and debugging

3. **Enhanced Observability**
   - `get_recovery_stats()` for monitoring (non-destructive)
   - `is_lock_stale()` for health checks
   - Warnings logged on recovery failures (non-fatal)
   - No breaking changes to existing API

4. **Explicit Recovery Call**
   - `detect_and_recover_stale()` can be called explicitly
   - Useful for periodic maintenance jobs
   - Useful for monitoring dashboards
   - Returns detailed statistics

### Retained Behavior

1. **Lock Acquisition Logic**
   - UPSERT with expiry check unchanged
   - Transaction boundaries unchanged
   - Retry logic unchanged
   - Mutual exclusion guarantees unchanged

2. **Manual Reclaim**
   - `reclaim_stale()` still works exactly as before
   - Can be called with custom threshold
   - Returns count of entries reclaimed
   - No changes to API or behavior

3. **Worker Lifecycle**
   - Workers call `next_with_lock()` as before
   - Workers call `release_processing_lock()` as before
   - Workers can still crash without cleanup
   - Recovery is now automatic, not manual

4. **Error Handling**
   - All existing error types preserved
   - Error propagation paths unchanged
   - Recovery failures are non-fatal
   - Database errors handled as before

### Code Changes Required

#### File: `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs`

**Add new struct after line 68 (after QueueStats):**
```rust
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    pub locks_cleaned: usize,
    pub entries_reclaimed: usize,
    pub recovery_timestamp: i64,
}
```

**Add new methods after `reclaim_stale()` (after line 1058):**
```rust
/// Detect and automatically recover stale locks and entries
///
/// This is called automatically by `next_with_lock()` before attempting
/// to claim work. It can also be called explicitly for monitoring purposes.
///
/// # Returns
/// Statistics about recovery actions performed
#[allow(clippy::cast_sign_loss)]
pub async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
    let now = Self::now();

    // Delete expired processing locks
    let locks_cleaned = sqlx::query(
        "DELETE FROM queue_processing_lock WHERE expires_at < ?1"
    )
    .bind(now)
    .execute(&self.pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to clean expired locks: {e}")))?
    .rows_affected();

    // Reset stale claimed entries to pending
    let cutoff = now - self.lock_timeout_secs;
    let entries_reclaimed = sqlx::query(
        "UPDATE merge_queue
         SET status = 'pending',
             started_at = NULL,
             agent_id = NULL,
             state_changed_at = ?1
         WHERE status = 'claimed'
           AND started_at IS NOT NULL
           AND started_at < ?2"
    )
    .bind(now)
    .bind(cutoff)
    .execute(&self.pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to reclaim stale entries: {e}")))?
    .rows_affected();

    Ok(RecoveryStats {
        locks_cleaned: locks_cleaned as usize,
        entries_reclaimed: entries_reclaimed as usize,
        recovery_timestamp: now,
    })
}

/// Get recovery statistics without performing recovery
#[allow(clippy::cast_sign_loss)]
pub async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
    let now = Self::now();

    // Count expired locks (without deleting)
    let locks_cleaned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM queue_processing_lock WHERE expires_at < ?1"
    )
    .bind(now)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to count expired locks: {e}")))?;

    // Count stale entries (without reclaiming)
    let cutoff = now - self.lock_timeout_secs;
    let entries_reclaimed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM merge_queue
         WHERE status = 'claimed'
           AND started_at IS NOT NULL
           AND started_at < ?1"
    )
    .bind(cutoff)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to count stale entries: {e}")))?;

    Ok(RecoveryStats {
        locks_cleaned: locks_cleaned as usize,
        entries_reclaimed: entries_reclaimed as usize,
        recovery_timestamp: now,
    })
}

/// Check if the processing lock is stale
pub async fn is_lock_stale(&self) -> Result<bool> {
    let lock = self.get_processing_lock().await?;
    let now = Self::now();

    Ok(match lock {
        Some(l) => l.expires_at < now,
        None => false,
    })
}
```

**Modify `next_with_lock()` (around line 834):**
```rust
pub async fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
    const MAX_RETRIES: u32 = 5;
    const INITIAL_BACKOFF_MS: u64 = 50;

    let mut attempt = 0;
    let mut backoff_ms = INITIAL_BACKOFF_MS;

    loop {
        attempt += 1;

        // NEW: Automatic recovery before claim attempt
        if let Err(e) = self.detect_and_recover_stale().await {
            eprintln!("Warning: Automatic recovery failed: {e}");
            // Continue anyway - lock acquisition may still succeed
        }

        // Existing logic unchanged from here...
        let result = self.try_claim_next_entry(agent_id).await;

        match &result {
            Ok(_entry) => {
                return result;
            }
            Err(e) => {
                let error_str = e.to_string();
                let is_constraint_violation = error_str.contains("UNIQUE constraint failed")
                    || error_str.contains("constraint");
                let is_db_locked = error_str.contains("database is locked")
                    || error_str.contains("database table is locked")
                    || error_str.contains("SQLITE_BUSY");

                if (is_constraint_violation || is_db_locked) && attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = backoff_ms.saturating_mul(2);
                    continue;
                }

                return result;
            }
        }
    }
}
```

#### File: `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_repository.rs`

**Add to trait (around line 100):**
```rust
async fn detect_and_recover_stale(&self) -> Result<RecoveryStats>;
async fn get_recovery_stats(&self) -> Result<RecoveryStats>;
```

**Add to impl (around line 1838):**
```rust
async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
    self.detect_and_recover_stale().await
}

async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
    self.get_recovery_stats().await
}
```

#### File: `/home/lewis/src/zjj/crates/zjj-core/tests/test_worker_crash_recovery.rs`

**Add new tests (see test plan)**

## Migration Notes

### Breaking Changes

**None** - This is a pure enhancement with full backwards compatibility

### Backwards Compatibility

1. **Manual Reclaim Still Works**
   - `reclaim_stale()` unchanged and functional
   - Custom threshold values still supported
   - Can be called alongside automatic recovery

2. **Worker Code Unchanged**
   - Workers continue using `next_with_lock()`
   - No changes to worker lifecycle
   - No changes to lock acquisition API

3. **Existing Tests Pass**
   - All current crash recovery tests pass
   - No test modifications required
   - New tests enhance coverage

### Testing Strategy

See `/home/lewis/src/zjj/contracts/bd-2i5-martin-fowler-tests.md` for comprehensive test plan covering:

- Automatic stale lock detection
- Automatic entry reclamation
- Recovery statistics accuracy
- Concurrent recovery safety
- Backwards compatibility
- Edge cases and error handling

## Verification Steps

### Manual Verification

1. **Verify automatic recovery:**
   ```bash
   # Run existing worker crash recovery tests
   cargo test test_worker_crash_recovery -p zjj-core

   # Verify tests still pass (manual reclaim works)
   cargo test test_stale_processing_lock_allows_new_claims -p zjj-core
   ```

2. **Verify new automatic recovery:**
   ```bash
   # Run new self-healing tests
   cargo test test_automatic_recovery_on_claim -p zjj-core
   cargo test test_recovery_stats_accuracy -p zjj-core
   ```

3. **Verify concurrent safety:**
   ```bash
   # Run concurrent recovery tests
   cargo test test_concurrent_automatic_recovery -p zjj-core
   ```

### Automated Verification

Run tests from `/home/lewis/src/zjj/contracts/bd-2i5-martin-fowler-tests.md`

## Impact Assessment

### Dependencies Added

**None** - Uses existing dependencies (sqlx, chrono, tokio)

### Code Changes

- **Files Modified:** 3
  - `crates/zjj-core/src/coordination/queue.rs` (~100 lines added)
  - `crates/zjj-core/src/coordination/queue_repository.rs` (~10 lines added)
  - `crates/zjj-core/tests/test_worker_crash_recovery.rs` (~500 lines added)

- **Lines Changed:** ~610
  - 1 new struct (RecoveryStats)
  - 3 new methods (~90 lines)
  - 1 modified method (~10 lines)
  - Repository trait updates (~10 lines)
  - Comprehensive tests (~500 lines)

### Risk Assessment

- **Risk Level:** LOW
  - Pure enhancement, no breaking changes
  - Backwards compatible
  - Existing tests pass unchanged
  - Recovery is best-effort (failures don't break normal flow)

- **Rollback:** Simple (remove automatic recovery call from `next_with_lock()`)

### Performance Impact

- **Minimal:** One additional SQL query per claim attempt
- **Query Cost:** O(1) DELETE + O(n) UPDATE where n = stale entries
- **Benefit:** Eliminates manual intervention, improves system resilience

---

**Contract Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
