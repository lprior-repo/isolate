# Contract Specification: Add Session to Merge Queue for Processing (bd-1lx)

**Bead ID:** bd-1lx
**Title:** Add session to merge queue for processing
**Version:** 1.0.0
**Status:** Draft

---

## 1. Overview

This document specifies the Design by Contract requirements for adding a session to the Graphite-style merge queue in zjj. The system provides a submission mechanism that enables AI agents to queue workspaces for sequential merge train processing.

### 1.1 Scope

The contract covers:
- Session submission to the merge queue
- Workspace validation before queueing
- Deduplication key management for preventing duplicate work
- Queue position assignment
- Status tracking and state transitions
- Idempotent submission operations
- Graphite-style merge queue semantics

### 1.2 Graphite-Style Merge Queue Semantics

This system implements Graphite-style merge queue semantics:
1. **Sequential Processing:** Entries are processed one at a time in priority order
2. **Deduplication:** Prevents duplicate work using stable identifiers (change_id)
3. **Idempotent Submission:** Multiple submissions of the same session update the existing entry
4. **Priority-Based Ordering:** Higher priority entries are processed first
5. **State Machine:** Entries progress through a well-defined lifecycle
6. **Terminal State Handling:** Terminal entries can be resubmitted by resetting to pending

---

## 2. Type Definitions

### 2.1 QueueSubmissionError (Exhaustive Error Taxonomy)

```rust
/// Semantic error variants for queue submission operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueSubmissionError {
    // === Validation Errors ===
    /// Session does not exist
    SessionNotFound {
        session: String,
    },

    /// Workspace name is invalid
    InvalidWorkspaceName {
        workspace: String,
        reason: String,
    },

    /// Head SHA is invalid or missing
    InvalidHeadSha {
        head_sha: String,
        reason: String,
    },

    /// Deduplication key is invalid
    InvalidDedupeKey {
        dedupe_key: String,
        reason: String,
    },

    // === Queue State Errors ===
    /// Session is already in queue with different dedupe_key
    AlreadyInQueue {
        session: String,
        existing_dedupe_key: String,
        provided_dedupe_key: String,
    },

    /// Active entry with same dedupe_key exists for different workspace
    DedupeKeyConflict {
        dedupe_key: String,
        existing_workspace: String,
        provided_workspace: String,
    },

    /// Queue is full (optional constraint)
    QueueFull {
        capacity: usize,
        current_count: usize,
    },

    // === Database Errors ===
    /// Failed to open queue database
    DatabaseOpenFailed {
        path: String,
        source: String,
    },

    /// Failed to initialize queue schema
    SchemaInitializationFailed {
        reason: String,
    },

    /// Database transaction failed
    TransactionFailed {
        operation: String,
        source: String,
    },

    /// Concurrent modification detected
    ConcurrentModification {
        entry_id: i64,
        operation: String,
    },

    // === JJ Integration Errors ===
    /// Failed to extract workspace identity
    IdentityExtractionFailed {
        workspace: String,
        reason: String,
    },

    /// Failed to get change_id from jj
    ChangeIdExtractionFailed {
        workspace: String,
        reason: String,
    },

    /// Failed to get head_sha from jj
    HeadShaExtractionFailed {
        workspace: String,
        reason: String,
    },

    /// Failed to push bookmark to remote
    BookmarkPushFailed {
        workspace: String,
        bookmark: String,
        reason: String,
    },

    /// Remote is unreachable
    RemoteUnreachable {
        remote: String,
        reason: String,
    },

    /// JJ command execution failed
    JjExecutionFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    // === State Transition Errors ===
    /// Invalid state transition attempted
    InvalidStateTransition {
        entry_id: i64,
        current_status: QueueStatus,
        target_status: QueueStatus,
    },

    /// Cannot modify terminal entry
    EntryIsTerminal {
        entry_id: i64,
        status: QueueStatus,
    },

    // === Authorization Errors ===
    /// Agent not authorized to submit to this workspace
    UnauthorizedWorkspace {
        agent_id: String,
        workspace: String,
    },

    /// Agent not authorized to modify queue entry
    UnauthorizedEntryModification {
        agent_id: String,
        entry_id: i64,
        owner: String,
    },
}
```

### 2.2 QueueSubmissionRequest

```rust
/// Request to submit a session to the merge queue
#[derive(Debug, Clone)]
pub struct QueueSubmissionRequest {
    /// Workspace name (must exist)
    pub workspace: String,

    /// Optional bead ID for traceability
    pub bead_id: Option<String>,

    /// Priority (lower = higher priority, default 0)
    pub priority: i32,

    /// Agent ID submitting the request
    pub agent_id: Option<String>,

    /// Deduplication key (format: "workspace:change_id")
    pub dedupe_key: String,

    /// Current HEAD SHA of the workspace
    pub head_sha: String,

    /// Optional dedupe_key for the tested_against_sha
    pub tested_against_sha: Option<String>,
}
```

### 2.3 QueueSubmissionResponse

```rust
/// Response from queue submission operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSubmissionResponse {
    /// Queue entry ID (assigned by database)
    pub entry_id: i64,

    /// Workspace name
    pub workspace: String,

    /// Assigned status after submission
    pub status: QueueStatus,

    /// Position in pending queue (1-indexed)
    pub position: Option<usize>,

    /// Total number of pending entries
    pub pending_count: usize,

    /// Whether this was a new entry or an update
    pub submission_type: SubmissionType,

    /// Timestamp of submission
    pub submitted_at: DateTime<Utc>,

    /// Optional bead ID
    pub bead_id: Option<String>,
}

/// Type of submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmissionType {
    /// New entry created
    New,

    /// Existing entry updated (same dedupe_key and workspace)
    Updated,

    /// Terminal entry reset to pending
    Resubmitted,
}
```

### 2.4 Function Signatures

```rust
/// Submit a session to the merge queue
///
/// This is the main entry point for adding workspaces to the merge queue.
/// It implements Graphite-style merge queue semantics with idempotent upsert.
///
/// # Preconditions
/// - Workspace must exist and be valid
/// - Workspace must have a valid bookmark
/// - Remote must be reachable (bookmark push verified)
/// - Head SHA must be valid
/// - Dedupe_key must be unique among active entries
///
/// # Postconditions
/// - Entry exists in merge_queue table
/// - Entry has unique ID
/// - Entry status is 'pending' (or updated from previous state)
/// - Position is assigned if status is 'pending'
/// - Dedupe_key is set and enforced
/// - Event audit trail is updated
///
/// # Errors
/// - Returns QueueSubmissionError for any validation or database failure
pub async fn submit_to_queue(
    request: QueueSubmissionRequest,
) -> Result<QueueSubmissionResponse, QueueSubmissionError>;

/// Validate workspace before submission
///
/// # Preconditions
/// - Workspace directory must exist
///
/// # Postconditions
/// - Returns true if workspace is valid
/// - Returns detailed error if validation fails
pub async fn validate_workspace(
    workspace: &str,
) -> Result<bool, QueueSubmissionError>;

/// Extract identity information from workspace
///
/// # Preconditions
/// - Must be in a valid JJ repository
/// - Workspace must exist
///
/// # Postconditions
/// - Returns change_id (stable across rebases)
/// - Returns head_sha (current commit)
/// - Returns bookmark name
pub async fn extract_workspace_identity(
    workspace: &str,
) -> Result<WorkspaceIdentity, QueueSubmissionError>;

/// Compute deduplication key from change_id and workspace
///
/// # Preconditions
/// - change_id must be non-empty
/// - workspace must be non-empty
///
/// # Postconditions
/// - Returns formatted dedupe_key: "workspace:change_id"
/// - Same inputs always produce same output
pub fn compute_dedupe_key(
    change_id: &str,
    workspace: &str,
) -> String;

/// Push bookmark to remote before queueing
///
/// # Preconditions
/// - Bookmark must exist locally
/// - Remote must be configured
///
/// # Postconditions
/// - Bookmark is pushed to remote
/// - Returns error if push fails
pub async fn push_bookmark_to_remote(
    workspace: &str,
    bookmark: &str,
) -> Result<(), QueueSubmissionError>;

/// Get current position in queue
///
/// # Preconditions
/// - Entry must exist
///
/// # Postconditions
/// - Returns position if status is 'pending'
/// - Returns None if status is not 'pending'
pub async fn get_queue_position(
    entry_id: i64,
) -> Result<Option<usize>, QueueSubmissionError>;

/// Check if session is already in queue
///
/// # Preconditions
/// - None
///
/// # Postconditions
/// - Returns true if entry exists for workspace
/// - Returns false otherwise
pub async fn is_in_queue(
    workspace: &str,
) -> Result<bool, QueueSubmissionError>;
```

---

## 3. Preconditions

### 3.1 Workspace State Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-WS-001 | Workspace must exist in workspace list | Check via `jj workspace list` |
| PRE-WS-002 | Workspace must not be abandoned | Check workspace state |
| PRE-WS-003 | Workspace must have a current bookmark | Check via `jj log` |
| PRE-WS-004 | Workspace must have valid commit (HEAD SHA exists) | Check via `jj log` |
| PRE-WS-005 | Workspace must be in a clean state or have --auto-commit | Check via `jj status` |

### 3.2 Remote State Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-REMOTE-001 | Remote must be configured | Check git config |
| PRE-REMOTE-002 | Remote must be reachable | Test connection |
| PRE-REMOTE-003 | Bookmark push must succeed | Verify push result |

### 3.3 Identity Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-ID-001 | change_id must be extractable | Run `jj log -T change_id` |
| PRE-ID-002 | head_sha must be extractable | Run `jj log -T commit_id` |
| PRE-ID-003 | bookmark name must be available | Run `jj log -T bookmarks` |

### 3.4 Queue State Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-Q-001 | Dedupe_key must not conflict with active entries | Query merge_queue table |
| PRE-Q-002 | If entry exists, workspace must match | Query merge_queue table |
| PRE-Q-003 | If entry is terminal, reset is allowed | Check entry status |

### 3.5 Authorization Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-AUTH-001 | Agent must be authorized to access workspace (if multi-agent) | Check workspace permissions |
| PRE-AUTH-002 | Agent must be authorized to submit to queue (if enforced) | Check queue permissions |

---

## 4. Postconditions

### 4.1 Entry Creation Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-ENTRY-001 | Entry exists in merge_queue table | SELECT by entry_id |
| POST-ENTRY-002 | Entry has unique ID (auto-increment) | Check ID > 0 |
| POST-ENTRY-003 | Entry workspace matches request | Verify workspace field |
| POST-ENTRY-004 | Entry dedupe_key matches request | Verify dedupe_key field |
| POST-ENTRY-005 | Entry head_sha matches request | Verify head_sha field |
| POST-ENTRY-006 | Entry status is 'pending' (for new) | Verify status field |
| POST-ENTRY-007 | Entry added_at is set and recent | Check timestamp |
| POST-ENTRY-008 | Entry workspace_state is 'created' | Verify workspace_state |

### 4.2 Position Assignment Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-POS-001 | Position is assigned if status is 'pending' | Check position > 0 |
| POST-POS-002 | Position is unique among pending entries | Check for duplicates |
| POST-POS-003 | Position respects priority ordering | Verify higher priority entries first |
| POST-POS-004 | Total pending count is accurate | COUNT pending entries |

### 4.3 Deduplication Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-DEDUPE-001 | No two active entries share same dedupe_key | UNIQUE constraint check |
| POST-DEDUPE-002 | Terminal entries can have same dedupe_key | Check terminal entries |
| POST-DEDUPE-003 | Resubmission resets terminal entry | Verify status transition |

### 4.4 Audit Trail Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-AUDIT-001 | Event is logged to queue_events table | Query by queue_id |
| POST-AUDIT-002 | Event type is 'created' or 'updated' | Check event_type |
| POST-AUDIT-003 | Event timestamp matches submission | Verify created_at |
| POST-AUDIT-004 | Agent ID is recorded (if provided) | Check event details |

### 4.5 Response Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-RESP-001 | Response entry_id matches database | Verify entry_id |
| POST-RESP-002 | Response status matches entry status | Verify status |
| POST-RESP-003 | Response position is accurate | Check position query |
| POST-RESP-004 | Response submission_type is correct | Verify NEW/UPDATED/RESUBMITTED |
| POST-RESP-005 | Response timestamp is recent | Check submitted_at |

---

## 5. Invariants

### 5.1 Queue Integrity Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-QUEUE-001 | No two active (non-terminal) entries have the same dedupe_key |
| INV-QUEUE-002 | Each workspace has at most one active entry |
| INV-QUEUE-003 | Position values are unique among pending entries |
| INV-QUEUE-004 | Position values form a contiguous sequence from 1 to N (no gaps) |
| INV-QUEUE-005 | Priority sorting: entry with higher priority (lower number) comes first |

### 5.2 State Consistency Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-STATE-001 | Entry status is one of the valid QueueStatus values |
| INV-STATE-002 | Workspace state is one of the valid WorkspaceQueueState values |
| INV-STATE-003 | Terminal entries cannot transition (except reset to pending) |
| INV-STATE-004 | Pending entries always have a position assigned |
| INV-STATE-005 | Non-pending entries never have a position assigned |

### 5.3 Identity Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-ID-001 | head_sha is a valid 40-character hex string (or full commit ID) |
| INV-ID-002 | change_id is stable across rebases for same logical change |
| INV-ID-003 | dedupe_key format is "workspace:change_id" |
| INV-ID-004 | bookmark_name exists in workspace bookmarks |

### 5.4 Concurrency Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-CONC-001 | Submission is atomic: either fully succeeds or fully fails |
| INV-CONC-002 | Concurrent submissions with same dedupe_key serialize correctly |
| INV-CONC-003 | No partial state is left after failed submission |
| INV-CONC-004 | Position updates are atomic with status changes |

### 5.5 Audit Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-AUDIT-001 | Every state transition has a corresponding audit event |
| INV-AUDIT-002 | Event IDs are monotonically increasing |
| INV-AUDIT-003 | Audit trail is append-only (no deletions) |
| INV-AUDIT-004 | Event timestamps are non-decreasing |

---

## 6. Error Recovery

### 6.1 Recoverable Errors

| Error | Recovery Strategy |
|-------|-------------------|
| `BookmarkPushFailed` | Retry with exponential backoff, or queue for later |
| `RemoteUnreachable` | Queue entry in 'offline' state, retry later |
| `ConcurrentModification` | Retry operation with updated state |
| `TransactionFailed` | Retry transaction, check for serialization failures |

### 6.2 Non-Recoverable Errors

| Error | Action |
|-------|--------|
| `SessionNotFound` | Abort with instructions to create workspace first |
| `InvalidWorkspaceName` | Abort with validation error |
| `InvalidHeadSha` | Abort with instructions to commit changes |
| `DedupeKeyConflict` | Abort with details about conflicting entry |
| `QueueFull` | Abort with capacity information |

### 6.3 Error Transformation

Some errors from lower layers are transformed into `QueueSubmissionError`:

| Source Error | Transformed To |
|--------------|----------------|
| `sqlx::Error::DatabaseLocked` | `TransactionFailed` |
| `std::io::Error` | `DatabaseOpenFailed` or `TransactionFailed` |
| `SubmitError::PushFailed` | `BookmarkPushFailed` |
| `SubmitError::NoBookmark` | `IdentityExtractionFailed` |

---

## 7. Security Considerations

### 7.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| **Unauthorized Queue Access:** Agent submitting to another's workspace | Workspace ownership validation; agent ID tracking |
| **Duplicate Work:** Same change submitted multiple times | Deduplication key enforcement with UNIQUE constraint |
| **Queue Starvation:** Low-priority entries never processed | Fair scheduling with aging; priority boost over time |
| **State Corruption:** Partial updates leaving inconsistent state | Atomic transactions; rollback on failure |
| **Information Disclosure:** Queue data visible to unauthorized agents | Access control on queue operations; audit logging |

### 7.2 Security Requirements

1. **SR-001:** All submissions must be attributable to an agent ID (if multi-agent)
2. **SR-002:** Audit log must capture: timestamp, agent, workspace, operation, outcome
3. **SR-003:** Workspace ownership must be verified before submission
4. **SR-004:** Deduplication keys must be validated for format and uniqueness
5. **SR-005:** Failed submissions must be logged with full context

### 7.3 Input Validation

All inputs must be validated:
- **Workspace name:** Non-empty, valid UTF-8, no special characters
- **Dedupe key:** Format "workspace:change_id", both parts non-empty
- **Head SHA:** Valid commit ID format
- **Priority:** Integer in valid range
- **Agent ID:** Non-empty if provided

---

## 8. Test Coverage Requirements

### 8.1 Contract Verification Tests

Every precondition, postcondition, and invariant must have at least one test:
- All PRE-* conditions must have positive tests (condition met) and negative tests (condition violated)
- All POST-* conditions must have verification tests
- All INV-* invariants must have property-based tests

### 8.2 Error Path Coverage

All `QueueSubmissionError` variants must be:
1. Constructible in tests
2. Displayable with meaningful message
3. Serializable to JSON
4. Deserializable from JSON
5. Recoverable where applicable

### 8.3 Integration Points

- JJ command execution (bookmark push, identity extraction)
- SQLite database operations (insert, update, query)
- Queue repository trait implementation
- State machine transitions

---

## 9. Graphite-Style Merge Queue Behavior

### 9.1 Queue Processing Model

The merge queue implements a sequential processing model:
1. **Pending:** Entry is waiting to be processed
2. **Claimed:** Entry has been claimed by a worker
3. **Rebasing:** Entry is being rebased onto target branch
4. **Testing:** Entry is undergoing CI/testing
5. **ReadyToMerge:** Entry has passed all checks
6. **Merging:** Entry is actively being merged
7. **Merged:** Entry has been successfully merged (terminal)
8. **FailedRetryable:** Entry failed but can be retried
9. **FailedTerminal:** Entry failed with unrecoverable error (terminal)
10. **Cancelled:** Entry was cancelled (terminal)

### 9.2 Deduplication Semantics

- **Purpose:** Prevent duplicate work when same change is submitted multiple times
- **Key Format:** "workspace:change_id" where change_id is stable across rebases
- **Scope:** Enforced only among active (non-terminal) entries
- **Idempotence:** Multiple submissions with same dedupe_key update existing entry
- **Terminal Handling:** Terminal entries can be resubmitted by resetting to pending

### 9.3 Position Assignment

- **Pending Only:** Only entries with status 'pending' have a position
- **Priority First:** Sorted by priority (lower number = higher priority)
- **FIFO Within Priority:** Within same priority, older entries (lower added_at) come first
- **Dynamic Update:** Positions are recalculated when entries change status

### 9.4 Idempotent Upsert Behavior

The `upsert_for_submit` operation implements idempotent submission:

1. **No Existing Entry:** Create new entry with status 'pending'
2. **Terminal Entry, Same Workspace:** Reset to 'pending', update head_sha
3. **Terminal Entry, Different Workspace:** Release dedupe_key, create new entry
4. **Active Entry, Same Workspace:** Update head_sha and timestamp
5. **Active Entry, Different Workspace:** Reject with DedupeKeyConflict

---

## 10. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-18 | Initial contract specification |

---

## 11. References

- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs` - Queue implementation
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_repository.rs` - Queue repository trait
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_status.rs` - State machine
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_entities.rs` - Database entities
- `/home/lewis/src/zjj/crates/zjj/src/commands/submit.rs` - Submit command implementation
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-023-queue-submit-cmd.cue` - Original bead specification
