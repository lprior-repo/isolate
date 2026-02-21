# Martin Fowler Test Plan: Add Session to Merge Queue for Processing (bd-1lx)

**Bead ID:** bd-1lx
**Title:** Add session to merge queue for processing
**Test Framework:** BDD with Given-When-Then scenarios
**Version:** 1.0.0

---

## 1. Overview

This test plan follows Martin Fowler's BDD approach with Given-When-Then scenarios for testing the session submission to the Graphite-style merge queue. Tests are organized by category: Happy Path, Error Path, Edge Cases, Contract Verification, and End-to-End Scenarios.

### 1.1 Test Categories

| Category | Purpose | Count |
|----------|---------|-------|
| Happy Path | Normal successful operations | 18 |
| Error Path | Failure handling and recovery | 22 |
| Edge Cases | Boundary conditions | 14 |
| Contract Verification | Pre/post/invariant validation | 24 |
| End-to-End | Full workflow scenarios | 12 |
| **Total** | | **90** |

---

## 2. Happy Path Tests

### 2.1 HP-001: Submit New Session Successfully

**Given** a valid workspace "feature-auth" exists
**And** workspace has a bookmark "feature-auth"
**And** workspace has committed changes (head_sha = "abc123")
**And** change_id is "kxyz789"
**And** remote is reachable
**When** session is submitted to queue
**Then** new entry is created with unique ID
**And** entry status is "pending"
**And** entry position is 1 (only entry)
**And** dedupe_key is "feature-auth:kxyz789"
**And** head_sha is "abc123"
**And** audit event "created" is logged

```rust
#[tokio::test]
async fn hp_001_submit_new_session_successfully() {
    // Arrange: Create valid workspace with bookmark and commits
    // Act: Submit session to queue
    // Assert: New entry created, status pending, position 1
}
```

---

### 2.2 HP-002: Submit Session with Priority

**Given** a valid workspace exists
**And** priority is set to 1 (high priority)
**When** session is submitted to queue
**Then** entry is created with priority 1
**And** entry appears before lower priority entries

```rust
#[tokio::test]
async fn hp_002_submit_session_with_priority() {
    // Arrange: Create workspace
    // Act: Submit with priority 1
    // Assert: Entry has priority 1, sorted correctly
}
```

---

### 2.3 HP-003: Idempotent Submission (Same Workspace)

**Given** an active entry exists for workspace "feature-auth"
**And** entry has dedupe_key "feature-auth:kxyz789"
**When** same workspace is submitted again with same dedupe_key
**Then** existing entry is updated (not duplicated)
**And** head_sha is updated to new value
**And** submission_type is "Updated"
**And** no new position is assigned

```rust
#[tokio::test]
async fn hp_003_idempotent_submission_same_workspace() {
    // Arrange: Create active entry
    // Act: Submit same workspace again
    // Assert: Entry updated, not duplicated
}
```

---

### 2.4 HP-004: Resubmit Terminal Entry (Same Workspace)

**Given** a terminal entry exists for workspace "feature-auth"
**And** entry status is "merged"
**And** entry has dedupe_key "feature-auth:kxyz789"
**When** same workspace is submitted again with same dedupe_key
**Then** existing entry is reset to "pending"
**And** head_sha is updated to new value
**And** started_at and completed_at are cleared
**And** submission_type is "Resubmitted"
**And** new position is assigned

```rust
#[tokio::test]
async fn hp_004_resubmit_terminal_entry_same_workspace() {
    // Arrange: Create terminal (merged) entry
    // Act: Submit same workspace again
    // Assert: Entry reset to pending, new position
}
```

---

### 2.5 HP-005: Submit Session with Bead ID

**Given** a valid workspace exists
**And** bead_id is "bd-1lx"
**When** session is submitted to queue
**Then** entry includes bead_id "bd-1lx"
**And** bead_id is preserved in audit log

```rust
#[tokio::test]
async fn hp_005_submit_session_with_bead_id() {
    // Arrange: Create workspace
    // Act: Submit with bead_id
    // Assert: Entry has correct bead_id
}
```

---

### 2.6 HP-006: Submit Session with Agent ID

**Given** a valid workspace exists
**And** agent_id is "agent-prod-001"
**When** session is submitted to queue
**Then** entry includes agent_id "agent-prod-001"
**And** agent_id is recorded in audit log

```rust
#[tokio::test]
async fn hp_006_submit_session_with_agent_id() {
    // Arrange: Create workspace
    // Act: Submit with agent_id
    // Assert: Entry has correct agent_id
}
```

---

### 2.7 HP-007: Multiple Sessions in Queue

**Given** 3 sessions are already in queue
**And** positions are 1, 2, 3
**When** new session is submitted
**Then** new entry is assigned position 4
**And** existing positions are unchanged
**And** total pending count is 4

```rust
#[tokio::test]
async fn hp_007_multiple_sessions_in_queue() {
    // Arrange: Create 3 entries
    // Act: Submit 4th entry
    // Assert: Position 4, count 4
}
```

---

### 2.8 HP-008: Priority Ordering in Queue

**Given** 3 entries exist with priorities 5, 3, 7
**And** positions are ordered by priority then time
**When** new entry is submitted with priority 1
**Then** new entry gets position 1 (highest priority)
**And** other positions shift to 2, 3, 4, 5

```rust
#[tokio::test]
async fn hp_008_priority_ordering_in_queue() {
    // Arrange: Create entries with mixed priorities
    // Act: Submit high-priority entry
    // Assert: Position 1, others shifted
}
```

---

### 2.9 HP-009: Compute Dedupe Key Correctly

**Given** workspace is "feature-auth"
**And** change_id is "kxyz789"
**When** dedupe_key is computed
**Then** result is "feature-auth:kxyz789"
**And** same inputs produce same output
**And** different inputs produce different output

```rust
#[tokio::test]
async fn hp_009_compute_dedupe_key_correctly() {
    // Arrange: workspace and change_id
    // Act: compute_dedupe_key()
    // Assert: Correct format, deterministic
}
```

---

### 2.10 HP-010: Extract Workspace Identity Successfully

**Given** a valid workspace exists
**And** workspace has bookmark "feature-auth"
**And** workspace has change_id "kxyz789"
**And** workspace has head_sha "abc123"
**When** workspace identity is extracted
**Then** all fields are populated correctly
**And** bookmark_name is "feature-auth"
**And** change_id is "kxyz789"
**And** head_sha is "abc123"

```rust
#[tokio::test]
async fn hp_010_extract_workspace_identity_successfully() {
    // Arrange: Create workspace with known state
    // Act: extract_workspace_identity()
    // Assert: All fields correct
}
```

---

### 2.11 HP-011: Push Bookmark to Remote Successfully

**Given** a workspace with bookmark "feature-auth"
**And** remote is configured and reachable
**When** bookmark is pushed to remote
**Then** push succeeds
**And** bookmark is available on remote
**And** no error is returned

```rust
#[tokio::test]
async fn hp_011_push_bookmark_to_remote_successfully() {
    // Arrange: Create workspace with bookmark
    // Act: push_bookmark_to_remote()
    // Assert: Push succeeds
}
```

---

### 2.12 HP-012: Validate Workspace Successfully

**Given** a valid workspace exists
**And** workspace is not abandoned
**And** workspace has a bookmark
**When** workspace is validated
**Then** validation returns true
**And** no errors are returned

```rust
#[tokio::test]
async fn hp_012_validate_workspace_successfully() {
    // Arrange: Create valid workspace
    // Act: validate_workspace()
    // Assert: Returns true
}
```

---

### 2.13 HP-013: Get Queue Position for Pending Entry

**Given** an entry with status "pending"
**And** entry is at position 3
**When** queue position is queried
**Then** position is Some(3)

```rust
#[tokio::test]
async fn hp_013_get_queue_position_for_pending_entry() {
    // Arrange: Create pending entry
    // Act: get_queue_position()
    // Assert: Returns Some(3)
}
```

---

### 2.14 HP-014: Check if Session is in Queue

**Given** an entry exists for workspace "feature-auth"
**When** checking if workspace is in queue
**Then** result is true

```rust
#[tokio::test]
async fn hp_014_check_if_session_is_in_queue() {
    // Arrange: Create entry
    // Act: is_in_queue()
    // Assert: Returns true
}
```

---

### 2.15 HP-015: Audit Trail Created on Submission

**Given** a session is submitted to queue
**When** submission completes
**Then** audit event is logged
**And** event type is "created"
**And** event timestamp is recent
**And** event references the entry ID

```rust
#[tokio::test]
async fn hp_015_audit_trail_created_on_submission() {
    // Arrange: Submit session
    // Act: Query audit log
    // Assert: Event exists with correct details
}
```

---

### 2.16 HP-016: Workspace State Set to Created

**Given** a new session is submitted
**When** submission completes
**Then** entry workspace_state is "created"
**And** state_changed_at is set

```rust
#[tokio::test]
async fn hp_016_workspace_state_set_to_created() {
    // Arrange: Submit session
    // Act: Query entry
    // Assert: workspace_state is "created"
}
```

---

### 2.17 HP-017: Response Contains All Required Fields

**Given** a session is submitted successfully
**When** response is returned
**Then** response has entry_id
**And** response has workspace
**And** response has status
**And** response has position
**And** response has pending_count
**And** response has submission_type
**And** response has submitted_at

```rust
#[tokio::test]
async fn hp_017_response_contains_all_required_fields() {
    // Arrange: Submit session
    // Act: Get response
    // Assert: All required fields present
}
```

---

### 2.18 HP-018: JSON Output is Valid

**Given** a session is submitted successfully
**When** response is serialized to JSON
**Then** JSON is valid
**And** all fields are present
**And** JSON can be deserialized back to response

```rust
#[tokio::test]
async fn hp_018_json_output_is_valid() {
    // Arrange: Submit session
    // Act: Serialize to JSON
    // Assert: Valid JSON, round-trip succeeds
}
```

---

## 3. Error Path Tests

### 3.1 EP-001: Session Not Found

**Given** workspace "nonexistent" does not exist
**When** session is submitted to queue
**Then** `SessionNotFound` error is returned
**And** error includes workspace name

```rust
#[tokio::test]
async fn ep_001_session_not_found() {
    // Arrange: Reference non-existent workspace
    // Act: Submit session
    // Assert: SessionNotFound error
}
```

---

### 3.2 EP-002: Invalid Workspace Name

**Given** workspace name is empty string
**When** session is submitted
**Then** `InvalidWorkspaceName` error is returned
**And** error includes reason

```rust
#[tokio::test]
async fn ep_002_invalid_workspace_name() {
    // Arrange: Use empty workspace name
    // Act: Submit session
    // Assert: InvalidWorkspaceName error
}
```

---

### 3.3 EP-003: Invalid Head SHA

**Given** head_sha is "not-a-valid-sha"
**When** session is submitted
**Then** `InvalidHeadSha` error is returned
**And** error includes reason

```rust
#[tokio::test]
async fn ep_003_invalid_head_sha() {
    // Arrange: Use invalid head_sha
    // Act: Submit session
    // Assert: InvalidHeadSha error
}
```

---

### 3.4 EP-004: Invalid Dedupe Key Format

**Given** dedupe_key is "invalid-format" (missing colon)
**When** session is submitted
**Then** `InvalidDedupeKey` error is returned
**And** error includes reason

```rust
#[tokio::test]
async fn ep_004_invalid_dedupe_key_format() {
    // Arrange: Use malformed dedupe_key
    // Act: Submit session
    // Assert: InvalidDedupeKey error
}
```

---

### 3.5 EP-005: Bookmark Push Failed

**Given** remote is unreachable
**When** bookmark push is attempted
**Then** `BookmarkPushFailed` error is returned
**And** error includes bookmark name and reason

```rust
#[tokio::test]
async fn ep_005_bookmark_push_failed() {
    // Arrange: Make remote unreachable
    // Act: Push bookmark
    // Assert: BookmarkPushFailed error
}
```

---

### 3.6 EP-006: Remote Unreachable

**Given** remote host is down
**When** submission is attempted
**Then** `RemoteUnreachable` error is returned
**And** error includes remote name

```rust
#[tokio::test]
async fn ep_006_remote_unreachable() {
    // Arrange: Configure unreachable remote
    // Act: Submit session
    // Assert: RemoteUnreachable error
}
```

---

### 3.7 EP-007: Identity Extraction Failed

**Given** workspace has no bookmark
**When** identity extraction is attempted
**Then** `IdentityExtractionFailed` error is returned
**And** error includes reason

```rust
#[tokio::test]
async fn ep_007_identity_extraction_failed() {
    // Arrange: Create workspace without bookmark
    // Act: Extract identity
    // Assert: IdentityExtractionFailed error
}
```

---

### 3.8 EP-008: Change ID Extraction Failed

**Given** jj log command fails for change_id
**When** identity extraction is attempted
**Then** `ChangeIdExtractionFailed` error is returned

```rust
#[tokio::test]
async fn ep_008_change_id_extraction_failed() {
    // Arrange: Mock jj log failure
    // Act: Extract identity
    // Assert: ChangeIdExtractionFailed error
}
```

---

### 3.9 EP-009: Head SHA Extraction Failed

**Given** jj log command fails for head_sha
**When** identity extraction is attempted
**Then** `HeadShaExtractionFailed` error is returned

```rust
#[tokio::test]
async fn ep_009_head_sha_extraction_failed() {
    // Arrange: Mock jj log failure
    // Act: Extract identity
    // Assert: HeadShaExtractionFailed error
}
```

---

### 3.10 EP-010: JJ Execution Failed

**Given** jj binary returns non-zero exit code
**When** jj command is executed
**Then** `JjExecutionFailed` error is returned
**And** error includes exit code and stderr

```rust
#[tokio::test]
async fn ep_010_jj_execution_failed() {
    // Arrange: Mock jj failure
    // Act: Run jj command
    // Assert: JjExecutionFailed error
}
```

---

### 3.11 EP-011: Database Open Failed

**Given** database file is corrupted
**When** queue database is opened
**Then** `DatabaseOpenFailed` error is returned
**And** error includes path

```rust
#[tokio::test]
async fn ep_011_database_open_failed() {
    // Arrange: Corrupt database file
    // Act: Open queue
    // Assert: DatabaseOpenFailed error
}
```

---

### 3.12 EP-012: Transaction Failed

**Given** database is locked by another process
**When** submission transaction is attempted
**Then** `TransactionFailed` error is returned

```rust
#[tokio::test]
async fn ep_012_transaction_failed() {
    // Arrange: Lock database
    // Act: Submit session
    // Assert: TransactionFailed error
}
```

---

### 3.13 EP-013: Concurrent Modification Detected

**Given** two agents submit same workspace simultaneously
**When** both submissions execute
**Then** one succeeds
**And** the other returns `ConcurrentModification` or succeeds with update

```rust
#[tokio::test]
async fn ep_013_concurrent_modification_detected() {
    // Arrange: Spawn two concurrent submissions
    // Act: Execute both
    // Assert: One succeeds, one handles conflict
}
```

---

### 3.14 EP-014: Dedupe Key Conflict (Different Workspace)

**Given** active entry exists with dedupe_key "ws-a:kxyz789"
**And** entry is for workspace "ws-a"
**When** workspace "ws-b" submits with same dedupe_key
**Then** `DedupeKeyConflict` error is returned
**And** error includes both workspace names

```rust
#[tokio::test]
async fn ep_014_dedupe_key_conflict_different_workspace() {
    // Arrange: Create entry for ws-a
    // Act: Submit ws-b with same dedupe_key
    // Assert: DedupeKeyConflict error
}
```

---

### 3.15 EP-015: Queue Full (Optional Constraint)

**Given** queue has capacity of 100
**And** 100 entries are already pending
**When** new session is submitted
**Then** `QueueFull` error is returned (if enforced)
**And** error includes capacity and current count

```rust
#[tokio::test]
async fn ep_015_queue_full() {
    // Arrange: Fill queue to capacity
    // Act: Submit one more
    // Assert: QueueFull error (or success if no limit)
}
```

---

### 3.16 EP-016: Invalid State Transition

**Given** entry status is "merged" (terminal)
**And** direct transition to "testing" is attempted
**When** transition is attempted
**Then** `InvalidStateTransition` error is returned

```rust
#[tokio::test]
async fn ep_016_invalid_state_transition() {
    // Arrange: Create terminal entry
    // Act: Attempt invalid transition
    // Assert: InvalidStateTransition error
}
```

---

### 3.17 EP-017: Entry is Terminal

**Given** entry status is "cancelled"
**When** modification is attempted (not reset)
**Then** `EntryIsTerminal` error is returned

```rust
#[tokio::test]
async fn ep_017_entry_is_terminal() {
    // Arrange: Create cancelled entry
    // Act: Attempt modification
    // Assert: EntryIsTerminal error
}
```

---

### 3.18 EP-018: Unauthorized Workspace

**Given** agent "agent-a" attempts to submit to workspace "ws-owned-by-b"
**And** workspace ownership is enforced
**When** submission is attempted
**Then** `UnauthorizedWorkspace` error is returned

```rust
#[tokio::test]
async fn ep_018_unauthorized_workspace() {
    // Arrange: Configure workspace ownership
    // Act: Agent-a submits to agent-b's workspace
    // Assert: UnauthorizedWorkspace error
}
```

---

### 3.19 EP-019: Unauthorized Entry Modification

**Given** entry is owned by agent "agent-a"
**And** agent "agent-b" attempts to modify it
**When** modification is attempted
**Then** `UnauthorizedEntryModification` error is returned

```rust
#[tokio::test]
async fn ep_019_unauthorized_entry_modification() {
    // Arrange: Create entry owned by agent-a
    // Act: Agent-b modifies it
    // Assert: UnauthorizedEntryModification error
}
```

---

### 3.20 EP-020: Workspace is Abandoned

**Given** workspace is in "abandoned" state
**When** submission is attempted
**Then** validation fails
**And** appropriate error is returned

```rust
#[tokio::test]
async fn ep_020_workspace_is_abandoned() {
    // Arrange: Abandon workspace
    // Act: Submit session
    // Assert: Validation error
}
```

---

### 3.21 EP-021: Workspace Has No Bookmark

**Given** workspace has no bookmarks
**When** identity extraction is attempted
**Then** `IdentityExtractionFailed` or `NoBookmark` error is returned

```rust
#[tokio::test]
async fn ep_021_workspace_has_no_bookmark() {
    // Arrange: Create workspace without bookmark
    // Act: Extract identity
    // Assert: NoBookmark error
}
```

---

### 3.22 EP-022: Dirty Workspace Without Auto-Commit

**Given** workspace has uncommitted changes
**And** --auto-commit is not set
**When** submission is attempted
**Then** validation fails
**And** error suggests --auto-commit or 'jj commit'

```rust
#[tokio::test]
async fn ep_022_dirty_workspace_without_auto_commit() {
    // Arrange: Create dirty workspace
    // Act: Submit without auto-commit
    // Assert: Dirty workspace error
}
```

---

## 4. Edge Case Tests

### 4.1 EC-001: Empty Workspace Name

**Given** workspace name is ""
**When** submission is attempted
**Then** error is returned
**And** error indicates invalid workspace name

```rust
#[tokio::test]
async fn ec_001_empty_workspace_name() {
    // Arrange: Use empty workspace name
    // Act: Submit
    // Assert: Invalid workspace name error
}
```

---

### 4.2 EC-002: Very Long Workspace Name

**Given** workspace name is 1000 characters
**When** submission is attempted
**Then** submission succeeds or fails gracefully
**And** no buffer overflow occurs

```rust
#[tokio::test]
async fn ec_002_very_long_workspace_name() {
    // Arrange: Create 1000-char workspace name
    // Act: Submit
    // Assert: Graceful handling
}
```

---

### 4.3 EC-003: Special Characters in Workspace Name

**Given** workspace name contains special characters: "../../etc/passwd"
**When** submission is attempted
**Then** path is validated or sanitized
**And** no directory traversal occurs

```rust
#[tokio::test]
async fn ec_003_special_characters_in_workspace_name() {
    // Arrange: Use path with special chars
    // Act: Submit
    // Assert: Path validation prevents traversal
}
```

---

### 4.4 EC-004: Unicode in Workspace Name

**Given** workspace name is "功能测试"
**When** submission is attempted
**Then** submission succeeds
**And** Unicode is preserved correctly

```rust
#[tokio::test]
async fn ec_004_unicode_in_workspace_name() {
    // Arrange: Create Unicode workspace name
    // Act: Submit
    // Assert: Unicode preserved
}
```

---

### 4.5 EC-005: Zero Priority

**Given** priority is set to 0
**When** submission is attempted
**Then** submission succeeds
**And** entry has highest priority

```rust
#[tokio::test]
async fn ec_005_zero_priority() {
    // Arrange: Use priority 0
    // Act: Submit
    // Assert: Priority 0 accepted
}
```

---

### 4.6 EC-006: Negative Priority

**Given** priority is set to -1
**When** submission is attempted
**Then** submission succeeds or fails with clear error

```rust
#[tokio::test]
async fn ec_006_negative_priority() {
    // Arrange: Use priority -1
    // Act: Submit
    // Assert: Accepted or rejected with clear error
}
```

---

### 4.7 EC-007: Very Large Priority Value

**Given** priority is set to i32::MAX
**When** submission is attempted
**Then** submission succeeds
**And** priority is stored correctly

```rust
#[tokio::test]
async fn ec_007_very_large_priority_value() {
    // Arrange: Use i32::MAX priority
    // Act: Submit
    // Assert: Priority stored correctly
}
```

---

### 4.8 EC-008: Empty Bead ID

**Given** bead_id is None
**When** submission is attempted
**Then** submission succeeds
**And** bead_id field is NULL in database

```rust
#[tokio::test]
async fn ec_008_empty_bead_id() {
    // Arrange: Use None for bead_id
    // Act: Submit
    // Assert: bead_id is NULL
}
```

---

### 4.9 EC-009: Empty Agent ID

**Given** agent_id is None
**When** submission is attempted
**Then** submission succeeds
**And** agent_id field is NULL in database

```rust
#[tokio::test]
async fn ec_009_empty_agent_id() {
    // Arrange: Use None for agent_id
    // Act: Submit
    // Assert: agent_id is NULL
}
```

---

### 4.10 EC-010: Concurrent Submissions with Same Dedupe Key

**Given** no entry exists for dedupe_key
**And** two agents submit same workspace simultaneously
**When** both submissions execute
**Then** only one entry is created
**And** the other submission updates the entry or returns success

```rust
#[tokio::test]
async fn ec_010_concurrent_submissions_same_dedupe_key() {
    // Arrange: Spawn two concurrent submissions
    // Act: Execute both
    // Assert: Only one entry created
}
```

---

### 4.11 EC-011: Submission During State Transition

**Given** entry is transitioning from "pending" to "claimed"
**When** submission with same dedupe_key is attempted
**Then** submission succeeds (updates entry)
**And** no race condition occurs

```rust
#[tokio::test]
async fn ec_011_submission_during_state_transition() {
    // Arrange: Start state transition
    // Act: Submit during transition
    // Assert: No race, entry updated correctly
}
```

---

### 4.12 EC-012: Empty Head SHA

**Given** head_sha is ""
**When** submission is attempted
**Then** `InvalidHeadSha` error is returned

```rust
#[tokio::test]
async fn ec_012_empty_head_sha() {
    // Arrange: Use empty head_sha
    // Act: Submit
    // Assert: InvalidHeadSha error
}
```

---

### 4.13 EC-013: Head SHA with Invalid Characters

**Given** head_sha is "not-valid!@#$%"
**When** submission is attempted
**Then** `InvalidHeadSha` error is returned

```rust
#[tokio::test]
async fn ec_013_head_sha_with_invalid_characters() {
    // Arrange: Use invalid head_sha
    // Act: Submit
    // Assert: InvalidHeadSha error
}
```

---

### 4.14 EC-014: Dedupe Key with Multiple Colons

**Given** dedupe_key is "ws:change:extra"
**When** submission is attempted
**Then** submission succeeds or fails with clear error
**And** format is validated consistently

```rust
#[tokio::test]
async fn ec_014_dedupe_key_with_multiple_colons() {
    // Arrange: Use dedupe_key with multiple colons
    // Act: Submit
    // Assert: Clear success or error
}
```

---

## 5. Contract Verification Tests

### 5.1 CV-001: PRE-WS-001 Verification

**Test:** Workspace must exist
**Verify:** Precondition fails for non-existent workspace

```rust
#[tokio::test]
async fn cv_001_pre_ws_001_verification() {
    // Test: Submit non-existent workspace
    // Assert: SessionNotFound error
}
```

---

### 5.2 CV-002: PRE-WS-002 Verification

**Test:** Workspace must not be abandoned
**Verify:** Precondition fails for abandoned workspace

```rust
#[tokio::test]
async fn cv_002_pre_ws_002_verification() {
    // Test: Submit abandoned workspace
    // Assert: Invalid workspace state error
}
```

---

### 5.3 CV-003: PRE-REMOTE-001 Verification

**Test:** Remote must be configured
**Verify:** Precondition fails without remote

```rust
#[tokio::test]
async fn cv_003_pre_remote_001_verification() {
    // Test: Remove remote, submit
    // Assert: Remote not configured error
}
```

---

### 5.4 CV-004: PRE-REMOTE-002 Verification

**Test:** Remote must be reachable
**Verify:** Precondition fails with unreachable remote

```rust
#[tokio::test]
async fn cv_004_pre_remote_002_verification() {
    // Test: Make remote unreachable, submit
    // Assert: RemoteUnreachable error
}
```

---

### 5.5 CV-005: PRE-REMOTE-003 Verification

**Test:** Bookmark push must succeed
**Verify:** Precondition fails if push fails

```rust
#[tokio::test]
async fn cv_005_pre_remote_003_verification() {
    // Test: Fail bookmark push
    // Assert: BookmarkPushFailed error
}
```

---

### 5.6 CV-006: PRE-ID-001 Verification

**Test:** change_id must be extractable
**Verify:** Precondition fails if extraction fails

```rust
#[tokio::test]
async fn cv_006_pre_id_001_verification() {
    // Test: Fail change_id extraction
    // Assert: ChangeIdExtractionFailed error
}
```

---

### 5.7 CV-007: PRE-ID-002 Verification

**Test:** head_sha must be extractable
**Verify:** Precondition fails if extraction fails

```rust
#[tokio::test]
async fn cv_007_pre_id_002_verification() {
    // Test: Fail head_sha extraction
    // Assert: HeadShaExtractionFailed error
}
```

---

### 5.8 CV-008: PRE-Q-001 Verification

**Test:** Dedupe_key must not conflict with active entries
**Verify:** Precondition fails on conflict

```rust
#[tokio::test]
async fn cv_008_pre_q_001_verification() {
    // Test: Create conflicting dedupe_key
    // Assert: DedupeKeyConflict error
}
```

---

### 5.9 CV-009: PRE-Q-002 Verification

**Test:** If entry exists, workspace must match
**Verify:** Precondition fails on workspace mismatch

```rust
#[tokio::test]
async fn cv_009_pre_q_002_verification() {
    // Test: Submit different workspace with same dedupe_key
    // Assert: DedupeKeyConflict or rejection
}
```

---

### 5.10 CV-010: PRE-Q-003 Verification

**Test:** Terminal entries can be reset
**Verify:** Reset succeeds for terminal entries

```rust
#[tokio::test]
async fn cv_010_pre_q_003_verification() {
    // Test: Resubmit terminal entry
    // Assert: Entry reset to pending
}
```

---

### 5.11 CV-011: POST-ENTRY-001 Verification

**Test:** Entry exists in merge_queue table
**Verify:** Postcondition holds after submission

```rust
#[tokio::test]
async fn cv_011_post_entry_001_verification() {
    // Test: Submit session
    // Verify: Entry exists in database
}
```

---

### 5.12 CV-012: POST-ENTRY-002 Verification

**Test:** Entry has unique ID
**Verify:** Postcondition holds

```rust
#[tokio::test]
async fn cv_012_post_entry_002_verification() {
    // Test: Submit session, check ID
    // Verify: ID > 0 and unique
}
```

---

### 5.13 CV-013: POST-ENTRY-006 Verification

**Test:** Entry status is 'pending' (for new)
**Verify:** Postcondition holds

```rust
#[tokio::test]
async fn cv_013_post_entry_006_verification() {
    // Test: Submit new session
    // Verify: status == QueueStatus::Pending
}
```

---

### 5.14 CV-014: POST-POS-001 Verification

**Test:** Position is assigned if status is 'pending'
**Verify:** Postcondition holds

```rust
#[tokio::test]
async fn cv_014_post_pos_001_verification() {
    // Test: Submit new session
    // Verify: position > 0
}
```

---

### 5.15 CV-015: POST-POS-004 Verification

**Test:** Total pending count is accurate
**Verify:** Postcondition holds

```rust
#[tokio::test]
async fn cv_015_post_pos_004_verification() {
    // Test: Submit session
    // Verify: pending_count == COUNT(*)
}
```

---

### 5.16 CV-016: POST-DEDUPE-001 Verification

**Test:** No two active entries share same dedupe_key
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_016_post_dedupe_001_verification() {
    // Property test: UNIQUE constraint holds
}
```

---

### 5.17 CV-017: POST-AUDIT-001 Verification

**Test:** Event is logged to queue_events table
**Verify:** Postcondition holds

```rust
#[tokio::test]
async fn cv_017_post_audit_001_verification() {
    // Test: Submit session
    // Verify: Audit event exists
}
```

---

### 5.18 CV-018: INV-QUEUE-001 Verification

**Test:** No two active entries have same dedupe_key
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_018_inv_queue_001_verification() {
    // Property test: Check all active entries
}
```

---

### 5.19 CV-019: INV-QUEUE-002 Verification

**Test:** Each workspace has at most one active entry
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_019_inv_queue_002_verification() {
    // Property test: GROUP BY workspace HAVING COUNT(*) <= 1
}
```

---

### 5.20 CV-020: INV-QUEUE-004 Verification

**Test:** Position values form contiguous sequence
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_020_inv_queue_004_verification() {
    // Property test: No gaps in positions
}
```

---

### 5.21 CV-021: INV-STATE-004 Verification

**Test:** Pending entries always have a position
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_021_inv_state_004_verification() {
    // Property test: All pending entries have position
}
```

---

### 5.22 CV-022: INV-STATE-005 Verification

**Test:** Non-pending entries never have a position
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_022_inv_state_005_verification() {
    // Property test: Non-pending entries have NULL position
}
```

---

### 5.23 CV-023: INV-CONC-001 Verification

**Test:** Submission is atomic
**Verify:** Invariant holds

```rust
#[tokio::test]
async fn cv_023_inv_conc_001_verification() {
    // Test: Kill submission mid-process
    // Verify: No partial state
}
```

---

### 5.24 CV-024: Error Variant Serialization

**Test:** All QueueSubmissionError variants are serializable
**Verify:** JSON round-trip for each variant

```rust
#[tokio::test]
async fn cv_024_error_variant_serialization() {
    // Test: Serialize/deserialize each error variant
    // Verify: Round-trip succeeds
}
```

---

## 6. End-to-End Scenarios

### 6.1 E2E-001: Full Submission Workflow

**Scenario:** Developer submits workspace for processing

**Given** a developer has workspace "feature-auth"
**And** workspace has 3 commits
**And** bookmark "feature-auth" exists
**And** remote is "origin" and is reachable
**When** developer runs `zjj queue submit feature-auth`
**Then** bookmark is pushed to remote
**And** workspace identity is extracted
**And** entry is created in merge queue
**And** entry status is "pending"
**And** position is assigned
**And** audit event is logged
**And** success response is returned

```rust
#[tokio::test]
async fn e2e_001_full_submission_workflow() {
    // Setup: Create workspace with commits and bookmark
    // Execute: Run zjj queue submit
    // Verify: All steps complete successfully
}
```

---

### 6.2 E2E-002: Submission with Auto-Commit

**Scenario:** Dirty workspace with --auto-commit

**Given** a workspace has uncommitted changes
**And** --auto-commit flag is set
**When** submission is executed
**Then** changes are committed automatically
**And** new head_sha is extracted
**And** submission proceeds with new head_sha
**And** entry is created successfully

```rust
#[tokio::test]
async fn e2e_002_submission_with_auto_commit() {
    // Setup: Create dirty workspace
    // Execute: Submit with --auto-commit
    // Verify: Changes committed, entry created
}
```

---

### 6.3 E2E-003: Idempotent Resubmission

**Scenario:** Same workspace submitted multiple times

**Given** workspace "feature-auth" is submitted once
**And** entry is in "pending" status
**When** same workspace is submitted again
**Then** existing entry is updated
**And** head_sha is updated
**And** no duplicate entry is created
**And** position remains the same

```rust
#[tokio::test]
async fn e2e_003_idempotent_resubmission() {
    // Setup: Submit workspace
    // Execute: Submit same workspace again
    // Verify: Entry updated, not duplicated
}
```

---

### 6.4 E2E-004: Terminal Entry Resubmission

**Scenario:** Resubmit after merge completion

**Given** workspace "feature-auth" was merged
**And** entry status is "merged"
**When** workspace is submitted again with new changes
**Then** entry is reset to "pending"
**And** started_at and completed_at are cleared
**And** new head_sha is set
**And** new position is assigned
**And** audit event "created" or "updated" is logged

```rust
#[tokio::test]
async fn e2e_004_terminal_entry_resubmission() {
    // Setup: Create merged entry
    // Execute: Resubmit with new head_sha
    // Verify: Entry reset to pending
}
```

---

### 6.5 E2E-005: Priority-Based Queue Ordering

**Scenario:** Multiple entries with different priorities

**Given** 3 entries exist with priorities 5, 3, 7
**And** positions are 1 (priority 3), 2 (priority 5), 3 (priority 7)
**When** new entry is submitted with priority 1
**Then** new entry gets position 1
**And** existing positions shift to 2, 3, 4, 5
**And** priority order is preserved

```rust
#[tokio::test]
async fn e2e_005_priority_based_queue_ordering() {
    // Setup: Create entries with mixed priorities
    // Execute: Submit high-priority entry
    // Verify: Positions shifted correctly
}
```

---

### 6.6 E2E-006: Deduplication Across Workspaces

**Scenario:** Same change_id in different workspaces

**Given** workspace "ws-a" has change_id "kxyz789"
**And** workspace "ws-b" has same change_id "kxyz789"
**When** both workspaces are submitted
**Then** both entries are created (different dedupe_keys)
**And** ws-a has dedupe_key "ws-a:kxyz789"
**And** ws-b has dedupe_key "ws-b:kxyz789"

```rust
#[tokio::test]
async fn e2e_006_deduplication_across_workspaces() {
    // Setup: Create two workspaces with same change_id
    // Execute: Submit both
    // Verify: Both entries created with unique dedupe_keys
}
```

---

### 6.7 E2E-007: Concurrent Submission Handling

**Scenario:** Two agents submit same workspace simultaneously

**Given** no entry exists for workspace "shared-feature"
**And** agent-A and agent-B submit simultaneously
**When** both submissions execute
**Then** only one entry is created
**And** both submissions return success
**And** one returns "New" type, other returns "Updated" type

```rust
#[tokio::test]
async fn e2e_007_concurrent_submission_handling() {
    // Setup: Spawn two concurrent submissions
    // Execute: Both submit same workspace
    // Verify: One entry created, both succeed
}
```

---

### 6.8 E2E-008: Full Queue Lifecycle

**Scenario:** Entry goes through full lifecycle

**Given** workspace is submitted to queue
**When** worker processes entry through states
**Then** entry transitions: pending -> claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
**And** each transition is logged in audit trail
**And** position is cleared when entry leaves "pending"
**And** terminal state prevents further modification

```rust
#[tokio::test]
async fn e2e_008_full_queue_lifecycle() {
    // Setup: Submit entry
    // Execute: Process through all states
    // Verify: All transitions logged correctly
}
```

---

### 6.9 E2E-009: Error Recovery and Retry

**Scenario:** Submission fails, then succeeds on retry

**Given** remote is unreachable
**When** submission is attempted
**Then** `RemoteUnreachable` error is returned
**When** remote becomes available
**And** submission is retried
**Then** submission succeeds
**And** entry is created

```rust
#[tokio::test]
async fn e2e_009_error_recovery_and_retry() {
    // Setup: Make remote unreachable
    // Execute: Submit (fails), fix remote, retry
    // Verify: Retry succeeds
}
```

---

### 6.10 E2E-010: Dry Run Submission

**Scenario:** Preview submission without creating entry

**Given** workspace "feature-auth" exists
**When** submission is executed with --dry-run
**Then** validation is performed
**And** bookmark push is NOT executed
**And** entry is NOT created
**And** preview of submission is shown
**And** return code is 0

```rust
#[tokio::test]
async fn e2e_010_dry_run_submission() {
    // Setup: Create workspace
    // Execute: Submit with --dry-run
    // Verify: No entry created, preview shown
}
```

---

### 6.11 E2E-011: Multi-Agent Queue Access

**Scenario:** Different agents submit to same queue

**Given** agent-A submits workspace "feature-a"
**And** agent-B submits workspace "feature-b"
**When** both submissions execute
**Then** both entries are created
**And** each entry has correct agent_id
**And** positions are assigned correctly
**And** audit trail shows both agents

```rust
#[tokio::test]
async fn e2e_011_multi_agent_queue_access() {
    // Setup: Configure two agents
    // Execute: Both submit to queue
    // Verify: Both entries created with correct agent_id
}
```

---

### 6.12 E2E-012: Queue Status Query After Submission

**Scenario:** Query queue status immediately after submission

**Given** workspace is submitted to queue
**When** queue status is queried
**Then** submitted entry appears in status output
**And** entry shows correct status, position, priority
**And** total pending count includes new entry

```rust
#[tokio::test]
async fn e2e_012_queue_status_query_after_submission() {
    // Setup: Submit workspace
    // Execute: Query queue status
    // Verify: Entry appears in status
}
```

---

## 7. Test Implementation Guidelines

### 7.1 Test Structure

```rust
// Standard test structure for E2E tests
#[tokio::test]
async fn test_name() {
    // === ARRANGE ===
    // Set up repository state
    // Create workspaces, configure remotes as needed

    // === ACT ===
    // Execute the operation under test

    // === ASSERT ===
    // Verify results match expected outcomes
    // Check all relevant postconditions
}
```

### 7.2 Test Helpers Required

```rust
/// Create a test repository with queue database
async fn create_test_queue_repo() -> TestQueueRepo;

/// Create a test workspace with specified state
async fn create_test_workspace(name: &str, options: WorkspaceOptions) -> TestWorkspace;

/// Submit a session to the queue
async fn submit_session(request: QueueSubmissionRequest) -> Result<QueueSubmissionResponse>;

/// Get queue entry by workspace
async fn get_entry_by_workspace(workspace: &str) -> Option<QueueEntry>;

/// Get queue position for entry
async fn get_position(entry_id: i64) -> Option<usize>;

/// Assert entry matches expected state
fn assert_entry_state(entry: &QueueEntry, expected: &ExpectedEntryState);
```

### 7.3 Test Isolation

- Each test must use a temporary queue database
- Tests must not share state
- Tests must clean up resources even on failure
- Tests must be parallelizable

### 7.4 Graphite-Style Semantics Testing

- Test deduplication with same change_id across rebases
- Test priority ordering and FIFO within priority
- Test position recalculation on status changes
- Test terminal state handling and resubmission
- Test idempotent upsert behavior

---

## 8. Coverage Matrix

| Component | Happy Path | Error Path | Edge Case | Contract | E2E |
|-----------|------------|------------|-----------|----------|-----|
| submit_to_queue | 5 | 8 | 6 | 6 | 6 |
| validate_workspace | 2 | 3 | 2 | 2 | 1 |
| extract_workspace_identity | 2 | 3 | 1 | 2 | 1 |
| compute_dedupe_key | 1 | 1 | 2 | 0 | 1 |
| push_bookmark_to_remote | 1 | 2 | 0 | 3 | 1 |
| upsert_for_submit | 3 | 2 | 2 | 4 | 1 |
| get_queue_position | 1 | 0 | 0 | 2 | 0 |
| is_in_queue | 1 | 0 | 0 | 0 | 0 |
| Error handling | 0 | 3 | 1 | 5 | 0 |
| **Total** | **16** | **22** | **14** | **24** | **12** |

---

## 9. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-18 | Initial test plan |

---

## 10. References

- Contract Specification: `/home/lewis/src/zjj/contracts/bd-1lx-contract-spec.md`
- Implementation: `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs`
- Queue Repository: `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_repository.rs`
- State Machine: `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_status.rs`
- Submit Command: `/home/lewis/src/zjj/crates/zjj/src/commands/submit.rs`
- Martin Fowler BDD: https://martinfowler.com/bliki/GivenWhenThen.html
