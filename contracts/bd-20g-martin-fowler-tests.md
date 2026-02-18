# Martin Fowler Test Plan: bd-20g - Delete interactive confirmation from clean command

**Bead ID:** bd-20g
**Title:** Delete interactive confirmation from clean command
**Test Framework:** Given-When-Then (BDD style)
**Coverage Target:** 100% of contract specification

## Test Suite Organization

```
bd-20g-tests/
├── happy_path/          # HP-001 to HP-010
├── error_path/          # EP-001 to EP-015
├── edge_cases/          # EC-001 to EC-010
└── contract_verification/ # CV-001 to CV-015
```

---

## Happy Path Tests (HP)

### HP-001: Basic non-interactive clean with stale sessions

**GIVEN** an initialized ZJJ repository with 3 sessions
**AND** 2 sessions have missing workspace directories (externally deleted)
**WHEN** user runs `zjj clean`
**THEN** stale sessions are removed immediately without prompting
**AND** stale sessions are removed from database
**AND** valid session remains in database
**AND** exit code is 0
**AND** output contains "Removed 2 stale session(s)"
**AND** output lists the 2 removed session names

```rust
#[test]
fn test_hp001_non_interactive_clean_succeeds() {
    // Implementation in crates/zjj/tests/test_clean_non_interactive.rs
}
```

---

### HP-002: Non-interactive clean with --force (backwards compatibility)

**GIVEN** an initialized ZJJ repository with 2 stale sessions
**WHEN** user runs `zjj clean --force`
**THEN** stale sessions are removed immediately
**AND** no confirmation prompt is shown
**AND** exit code is 0
**AND** behavior is identical to `zjj clean` (force is no-op)

```rust
#[test]
fn test_hp002_force_flag_is_no_op() {
    // Implementation
}
```

---

### HP-003: Clean with no stale sessions

**GIVEN** an initialized ZJJ repository with 3 sessions
**AND** all sessions have valid workspace directories
**WHEN** user runs `zjj clean`
**THEN** no sessions are removed
**AND** no database changes occur
**AND** exit code is 0
**AND** output contains "No stale sessions found"

```rust
#[test]
fn test_hp003_clean_with_no_stale_sessions() {
    // Implementation
}
```

---

### HP-004: Dry-run mode with stale sessions

**GIVEN** an initialized ZJJ repository with 2 stale sessions
**WHEN** user runs `zjj clean --dry-run`
**THEN** NO changes are made to database
**AND** exit code is 0
**AND** output contains "Found 2 stale session(s) (dry-run, no changes made):"
**AND** output lists the 2 stale session names
**AND** output suggests "Run 'zjj clean --force' to remove these sessions"

```rust
#[test]
fn test_hp004_dry_run_shows_preview_without_changes() {
    // Implementation
}
```

---

### HP-005: JSON output format

**GIVEN** an initialized ZJJ repository with 2 stale sessions
**WHEN** user runs `zjj clean --json`
**THEN** stale sessions are removed
**AND** output is valid JSON
**AND** JSON is wrapped in SchemaEnvelope
**AND** `$schema` field is "zjj://clean/v1"
**AND** `schema_type` field is "single"
**AND** payload contains `stale_count`, `removed_count`, and `stale_sessions` fields
**AND** exit code is 0

```rust
#[test]
fn test_hp005_json_output_has_correct_schema() {
    // Implementation
}
```

---

### HP-006: Clean with all stale sessions

**GIVEN** an initialized ZJJ repository with 5 sessions
**AND** all 5 sessions have missing workspace directories
**WHEN** user runs `zjj clean`
**THEN** all 5 sessions are removed from database
**AND** database is empty
**AND** exit code is 0
**AND** output contains "Removed 5 stale session(s)"

```rust
#[test]
fn test_hp006_clean_all_stale_sessions() {
    // Implementation
}
```

---

### HP-007: Clean with many stale sessions (performance test)

**GIVEN** an initialized ZJJ repository with 100 sessions
**AND** 50 sessions have missing workspace directories
**WHEN** user runs `zjj clean`
**THEN** all 50 stale sessions are removed
**AND** operation completes in reasonable time (< 5 seconds)
**AND** exit code is 0
**AND** output lists all 50 removed sessions

```rust
#[test]
fn test_hp007_clean_many_stale_sessions() {
    // Implementation
}
```

---

### HP-008: Dry-run with no stale sessions

**GIVEN** an initialized ZJJ repository with valid sessions
**WHEN** user runs `zjj clean --dry-run`
**THEN** NO changes made
**AND** exit code is 0
**AND** output contains "No stale sessions found"

```rust
#[test]
fn test_hp008_dry_run_with_no_stale_sessions() {
    // Implementation
}
```

---

### HP-009: JSON output with no stale sessions

**GIVEN** an initialized ZJJ repository with valid sessions
**WHEN** user runs `zjj clean --json`
**THEN** output is valid JSON
**AND** `stale_count` is 0
**AND** `removed_count` is 0
**AND** `stale_sessions` is empty array
**AND** exit code is 0

```rust
#[test]
fn test_hp009_json_output_no_stale_sessions() {
    // Implementation
}
```

---

### HP-010: Force flag with dry-run

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean --force --dry-run`
**THEN** NO changes made (dry-run takes precedence)
**AND** exit code is 0
**AND** output indicates dry-run mode

```rust
#[test]
fn test_hp010_force_with_dry_run_is_still_dry_run() {
    // Implementation
}
```

---

## Error Path Tests (EP)

### EP-001: Database not accessible

**GIVEN** a JJ repository WITHOUT ZJJ initialized
**WHEN** user runs `zjj clean`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error message suggests running `zjj init`
**AND** no confirmation prompt was shown

```rust
#[test]
fn test_ep001_clean_fails_when_not_initialized() {
    // Implementation
}
```

---

### EP-002: Database lock contention

**GIVEN** an initialized ZJJ repository
**AND** database is locked by another process
**WHEN** user runs `zjj clean`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error indicates lock contention

```rust
#[test]
fn test_ep002_database_lock_contention() {
    // Implementation
}
```

---

### EP-003: Corrupted database record

**GIVEN** an initialized ZJJ repository with corrupted session record
**WHEN** user runs `zjj clean`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error indicates corruption

```rust
#[test]
fn test_ep003_corrupted_database_record() {
    // Implementation
}
```

---

### EP-004: Workspace path permission denied

**GIVEN** an initialized ZJJ repository with session
**AND** workspace directory has no read permissions
**WHEN** user runs `zjj clean`
**THEN** command fails OR succeeds with session marked as stale
**AND** behavior is documented (permission denied = stale)
**AND** no confirmation prompt shown

```rust
#[test]
fn test_ep004_workspace_permission_denied() {
    // Implementation
}
```

---

### EP-005: Workspace path with special characters

**GIVEN** an initialized ZJJ repository with session workspace path containing spaces
**AND** workspace directory is missing
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale
**AND** session is removed from database
**AND** exit code is 0
**AND** path with spaces handled correctly

```rust
#[test]
fn test_ep005_workspace_path_with_spaces() {
    // Implementation
}
```

---

### EP-006: Session deletion failure (database error)

**GIVEN** an initialized ZJJ repository with stale session
**AND** database becomes read-only during deletion
**WHEN** user runs `zjj clean`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error indicates deletion failure

```rust
#[test]
fn test_ep006_session_deletion_failure() {
    // Implementation
}
```

---

### EP-007: Partial deletion (some succeed, some fail)

**GIVEN** an initialized ZJJ repository with 3 stale sessions
**AND** deletion of 2nd session fails
**WHEN** user runs `zjj clean`
**THEN** command may fail OR continue with warnings
**AND** behavior is documented
**AND** at least 1st and 3rd sessions deleted

```rust
#[test]
fn test_ep007_partial_deletion_failure() {
    // Implementation
}
```

---

### EP-008: Empty session database

**GIVEN** an initialized ZJJ repository with no sessions
**WHEN** user runs `zjj clean`
**THEN** command succeeds
**AND** exit code is 0
**AND** output contains "No stale sessions found"

```rust
#[test]
fn test_ep008_empty_database() {
    // Implementation
}
```

---

### EP-009: Session with non-existent parent directory

**GIVEN** an initialized ZJJ repository with session
**AND** workspace path parent directory doesn't exist
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale
**AND** session is removed from database
**AND** exit code is 0

```rust
#[test]
fn test_ep009_workspace_parent_missing() {
    // Implementation
}
```

---

### EP-010: JSON output with error

**GIVEN** a JJ repository WITHOUT ZJJ initialized
**WHEN** user runs `zjj clean --json`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** stderr contains valid JSON error
**AND** JSON error follows schema with error fields

```rust
#[test]
fn test_ep010_json_error_output_format() {
    // Implementation
}
```

---

### EP-011: Concurrent clean operations

**GIVEN** an initialized ZJJ repository with 2 stale sessions
**AND** two processes simultaneously run `zjj clean`
**WHEN** both processes execute
**THEN** both processes succeed
**AND** both sessions are removed
**AND** NO database corruption occurs
**AND** both exit codes are 0

```rust
#[test]
fn test_ep011_concurrent_clean_operations() {
    // Implementation (requires multiprocessing)
}
```

---

### EP-012: Session with very long path

**GIVEN** an initialized ZJJ repository with session
**AND** workspace path is 1000 characters long
**AND** workspace directory is missing
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale
**AND** session is removed from database
**AND** exit code is 0

```rust
#[test]
fn test_ep012_very_long_workspace_path() {
    // Implementation
}
```

---

### EP-013: Workspace is a symlink pointing to missing target

**GIVEN** an initialized ZJJ repository with session
**AND** workspace is a symlink pointing to non-existent target
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale OR treated as valid
**AND** behavior is documented
**AND** exit code is 0

```rust
#[test]
fn test_ep013_workspace_is_broken_symlink() {
    // Implementation
}
```

---

### EP-014: Database file deleted during operation

**GIVEN** an initialized ZJJ repository with stale sessions
**AND** database file is deleted after listing sessions
**WHEN** user runs `zjj clean`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error indicates database became inaccessible

```rust
#[test]
fn test_ep014_database_deleted_during_operation() {
    // Implementation
}
```

---

### EP-015: Very large stale session list (memory test)

**GIVEN** an initialized ZJJ repository with 1000 sessions
**AND** all 1000 sessions are stale
**WHEN** user runs `zjj clean`
**THEN** all sessions are removed
**AND** memory usage remains reasonable
**AND** operation completes
**AND** exit code is 0

```rust
#[test]
fn test_ep015_very_large_stale_list() {
    // Implementation
}
```

---

## Edge Case Tests (EC)

### EC-001: Clean immediately after session creation with workspace deletion

**GIVEN** an initialized ZJJ repository
**WHEN** user creates session then immediately deletes workspace then runs clean
**THEN** session is identified as stale
**AND** session is removed
**AND** no race conditions occur

```rust
#[test]
fn test_ec001_immediate_clean_after_workspace_deletion() {
    // Implementation
}
```

---

### EC-002: Session with Unicode name

**GIVEN** an initialized ZJJ repository with session "café"
**AND** workspace directory is missing
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale
**AND** session is removed from database
**AND** Unicode handled correctly in output

```rust
#[test]
fn test_ec002_unicode_session_name() {
    // Implementation
}
```

---

### EC-003: Workspace path with Unicode characters

**GIVEN** an initialized ZJJ repository with session
**AND** workspace path contains Unicode characters (e.g., "/tmp/テスト")
**AND** workspace directory is missing
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale
**AND** session is removed from database

```rust
#[test]
fn test_ec003_unicode_in_workspace_path() {
    // Implementation
}
```

---

### EC-004: Session with trailing slash in workspace path

**GIVEN** an initialized ZJJ repository with session
**AND** workspace path has trailing "/" in database
**AND** workspace directory is missing
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale
**AND** path normalization handled correctly

```rust
#[test]
fn test_ec004_workspace_path_with_trailing_slash() {
    // Implementation
}
```

---

### EC-005: Repeated clean operations (idempotent)

**GIVEN** an initialized ZJJ repository with 2 stale sessions
**WHEN** user runs `zjj clean` twice consecutively
**THEN** first invocation removes 2 sessions
**AND** second invocation finds no stale sessions
**AND** both invocations succeed
**AND** both exit codes are 0

```rust
#[test]
fn test_ec005_repeated_clean_is_idempotent() {
    // Implementation
}
```

---

### EC-006: Clean with mix of stale and valid sessions

**GIVEN** an initialized ZJJ repository with 10 sessions
**AND** 3 sessions have missing workspaces
**AND** 7 sessions have valid workspaces
**WHEN** user runs `zjj clean`
**THEN** only 3 stale sessions are removed
**AND** 7 valid sessions remain in database
**AND** output shows "Removed 3 stale session(s)"

```rust
#[test]
fn test_ec006_mixed_stale_and_valid_sessions() {
    // Implementation
}
```

---

### EC-007: Clean in periodic mode (different code path)

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean --periodic`
**THEN** periodic daemon starts
**AND** runs indefinitely
**AND** already non-interactive (unchanged by this bead)
**AND** uses age threshold for filtering

```rust
#[test]
fn test_ec007_periodic_mode_unchanged() {
    // Implementation
}
```

---

### EC-008: Clean with age-threshold flag (non-periodic mode)

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj clean --age-threshold 3600`
**THEN** flag is ignored in non-periodic mode
**AND** all stale sessions (missing workspace) are removed
**AND** behavior is documented

```rust
#[test]
fn test_ec008_age_threshold_ignored_in_standard_mode() {
    // Implementation
}
```

---

### EC-009: Clean with workspace as file (not directory)

**GIVEN** an initialized ZJJ repository with session
**AND** workspace path points to a file (not directory)
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale (file != directory)
**AND** session is removed from database

```rust
#[test]
fn test_ec009_workspace_is_file_not_directory() {
    // Implementation
}
```

---

### EC-010: Clean with no workspace path field (null/empty)

**GIVEN** an initialized ZJJ repository with session
**AND** session's workspace_path field is null or empty
**WHEN** user runs `zjj clean`
**THEN** session is identified as stale (no valid path)
**AND** session is removed from database
**OR** error is raised (documented behavior)

```rust
#[test]
fn test_ec010_null_workspace_path() {
    // Implementation
}
```

---

## Contract Verification Tests (CV)

### CV-001: Verify precondition - database accessible

**GIVEN** an initialized ZJJ repository
**AND** database file is readable
**WHEN** running clean operation
**THEN** precondition check passes
**AND** operation proceeds

```rust
#[test]
fn test_cv001_database_accessible_precondition() {
    // Implementation
}
```

---

### CV-002: Verify precondition violation - database not accessible

**GIVEN** a JJ repository WITHOUT database
**WHEN** user runs `zjj clean`
**THEN** precondition fails
**AND** Error::DatabaseError returned
**AND** exit code is 3

```rust
#[test]
fn test_cv002_database_not_accessible_violates_precondition() {
    // Implementation
}
```

---

### CV-003: Verify postcondition - stale sessions removed from database

**GIVEN** an initialized ZJJ repository with 3 stale sessions
**WHEN** user runs `zjj clean`
**THEN** `db.get(&stale_session_name).await` returns `None` for each
**AND** stale sessions not in database

```rust
#[test]
fn test_cv003_stale_sessions_removed_postcondition() {
    // Implementation
}
```

---

### CV-004: Verify postcondition - valid sessions remain in database

**GIVEN** an initialized ZJJ repository with 2 stale and 3 valid sessions
**WHEN** user runs `zjj clean`
**THEN** valid sessions still exist in database
**AND** `db.get(&valid_session_name).await` returns `Some(session)`

```rust
#[test]
fn test_cv004_valid_sessions_remain_postcondition() {
    // Implementation
}
```

---

### CV-005: Verify postcondition - JSON output format

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean --json`
**THEN** output is valid JSON
**AND** SchemaEnvelope wrapper present
**AND** `$schema` field follows `zjj://clean/v1` pattern
**AND** `schema_type` is "single"

```rust
#[test]
fn test_cv005_json_output_format_postcondition() {
    // Implementation
}
```

---

### CV-006: Verify invariant - no interactive prompting

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean`
**THEN** NO stdin reading occurs
**AND** NO "y/N" prompt displayed to stderr
**AND** cleanup executes immediately

```rust
#[test]
fn test_cv006_no_interactive_prompting_invariant() {
    // Implementation (requires stdin mocking)
}
```

---

### CV-007: Verify invariant - force flag no-op

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean --force`
**THEN** behavior is identical to `zjj clean`
**AND** no confirmation prompt occurs in either case
**AND** both remove same sessions
**AND** both succeed

```rust
#[test]
fn test_cv007_force_flag_no_op_invariant() {
    // Implementation
}
```

---

### CV-008: Verify invariant - no workspace deletion

**GIVEN** an initialized ZJJ repository with stale sessions
**AND** workspace directories already missing
**WHEN** user runs `zjj clean`
**THEN** clean command only removes database records
**AND** does NOT attempt to delete workspace directories
**AND** no filesystem operations on workspace paths

```rust
#[test]
fn test_cv008_no_workspace_deletion_invariant() {
    // Implementation
}
```

---

### CV-009: Verify invariant - stale detection consistency

**GIVEN** an initialized ZJJ repository with sessions
**AND** some workspaces exist, some don't
**WHEN** user runs `zjj clean`
**THEN** only sessions with missing workspaces are removed
**AND** sessions with existing workspaces are never removed

```rust
#[test]
fn test_cv009_stale_detection_consistency_invariant() {
    // Implementation
}
```

---

### CV-010: Verify invariant - output format consistency

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean --json`
**THEN** output always includes `stale_count`, `removed_count`, `stale_sessions`
**AND** `stale_count` equals `removed_count` when not dry-run
**AND** `removed_count` is 0 when dry-run

```rust
#[test]
fn test_cv010_output_format_consistency_invariant() {
    // Implementation
}
```

---

### CV-011: Verify error taxonomy - DatabaseError exit code

**GIVEN** a JJ repository WITHOUT database
**WHEN** user runs `zjj clean`
**THEN** error is `Error::DatabaseError` or `CleanError::DatabaseError`
**AND** exit code is 3

```rust
#[test]
fn test_cv011_error_taxonomy_database_error() {
    // Implementation
}
```

---

### CV-012: Verify dry-run mode no changes

**GIVEN** an initialized ZJJ repository with stale sessions
**WHEN** user runs `zjj clean --dry-run`
**THEN** NO database changes occur
**AND** stale sessions still exist after command
**AND** exit code is 0

```rust
#[test]
fn test_cv012_dry_run_no_changes_invariant() {
    // Implementation
}
```

---

### CV-013: Verify backwards compatibility - force flag accepted

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj clean -f`
**THEN** command succeeds
**AND** flag is accepted (no "unknown argument" error)
**AND** behavior is identical to without flag

```rust
#[test]
fn test_cv013_backwards_compatibility_force_flag() {
    // Implementation
}
```

---

### CV-014: Verify atomic database operations

**GIVEN** an initialized ZJJ repository with 3 stale sessions
**AND** deletion of 2nd session fails
**WHEN** user runs `zjj clean`
**THEN** 1st and 3rd sessions are deleted
**AND** failure is reported
**AND** no partial deletion state corruption

```rust
#[test]
fn test_cv014_atomic_database_operations() {
    // Implementation
}
```

---

### CV-015: Verify confirm_removal function removed

**GIVEN** the clean command implementation
**WHEN** searching for `confirm_removal` function
**THEN** function is not found
**AND** no calls to `confirm_removal` exist
**AND** no stdin reading in clean command

```rust
#[test]
fn test_cv015_confirm_removal_function_removed() {
    // Implementation (compile-time verification)
}
```

---

## Test Execution Order

### Phase 1: Contract Verification (CV-001 to CV-015)
- Run first to verify contract fundamentals
- All MUST pass before proceeding

### Phase 2: Happy Path (HP-001 to HP-010)
- Basic functionality tests
- All MUST pass

### Phase 3: Error Path (EP-001 to EP-015)
- Error handling tests
- All MUST pass

### Phase 4: Edge Cases (EC-001 to EC-010)
- Boundary condition tests
- All SHOULD pass

## Success Criteria

- **P0 (Critical):** All CV tests pass (contract verification)
- **P0 (Critical):** All HP tests pass (happy path)
- **P0 (Critical):** All EP tests pass (error handling)
- **P1 (High):** All EC tests pass (edge cases)
- **Coverage:** 100% of contract specification tested

## Test Metrics

- Total tests: 50
- Critical (P0): 40 tests
- High priority (P1): 10 tests
- Estimated execution time: 3-5 minutes (serial)

## Implementation Location

Tests should be implemented in:
```
/home/lewis/src/zjj/crates/zjj/tests/test_clean_non_interactive.rs
```

Following the pattern from:
```
/home/lewis/src/zjj/crates/zjj/tests/remove_non_interactive.rs
```

---

**Test Plan Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
