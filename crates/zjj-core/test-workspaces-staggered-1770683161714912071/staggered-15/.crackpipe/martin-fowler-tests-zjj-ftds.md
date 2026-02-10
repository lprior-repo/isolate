# Martin Fowler Test Plan: `--idempotent` Flag Verification

## Overview

This test plan verifies that the `--idempotent` flag works correctly for the `add`, `work`, and `remove` commands. Tests are organized by **happy path**, **error path**, and **edge cases** with expressive names following Given-When-Then format.

**Key Principle**: Idempotent operations are safe to retry - they succeed whether or not the resource already exists.

## Happy Path Tests

### `add` Command Tests

#### test_add_idempotent_succeeds_when_session_already_exists
**Given**: An initialized ZJJ repository with an existing session "test-session"
**When**: User runs `zjj add test-session --idempotent --no-open`
**Then**:
- Command exits with code 0
- No error message is displayed
- Session record is unchanged in database
- Workspace directory is unchanged
- Output indicates session already exists (human-readable) OR shows success without "created" message
- JSON output (if `--json` used) includes `"created": false` or `"status": "already exists"`

#### test_add_idempotent_creates_session_when_not_exists
**Given**: An initialized ZJJ repository with no existing session "new-session"
**When**: User runs `zjj add new-session --idempotent`
**Then**:
- Command exits with code 0
- Session "new-session" is created in database with status "active"
- Workspace directory is created at `.zjj/workspaces/new-session`
- Zellij tab "zjj:new-session" is created
- Output indicates successful creation

#### test_add_idempotent_with_bead_id_succeeds_on_duplicate
**Given**: An initialized ZJJ repository with existing session "bugfix-123" linked to bead "zjj-abc"
**When**: User runs `zjj add bugfix-123 --idempotent --bead zjj-abc --no-open`
**Then**:
- Command exits with code 0
- Session bead association remains unchanged
- No duplicate session created

#### test_add_idempotent_json_output_includes_created_field
**Given**: An initialized ZJJ repository with optional existing session "json-test"
**When**: User runs `zjj add json-test --idempotent --json --no-open`
**Then**:
- Command exits with code 0
- Output is valid JSON matching schema
- JSON includes `"created": true` if session was created
- JSON includes `"created": false` if session already existed
- JSON includes `"idempotent": true` field

### `work` Command Tests

#### test_work_idempotent_succeeds_when_already_in_target_workspace
**Given**: User is already in workspace "feature-auth" in a Zellij session
**When**: User runs `zjj work feature-auth --idempotent`
**Then**:
- Command exits with code 0
- No new workspace created
- No error raised about already being in a workspace
- JSON output includes `"created": false`
- JSON output includes workspace path and environment variables
- Human-readable output shows existing workspace info

#### test_work_idempotent_creates_workspace_when_not_exists
**Given**: User is on main branch (not in a workspace)
**When**: User runs `zjj work new-feature --idempotent`
**Then**:
- Command exits with code 0
- Workspace "new-feature" is created
- Agent is registered (unless `--no-agent`)
- JSON output includes `"created": true`

#### test_work_idempotent_fails_when_in_different_workspace
**Given**: User is already in workspace "feature-auth"
**When**: User runs `zjj work different-feature --idempotent`
**Then**:
- Command exits with code 1 (error)
- Error message indicates already in workspace "feature-auth"
- Suggests `zjj done` or `zjj abort`
- No new workspace created

#### test_work_idempotent_with_agent_id_reregisters_successfully
**Given**: An existing workspace "agent-task" with agent "agent-1" already registered
**When**: User runs `zjj work agent-task --idempotent --agent-id agent-1`
**Then**:
- Command exits with code 0
- Agent "agent-1" is re-registered (heartbeat updated)
- No duplicate agent registration error

### `remove` Command Tests

#### test_remove_idempotent_succeeds_when_session_doesnt_exist
**Given**: An initialized ZJJ repository with no session "nonexistent"
**When**: User runs `zjj remove nonexistent --idempotent`
**Then**:
- Command exits with code 0
- No error message about session not found
- No changes to database (no record to remove)
- Output indicates "already removed" or similar

#### test_remove_idempotent_removes_session_when_exists
**Given**: An initialized ZJJ repository with existing session "old-session"
**When**: User runs `zjj remove old-session --idempotent`
**Then**:
- Command exits with code 0
- Session "old-session" is removed from database
- Workspace directory is deleted
- Zellij tab is closed (if open)

#### test_remove_idempotent_with_force_flag_is_redundant
**Given**: An initialized ZJJ repository with optional existing session "test"
**When**: User runs `zjj remove test --idempotent -f` (both flags)
**Then**:
- Command exits with code 0
- Behavior is identical to `--force` alone
- No conflict between flags

## Error Path Tests

### `add` Command Error Tests

#### test_add_idempotent_fails_on_invalid_session_name
**Given**: An initialized ZJJ repository
**When**: User runs `zjj add 123-invalid --idempotent` (starts with number)
**Then**:
- Command exits with code 1
- Error message indicates invalid session name
- No session created

#### test_add_idempotent_fails_when_not_initialized
**Given**: A JJ repository without ZJJ initialized (no `.zjj`)
**When**: User runs `zjj add test --idempotent`
**Then**:
- Command exits with code 1
- Error message indicates ZJJ not initialized
- Suggests running `zjj init`

#### test_add_idempotent_fails_on_hook_execution_with_existing_session
**Given**: An initialized ZJJ repository
**And**: A post_create hook that fails
**And**: An existing session "hook-test" that was created successfully previously
**When**: User runs `zjj add hook-test --idempotent`
**Then**:
- Command exits with code 0 (success)
- Hooks are NOT executed (session already exists)
- Session remains unchanged

### `work` Command Error Tests

#### test_work_idempotent_fails_when_not_in_jj_repo
**Given**: A directory that is not a JJ repository
**When**: User runs `zjj work test --idempotent`
**Then**:
- Command exits with code 1
- Error message indicates not in JJ repository

#### test_work_idempotent_fails_on_session_creation_failure
**Given**: User is on main branch
**And**: Database is locked or corrupted
**When**: User runs `zjj work new-task --idempotent`
**Then**:
- Command exits with code 1
- Error message indicates session creation failed
- No partial state left behind

### `remove` Command Error Tests

#### test_remove_idempotent_fails_on_workspace_deletion_error
**Given**: An initialized ZJJ repository with existing session "locked-session"
**And**: Workspace directory has permissions that prevent deletion
**When**: User runs `zjj remove locked-session --idempotent`
**Then**:
- Command exits with code 1
- Session is removed from database (best effort)
- Error message indicates workspace cleanup failed

#### test_remove_idempotent_fails_when_not_initialized
**Given**: A JJ repository without ZJJ initialized
**When**: User runs `zjj remove test --idempotent`
**Then**:
- Command exits with code 1
- Error message indicates ZJJ not initialized

## Edge Case Tests

### Session State Edge Cases

#### test_add_idempotent_handles_failed_session_state
**Given**: An initialized ZJJ repository
**And**: A session "failed-session" with status "failed" (from previous failed creation)
**When**: User runs `zjj add failed-session --idempotent`
**Then**:
- Command exits with code 0 OR 1 (to be decided)
- [Option A]: Treats as existing session, returns success (idempotent)
- [Option B]: Treats as invalid state, returns error (safe default)
- No new session created

#### test_add_idempotent_handles_missing_workspace_directory
**Given**: An initialized ZJJ repository
**And**: A session "orphaned-session" in database but workspace directory missing
**When**: User runs `zjj add orphaned-session --idempotent`
**Then**:
- Command exits with code 0 (success)
- Returns existing session info from database
- Does not attempt to recreate workspace

#### test_add_idempotent_with_dry_run_shows_existing_session
**Given**: An initialized ZJJ repository with existing session "dry-test"
**When**: User runs `zjj add dry-test --idempotent --dry-run`
**Then**:
- Command exits with code 0
- Output indicates session already exists
- No changes made (dry run)
- Shows what would happen (nothing)

### Concurrency Edge Cases

#### test_add_idempotent_concurrent_calls_handle_race_condition
**Given**: An initialized ZJJ repository with no session "race-test"
**When**: Two concurrent processes run `zjj add race-test --idempotent` simultaneously
**Then**:
- Both processes exit with code 0 (success)
- Exactly one session is created
- Other process sees existing session and returns idempotent success
- No duplicate session records
- Database maintains consistency

#### test_add_idempotent_during_concurrent_normal_add
**Given**: An initialized ZJJ repository with no session "concurrent-test"
**When**: Process A runs `zjj add concurrent-test` (normal)
**And**: Process B runs `zjj add concurrent-test --idempotent` simultaneously
**Then**:
- Process A exits with code 0 (created session)
- Process B exits with code 0 (idempotent success)
- No errors from either process

### Output Format Edge Cases

#### test_add_idempotent_json_output_schema_validation
**Given**: An initialized ZJJ repository with optional existing session
**When**: User runs `zjj add schema-test --idempotent --json`
**Then**:
- Output is valid JSON
- JSON matches SchemaEnvelope structure
- `schema` field is "add-response"
- `type` field is "single"
- `data` field includes all required fields
- No extra fields without documentation

#### test_work_idempotent_human_readable_output_format
**Given**: User is in workspace "output-test" (or not, depending on scenario)
**When**: User runs `zjj work output-test --idempotent` (without `--json`)
**Then**:
- Output is human-readable text (not JSON)
- Includes session name
- Includes workspace path
- Includes status information
- No stack traces or debug info

## Contract Verification Tests

### Precondition Verification

#### test_add_idempotent_validates_session_name_before_checking_existence
**Given**: An initialized ZJJ repository
**When**: User runs `zjj add "invalid name" --idempotent` (space in name)
**Then**:
- Validation happens BEFORE checking if session exists
- Command exits with code 1
- Error message is about invalid name, not about existence

#### test_work_idempotent_checks_current_workspace_before_creating
**Given**: User is in workspace "current-workspace"
**When**: User runs `zjj work different-workspace --idempotent`
**Then**:
- Check for existing workspace happens BEFORE creation attempt
- Command exits with code 1
- Error message about already being in a workspace

### Postcondition Verification

#### test_add_idempotent_preserves_existing_session_metadata
**Given**: An initialized ZJJ repository with session "metadata-test" with bead_id "zjj-123"
**When**: User runs `zjj add metadata-test --idempotent --bead zjj-456 --no-open`
**Then**:
- Session bead_id remains "zjj-123" (unchanged)
- Command exits with code 0
- No metadata updates occur

#### test_work_idempotent_does_not_reregister_agent_with_different_id
**Given**: Existing workspace "agent-test" with registered agent "agent-1"
**When**: User runs `zjj work agent-test --idempotent --agent-id agent-2`
**Then**:
- Command exits with code 0 OR 1 (to be decided)
- [Option A]: Ignores new agent-id, keeps "agent-1" (idempotent)
- [Option B]: Updates to "agent-2" (allows re-registration)

### Invariant Verification

#### test_add_idempotent_never_modifies_existing_state
**Given**: An initialized ZJJ repository with session "invariant-test"
**And**: Session has specific status, workspace_path, and zellij_tab
**When**: User runs `zjj add invariant-test --idempotent --no-open`
**Then**:
- Session status in database is unchanged
- Workspace directory contents are unchanged
- Zellij tab is unchanged
- No files are created or modified

#### test_remove_idempotent_never_fails_on_nonexistent_session
**Given**: An initialized ZJJ repository with no session "safe-remove"
**When**: User runs `zjj remove safe-remove --idempotent`
**Then**:
- Always succeeds (exit code 0)
- No error raised
- Safe to retry indefinitely

## Integration Tests (End-to-End Scenarios)

### Scenario 1: Agent Workflow with Retry
**Given**: An AI agent starting work on bead "zjj-ftds"
**When**: Agent runs `zjj work zjj-ftds --bead zjj-ftds --idempotent`
**And**: Command fails due to network issue (first attempt)
**And**: Agent retries same command (second attempt)
**Then**:
- First attempt: May fail or succeed depending on timing
- Second attempt: Always succeeds (idempotent)
- Agent can proceed to workspace without checking if it was created
- No duplicate sessions created

### Scenario 2: CI/CD Pipeline with Idempotent Setup
**Given**: A CI/CD pipeline that runs on every commit
**When**: Pipeline runs `zjj add ci-build --idempotent --no-open --no-hooks`
**Then**:
- First run: Session "ci-build" is created
- Subsequent runs: Command succeeds immediately (session exists)
- Pipeline doesn't need to check if session exists first
- No errors on duplicate runs

### Scenario 3: Manual Developer Workflow
**Given**: Developer wants to ensure workspace exists before starting work
**When**: Developer runs `zjj add feature-auth --idempotent`
**Then**:
- If workspace doesn't exist: It's created
- If workspace already exists: Nothing happens (success)
- Developer can proceed to work regardless of prior state
- No need to check `zjj list` first

## Performance Tests

#### test_add_idempotent_performance_with_existing_session
**Given**: An initialized ZJJ repository with 100 existing sessions
**When**: User runs `zjj add existing-session --idempotent`
**Then**:
- Command completes in < 100ms (database lookup only)
- No filesystem operations for workspace creation
- No Zellij operations

#### test_work_idempotent_performance_when_already_in_workspace
**Given**: User is in workspace "perf-test"
**When**: User runs `zjj work perf-test --idempotent`
**Then**:
- Command completes in < 50ms (location check + session lookup)
- No workspace creation operations

## Test Organization

### File Structure
```
crates/zjj/tests/
├── test_add_idempotent.rs       # add command tests
├── test_work_idempotent.rs      # work command tests
├── test_remove_idempotent.rs    # remove command tests
└── drq_adversarial.rs           # Update existing tests

crates/zjj/src/commands/
├── add.rs                       # Add module tests (add to existing)
├── work.rs                      # Work module tests (add to existing)
└── remove.rs                    # Remove module tests (add if exists)
```

### Test Helpers
```rust
// Test harness helpers for idempotent testing
impl TestHarness {
    /// Assert add command with idempotent succeeds
    fn assert_add_idempotent(&self, name: &str, should_exist: bool);

    /// Create session, then assert idempotent add succeeds
    fn assert_add_idempotent_on_existing(&self, name: &str);

    /// Assert work command with idempotent succeeds
    fn assert_work_idempotent(&self, name: &str, already_in_workspace: bool);

    /// Assert remove command with idempotent succeeds
    fn assert_remove_idempotent(&self, name: &str, should_exist: bool);
}
```

## Test Priority

### P0 (Must Pass - Block Release)
- test_add_idempotent_succeeds_when_session_already_exists
- test_add_idempotent_creates_session_when_not_exists
- test_work_idempotent_succeeds_when_already_in_target_workspace
- test_remove_idempotent_succeeds_when_session_doesnt_exist

### P1 (Should Pass - Block Merge)
- test_add_idempotent_json_output_includes_created_field
- test_work_idempotent_creates_workspace_when_not_exists
- test_add_idempotent_handles_failed_session_state
- test_add_idempotent_concurrent_calls_handle_race_condition

### P2 (Nice to Have - Don't Block)
- Performance tests
- Edge case tests for unusual session states
- Integration tests for CI/CD scenarios

## Summary

**Total Test Count**: 37 tests
- Happy Path: 13 tests
- Error Path: 9 tests
- Edge Cases: 11 tests
- Contract Verification: 4 tests
- Integration: 3 tests
- Performance: 2 tests

**Coverage Goal**: 100% of idempotent code paths across all three commands

**Estimated Implementation Time**: 2-3 hours (including test infrastructure)
