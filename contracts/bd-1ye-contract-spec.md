# Contract Specification: End-to-End Tests for Merge Train Processing (bd-1ye)

**Bead ID:** bd-1ye
**Title:** Create end-to-end tests for merge train processing
**Version:** 1.0.0
**Status:** Draft

---

## 1. Overview

This document specifies the Design by Contract requirements for end-to-end testing of the merge train processing system in zjj. The merge train implements Graphite-style sequential merging where queue entries are processed one at a time in position order, with automatic testing, conflict detection, and failure recovery.

### 1.1 Scope

The contract covers:
- Merge train processing workflow from pending entries to merged
- State transitions during train processing
- Train failure detection and auto-rebase recovery
- JSONL output emission (TrainStep, TrainResult, Train)
- Test execution and conflict detection
- Merge operations with rollback on failure
- Kick functionality for failed sessions
- Position reassignment after failures

### 1.2 Graphite-Style Merge Train Semantics

This system implements Graphite-style merge train semantics:
1. **Sequential Processing:** Entries processed one at a time in position order
2. **State Machine Progression:** pending -> claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
3. **Failure Detection:** Test failures or conflicts stop the train
4. **Auto-Rebase Recovery:** Failed entries are kicked, subsequent entries rebased
5. **Position Recalculation:** Positions updated after kicks and rebases
6. **JSONL Audit Trail:** Every step emits structured output

---

## 2. Type Definitions

### 2.1 TrainProcessingError (Exhaustive Error Taxonomy)

```rust
/// Semantic error variants for merge train processing operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainProcessingError {
    // === Queue State Errors ===
    /// No pending entries to process
    NoPendingEntries,

    /// Entry not found in queue
    EntryNotFound {
        entry_id: i64,
    },

    /// Entry is not in expected state for train processing
    InvalidEntryState {
        entry_id: i64,
        current_state: QueueStatus,
        expected_state: QueueStatus,
    },

    /// Train is already processing
    TrainAlreadyActive {
        agent_id: String,
        acquired_at: i64,
    },

    /// Failed to acquire processing lock
    LockAcquisitionFailed {
        reason: String,
    },

    // === Test Execution Errors ===
    /// Test execution failed
    TestExecutionFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
        exit_code: Option<i32>,
    },

    /// Tests timed out
    TestTimeout {
        entry_id: i64,
        workspace: String,
        timeout_secs: i64,
    },

    /// Test framework not configured
    TestFrameworkNotConfigured {
        workspace: String,
    },

    // === Conflict Detection Errors ===
    /// Merge conflicts detected
    MergeConflictsDetected {
        entry_id: i64,
        workspace: String,
        conflict_count: usize,
    },

    /// Conflict detection failed
    ConflictDetectionFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
    },

    // === Merge Operation Errors ===
    /// Merge operation failed
    MergeFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
    },

    /// Merge verification failed
    MergeVerificationFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
    },

    /// Bookmark update failed after merge
    BookmarkUpdateFailed {
        entry_id: i64,
        workspace: String,
        bookmark: String,
        reason: String,
    },

    // === Rebase Operation Errors ===
    /// Rebase operation failed
    RebaseFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
    },

    /// Rebase conflict detected
    RebaseConflictDetected {
        entry_id: i64,
        workspace: String,
        conflict_count: usize,
    },

    /// Rebase verification failed
    RebaseVerificationFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
    },

    // === Kick Operation Errors ===
    /// Failed to kick entry from queue
    KickFailed {
        entry_id: i64,
        workspace: String,
        reason: String,
    },

    /// Position update failed after kick
    PositionUpdateFailed {
        reason: String,
    },

    // === Database Errors ===
    /// Database transaction failed
    TransactionFailed {
        operation: String,
        reason: String,
    },

    /// Concurrent modification detected
    ConcurrentModification {
        entry_id: i64,
        operation: String,
    },

    // === JJ Integration Errors ===
    /// JJ command execution failed
    JjExecutionFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    /// Failed to get current revision
    RevisionQueryFailed {
        workspace: String,
        reason: String,
    },

    // === Output Errors ===
    /// Failed to emit JSONL output
    OutputEmissionFailed {
        output_type: String,
        reason: String,
    },

    // === Recovery Errors ===
    /// Train recovery failed
    TrainRecoveryFailed {
        reason: String,
    },

    /// Cannot recover from multiple failures
    MultipleUnrecoverableFailures {
        failed_count: usize,
    },
}
```

### 2.2 TrainProcessor

```rust
/// Merge train processor
///
/// Processes queue entries sequentially in position order,
/// implementing Graphite-style merge train semantics.
pub struct TrainProcessor {
    db: SqlitePool,
    lock_timeout_secs: i64,
    test_timeout_secs: i64,
}

impl TrainProcessor {
    /// Create a new train processor
    pub fn new(db: SqlitePool) -> Self;

    /// Process all pending entries in the merge queue
    ///
    /// # Preconditions
    /// - Database connection is valid
    /// - Processing lock is not held by another agent
    /// - At least one pending entry exists
    ///
    /// # Postconditions
    /// - All processable entries have been attempted
    /// - Successful entries are marked as merged
    /// - Failed entries are marked appropriately
    /// - JSONL output has been emitted for each step
    /// - Processing lock is released
    pub async fn process_train(&self) -> Result<TrainResult, TrainProcessingError>;

    /// Process a single queue entry
    ///
    /// # Preconditions
    /// - Entry exists in queue
    /// - Entry is in 'claimed' state
    /// - Workspace is valid and accessible
    ///
    /// # Postconditions
    /// - Entry has progressed through state machine
    /// - Tests have been executed
    /// - Merge has been attempted if tests passed
    /// - Entry is in terminal state (merged or failed)
    /// - JSONL events have been emitted
    pub async fn process_entry(&self, entry: &QueueEntry) -> Result<TrainStep, TrainProcessingError>;
}
```

### 2.3 TrainResult

```rust
/// Result of merge train processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainResult {
    /// Total entries processed
    pub total_processed: usize,

    /// Successfully merged entries
    pub merged: Vec<MergedEntry>,

    /// Failed entries
    pub failed: Vec<FailedEntry>,

    /// Kicked entries (removed due to failure)
    pub kicked: Vec<KickedEntry>,

    /// Train duration in seconds
    pub duration_secs: i64,

    /// Timestamp when train started
    pub started_at: DateTime<Utc>,

    /// Timestamp when train completed
    pub completed_at: DateTime<Utc>,

    /// Processing agent ID
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedEntry {
    pub entry_id: i64,
    pub workspace: String,
    pub position: usize,
    pub merged_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedEntry {
    pub entry_id: i64,
    pub workspace: String,
    pub position: usize,
    pub reason: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickedEntry {
    pub entry_id: i64,
    pub workspace: String,
    pub previous_position: usize,
    pub kicked_at: DateTime<Utc>,
}
```

### 2.4 TrainStep

```rust
/// Single step in merge train processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainStep {
    /// Entry being processed
    pub entry_id: i64,

    /// Workspace name
    pub workspace: String,

    /// Position in queue
    pub position: usize,

    /// Action performed
    pub action: TrainAction,

    /// Status after action
    pub status: QueueStatus,

    /// Timestamp of this step
    pub timestamp: DateTime<Utc>,

    /// Optional error message
    pub error: Option<String>,

    /// Optional details JSON
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrainAction {
    /// Entry was claimed for processing
    Claimed,

    /// Entry is being rebased
    Rebasing,

    /// Entry is being tested
    Testing,

    /// Entry passed tests
    TestsPassed,

    /// Entry failed tests
    TestsFailed,

    /// Conflicts detected
    ConflictsDetected,

    /// Entry is ready to merge
    ReadyToMerge,

    /// Entry is being merged
    Merging,

    /// Entry was merged successfully
    Merged,

    /// Entry failed and will be kicked
    Kicked,

    /// Entry was skipped
    Skipped,
}
```

### 2.5 TrainFailureHandler

```rust
/// Handles train failure and auto-rebase recovery
pub struct TrainFailureHandler {
    db: SqlitePool,
}

impl TrainFailureHandler {
    /// Handle a train failure by kicking failed entry and rebasing subsequent entries
    ///
    /// # Preconditions
    /// - Failed entry exists in queue
    /// - Failed entry is in non-terminal state
    /// - Subsequent entries exist (positions > failed position)
    ///
    /// # Postconditions
    /// - Failed entry is removed from queue
    /// - Subsequent entries are rebased onto latest base
    /// - Positions are recalculated
    /// - TrainResult includes kicked entry
    /// - JSONL events emitted for kicks and rebases
    pub async fn handle_failure(
        &self,
        failed_entry_id: i64,
        failure_reason: &str,
    ) -> Result<TrainResult, TrainProcessingError>;

    /// Kick an entry from the queue
    ///
    /// # Preconditions
    /// - Entry exists
    /// - Entry is not already terminal
    ///
    /// # Postconditions
    /// - Entry is marked as cancelled
    /// - Entry is removed from position ordering
    /// - Audit event is logged
    pub async fn kick_entry(&self, entry_id: i64, reason: &str) -> Result<(), TrainProcessingError>;

    /// Rebase subsequent entries onto new base
    ///
    /// # Preconditions
    /// - Base entry has been merged
    /// - Subsequent entries exist
    /// - Workspaces are accessible
    ///
    /// # Postconditions
    /// - Each subsequent entry is rebased
    /// - Entries that fail rebase are marked
    /// - Positions are updated
    /// - JSONL events emitted
    pub async fn rebase_subsequent_entries(
        &self,
        from_position: usize,
        new_base: &str,
    ) -> Result<RebaseResult, TrainProcessingError>;
}
```

### 2.6 RebaseResult

```rust
/// Result of rebase operation
#[derive(Debug, Clone)]
pub struct RebaseResult {
    /// Entries successfully rebased
    pub rebased: Vec<RebasedEntry>,

    /// Entries that failed rebase
    pub failed: Vec<RebaseFailedEntry>,

    /// Duration of rebase operations
    pub duration_secs: i64,
}

#[derive(Debug, Clone)]
pub struct RebasedEntry {
    pub entry_id: i64,
    pub workspace: String,
    pub old_position: usize,
    pub new_position: usize,
    pub new_head_sha: String,
}

#[derive(Debug, Clone)]
pub struct RebaseFailedEntry {
    pub entry_id: i64,
    pub workspace: String,
    pub position: usize,
    pub reason: String,
}
```

---

## 3. Preconditions

### 3.1 Train Processing Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-TRAIN-001 | Database connection is valid and pool is healthy | Ping database |
| PRE-TRAIN-002 | Processing lock is not held or has expired | Check lock table |
| PRE-TRAIN-003 | At least one pending entry exists | Query pending count |
| PRE-TRAIN-004 | JJ binary is available and executable | Check PATH |
| PRE-TRAIN-005 | Test framework is configured (if running tests) | Check test config |

### 3.2 Entry Processing Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-ENTRY-001 | Entry exists in merge_queue table | SELECT by entry_id |
| PRE-ENTRY-002 | Entry is in 'claimed' state | Check status field |
| PRE-ENTRY-003 | Workspace directory exists and is accessible | Filesystem check |
| PRE-ENTRY-004 | Workspace has valid bookmark | Query jj bookmarks |
| PRE-ENTRY-005 | Entry has dedupe_key assigned | Check dedupe_key field |
| PRE-ENTRY-006 | Entry has head_sha assigned | Check head_sha field |

### 3.3 Test Execution Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-TEST-001 | Workspace has tests configured | Check test config |
| PRE-TEST-002 | Test runner is available | Check test binary |
| PRE-TEST-003 | Workspace is in clean state | Check jj status |
| PRE-TEST-004 | Sufficient disk space for test artifacts | Check df |
| PRE-TEST-005 | Test timeout is configured | Check timeout config |

### 3.4 Merge Operation Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-MERGE-001 | Tests passed for this entry | Check test results |
| PRE-MERGE-002 | No conflicts detected | Run jj merge check |
| PRE-MERGE-003 | Target branch exists and is reachable | Check remote |
| PRE-MERGE-004 | Entry is in 'ready_to_merge' state | Check status |
| PRE-MERGE-005 | Merge lock is available | Check lock table |

### 3.5 Failure Recovery Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-RECOVERY-001 | Failed entry exists | SELECT by entry_id |
| PRE-RECOVERY-002 | Failed entry is not terminal | Check status |
| PRE-RECOVERY-003 | Failure reason is recorded | Check error_message |
| PRE-RECOVERY-004 | At least one subsequent entry exists | Query positions |
| PRE-RECOVERY-005 | Workspaces are accessible | Filesystem check |

### 3.6 Rebase Operation Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-REBASE-001 | New base commit exists | Check commit |
| PRE-REBASE-002 | Workspace is not in conflicting state | Check jj status |
| PRE-REBASE-003 | Workspace has uncommitted changes or is ready | Check state |
| PRE-REBASE-004 | Rebase timeout is configured | Check config |
| PRE-REBASE-005 | Rollback capability exists | Check backup |

---

## 4. Postconditions

### 4.1 Train Processing Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-TRAIN-001 | All processable entries were attempted | Query processed entries |
| POST-TRAIN-002 | Successful entries have status 'merged' | Check statuses |
| POST-TRAIN-003 | Failed entries have appropriate status | Check statuses |
| POST-TRAIN-004 | Processing lock is released | Check lock table |
| POST-TRAIN-005 | Train summary JSONL was emitted | Check output |
| POST-TRAIN-006 | Duration was recorded | Check timestamp |

### 4.2 Entry Processing Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-ENTRY-001 | Entry progressed through state machine | Check audit events |
| POST-ENTRY-002 | All TrainStep events were emitted | Check output |
| POST-ENTRY-003 | Final status is terminal | Check status field |
| POST-ENTRY-004 | Error message set if failed | Check error_message |
| POST-ENTRY-005 | Attempt count incremented | Check attempt_count |
| POST-ENTRY-006 | Timestamps updated appropriately | Check time fields |

### 4.3 Test Execution Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-TEST-001 | Tests were executed | Check test logs |
| POST-TEST-002 | Test results recorded | Check results |
| POST-TEST-003 | Timeout enforced if exceeded | Check duration |
| POST-TEST-004 | TestStep event emitted | Check output |
| POST-TEST-005 | Workspace state unchanged if tests passed | Check git status |

### 4.4 Merge Operation Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-MERGE-001 | Changes were merged into target | Check target branch |
| POST-MERGE-002 | Bookmark updated to new commit | Check bookmark |
| POST-MERGE-003 | Entry status is 'merged' | Check status |
| POST-MERGE-004 | Merge completed_at timestamp set | Check timestamp |
| POST-MERGE-005 | MergeResult event emitted | Check output |

### 4.5 Failure Recovery Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-RECOVERY-001 | Failed entry was kicked | Check entry removed |
| POST-RECOVERY-002 | Kick event emitted | Check output |
| POST-RECOVERY-003 | Subsequent entries identified | Query positions |
| POST-RECOVERY-004 | Rebase operation initiated | Check rebase logs |
| POST-RECOVERY-005 | TrainResult includes kicked list | Check result |

### 4.6 Rebase Operation Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-REBASE-001 | Each subsequent entry was rebased | Check entries |
| POST-REBASE-002 | head_sha updated for rebased entries | Check head_sha |
| POST-REBASE-003 | Positions recalculated sequentially | Check positions |
| POST-REBASE-004 | Failed rebases marked appropriately | Check statuses |
| POST-REBASE-005 | RebaseStep events emitted | Check output |

---

## 5. Invariants

### 5.1 Train Processing Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-TRAIN-001 | Train processes entries in strict position order (1, 2, 3, ...) |
| INV-TRAIN-002 | Only one entry is processed at a time (sequential) |
| INV-TRAIN-003 | Processing stops at first unrecoverable failure |
| INV-TRAIN-004 | Each entry emits at least one TrainStep event |
| INV-TRAIN-005 | Train always emits final TrainResult summary |

### 5.2 State Machine Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-STATE-001 | State transitions follow valid state machine |
| INV-STATE-002 | Entry cannot skip states (no teleporting) |
| INV-STATE-003 | Terminal states are absorbing (no exit) |
| INV-STATE-004 | Each state transition has audit event |
| INV-STATE-005 | Status changes are atomic with updates |

### 5.3 Position Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-POS-001 | Positions are unique among pending entries |
| INV-POS-002 | Positions form contiguous sequence 1..N (no gaps) |
| INV-POS-003 | Positions are recalculated after kick |
| INV-POS-004 | Position is None for non-pending entries |
| INV-POS-005 | Position changes are atomic with status changes |

### 5.4 Deduplication Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-DEDUPE-001 | No two active entries share same dedupe_key |
| INV-DEDUPE-002 | dedupe_key stable across rebase for same logical change |
| INV-DEDUPE-003 | head_sha changes after rebase |
| INV-DEDUPE-004 | tested_against_sha updated after merge |

### 5.5 Merge Integrity Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-MERGE-001 | Only entries with passing tests are merged |
| INV-MERGE-002 | Only entries without conflicts are merged |
| INV-MERGE-003 | Merge commits reference correct parents |
| INV-MERGE-004 | Bookmark points to merge commit |
| INV-MERGE-005 | Merge is atomic (all-or-nothing) |

### 5.6 Failure Recovery Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-RECOVERY-001 | Failed entry is always kicked |
| INV-RECOVERY-002 | All subsequent entries are rebased |
| INV-RECOVERY-003 | Rebase failures are marked, not hidden |
| INV-RECOVERY-004 | Train stops after multiple unrecoverable failures |
| INV-RECOVERY-005 | Kick is atomic (cannot partially remove) |

### 5.7 Output Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-OUTPUT-001 | Every state change emits JSONL event |
| INV-OUTPUT-002 | Events have monotonically increasing timestamps |
| INV-OUTPUT-003 | TrainResult includes all outcomes (merged/failed/kicked) |
| INV-OUTPUT-004 | Error messages are human-readable |
| INV-OUTPUT-005 | Output is valid JSONL (one JSON per line) |

### 5.8 Concurrency Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-CONC-001 | Only one train can run at a time |
| INV-CONC-002 | Lock acquisition is atomic |
| INV-CONC-003 | Lock expires after timeout |
| INV-CONC-004 | Concurrent submissions serialize correctly |
| INV-CONC-005 | No race conditions in position updates |

---

## 6. Error Recovery

### 6.1 Recoverable Errors

| Error | Recovery Strategy | Retry Logic |
|-------|-------------------|-------------|
| `TestExecutionFailed` | Mark as failed_retryable, continue with next entry | Can be resubmitted |
| `TestTimeout` | Mark as failed_retryable, continue | Can be resubmitted with higher timeout |
| `MergeConflictsDetected` | Kick entry, rebase subsequent entries, restart train | Manual resolution needed |
| `RebaseFailed` | Mark entry as failed_terminal, continue with next | Manual intervention needed |
| `BookmarkUpdateFailed` | Retry bookmark update, mark as warning if persistent | Retry 3 times with backoff |
| `TransactionFailed` | Retry transaction, check for serialization failures | Retry with exponential backoff |
| `LockAcquisitionFailed` | Wait and retry | Retry every 5 seconds for 1 minute |

### 6.2 Non-Recoverable Errors

| Error | Action | Train Continuation |
|-------|--------|-------------------|
| `NoPendingEntries` | Exit successfully | N/A (train complete) |
| `EntryNotFound` | Log error, skip to next entry | Yes |
| `InvalidEntryState` | Log error, skip to next entry | Yes |
| `TestFrameworkNotConfigured` | Mark all entries as failed_terminal | No |
| `MultipleUnrecoverableFailures` | Stop train, emit error | No |
| `JjExecutionFailed` (critical) | Stop train, emit error | No |

### 6.3 Error Transformation

| Source Error | Transformed To | Context |
|--------------|----------------|---------|
| `sqlx::Error::DatabaseLocked` | `TransactionFailed` | During state updates |
| `std::io::Error` (workspace not found) | `EntryNotFound` | During workspace access |
| `tokio::process::Error` (jj not found) | `JjExecutionFailed` | During JJ commands |
| `TimeoutError` | `TestTimeout` | During test execution |
| `ConflictError` (from git) | `MergeConflictsDetected` | During merge check |

---

## 7. Security Considerations

### 7.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| **Test Code Execution:** Malicious tests could exploit CI system | Sandbox test execution; limit permissions |
| **Merge Injection:** Forced merge could introduce vulnerabilities | Require tests; require conflict-free merge |
| **Denial of Service:** Long-running tests block train | Enforce timeouts; parallel test execution |
| **Workspace Tampering:** Modifying workspace during train | Checkout clean state before processing |
| **Information Disclosure:** Train output leaks sensitive data | Sanitize output; redact tokens |

### 7.2 Security Requirements

1. **SR-001:** All test execution must be time-bounded
2. **SR-002:** Test execution must run with minimal privileges
3. **SR-003:** Workspace changes during train must be detectable
4. **SR-004:** Merge operations must be verifiable (commit signing)
5. **SR-005:** Failed entries must preserve full context for audit

### 7.3 Input Validation

All inputs must be validated:
- **Workspace name:** Non-empty, valid UTF-8, no path traversal
- **Entry ID:** Positive integer, exists in database
- **Test command:** Whitelisted commands only
- **Timeout value:** Positive integer, within configured bounds
- **Commit SHA:** Valid hex string of appropriate length

---

## 8. Test Coverage Requirements

### 8.1 Contract Verification Tests

Every precondition, postcondition, and invariant must have at least one test:
- All PRE-* conditions must have positive and negative tests
- All POST-* conditions must have verification tests
- All INV-* invariants must have property-based tests
- All error variants must be tested

### 8.2 State Machine Coverage

- Every valid state transition must be tested
- Every invalid state transition must be tested
- Terminal state handling must be tested
- State rollback scenarios must be tested

### 8.3 Integration Points

- JJ command execution (log, merge, rebase, bookmark)
- SQLite database operations (transactions, locks)
- Test execution (run tests, collect results)
- JSONL output emission
- Queue repository operations

### 8.4 Performance Requirements

| Metric | Target | Measurement |
|--------|--------|-------------|
| Train processing throughput | > 10 entries/hour | End-to-end duration |
| Single entry processing | < 5 minutes | Entry start to merge |
| Test execution timeout | < 10 minutes | Configurable per entry |
| Lock acquisition | < 1 second | Time to acquire |
| Position recalculation | < 100ms | After kick |

---

## 9. Graphite-Style Merge Train Behavior

### 9.1 Train Processing Flow

```
1. Acquire processing lock
2. Query all pending entries ordered by position
3. For each entry in order:
   a. Update status to 'claimed'
   b. Emit TrainStep(Claimed)
   c. Rebase onto target (if needed)
   d. Emit TrainStep(Rebasing)
   e. Run tests
   f. Emit TrainStep(Testing)
   g. If tests pass:
      - Check for conflicts
      - If no conflicts:
        - Update status to 'merging'
        - Emit TrainStep(Merging)
        - Perform merge
        - Update status to 'merged'
        - Emit TrainStep(Merged)
      - If conflicts:
        - Kick entry
        - Rebase subsequent entries
        - Restart train
   h. If tests fail:
      - Mark as failed_retryable
      - Emit TrainStep(TestFailed)
      - Continue with next entry
4. Release processing lock
5. Emit TrainResult summary
```

### 9.2 Failure Recovery Flow

```
1. Detect failure (test failure or conflict)
2. Mark failed entry as 'cancelled' (kick)
3. Remove entry from position ordering
4. For each subsequent entry:
   a. Checkout workspace
   b. Rebase onto latest target
   c. Update head_sha
   d. If rebase fails:
      - Mark as failed_terminal
      - Continue with next entry
   e. If rebase succeeds:
      - Update position
      - Emit RebaseStep event
5. Recalculate all positions
6. Restart train processing
```

### 9.3 Position Management

- **Initial Assignment:** When entry is submitted, position = COUNT(pending) + 1
- **After Merge:** Position is set to NULL (no longer pending)
- **After Kick:** All positions > kicked_position are decremented
- **After Rebase:** Positions remain the same, only head_sha changes
- **Recalculation:** Periodically ensure positions are contiguous 1..N

### 9.4 JSONL Output Format

```jsonl
{"type":"Train","agent_id":"agent-001","started_at":"2026-02-18T10:00:00Z"}
{"type":"TrainStep","entry_id":1,"workspace":"feature-auth","position":1,"action":"Claimed","status":"claimed","timestamp":"2026-02-18T10:00:01Z"}
{"type":"TrainStep","entry_id":1,"workspace":"feature-auth","position":1,"action":"Testing","status":"testing","timestamp":"2026-02-18T10:00:10Z"}
{"type":"TrainStep","entry_id":1,"workspace":"feature-auth","position":1,"action":"Merged","status":"merged","timestamp":"2026-02-18T10:05:00Z"}
{"type":"TrainResult","total_processed":1,"merged":[{"entry_id":1,"workspace":"feature-auth","position":1,"merged_at":"2026-02-18T10:05:00Z"}],"failed":[],"kicked":[],"duration_secs":300,"agent_id":"agent-001"}
```

---

## 10. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-18 | Initial contract specification |

---

## 11. References

- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs` - Queue implementation
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_status.rs` - State machine
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_submission.rs` - Submission API
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_entities.rs` - Database entities
- `/home/lewis/src/zjj/contracts/bd-1lx-contract-spec.md` - Queue submission contract
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-024-merge-train-logic.cue` - Original train bead
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-025-train-failure-auto-rebase.cue` - Failure recovery bead
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-033-test-merge-train.cue` - Test bead
