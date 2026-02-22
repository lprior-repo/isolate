# Martin Fowler Test Plan: bd-foy

## Replace add, remove, sync, focus Commands with JSONL Output

**Bead ID**: bd-foy
**Created**: 2026-02-21
**Status**: Test Plan Phase

---

## Test Strategy

This test plan follows Martin Fowler's testing patterns:
- **Expressive test names** that describe behavior, not implementation
- **Given-When-Then** format for clarity
- **Happy path, error path, and edge case** coverage
- **Contract verification** tests for pre/post/invariants

---

## Happy Path Tests

### add.rs Happy Path

#### `given_valid_name_when_add_then_emits_session_output`
```
Given: zjj initialized
  And: no existing session named "feature-auth"
When: add called with name="feature-auth"
Then:
  - Line 1: Action with verb="create", target="workspace", status="completed"
  - Line 2: Action with verb="create", target="database_record", status="completed"
  - Line 3: Action with verb="create", target="zellij_tab", status="completed"
  - Line 4: SessionOutput with name="feature-auth", status="active"
  - Line 5: ResultOutput with success=true, kind="command"
```

#### `given_valid_name_with_bead_when_add_then_emits_session_with_bead_id`
```
Given: zjj initialized
  And: bead "bd-123" exists
When: add called with name="feature-auth" and bead_id="bd-123"
Then:
  - SessionOutput contains bead_id="bd-123" in data field
  - ResultOutput success=true
```

#### `given_existing_session_when_add_idempotent_then_emits_existing_session`
```
Given: zjj initialized
  And: session "feature-auth" already exists
When: add called with name="feature-auth" and idempotent=true
Then:
  - Line 1: SessionOutput with name="feature-auth"
  - Line 2: ResultOutput with success=true, message contains "already exists"
  - No Issue line emitted
```

#### `given_dry_run_when_add_then_emits_preview`
```
Given: zjj initialized
When: add called with dry_run=true
Then:
  - SessionOutput emitted with workspace path
  - ResultOutput success=true
  - No actual workspace created
```

### remove.rs Happy Path

#### `given_existing_session_when_remove_then_emits_success`
```
Given: zjj initialized
  And: session "old-feature" exists
When: remove called with name="old-feature"
Then:
  - Line 1: Action with verb="remove", target="workspace", status="completed"
  - Line 2: Action with verb="remove", target="database_record", status="completed"
  - Line 3: ResultOutput with success=true, kind="command"
```

#### `given_missing_session_when_remove_idempotent_then_emits_success`
```
Given: zjj initialized
  And: session "old-feature" does NOT exist
When: remove called with name="old-feature" and idempotent=true
Then:
  - Line 1: ResultOutput with success=true, message contains "already removed"
  - No Issue line emitted
```

#### `given_dry_run_when_remove_then_emits_preview`
```
Given: zjj initialized
  And: session "test" exists
When: remove called with dry_run=true
Then:
  - ResultOutput success=true
  - Message contains "DRY-RUN"
  - No actual removal
```

### sync.rs Happy Path

#### `given_single_session_when_sync_then_emits_action_and_result`
```
Given: zjj initialized
  And: session "feature-auth" exists with workspace
  And: main branch exists
When: sync called with name="feature-auth"
Then:
  - Line 1: Action with verb="rebase", target="feature-auth", status="completed"
  - Line 2: ResultOutput with success=true, kind="operation"
```

#### `given_all_sessions_when_sync_then_emits_actions_summary_result`
```
Given: zjj initialized
  And: 3 sessions exist, all active
  And: main branch exists
When: sync called with all=true
Then:
  - Line 1: Action with verb="rebase", target="session1", status="completed"
  - Line 2: Action with verb="rebase", target="session2", status="completed"
  - Line 3: Action with verb="rebase", target="session3", status="completed"
  - Line 4: Summary with message containing "3 session(s)"
  - Line 5: ResultOutput with success=true, kind="operation"
```

#### `given_no_sessions_when_sync_all_then_emits_empty_summary`
```
Given: zjj initialized
  And: no sessions exist
When: sync called with all=true
Then:
  - Line 1: Summary with message containing "0 session(s)"
  - Line 2: ResultOutput with success=true
  - No Action lines
```

### focus.rs Happy Path

#### `given_existing_session_when_focus_then_emits_session_and_result`
```
Given: zjj initialized
  And: session "feature-auth" exists
  And: inside Zellij
When: focus called with name="feature-auth"
Then:
  - Line 1: SessionOutput with name="feature-auth", zellij_tab="zjj:feature-auth"
  - Line 2: ResultOutput with success=true, kind="command"
```

#### `given_no_zellij_when_focus_then_emits_info_only`
```
Given: zjj initialized
  And: session "feature-auth" exists
  And: no_zellij=true
When: focus called with name="feature-auth"
Then:
  - Line 1: SessionOutput
  - Line 2: ResultOutput with success=true, message contains "Zellij disabled"
```

---

## Error Path Tests

### add.rs Error Path

#### `given_empty_name_when_add_then_emits_validation_issue`
```
Given: zjj initialized
When: add called with name=""
Then:
  - Line 1: Issue with kind="validation", severity="error"
  - Line 2: ResultOutput with success=false
  - SessionOutput NOT emitted
```

#### `given_invalid_name_when_add_then_emits_validation_issue`
```
Given: zjj initialized
When: add called with name="123-invalid"
Then:
  - Issue with kind="validation", field="name"
  - ResultOutput success=false
```

#### `given_non_ascii_name_when_add_then_emits_validation_issue`
```
Given: zjj initialized
When: add called with name="feature-🚀"
Then:
  - Issue with kind="validation", field="name", reason contains "ASCII"
  - ResultOutput success=false
```

#### `given_existing_session_when_add_then_emits_conflict_issue`
```
Given: zjj initialized
  And: session "feature-auth" already exists
When: add called with name="feature-auth" (no idempotent)
Then:
  - Issue with kind="state_conflict", severity="error"
  - Issue.session = "feature-auth"
  - ResultOutput success=false
```

#### `given_not_initialized_when_add_then_emits_config_issue`
```
Given: zjj NOT initialized
When: add called with name="test"
Then:
  - Issue with kind="configuration", severity="error"
  - Issue.suggestion contains "zjj init"
  - ResultOutput success=false
```

#### `given_unwritable_directory_when_add_then_emits_permission_issue`
```
Given: zjj initialized
  And: workspace directory is read-only
When: add called with name="test"
Then:
  - Issue with kind="permission_denied", severity="error"
  - ResultOutput success=false
```

### remove.rs Error Path

#### `given_missing_session_when_remove_then_emits_not_found_issue`
```
Given: zjj initialized
  And: session "nonexistent" does NOT exist
When: remove called with name="nonexistent" (no idempotent)
Then:
  - Issue with kind="resource_not_found", severity="error"
  - Issue.session = "nonexistent"
  - ResultOutput success=false
```

#### `given_locked_session_when_remove_then_emits_conflict_issue`
```
Given: zjj initialized
  And: session "locked-session" is locked by another agent
When: remove called with name="locked-session"
Then:
  - Issue with kind="state_conflict", severity="error"
  - Issue.message contains "locked"
  - ResultOutput success=false
```

### sync.rs Error Path

#### `given_missing_session_when_sync_then_emits_not_found_issue`
```
Given: zjj initialized
When: sync called with name="nonexistent"
Then:
  - Issue with kind="resource_not_found", severity="error"
  - ResultOutput success=false
```

#### `given_rebase_conflict_when_sync_then_emits_external_issue`
```
Given: zjj initialized
  And: session "conflict-session" has conflicts with main
When: sync called with name="conflict-session"
Then:
  - Action with verb="rebase", status="failed"
  - Issue with kind="external", severity="error"
  - Issue.suggestion contains "jj resolve"
  - ResultOutput success=false
```

#### `given_partial_failure_when_sync_all_then_emits_mixed_output`
```
Given: zjj initialized
  And: 3 sessions exist
  And: 2 can sync, 1 fails with conflict
When: sync called with all=true
Then:
  - Action with verb="rebase", target="session1", status="completed"
  - Action with verb="rebase", target="session2", status="completed"
  - Action with verb="rebase", target="session3", status="failed"
  - Issue with kind="external", session="session3"
  - Summary with counts showing 2 success, 1 failed
  - ResultOutput with success=false, data.synced_count=2, data.failed_count=1
```

### focus.rs Error Path

#### `given_missing_name_when_focus_then_emits_validation_issue`
```
Given: zjj initialized
When: focus called with name=None
Then:
  - Issue with kind="validation", severity="error"
  - Issue.message contains "required"
  - ResultOutput success=false
```

#### `given_missing_session_when_focus_then_emits_not_found_issue`
```
Given: zjj initialized
  And: session "nonexistent" does NOT exist
When: focus called with name="nonexistent"
Then:
  - Issue with kind="resource_not_found", severity="error"
  - Issue.session = "nonexistent"
  - ResultOutput success=false
```

---

## Edge Case Tests

### Empty/Zero Cases

#### `given_no_sessions_when_list_then_emits_empty_summary`
```
Given: zjj initialized
  And: no sessions exist
When: list called
Then:
  - Summary with message containing "0 session(s)"
  - No SessionOutput lines
```

#### `given_empty_beads_when_list_then_emits_zero_counts`
```
Given: zjj initialized
  And: sessions exist
  And: no beads in repository
When: list called
Then:
  - Summary contains "beads: 0/0/0"
```

### Boundary Cases

#### `given_max_length_name_when_add_then_succeeds`
```
Given: zjj initialized
  And: name is 64 characters (maximum allowed)
When: add called
Then:
  - SessionOutput emitted
  - ResultOutput success=true
```

#### `given_name_exceeds_max_when_add_then_fails`
```
Given: zjj initialized
  And: name is 65 characters
When: add called
Then:
  - Issue with kind="validation", field="name"
  - ResultOutput success=false
```

### Concurrent Access

#### `given_concurrent_add_same_name_then_one_fails`
```
Given: zjj initialized
When: two add calls with same name execute concurrently
Then:
  - Exactly one succeeds with ResultOutput success=true
  - Exactly one fails with Issue kind="state_conflict"
```

### Resource Cleanup

#### `given_hook_fails_when_add_then_emits_rollback_actions`
```
Given: zjj initialized
  And: post_create hook will fail
When: add called
Then:
  - Action with verb="create", status="completed"
  - Action with verb="rollback", status="completed"
  - Issue with kind="external", message contains "hook"
  - ResultOutput success=false
  - No orphaned resources
```

---

## Contract Verification Tests

### Precondition Tests

#### `test_precondition_add_valid_name`
```
Verify: Invalid name causes Issue with Validation kind
Method: Call add with invalid names, check Issue.kind == Validation
```

#### `test_precondition_add_initialized`
```
Verify: Uninitialized zjj causes Issue with Configuration kind
Method: Remove .zjj, call add, check Issue.kind == Configuration
```

#### `test_precondition_remove_session_exists`
```
Verify: Missing session causes Issue with ResourceNotFound kind (without idempotent)
Method: Call remove on nonexistent, check Issue.kind == ResourceNotFound
```

#### `test_precondition_sync_in_jj_repo`
```
Verify: Non-JJ repo causes Issue with Configuration kind
Method: Call sync outside JJ repo, check Issue.kind == Configuration
```

### Postcondition Tests

#### `test_postcondition_add_emits_session_output`
```
Verify: Successful add emits SessionOutput
Method: Call add with valid args, check OutputLine::Session in output
```

#### `test_postcondition_add_emits_result_final`
```
Verify: Final line is always ResultOutput
Method: Call add success/failure, check last line is ResultOutput
```

#### `test_postcondition_remove_idempotent_no_issue`
```
Verify: Idempotent remove of missing session does NOT emit Issue
Method: Call remove --idempotent on missing, check no Issue line
```

#### `test_postcondition_sync_all_counts_match`
```
Verify: synced_count + failed_count == total_sessions
Method: Call sync --all, check counts sum correctly
```

### Invariant Tests

#### `test_invariant_all_lines_valid_jsonl`
```
Verify: Every emitted line is valid JSON
Method: Capture all stdout, parse each line as JSON
```

#### `test_invariant_all_lines_have_type_field`
```
Verify: Every JSONL object has "type" field
Method: For each output line, check JSON contains "type" key
```

#### `test_invariant_session_name_matches_input`
```
Verify: SessionOutput.name equals input name
Method: Call add with specific name, check SessionOutput.name matches
```

#### `test_invariant_no_unwrap_in_output_path`
```
Verify: No panic possible in output generation
Method: Grep for unwrap/expect/panic in output.rs files
```

#### `test_invariant_result_is_final_line`
```
Verify: ResultOutput is always the last line emitted
Method: For each command, verify final output is ResultOutput variant
```

#### `test_invariant_timestamps_present`
```
Verify: All OutputLine variants have valid timestamps
Method: Check each output type has non-zero timestamp
```

---

## JSONL Parseability Tests

### `test_output_parseable_by_jq`
```
Given: Any command execution
When: Output is piped to jq
Then: jq successfully parses each line
Command: zjj <cmd> | jq .
```

### `test_output_has_consistent_schema`
```
Given: Multiple runs of same command
When: Output schemas are compared
Then: All runs produce same JSON structure
```

### `test_session_output_has_required_fields`
```
Verify: SessionOutput contains all required fields
Fields: name, status, state, workspace_path, created_at, updated_at
```

### `test_issue_output_has_required_fields`
```
Verify: Issue contains all required fields
Fields: id, title, kind, severity
Optional: session, suggestion
```

### `test_result_output_has_required_fields`
```
Verify: ResultOutput contains all required fields
Fields: kind, success, message, timestamp
Optional: data
```

---

## Test Count Summary

| Category | Count |
|----------|-------|
| Happy Path Tests | 15 |
| Error Path Tests | 16 |
| Edge Case Tests | 6 |
| Contract Verification Tests | 13 |
| JSONL Parseability Tests | 5 |
| **Total** | **55** |

---

## Test Execution Order

### Phase 1: JSONL Infrastructure
1. `test_output_parseable_by_jq`
2. `test_invariant_all_lines_valid_jsonl`
3. `test_invariant_all_lines_have_type_field`

### Phase 2: focus.rs (Simplest)
1. `given_existing_session_when_focus_then_emits_session_and_result`
2. `given_missing_name_when_focus_then_emits_validation_issue`
3. `given_missing_session_when_focus_then_emits_not_found_issue`
4. All focus contract verification tests

### Phase 3: remove.rs
1. `given_existing_session_when_remove_then_emits_success`
2. `given_missing_session_when_remove_idempotent_then_emits_success`
3. `given_missing_session_when_remove_then_emits_not_found_issue`
4. All remove contract verification tests

### Phase 4: sync.rs
1. `given_single_session_when_sync_then_emits_action_and_result`
2. `given_all_sessions_when_sync_then_emits_actions_summary_result`
3. `given_partial_failure_when_sync_all_then_emits_mixed_output`
4. All sync contract verification tests

### Phase 5: add.rs (Most Complex)
1. `given_valid_name_when_add_then_emits_session_output`
2. `given_existing_session_when_add_idempotent_then_emits_existing_session`
3. `given_hook_fails_when_add_then_emits_rollback_actions`
4. All add contract verification tests

---

## Test File Locations

Tests should be added to:
- `crates/zjj/src/commands/add.rs` (existing `#[cfg(test)]` module)
- `crates/zjj/src/commands/remove.rs` (existing `#[cfg(test)]` module)
- `crates/zjj/src/commands/sync.rs` (existing `#[cfg(test)]` module)
- `crates/zjj/src/commands/focus.rs` (existing `#[cfg(test)]` module)

Integration tests:
- `tests/jsonl_output_tests.rs` (new file for cross-command tests)

---

## Mock/Stub Strategy

For testing without actual JJ repos or Zellij:

```rust
// Trait for dependency injection
#[async_trait]
pub trait OutputEmitter {
    async fn emit(&self, line: &OutputLine) -> io::Result<()>;
}

// Production implementation
pub struct StdoutEmitter;
impl OutputEmitter for StdoutEmitter {
    async fn emit(&self, line: &OutputLine) -> io::Result<()> {
        emit_stdout(line)
    }
}

// Test implementation
pub struct VecEmitter(pub RefCell<Vec<OutputLine>>);
impl OutputEmitter for VecEmitter {
    async fn emit(&self, line: &OutputLine) -> io::Result<()> {
        self.0.borrow_mut().push(line.clone());
        Ok(())
    }
}
```

---

## Acceptance Criteria

All tests pass when:

1. **Every command emits valid JSONL** - Parseable by jq
2. **Every success path ends with ResultOutput::success** - Last line check
3. **Every failure path emits Issue** - Error line present
4. **No unwrap/expect/panic** - Static analysis pass
5. **All invariants hold** - Contract tests pass
6. **Idempotent operations never fail for missing resources** - Specific test pass
7. **Counts match reality** - sync counts verified
