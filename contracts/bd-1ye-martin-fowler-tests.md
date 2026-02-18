# Martin Fowler Test Plan: End-to-End Tests for Merge Train Processing (bd-1ye)

**Bead ID:** bd-1ye
**Title:** Create end-to-end tests for merge train processing
**Test Framework:** BDD with Given-When-Then scenarios
**Version:** 1.0.0

---

## 1. Overview

This test plan follows Martin Fowler's BDD approach with Given-When-Then scenarios for testing end-to-end merge train processing. Tests are organized by category: Happy Path, Error Path, Edge Cases, Contract Verification, and End-to-End Scenarios.

### 1.1 Test Categories

| Category | Purpose | Count |
|----------|---------|-------|
| Happy Path | Normal successful train operations | 25 |
| Error Path | Failure handling and recovery | 30 |
| Edge Cases | Boundary conditions | 20 |
| Contract Verification | Pre/post/invariant validation | 35 |
| End-to-End | Full workflow scenarios | 15 |
| **Total** | | **125** |

---

## 2. Happy Path Tests

### 2.1 HP-001: Process Single Entry Train Successfully

**Given** a single entry exists in queue with position 1
**And** entry status is "pending"
**And** workspace "feature-auth" has passing tests
**And** workspace has no conflicts with target
**When** train processing starts
**Then** entry status transitions to "claimed"
**And** entry status transitions to "testing"
**And** entry status transitions to "ready_to_merge"
**And** entry status transitions to "merging"
**And** entry status transitions to "merged"
**And** TrainStep events are emitted for each action
**And** TrainResult summary shows 1 merged entry

```rust
#[tokio::test]
async fn hp_001_process_single_entry_train_successfully() {
    // Arrange: Create single pending entry
    // Act: Process train
    // Assert: All state transitions, merged status, correct events
}
```

---

### 2.2 HP-002: Process Multiple Entries Sequentially

**Given** 3 entries exist in queue with positions 1, 2, 3
**And** all entries have passing tests
**And** all entries have no conflicts
**When** train processing starts
**Then** entries are processed in position order
**And** each entry transitions to "merged"
**And** TrainStep events are emitted for each entry
**And** TrainResult shows 3 merged entries
**And** processing duration is reasonable

```rust
#[tokio::test]
async fn hp_002_process_multiple_entries_sequentially() {
    // Arrange: Create 3 pending entries
    // Act: Process train
    // Assert: Sequential processing, all merged, correct order
}
```

---

### 2.3 HP-003: Emit TrainStep for Each State Transition

**Given** an entry is being processed
**And** entry transitions through all states
**When** each state transition occurs
**Then** TrainStep event is emitted with correct action
**And** TrainStep includes entry_id, workspace, position
**And** TrainStep includes timestamp
**And** TrainStep status matches current state

```rust
#[tokio::test]
async fn hp_003_emit_train_step_for_each_state_transition() {
    // Arrange: Create entry
    // Act: Process entry and capture events
    // Assert: Each transition has corresponding TrainStep
}
```

---

### 2.4 HP-004: Emit TrainResult Summary at End

**Given** train processing completes
**And** 2 entries merged, 1 entry failed
**When** train finishes
**Then** TrainResult event is emitted
**And** TrainResult shows total_processed: 3
**And** TrainResult shows 2 merged entries
**And** TrainResult shows 1 failed entry
**And** TrainResult shows duration_secs
**And** TrainResult shows agent_id

```rust
#[tokio::test]
async fn hp_004_emit_train_result_summary_at_end() {
    // Arrange: Process mixed results
    // Act: Get final TrainResult
    // Assert: Correct summary fields populated
}
```

---

### 2.5 HP-005: Acquire and Release Processing Lock

**Given** no processing lock is held
**When** train processing starts
**Then** processing lock is acquired
**And** lock table shows active lock
**And** lock includes agent_id and expires_at
**When** train processing completes
**Then** processing lock is released
**And** lock table is empty

```rust
#[tokio::test]
async fn hp_005_acquire_and_release_processing_lock() {
    // Arrange: Ensure no lock
    // Act: Process train
    // Assert: Lock acquired during processing, released after
}
```

---

### 2.6 HP-006: Update Entry Status Through State Machine

**Given** entry is in "pending" state
**When** entry is claimed for processing
**Then** entry status becomes "claimed"
**When** rebase operation starts
**Then** entry status becomes "rebasing"
**When** rebase completes
**Then** entry status becomes "testing"
**When** tests pass
**Then** entry status becomes "ready_to_merge"
**When** merge starts
**Then** entry status becomes "merging"
**When** merge completes
**Then** entry status becomes "merged"

```rust
#[tokio::test]
async fn hp_006_update_entry_status_through_state_machine() {
    // Arrange: Create pending entry
    // Act: Process through all states
    // Assert: Each state transition is correct
}
```

---

### 2.7 HP-007: Execute Tests Successfully

**Given** workspace has test suite configured
**And** all tests pass
**When** train executes tests
**Then** tests run to completion
**And** test results are captured
**And** exit code is 0
**And** TrainStep shows "TestsPassed"
**And** entry progresses to next state

```rust
#[tokio::test]
async fn hp_007_execute_tests_successfully() {
    // Arrange: Create workspace with passing tests
    // Act: Run tests via train
    // Assert: Tests executed, passed state, correct event
}
```

---

### 2.8 HP-008: Detect No Conflicts Before Merge

**Given** entry has passed tests
**And** workspace has no merge conflicts with target
**When** conflict check runs
**Then** no conflicts are detected
**And** entry status becomes "ready_to_merge"
**And** merge operation proceeds

```rust
#[tokio::test]
async fn hp_008_detect_no_conflicts_before_merge() {
    // Arrange: Create entry with clean merge
    // Act: Check conflicts
    // Assert: No conflicts, ready_to_merge state
}
```

---

### 2.9 HP-009: Merge Entry Successfully

**Given** entry is in "ready_to_merge" state
**And** entry has passing tests
**And** entry has no conflicts
**When** merge operation executes
**Then** changes are merged into target branch
**And** bookmark is updated to merge commit
**And** entry status becomes "merged"
**And** completed_at timestamp is set
**And** TrainStep shows "Merged"

```rust
#[tokio::test]
async fn hp_009_merge_entry_successfully() {
    // Arrange: Create ready_to_merge entry
    // Act: Perform merge
    // Assert: Merge committed, bookmark updated, merged status
}
```

---

### 2.10 HP-010: Process Empty Queue Gracefully

**Given** no pending entries exist in queue
**When** train processing starts
**Then** train exits immediately
**And** TrainResult shows total_processed: 0
**And** no errors are reported
**And** lock is released

```rust
#[tokio::test]
async fn hp_010_process_empty_queue_gracefully() {
    // Arrange: Empty queue
    // Act: Process train
    // Assert: Graceful exit, empty result
}
```

---

### 2.11 HP-011: Handle Entry with Retry History

**Given** entry has attempt_count = 2
**And** entry was previously failed_retryable
**And** entry was resubmitted
**When** train processes entry
**Then** entry is processed normally
**And** attempt_count is incremented to 3
**And** processing succeeds if tests pass

```rust
#[tokio::test]
async fn hp_011_handle_entry_with_retry_history() {
    // Arrange: Create entry with attempt_count = 2
    // Act: Process train
    // Assert: Processes normally, count incremented
}
```

---

### 2.12 HP-012: Process Entry with High Priority

**Given** 5 entries exist with mixed priorities
**And** one entry has priority 1 (highest)
**When** train processes entries
**Then** entries are processed in priority order
**And** high priority entry is processed first
**And** positions are respected within priority

```rust
#[tokio::test]
async fn hp_012_process_entry_with_high_priority() {
    // Arrange: Create entries with different priorities
    // Act: Process train
    // Assert: Priority ordering respected
}
```

---

### 2.13 HP-013: Update TestedAgainstSha After Merge

**Given** entry is merged successfully
**And** merge commit SHA is "abc123"
**When** merge completes
**Then** entry tested_against_sha is set to "abc123"
**And** subsequent entries can rebase onto this SHA

```rust
#[tokio::test]
async fn hp_013_update_tested_against_sha_after_merge() {
    // Arrange: Merge entry
    // Act: Check entry after merge
    // Assert: tested_against_sha populated correctly
}
```

---

### 2.14 HP-014: Record Timestamps for Each State

**Given** entry progresses through states
**When** each state transition occurs
**Then** appropriate timestamp is set
**And** added_at is set on creation
**And** started_at is set on claimed
**And** completed_at is set on terminal
**And** state_changed_at is updated on each transition

```rust
#[tokio::test]
async fn hp_014_record_timestamps_for_each_state() {
    // Arrange: Process entry
    // Act: Check timestamps after each transition
    // Assert: All timestamps recorded correctly
}
```

---

### 2.15 HP-015: Preserve DedupeKey Through Processing

**Given** entry has dedupe_key "feature-auth:kxyz789"
**When** entry is processed through entire train
**Then** dedupe_key remains unchanged
**And** dedupe_key is preserved in database

```rust
#[tokio::test]
async fn hp_015_preserve_dedupe_key_through_processing() {
    // Arrange: Create entry with dedupe_key
    // Act: Process through train
    // Assert: dedupe_key unchanged
}
```

---

### 2.16 HP-016: Emit Valid JSONL Output

**Given** train processing completes
**When** JSONL output is parsed
**Then** each line is valid JSON
**And** each JSON has "type" field
**And** timestamps are ISO 8601 format
**And** all required fields are present

```rust
#[tokio::test]
async fn hp_016_emit_valid_jsonl_output() {
    // Arrange: Process train
    // Act: Capture and parse JSONL
    // Assert: All valid JSON, correct fields
}
```

---

### 2.17 HP-017: Handle Concurrent Train Requests

**Given** train is actively processing
**When** another agent attempts to process train
**Then** second request receives TrainAlreadyActive error
**And** first train continues uninterrupted
**And** lock remains with first agent

```rust
#[tokio::test]
async fn hp_017_handle_concurrent_train_requests() {
    // Arrange: Start train processing
    // Act: Attempt second train
    // Assert: Error returned, first train continues
}
```

---

### 2.18 HP-018: Process Entry with Bead ID

**Given** entry has bead_id "bd-1ye"
**When** entry is processed
**Then** bead_id is preserved in TrainStep events
**And** bead_id is included in TrainResult

```rust
#[tokio::test]
async fn hp_018_process_entry_with_bead_id() {
    // Arrange: Create entry with bead_id
    // Act: Process train
    // Assert: bead_id in events
}
```

---

### 2.19 HP-019: Process Entry with Agent ID

**Given** entry was submitted by agent_id "agent-prod-001"
**When** train processes entry
**Then** agent_id is recorded in audit events
**And** agent_id is included in TrainResult

```rust
#[tokio::test]
async fn hp_019_process_entry_with_agent_id() {
    // Arrange: Create entry with agent_id
    // Act: Process train
    // Assert: agent_id in result and audit
}
```

---

### 2.20 HP-020: Enforce Test Timeout

**Given** entry has test_timeout_secs = 60
**And** tests take longer than 60 seconds
**When** test execution runs
**Then** tests are terminated after timeout
**And** TestTimeout error is returned
**And** entry is marked as failed_retryable

```rust
#[tokio::test]
async fn hp_020_enforce_test_timeout() {
    // Arrange: Create entry with slow test
    // Act: Process with timeout
    // Assert: Timeout enforced, error returned
}
```

---

### 2.21 HP-021: Skip Already Merged Entries

**Given** entry status is already "merged"
**When** train processes queue
**Then** entry is skipped
**And** no TrainStep events are emitted
**And** position is removed

```rust
#[tokio::test]
async fn hp_021_skip_already_merged_entries() {
    // Arrange: Create merged entry
    // Act: Process train
    // Assert: Entry skipped, no events
}
```

---

### 2.22 HP-022: Calculate Train Duration Accurately

**Given** train starts at time T1
**And** train completes at time T2
**When** TrainResult is emitted
**Then** duration_secs = T2 - T1
**And** duration is reasonable for workload

```rust
#[tokio::test]
async fn hp_022_calculate_train_duration_accurately() {
    // Arrange: Record start time
    // Act: Process train
    // Assert: Duration matches actual time
}
```

---

### 2.23 HP-023: Update Attempt Count on Failure

**Given** entry fails during processing
**And** attempt_count was 0
**When** failure is recorded
**Then** attempt_count becomes 1
**And** failed_retryable status is set

```rust
#[tokio::test]
async fn hp_023_update_attempt_count_on_failure() {
    // Arrange: Create entry that will fail
    // Act: Process train
    // Assert: attempt_count incremented
}
```

---

### 2.24 HP-024: Set ErrorMessage on Failure

**Given** entry fails with reason "tests failed"
**When** failure is recorded
**Then** error_message field contains "tests failed"
**And** error is human-readable
**And** error is included in TrainResult

```rust
#[tokio::test]
async fn hp_024_set_error_message_on_failure() {
    // Arrange: Create failing entry
    // Act: Process train
    // Assert: error_message set correctly
}
```

---

### 2.25 HP-025: Process Entry Successfully After Retry

**Given** entry failed on first attempt
**And** issue was fixed
**And** entry was resubmitted
**When** train processes entry
**Then** entry is processed successfully
**And** attempt_count reflects retry
**And** entry reaches merged status

```rust
#[tokio::test]
async fn hp_025_process_entry_successfully_after_retry() {
    // Arrange: Fail, fix, resubmit
    // Act: Process train
    // Assert: Success on retry
}
```

---

## 3. Error Path Tests

### 3.1 EP-001: Handle Test Failure

**Given** entry has failing tests
**When** test execution completes with non-zero exit
**Then** TestExecutionFailed error is returned
**And** entry status becomes "failed_retryable"
**And** error_message describes test failure
**And** attempt_count is incremented
**And** TrainStep shows "TestsFailed"
**And** train continues with next entry

```rust
#[tokio::test]
async fn ep_001_handle_test_failure() {
    // Arrange: Create entry with failing test
    // Act: Process train
    // Assert: Error handled, entry marked failed_retryable
}
```

---

### 3.2 EP-002: Handle Merge Conflicts

**Given** entry has passing tests
**And** workspace has conflicts with target branch
**When** conflict check runs
**Then** MergeConflictsDetected error is returned
**And** entry is kicked from queue
**And** subsequent entries are rebased
**And** train restarts from beginning
**And** TrainResult includes kicked entry

```rust
#[tokio::test]
async fn ep_002_handle_merge_conflicts() {
    // Arrange: Create entry with conflicts
    // Act: Process train
    // Assert: Kick triggered, rebase initiated, train restarted
}
```

---

### 3.3 EP-003: Handle Merge Operation Failure

**Given** entry has passing tests and no conflicts
**And** merge command fails due to internal error
**When** merge operation executes
**Then** MergeFailed error is returned
**And** entry status becomes "failed_retryable"
**And** error_message describes merge failure
**And** train continues with next entry

```rust
#[tokio::test]
async fn ep_003_handle_merge_operation_failure() {
    // Arrange: Create entry, mock merge failure
    // Act: Process train
    // Assert: Merge failed error, entry retryable
}
```

---

### 3.4 EP-004: Handle Rebase Failure During Recovery

**Given** entry was kicked due to conflicts
**And** subsequent entry fails to rebase
**When** rebase operation executes
**Then** RebaseFailed error is returned
**And** failing entry is marked as "failed_terminal"
**And** other entries continue rebasing
**And** train processes remaining entries

```rust
#[tokio::test]
async fn ep_004_handle_rebase_failure_during_recovery() {
    // Arrange: Kick entry, create un-rebasable entry
    // Act: Handle failure
    // Assert: Rebase failed, entry terminal
}
```

---

### 3.5 EP-005: Handle Test Framework Not Configured

**Given** workspace has no test configuration
**When** test execution attempts
**Then** TestFrameworkNotConfigured error is returned
**And** entry is marked as "failed_terminal"
**And** train stops processing

```rust
#[tokio::test]
async fn ep_005_handle_test_framework_not_configured() {
    // Arrange: Create entry without tests
    // Act: Process train
    // Assert: Fatal error, train stops
}
```

---

### 3.6 EP-006: Handle Workspace Not Found

**Given** entry references workspace "missing-workspace"
**And** workspace directory does not exist
**When** train processes entry
**Then** EntryNotFound error is returned
**And** entry is skipped
**And** train continues with next entry

```rust
#[tokio::test]
async fn ep_006_handle_workspace_not_found() {
    // Arrange: Create entry with non-existent workspace
    // Act: Process train
    // Assert: Entry skipped, train continues
}
```

---

### 3.7 EP-007: Handle Invalid Entry State

**Given** entry status is "merged" (terminal)
**And** entry is in pending queue
**When** train attempts to process entry
**Then** InvalidEntryState error is returned
**And** entry is skipped
**And** train continues

```rust
#[tokio::test]
async fn ep_007_handle_invalid_entry_state() {
    // Arrange: Create terminal entry in queue
    // Act: Process train
    // Assert: Invalid state error, entry skipped
}
```

---

### 3.8 EP-008: Handle Database Transaction Failure

**Given** database becomes unavailable during processing
**When** transaction attempts to update entry
**Then** TransactionFailed error is returned
**And** partial state is rolled back
**And** lock is released
**And** error is reported

```rust
#[tokio::test]
async fn ep_008_handle_database_transaction_failure() {
    // Arrange: Mock DB failure
    // Act: Process train
    // Assert: Transaction error, rollback, lock released
}
```

---

### 3.9 EP-009: Handle Lock Acquisition Failure

**Given** processing lock is held by another agent
**And** lock has not expired
**When** train attempts to start
**Then** LockAcquisitionFailed error is returned
**And** train does not start
**And** existing lock is unaffected

```rust
#[tokio::test]
async fn ep_009_handle_lock_acquisition_failure() {
    // Arrange: Acquire lock
    // Act: Attempt second train
    // Assert: Lock acquisition fails
}
```

---

### 3.10 EP-010: Handle JJ Command Execution Failure

**Given** JJ binary is not available
**When** train attempts to run JJ command
**Then** JjExecutionFailed error is returned
**And** entry is marked as failed_retryable
**And** error includes command and exit code

```rust
#[tokio::test]
async fn ep_010_handle_jj_command_execution_failure() {
    // Arrange: Mock JJ not found
    // Act: Process train
    // Assert: JJ execution error
}
```

---

### 3.11 EP-011: Handle Multiple Entries Failing

**Given** 5 entries in queue
**And** 3 entries have failing tests
**When** train processes
**Then** all 5 entries are attempted
**And** 2 entries merge successfully
**And** 3 entries fail_retryable
**And** TrainResult shows correct counts

```rust
#[tokio::test]
async fn ep_011_handle_multiple_entries_failing() {
    // Arrange: Create mixed queue
    // Act: Process train
    // Assert: Correct success/failure counts
}
```

---

### 3.12 EP-012: Handle Train Stopping After Multiple Failures

**Given** 3 entries fail with failed_terminal status
**When** third terminal failure occurs
**Then** train stops processing
**And** MultipleUnrecoverableFailures error is returned
**And** remaining entries are not processed
**And** TrainResult shows partial completion

```rust
#[tokio::test]
async fn ep_012_handle_train_stopping_after_multiple_failures() {
    // Arrange: Create queue with many fatal failures
    // Act: Process train
    // Assert: Train stops after threshold
}
```

---

### 3.13 EP-013: Handle Bookmark Update Failure

**Given** merge completes successfully
**And** bookmark push fails
**When** bookmark update attempts
**Then** BookmarkUpdateFailed error is returned
**And** entry is marked as failed_retryable
**And** merge commit exists but bookmark not updated
**And** manual recovery needed

```rust
#[tokio::test]
async fn ep_013_handle_bookmark_update_failure() {
    // Arrange: Merge entry, mock bookmark failure
    // Act: Update bookmark
    // Assert: Bookmark error, entry retryable
}
```

---

### 3.14 EP-014: Handle Conflict Detection Failure

**Given** conflict detection command fails
**When** conflict check runs
**Then** ConflictDetectionFailed error is returned
**And** entry is marked as failed_retryable
**And** error includes reason

```rust
#[tokio::test]
async fn ep_014_handle_conflict_detection_failure() {
    // Arrange: Mock conflict check failure
    // Act: Check conflicts
    // Assert: Detection error
}
```

---

### 3.15 EP-015: Handle Rebase Conflict Detected

**Given** entry is being rebased during recovery
**And** rebase encounters conflicts
**When** rebase executes
**Then** RebaseConflictDetected error is returned
**And** entry is marked as failed_terminal
**And** rebase stops for this entry
**And** other entries continue rebasing

```rust
#[tokio::test]
async fn ep_015_handle_rebase_conflict_detected() {
    // Arrange: Setup rebase with conflict
    // Act: Execute rebase
    // Assert: Conflict error, entry terminal
}
```

---

### 3.16 EP-016: Handle Kick Operation Failure

**Given** entry needs to be kicked
**And** database update fails
**When** kick attempts
**Then** KickFailed error is returned
**And** entry remains in current state
**And** train recovery fails

```rust
#[tokio::test]
async fn ep_016_handle_kick_operation_failure() {
    // Arrange: Setup kick, mock DB failure
    // Act: Kick entry
    // Assert: Kick fails
}
```

---

### 3.17 EP-017: Handle Position Update Failure

**Given** entries were kicked or rebased
**And** position recalculation fails
**When** position update attempts
**Then** PositionUpdateFailed error is returned
**And** positions may be inconsistent
**And** manual intervention needed

```rust
#[tokio::test]
async fn ep_017_handle_position_update_failure() {
    // Arrange: Kick entry, mock position failure
    // Act: Update positions
    // Assert: Position error
}
```

---

### 3.18 EP-018: Handle Concurrent Modification

**Given** entry is being processed by train
**And** another agent modifies same entry
**When** train attempts state update
**Then** ConcurrentModification error is returned
**And** transaction is rolled back
**And** train may retry or skip entry

```rust
#[tokio::test]
async fn ep_018_handle_concurrent_modification() {
    // Arrange: Process entry, concurrent modify
    // Act: Update state
    // Assert: Concurrent modification error
}
```

---

### 3.19 EP-019: Handle Train Recovery Failure

**Given** train failure occurred
**And** recovery cannot complete
**When** recovery attempts
**Then** TrainRecoveryFailed error is returned
**And** train is in inconsistent state
**And** manual intervention required

```rust
#[tokio::test]
async fn ep_019_handle_train_recovery_failure() {
    // Arrange: Create unrecoverable failure scenario
    // Act: Attempt recovery
    // Assert: Recovery fails
}
```

---

### 3.20 EP-020: Handle Output Emission Failure

**Given** TrainStep event needs to be emitted
**And** output stream is closed or fails
**When** emission attempts
**Then** OutputEmissionFailed error is logged
**And** processing continues
**And** event may be lost

```rust
#[tokio::test]
async fn ep_020_handle_output_emission_failure() {
    // Arrange: Close output stream
    // Act: Emit event
    // Assert: Output error logged
}
```

---

### 3.21 EP-021: Handle Entry Exceeding Max Attempts

**Given** entry has attempt_count = 3
**And** max_attempts = 3
**When** entry fails again
**Then** entry is marked as failed_terminal
**And** MaxAttemptsExceeded error is returned
**And** entry cannot be retried

```rust
#[tokio::test]
async fn ep_021_handle_entry_exceeding_max_attempts() {
    // Arrange: Create entry with max attempts
    // Act: Process and fail
    // Assert: Terminal status, max attempts error
}
```

---

### 3.22 EP-022: Handle Workspace State Conflict

**Given** workspace_state is "conflict"
**And** entry is being processed
**When** train attempts to operate on workspace
**Then** entry is marked as failed_retryable
**And** error indicates workspace conflict
**And** manual resolution needed

```rust
#[tokio::test]
async fn ep_022_handle_workspace_state_conflict() {
    // Arrange: Set workspace state to conflict
    // Act: Process train
    // Assert: Conflict state error
}
```

---

### 3.23 EP-023: Handle Missing DedupeKey

**Given** entry has dedupe_key = NULL
**When** train attempts to process entry
**Then** entry is marked as failed_terminal
**And** error indicates missing dedupe_key
**And** entry is skipped

```rust
#[tokio::test]
async fn ep_023_handle_missing_dedupe_key() {
    // Arrange: Create entry without dedupe_key
    // Act: Process train
    // Assert: Missing key error
}
```

---

### 3.24 EP-024: Handle Missing HeadSha

**Given** entry has head_sha = NULL
**When** train attempts to process entry
**Then** entry is marked as failed_terminal
**And** error indicates missing head_sha
**And** entry is skipped

```rust
#[tokio::test]
async fn ep_024_handle_missing_head_sha() {
    // Arrange: Create entry without head_sha
    // Act: Process train
    // Assert: Missing SHA error
}
```

---

### 3.25 EP-025: Handle Revision Query Failure

**Given** workspace exists
**And** JJ log command fails
**When** train queries current revision
**Then** RevisionQueryFailed error is returned
**And** entry is marked as failed_retryable
**And** error includes reason

```rust
#[tokio::test]
async fn ep_025_handle_revision_query_failure() {
    // Arrange: Mock JJ log failure
    // Act: Query revision
    // Assert: Query error
}
```

---

### 3.26 EP-026: Handle Merge Verification Failure

**Given** merge command succeeds
**And** verification check fails
**When** merge verification runs
**Then** MergeVerificationFailed error is returned
**And** entry is marked as failed_retryable
**And** merge may need to be reverted

```rust
#[tokio::test]
async fn ep_026_handle_merge_verification_failure() {
    // Arrange: Merge, mock verification failure
    // Act: Verify merge
    // Assert: Verification error
}
```

---

### 3.27 EP-027: Handle Rebase Verification Failure

**Given** rebase command succeeds
**And** verification check fails
**When** rebase verification runs
**Then** RebaseVerificationFailed error is returned
**And** entry is marked as failed_retryable
**And** rebase may need to be reverted

```rust
#[tokio::test]
async fn ep_027_handle_rebase_verification_failure() {
    // Arrange: Rebase, mock verification failure
    // Act: Verify rebase
    // Assert: Verification error
}
```

---

### 3.28 EP-028: Handle Disk Space Exhaustion

**Given** disk has insufficient space for test artifacts
**When** test execution attempts
**Then** test fails with I/O error
**And** entry is marked as failed_retryable
**And** error indicates disk space issue

```rust
#[tokio::test]
async fn ep_028_handle_disk_space_exhaustion() {
    // Arrange: Fill disk
    // Act: Run tests
    // Assert: Disk space error
}
```

---

### 3.29 EP-029: Handle Network Timeout During Remote Operations

**Given** remote operation is in progress
**And** network times out
**When** operation waits for response
**Then** operation fails with timeout error
**And** entry is marked as failed_retryable
**And** operation can be retried

```rust
#[tokio::test]
async fn ep_029_handle_network_timeout_during_remote_operations() {
    // Arrange: Mock network timeout
    // Act: Perform remote operation
    // Assert: Timeout error
}
```

---

### 3.30 EP-030: Handle Multiple Conflicts in Same Entry

**Given** workspace has 5 merge conflicts
**When** conflict check runs
**Then** MergeConflictsDetected includes conflict_count = 5
**And** entry is kicked
**And** all conflicts are reported

```rust
#[tokio::test]
async fn ep_030_handle_multiple_conflicts_in_same_entry() {
    // Arrange: Create entry with 5 conflicts
    // Act: Check conflicts
    // Assert: Correct count, all reported
}
```

---

## 4. Edge Case Tests

### 4.1 EC-001: Process Queue with Single Entry

**Given** queue has exactly 1 entry
**When** train processes
**Then** entry is processed normally
**And** TrainResult shows total_processed: 1
**And** position calculations work correctly

```rust
#[tokio::test]
async fn ec_001_process_queue_with_single_entry() {
    // Arrange: Single entry
    // Act: Process train
    // Assert: Normal processing
}
```

---

### 4.2 EC-002: Process Very Large Queue (100 entries)

**Given** queue has 100 entries
**When** train processes
**Then** all entries are attempted
**And** train completes in reasonable time
**And** memory usage is bounded
**And** all events are emitted

```rust
#[tokio::test]
async fn ec_002_process_very_large_queue() {
    // Arrange: Create 100 entries
    // Act: Process train
    // Assert: All processed, reasonable time, bounded memory
}
```

---

### 4.3 EC-003: Process Entry with Zero Timeout

**Given** entry has test_timeout_secs = 0
**When** test execution attempts
**Then** default timeout is used
**Or** test runs without timeout
**And** processing continues

```rust
#[tokio::test]
async fn ec_003_process_entry_with_zero_timeout() {
    // Arrange: Create entry with zero timeout
    // Act: Process train
    // Assert: Default timeout or no timeout
}
```

---

### 4.4 EC-004: Process Entry with Very Long Timeout

**Given** entry has test_timeout_secs = 3600 (1 hour)
**When** test execution attempts
**Then** timeout is enforced correctly
**And** train does not hang indefinitely

```rust
#[tokio::test]
async fn ec_004_process_entry_with_very_long_timeout() {
    // Arrange: Create entry with long timeout
    // Act: Process with slow test
    // Assert: Timeout enforced
}
```

---

### 4.5 EC-005: Process Entry with Special Characters in Name

**Given** workspace name contains special chars "feature-@#$%"
**When** entry is processed
**Then** workspace is handled correctly
**And** commands succeed
**And** events contain correct name

```rust
#[tokio::test]
async fn ec_005_process_entry_with_special_characters_in_name() {
    // Arrange: Create entry with special chars
    // Act: Process train
    // Assert: Handled correctly
}
```

---

### 4.6 EC-006: Process Entry with Unicode in Name

**Given** workspace name contains unicode "feature-ño"
**When** entry is processed
**Then** unicode is preserved correctly
**And** commands handle unicode
**And** JSON output is valid UTF-8

```rust
#[tokio::test]
async fn ec_006_process_entry_with_unicode_in_name() {
    // Arrange: Create entry with unicode
    // Act: Process train
    // Assert: Unicode preserved
}
```

---

### 4.7 EC-007: Handle Empty Workspace Name

**Given** workspace name is empty string ""
**When** entry is submitted
**Then** validation fails
**And** entry is not created
**Or** entry is marked as failed_terminal

```rust
#[tokio::test]
async fn ec_007_handle_empty_workspace_name() {
    // Arrange: Create entry with empty name
    // Act: Process train
    // Assert: Validation error or fatal
}
```

---

### 4.8 EC-008: Process Entry with Very Long Error Message

**Given** entry fails with 10KB error message
**When** error is recorded
**Then** error_message is stored
**And** error is included in TrainResult
**And** JSON output handles long message

```rust
#[tokio::test]
async fn ec_008_process_entry_with_very_long_error_message() {
    // Arrange: Create long error message
    // Act: Process and fail
    // Assert: Error stored and emitted
}
```

---

### 4.9 EC-009: Handle Expired Lock

**Given** processing lock exists
**And** lock expires_at is in the past
**When** new train attempts to start
**Then** lock is acquired successfully
**And** old lock is overwritten
**And** train processes normally

```rust
#[tokio::test]
async fn ec_009_handle_expired_lock() {
    // Arrange: Create expired lock
    // Act: Start new train
    // Assert: Lock acquired
}
```

---

### 4.10 EC-010: Process Entry with Negative Priority

**Given** entry has priority = -1
**When** train processes
**Then** entry is processed first (highest priority)
**And** priority sorting works correctly

```rust
#[tokio::test]
async fn ec_010_process_entry_with_negative_priority() {
    // Arrange: Create entry with negative priority
    // Act: Process train
    // Assert: Processed first
}
```

---

### 4.11 EC-011: Handle Very High Attempt Count

**Given** entry has attempt_count = 999
**And** max_attempts = 1000
**When** entry fails and is retried
**Then** attempt_count increments to 1000
**And** entry is still retryable

```rust
#[tokio::test]
async fn ec_011_handle_very_high_attempt_count() {
    // Arrange: Create entry with high count
    // Act: Process and fail
    // Assert: Count increments, still retryable
}
```

---

### 4.12 EC-012: Process Entry That Requires Manual Merge

**Given** entry has conflicts that need manual resolution
**When** automatic merge fails
**Then** entry is kicked
**And** marked for manual intervention
**And** subsequent entries rebased

```rust
#[tokio::test]
async fn ec_012_process_entry_that_requires_manual_merge() {
    // Arrange: Create entry requiring manual merge
    // Act: Process train
    // Assert: Kicked, manual intervention needed
}
```

---

### 4.13 EC-013: Handle Duplicate DedupeKey After Kick

**Given** entry A is kicked
**And** entry B has same dedupe_key as A
**When** rebase occurs
**Then** dedupe_key conflict is handled
**And** entries are deduplicated correctly

```rust
#[tokio::test]
async fn ec_013_handle_duplicate_dedupe_key_after_kick() {
    // Arrange: Kick entry, create duplicate
    // Act: Rebase
    // Assert: Conflict handled
}
```

---

### 4.14 EC-014: Process Entry with No TestedAgainstSha

**Given** entry has tested_against_sha = NULL
**When** entry is processed
**Then** tested_against_sha is set after merge
**And** rebase uses current target

```rust
#[tokio::test]
async fn ec_014_process_entry_with_no_tested_against_sha() {
    // Arrange: Create entry without tested_against_sha
    // Act: Process train
    // Assert: SHA set after merge
}
```

---

### 4.15 EC-015: Handle Concurrent Queue Modifications

**Given** train is processing
**And** new entry is submitted
**When** train queries pending entries
**Then** new entry may or may not be included
**And** no inconsistency occurs

```rust
#[tokio::test]
async fn ec_015_handle_concurrent_queue_modifications() {
    // Arrange: Start train, submit concurrently
    // Act: Process train
    // Assert: No inconsistency
}
```

---

### 4.16 EC-016: Process Entry with Identical HeadSha to Target

**Given** entry head_sha equals target branch SHA
**When** train processes entry
**Then** entry is skipped or merged immediately
**And** no conflicts occur

```rust
#[tokio::test]
async fn ec_016_process_entry_with_identical_head_sha_to_target() {
    // Arrange: Create entry matching target
    // Act: Process train
    // Assert: Skipped or immediate merge
}
```

---

### 4.17 EC-017: Handle Position Gaps After Multiple Kicks

**Given** entries at positions 1, 3, 5 (gaps from previous kicks)
**When** train processes
**Then** positions are recalculated to 1, 2, 3
**And** no gaps remain

```rust
#[tokio::test]
async fn ec_017_handle_position_gaps_after_multiple_kicks() {
    // Arrange: Create gaps in positions
    // Act: Process train
    // Assert: Positions recalculated
}
```

---

### 4.18 EC-018: Process Entry with Merge Commit as Head

**Given** entry head_sha is a merge commit
**When** train processes entry
**Then** merge is handled correctly
**And** no circular merge occurs

```rust
#[tokio::test]
async fn ec_018_process_entry_with_merge_commit_as_head() {
    // Arrange: Create entry with merge commit
    // Act: Process train
    // Assert: Handled correctly
}
```

---

### 4.19 EC-019: Handle Very Large Commit Count

**Given** entry includes 1000 commits
**When** train processes entry
**Then** all commits are merged
**And** merge completes in reasonable time

```rust
#[tokio::test]
async fn ec_019_handle_very_large_commit_count() {
    // Arrange: Create entry with many commits
    // Act: Process train
    // Assert: All merged, reasonable time
}
```

---

### 4.20 EC-020: Process Entry with Binary Files

**Given** workspace includes large binary files
**When** train processes entry
**Then** binary files are handled correctly
**And** merge succeeds
**And** LFS pointers preserved if used

```rust
#[tokio::test]
async fn ec_020_process_entry_with_binary_files() {
    // Arrange: Create entry with binaries
    // Act: Process train
    // Assert: Binaries handled
}
```

---

## 5. Contract Verification Tests

### 5.1 CV-001: Verify PRE-TRAIN-001 (Database Connection Valid)

**Given** database connection is healthy
**When** train processing starts
**Then** train proceeds normally
**And** no database errors occur

**Given** database connection is broken
**When** train processing starts
**Then** TransactionFailed error is returned
**And** train does not start

```rust
#[tokio::test]
async fn cv_001_verify_database_connection_precondition() {
    // Arrange: Healthy and broken DB
    // Act: Start train
    // Assert: Success or error as expected
}
```

---

### 5.2 CV-002: Verify PRE-TRAIN-002 (Processing Lock Available)

**Given** processing lock is available
**When** train starts
**Then** lock is acquired
**And** train processes

**Given** processing lock is held
**When** train starts
**Then** TrainAlreadyActive error is returned
**And** train does not start

```rust
#[tokio::test]
async fn cv_002_verify_processing_lock_precondition() {
    // Arrange: Lock available and held
    // Act: Start train
    // Assert: Lock acquired or error
}
```

---

### 5.3 CV-003: Verify POST-TRAIN-001 (All Processable Entries Attempted)

**Given** 10 entries in queue
**And** 3 fail_retryable, 2 fail_terminal, 5 merge
**When** train completes
**Then** total_processed = 10
**And** all entries were attempted

```rust
#[tokio::test]
async fn cv_003_verify_all_entries_attempted_postcondition() {
    // Arrange: Create queue with various outcomes
    // Act: Process train
    // Assert: All entries attempted
}
```

---

### 5.4 CV-004: Verify POST-TRAIN-004 (Lock Released)

**Given** train acquires lock
**When** train completes (success or failure)
**Then** lock is released
**And** lock table is empty

```rust
#[tokio::test]
async fn cv_004_verify_lock_released_postcondition() {
    // Arrange: Start train
    // Act: Complete train
    // Assert: Lock released
}
```

---

### 5.5 CV-005: Verify INV-TRAIN-001 (Sequential Position Processing)

**Given** entries at positions 3, 1, 2
**When** train processes
**Then** entries processed in order: 1, 2, 3
**And** processing order matches position order

```rust
#[tokio::test]
async fn cv_005_verify_sequential_processing_invariant() {
    // Arrange: Create entries out of order
    // Act: Process train
    // Assert: Processed in position order
}
```

---

### 5.6 CV-006: Verify INV-TRAIN-002 (Only One Entry at a Time)

**Given** train is processing entry at position 2
**When** entry 2 is in 'testing' state
**Then** entry 3 is not started
**And** entry 1 is not still active

```rust
#[tokio::test]
async fn cv_006_verify_single_entry_processing_invariant() {
    // Arrange: Process train
    // Act: Check mid-processing
    // Assert: Only one entry active
}
```

---

### 5.7 CV-007: Verify INV-STATE-001 (Valid State Transitions)

**Given** entry is in 'pending' state
**When** entry transitions
**Then** new state is 'claimed'
**And** transition is valid

**Given** entry is in 'testing' state
**When** entry transitions
**Then** new state is one of: ready_to_merge, failed_retryable, failed_terminal
**And** transition is valid

```rust
#[tokio::test]
async fn cv_007_verify_valid_state_transitions_invariant() {
    // Arrange: Entry in various states
    // Act: Transition states
    // Assert: All transitions valid
}
```

---

### 5.8 CV-008: Verify INV-POS-001 (Unique Positions)

**Given** 10 pending entries exist
**When** positions are queried
**Then** each position 1-10 appears exactly once
**And** no duplicates exist

```rust
#[tokio::test]
async fn cv_008_verify_unique_positions_invariant() {
    // Arrange: Create 10 entries
    // Act: Query positions
    // Assert: All unique
}
```

---

### 5.9 CV-009: Verify INV-POS-002 (Contiguous Positions)

**Given** pending entries exist
**When** positions are listed
**Then** positions form sequence 1, 2, 3, ..., N
**And** no gaps exist

```rust
#[tokio::test]
async fn cv_009_verify_contiguous_positions_invariant() {
    // Arrange: Create entries
    // Act: Query positions
    // Assert: No gaps
}
```

---

### 5.10 CV-010: Verify INV-MERGE-001 (Only Passing Tests Merged)

**Given** entry failed tests
**When** merge is attempted
**Then** merge is blocked
**And** entry is not merged

```rust
#[tokio::test]
async fn cv_010_verify_only_passing_tests_merged_invariant() {
    // Arrange: Create entry with failing test
    // Act: Attempt merge
    // Assert: Merge blocked
}
```

---

### 5.11 CV-011: Verify INV-MERGE-002 (Only Conflict-Free Merged)

**Given** entry has merge conflicts
**When** merge is attempted
**Then** merge is blocked
**And** entry is kicked

```rust
#[tokio::test]
async fn cv_011_verify_only_conflict_free_merged_invariant() {
    // Arrange: Create entry with conflicts
    // Act: Attempt merge
    // Assert: Merge blocked
}
```

---

### 5.12 CV-012: Verify INV-RECOVERY-001 (Failed Entry Always Kicked)

**Given** entry fails with merge conflict
**When** failure handler runs
**Then** entry is kicked (status: cancelled)
**And** entry is removed from queue

```rust
#[tokio::test]
async fn cv_012_verify_failed_entry_kicked_invariant() {
    // Arrange: Create failing entry
    // Act: Handle failure
    // Assert: Entry kicked
}
```

---

### 5.13 CV-013: Verify PRE-ENTRY-001 (Entry Exists)

**Given** entry_id = 999
**And** entry does not exist
**When** train attempts to process entry
**Then** EntryNotFound error is returned
**And** train continues with next entry

```rust
#[tokio::test]
async fn cv_013_verify_entry_exists_precondition() {
    // Arrange: Non-existent entry
    // Act: Process entry
    // Assert: Entry not found error
}
```

---

### 5.14 CV-014: Verify POST-ENTRY-001 (State Machine Progress)

**Given** entry starts in 'pending'
**When** entry is processed
**Then** entry transitions through states
**And** each transition is logged
**And** audit trail is complete

```rust
#[tokio::test]
async fn cv_014_verify_state_machine_progress_postcondition() {
    // Arrange: Create entry
    // Act: Process entry
    // Assert: All transitions logged
}
```

---

### 5.15 CV-015: Verify PRE-TEST-001 (Tests Configured)

**Given** workspace has no test configuration
**When** test execution attempts
**Then** TestFrameworkNotConfigured error is returned
**And** entry is marked terminal

```rust
#[tokio::test]
async fn cv_015_verify_tests_configured_precondition() {
    // Arrange: Workspace without tests
    // Act: Run tests
    // Assert: Framework error
}
```

---

### 5.16 CV-016: Verify POST-TEST-001 (Tests Executed)

**Given** entry is in 'testing' state
**When** test execution completes
**Then** test results are recorded
**And** exit code is captured
**And** output is available

```rust
#[tokio::test]
async fn cv_016_verify_tests_executed_postcondition() {
    // Arrange: Entry in testing
    // Act: Complete tests
    // Assert: Results recorded
}
```

---

### 5.17 CV-017: Verify PRE-MERGE-001 (Tests Passed)

**Given** entry failed tests
**When** merge is attempted
**Then** merge is blocked
**And** entry is not in 'ready_to_merge' state

```rust
#[tokio::test]
async fn cv_017_verify_tests_passed_precondition() {
    // Arrange: Entry with failing tests
    // Act: Attempt merge
    // Assert: Merge blocked
}
```

---

### 5.18 CV-018: Verify POST-MERGE-001 (Merge Committed)

**Given** entry is in 'merging' state
**When** merge completes
**Then** changes are in target branch
**And** merge commit exists
**And** commit has correct parents

```rust
#[tokio::test]
async fn cv_018_verify_merge_committed_postcondition() {
    // Arrange: Entry merging
    // Act: Complete merge
    // Assert: Committed correctly
}
```

---

### 5.19 CV-019: Verify PRE-RECOVERY-001 (Failed Entry Exists)

**Given** failure handler is called
**And** failed entry does not exist
**When** recovery attempts
**Then** EntryNotFound error is returned
**And** recovery aborts

```rust
#[tokio::test]
async fn cv_019_verify_failed_entry_exists_precondition() {
    // Arrange: Call handler with non-existent entry
    // Act: Attempt recovery
    // Assert: Entry not found
}
```

---

### 5.20 CV-020: Verify POST-RECOVERY-001 (Entry Kicked)

**Given** entry failed
**When** recovery completes
**Then** entry status is 'cancelled'
**And** entry has no position
**And** kick event was emitted

```rust
#[tokio::test]
async fn cv_020_verify_entry_kicked_postcondition() {
    // Arrange: Failed entry
    // Act: Recover
    // Assert: Entry kicked
}
```

---

### 5.21 CV-021: Verify PRE-REBASE-001 (New Base Exists)

**Given** new_base commit does not exist
**When** rebase attempts
**Then** RevisionQueryFailed error is returned
**And** rebase does not proceed

```rust
#[tokio::test]
async fn cv_021_verify_new_base_exists_precondition() {
    // Arrange: Non-existent base
    // Act: Attempt rebase
    // Assert: Query failed
}
```

---

### 5.22 CV-022: Verify POST-REBASE-001 (Entries Rebased)

**Given** 5 subsequent entries exist
**When** recovery rebase completes
**Then** all 5 entries have new head_sha
**And** all 5 were rebased onto new_base
**And** all rebase events were emitted

```rust
#[tokio::test]
async fn cv_022_verify_entries_rebased_postcondition() {
    // Arrange: 5 subsequent entries
    // Act: Rebase
    // Assert: All rebased
}
```

---

### 5.23 CV-023: Verify INV-OUTPUT-001 (Event for Each State Change)

**Given** entry transitions through N states
**When** processing completes
**Then** exactly N TrainStep events exist
**And** each event matches a state

```rust
#[tokio::test]
async fn cv_023_verify_event_for_each_state_change_invariant() {
    // Arrange: Process entry
    // Act: Count events
    // Assert: Event for each state
}
```

---

### 5.24 CV-024: Verify INV-CONC-001 (Only One Train at a Time)

**Given** train is active
**When** second train attempts to start
**Then** second train fails with TrainAlreadyActive
**And** first train continues

```rust
#[tokio::test]
async fn cv_024_verify_only_one_train_invariant() {
    // Arrange: Start train
    // Act: Start second train
    // Assert: Second fails
}
```

---

### 5.25 CV-025: Verify PRE-ENTRY-006 (HeadSha Assigned)

**Given** entry has head_sha = NULL
**When** train processes entry
**Then** entry is marked as failed_terminal
**And** error indicates missing head_sha

```rust
#[tokio::test]
async fn cv_025_verify_head_sha_assigned_precondition() {
    // Arrange: Entry without head_sha
    // Act: Process entry
    // Assert: Fatal error
}
```

---

### 5.26 CV-026: Verify POST-MERGE-004 (CompletedAt Set)

**Given** entry is merged
**When** merge completes
**Then** completed_at timestamp is set
**And** timestamp is recent
**And** timestamp >= started_at

```rust
#[tokio::test]
async fn cv_026_verify_completed_at_set_postcondition() {
    // Arrange: Merge entry
    // Act: Check timestamp
    // Assert: completed_at set
}
```

---

### 5.27 CV-027: Verify INV-DEDUPE-001 (Unique DedupeKeys)

**Given** 10 active entries exist
**When** dedupe_keys are queried
**Then** all dedupe_keys are unique
**And** no duplicates exist

```rust
#[tokio::test]
async fn cv_027_verify_unique_dedupe_keys_invariant() {
    // Arrange: Create 10 entries
    // Act: Query dedupe_keys
    // Assert: All unique
}
```

---

### 5.28 CV-028: Verify INV-DEDUPE-003 (HeadSha Changes After Rebase)

**Given** entry has head_sha = "abc123"
**When** entry is rebased
**Then** head_sha is different from "abc123"
**And** new head_sha points to rebased commit

```rust
#[tokio::test]
async fn cv_028_verify_head_sha_changes_after_rebase_invariant() {
    // Arrange: Create entry
    // Act: Rebase entry
    // Assert: head_sha changed
}
```

---

### 5.29 CV-029: Verify POST-REBASE-003 (Positions Recalculated)

**Given** positions were 1, 2, 3, 4, 5
**And** entry at position 2 was kicked
**When** positions are recalculated
**Then** new positions are 1, 2, 3, 4
**And** no gaps exist

```rust
#[tokio::test]
async fn cv_029_verify_positions_recalculated_postcondition() {
    // Arrange: Kick entry
    // Act: Recalculate positions
    // Assert: Contiguous
}
```

---

### 5.30 CV-030: Verify INV-STATE-004 (Terminal States Absorbing)

**Given** entry is in 'merged' state
**When** state transition is attempted
**Then** transition fails
**And** InvalidStateTransition error is returned

```rust
#[tokio::test]
async fn cv_030_verify_terminal_states_absorbing_invariant() {
    // Arrange: Terminal entry
    // Act: Attempt transition
    // Assert: Transition fails
}
```

---

### 5.31 CV-031: Verify PRE-TEST-005 (Timeout Configured)

**Given** entry has no timeout configured
**When** test execution runs
**Then** default timeout is used
**And** tests do not run forever

```rust
#[tokio::test]
async fn cv_031_verify_timeout_configured_precondition() {
    // Arrange: Entry without timeout
    // Act: Run tests
    // Assert: Default timeout used
}
```

---

### 5.32 CV-032: Verify POST-RECOVERY-004 (Rebase Initiated)

**Given** entry was kicked
**And** subsequent entries exist
**When** recovery completes
**Then** rebase was attempted for all subsequent entries
**And** rebase results are recorded

```rust
#[tokio::test]
async fn cv_032_verify_rebase_initiated_postcondition() {
    // Arrange: Kick entry
    // Act: Recover
    // Assert: Rebase attempted
}
```

---

### 5.33 CV-033: Verify INV-TRAIN-003 (Stop at Unrecoverable Failure)

**Given** train processes 10 entries
**And** entry 5 fails with failed_terminal
**When** failure is encountered
**Then** train stops after entry 5
**And** entries 6-10 are not processed

```rust
#[tokio::test]
async fn cv_033_verify_stop_at_unrecoverable_failure_invariant() {
    // Arrange: Queue with terminal failure
    // Act: Process train
    // Assert: Train stops
}
```

---

### 5.34 CV-034: Verify PRE-RECOVERY-003 (Failure Reason Recorded)

**Given** entry failed
**And** error_message is NULL
**When** recovery attempts
**Then** recovery uses generic failure reason
**And** kick proceeds

```rust
#[tokio::test]
async fn cv_034_verify_failure_reason_recorded_precondition() {
    // Arrange: Entry without error_message
    // Act: Recover
    // Assert: Generic reason used
}
```

---

### 5.35 CV-035: Verify INV-OUTPUT-003 (TrainResult Complete)

**Given** train processed 10 entries (7 merged, 2 failed, 1 kicked)
**When** TrainResult is emitted
**Then** merged list has 7 entries
**And** failed list has 2 entries
**And** kicked list has 1 entry
**And** total_processed = 10

```rust
#[tokio::test]
async fn cv_035_verify_train_result_complete_invariant() {
    // Arrange: Process train
    // Act: Check TrainResult
    // Assert: All outcomes listed
}
```

---

## 6. End-to-End Scenarios

### 6.1 E2E-001: Full Train Processing Workflow

**Given** 5 entries submitted to queue
**And** all entries have passing tests
**And** all entries merge cleanly
**When** train processing runs
**Then** lock is acquired
**And** all 5 entries are processed in order
**And** all 5 entries transition to 'merged'
**And** 5 TrainStep events per entry (25 total)
**And** TrainResult shows 5 merged, 0 failed, 0 kicked
**And** lock is released
**And** queue is empty

```rust
#[tokio::test]
async fn e2e_001_full_train_processing_workflow() {
    // Arrange: Submit 5 entries
    // Act: Process train
    // Assert: All merged, correct events
}
```

---

### 6.2 E2E-002: Train with Conflict Recovery

**Given** 4 entries in queue
**And** entry 2 has merge conflicts
**When** train processes
**Then** entry 1 merges successfully
**And** entry 2 conflict is detected
**And** entry 2 is kicked
**And** entries 3-4 are rebased onto entry 1
**And** train restarts
**And** entries 3-4 are processed
**And** TrainResult shows 3 merged, 0 failed, 1 kicked

```rust
#[tokio::test]
async fn e2e_002_train_with_conflict_recovery() {
    // Arrange: 4 entries, entry 2 has conflict
    // Act: Process train
    // Assert: Kick, rebase, restart, 3 merged
}
```

---

### 6.3 E2E-003: Train with Multiple Failures

**Given** 6 entries in queue
**And** entry 2 has failing tests
**And** entry 4 has merge conflicts
**When** train processes
**Then** entry 1 merges
**And** entry 2 fails_retryable (test failure)
**And** entry 3 merges
**And** entry 4 is kicked (conflict)
**And** entry 5-6 are rebased
**And** train restarts
**And** entries 5-6 are processed
**And** TrainResult shows 3 merged, 1 failed, 1 kicked

```rust
#[tokio::test]
async fn e2e_003_train_with_multiple_failures() {
    // Arrange: 6 entries, various failures
    // Act: Process train
    // Assert: Mixed outcomes, correct handling
}
```

---

### 6.4 E2E-004: Queue Submission to Merge Complete Flow

**Given** workspace "feature-auth" is ready
**When** session is submitted to queue
**And** train processing runs immediately
**Then** entry is created with status 'pending'
**And** train claims entry
**And** tests run and pass
**And** merge completes
**And** entry status is 'merged'
**And** bookmark is updated

```rust
#[tokio::test]
async fn e2e_004_queue_submission_to_merge_complete_flow() {
    // Arrange: Ready workspace
    // Act: Submit and process
    // Assert: Full flow completes
}
```

---

### 6.5 E2E-005: Resubmit Failed Entry After Fix

**Given** entry failed_retryable with test failure
**And** issue is fixed
**When** entry is resubmitted
**And** train processes
**Then** entry resets to 'pending'
**And** attempt_count increments
**And** tests pass on retry
**And** entry merges successfully

```rust
#[tokio::test]
async fn e2e_005_resubmit_failed_entry_after_fix() {
    // Arrange: Fail, fix
    // Act: Resubmit and process
    // Assert: Success on retry
}
```

---

### 6.6 E2E-006: Priority-Based Processing

**Given** 5 entries with priorities: 5, 1, 3, 1, 5
**When** train processes
**Then** entries processed in priority order: 1, 1, 3, 5, 5
**And** FIFO within same priority

```rust
#[tokio::test]
async fn e2e_006_priority_based_processing() {
    // Arrange: Mixed priorities
    // Act: Process train
    // Assert: Priority order respected
}
```

---

### 6.7 E2E-007: Train Timeout and Recovery

**Given** train is processing
**And** lock timeout is 60 seconds
**And** processing takes 90 seconds
**When** 60 seconds elapse
**Then** lock expires
**And** another train can acquire lock
**And** original train continues or fails gracefully

```rust
#[tokio::test]
async fn e2e_007_train_timeout_and_recovery() {
    // Arrange: Start long train
    // Act: Wait for timeout
    // Assert: Lock expires, recovery possible
}
```

---

### 6.8 E2E-008: Concurrent Queue Operations

**Given** train is processing
**When** new entries are submitted
**And** entries are cancelled
**And** entries are queried
**Then** all operations succeed
**And** train processing is not disrupted
**And** no inconsistencies occur

```rust
#[tokio::test]
async fn e2e_008_concurrent_queue_operations() {
    // Arrange: Start train
    // Act: Concurrent operations
    // Assert: No disruption, consistent state
}
```

---

### 6.9 E2E-009: Full Recovery After Conflict

**Given** 10 entries in queue
**And** entry 5 has conflicts
**When** train processes
**Then** entries 1-4 merge
**And** entry 5 is kicked
**And** entries 6-10 are rebased
**And** train restarts
**And** entries 6-10 are processed
**And** all remaining entries merge successfully

```rust
#[tokio::test]
async fn e2e_009_full_recovery_after_conflict() {
    // Arrange: 10 entries, entry 5 conflict
    // Act: Process train
    // Assert: Kick, rebase all, restart, all merge
}
```

---

### 6.10 E2E-010: Train with Rebase Failures

**Given** 5 entries in queue
**And** entry 1 is kicked
**And** entries 3-4 have rebase conflicts
**When** train recovers
**Then** entry 1 is kicked
**And** entries 2-5 are rebased
**And** entry 3 fails rebase (terminal)
**And** entry 4 fails rebase (terminal)
**And** entry 2 and 5 merge successfully
**And** TrainResult shows 2 merged, 2 failed, 1 kicked

```rust
#[tokio::test]
async fn e2e_010_train_with_rebase_failures() {
    // Arrange: Set up rebase failures
    // Act: Process and recover
    // Assert: Correct outcomes
}
```

---

### 6.11 E2E-011: Empty Queue to Full to Empty

**Given** queue is empty
**When** 3 entries are submitted
**And** train processes
**And** all 3 merge
**Then** queue is empty again
**And** all audit events are present
**And** workspace state is clean

```rust
#[tokio::test]
async fn e2e_011_empty_queue_to_full_to_empty() {
    // Arrange: Empty queue
    // Act: Submit, process, merge
    // Assert: Empty again, clean state
}
```

---

### 6.12 E2E-012: Maximum Retry Flow

**Given** entry has max_attempts = 3
**And** entry fails 3 times
**When** 4th submission is attempted
**Then** entry is marked as failed_terminal
**And** no further retries are allowed
**And** error indicates max attempts exceeded

```rust
#[tokio::test]
async fn e2e_012_maximum_retry_flow() {
    // Arrange: Entry with max 3
    // Act: Fail 3 times, attempt 4th
    // Assert: Terminal, max exceeded
}
```

---

### 6.13 E2E-013: Cross-Workspace Dependencies

**Given** entry A (workspace "auth") depends on entry B (workspace "user")
**And** entry B is submitted first
**When** train processes
**Then** entry B merges first
**And** entry A merges after B
**And** entry A is rebased onto B if needed

```rust
#[tokio::test]
async fn e2e_013_cross_workspace_dependencies() {
    // Arrange: Dependent entries
    // Act: Process train
    // Assert: Correct order, rebase if needed
}
```

---

### 6.14 E2E-014: Train Processing with Agent Identity

**Given** agent "agent-prod-001" submits entries
**When** train processes
**Then** agent_id is recorded in all events
**And** TrainResult includes agent_id
**And** audit trail shows agent actions

```rust
#[tokio::test]
async fn e2e_014_train_processing_with_agent_identity() {
    // Arrange: Agent submits entries
    // Act: Process train
    // Assert: Agent identity in all events
}
```

---

### 6.15 E2E-015: Complete Lifecycle from Draft to Merged

**Given** workspace is created
**When** changes are committed
**And** workspace is submitted to queue (status: pending)
**And** train processes (status: claimed -> testing -> ready_to_merge -> merging)
**And** merge completes (status: merged)
**Then** full lifecycle is audited
**And** all events are present
**And** workspace state is 'merged'
**And** bookmark points to merge commit

```rust
#[tokio::test]
async fn e2e_015_complete_lifecycle_from_draft_to_merged() {
    // Arrange: Create workspace
    // Act: Commit, submit, process, merge
    // Assert: Full lifecycle audited
}
```

---

## 7. Test Implementation Guidelines

### 7.1 Test Structure

Each test should follow this structure:
1. **Arrange**: Set up test data, mock external dependencies
2. **Act**: Execute the operation being tested
3. **Assert**: Verify expected outcomes

### 7.2 Test Helpers

Create reusable helpers for:
- Creating test entries in various states
- Setting up test workspaces
- Mocking JJ commands
- Capturing JSONL output
- Verifying database state

### 7.3 Test Isolation

- Use in-memory database for each test
- Clean up file system artifacts after tests
- Reset train state between tests
- Ensure tests can run in parallel

### 7.4 Test Data Management

- Use deterministic test data (fixed SHAs, names)
- Create test fixtures for common scenarios
- Use factories for complex object creation
- Avoid random data in tests

### 7.5 Assertion Guidelines

- Use specific assertions (assert_eq, not just assert)
- Include failure messages in assertions
- Check all postconditions, not just primary outcome
- Verify audit trail for state-changing operations

---

## 8. Coverage Metrics

### 8.1 Code Coverage Targets

| Component | Target Coverage |
|-----------|----------------|
| Train processing logic | 95% |
| State machine transitions | 100% |
| Error handling paths | 90% |
| JSONL output emission | 100% |
| Database operations | 95% |

### 8.2 Scenario Coverage

- All error variants: 100%
- All state transitions: 100%
- All preconditions: 100%
- All postconditions: 100%
- All invariants: 100%

---

## 9. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-18 | Initial test plan specification |

---

## 10. References

- `/home/lewis/src/zjj/contracts/bd-1ye-contract-spec.md` - Contract specification
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs` - Queue implementation
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_status.rs` - State machine
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_submission.rs` - Submission API
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-024-merge-train-logic.cue` - Train logic bead
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-025-train-failure-auto-rebase.cue` - Failure recovery bead
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-033-test-merge-train.cue` - Test bead
- Martin Fowler's BDD articles: https://martinfowler.com/tags/bdd.html
