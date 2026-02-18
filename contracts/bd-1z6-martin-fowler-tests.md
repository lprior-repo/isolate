# Martin Fowler Test Plan: bd-1z6 - Delete interactive confirmation from remove command

**Bead ID:** bd-1z6
**Title:** Delete interactive confirmation from remove command
**Test Framework:** Given-When-Then (BDD style)
**Coverage Target:** 100% of contract specification

## Test Suite Organization

```
bd-1z6-tests/
├── happy_path/          # HP-001 to HP-010
├── error_path/          # EP-001 to EP-025
├── edge_cases/          # EC-001 to EC-015
└── contract_verification/ # CV-001 to CV-020
```

---

## Happy Path Tests (HP)

### HP-001: Basic non-interactive removal

**GIVEN** an initialized ZJJ repository with existing session "feature-auth"
**WHEN** user runs `zjj remove feature-auth`
**THEN** session is removed immediately without prompting
**AND** workspace directory is deleted
**AND** session is removed from database
**AND** exit code is 0
**AND** output contains "Removed session 'feature-auth'"

```rust
#[test]
fn test_hp001_non_interactive_remove_succeeds() {
    // Implementation in crates/zjj/tests/test_remove_non_interactive.rs
}
```

---

### HP-002: Non-interactive removal with --force (backwards compatibility)

**GIVEN** an initialized ZJJ repository with existing session "bugfix-123"
**WHEN** user runs `zjj remove bugfix-123 --force`
**THEN** session is removed immediately
**AND** no confirmation prompt is shown
**AND** exit code is 0
**AND** behavior is identical to `zjj remove bugfix-123` (force is no-op)

```rust
#[test]
fn test_hp002_force_flag_is_no_op() {
    // Implementation
}
```

---

### HP-003: Removal with --merge flag

**GIVEN** an initialized ZJJ repository with session containing uncommitted changes
**WHEN** user runs `zjj remove feature-x --merge`
**THEN** changes are squash-merged to main branch
**AND** JJ workspace is forgotten
**AND** workspace directory is deleted
**AND** session is removed from database
**AND** exit code is 0
**AND** output indicates merge occurred

```rust
#[test]
fn test_hp003_remove_with_merge_squashes_to_main() {
    // Implementation
}
```

---

### HP-004: Idempotent removal - session exists

**GIVEN** an initialized ZJJ repository with existing session "test-session"
**WHEN** user runs `zjj remove test-session --idempotent`
**THEN** session is removed immediately
**AND** workspace is deleted
**AND** exit code is 0
**AND** no confirmation prompt shown

```rust
#[test]
fn test_hp004_idempotent_remove_existing_session() {
    // Implementation
}
```

---

### HP-005: Idempotent removal - session doesn't exist

**GIVEN** an initialized ZJJ repository with NO session "missing-session"
**WHEN** user runs `zjj remove missing-session --idempotent`
**THEN** command succeeds immediately
**AND** NO error is returned
**AND** exit code is 0
**AND** output contains "already removed"
**AND** no confirmation prompt shown

```rust
#[test]
fn test_hp005_idempotent_remove_missing_session_succeeds() {
    // Implementation
}
```

---

### HP-006: Dry-run mode

**GIVEN** an initialized ZJJ repository with existing session "preview-session"
**WHEN** user runs `zjj remove preview-session --dry-run`
**THEN** NO changes are made to filesystem
**AND** NO changes are made to database
**AND** exit code is 0
**AND** output starts with "DRY-RUN:"
**AND** output includes session name and workspace path

```rust
#[test]
fn test_hp006_dry_run_shows_preview_without_changes() {
    // Implementation
}
```

---

### HP-007: JSON output format

**GIVEN** an initialized ZJJ repository with existing session "json-test"
**WHEN** user runs `zjj remove json-test --json`
**THEN** session is removed
**AND** output is valid JSON
**AND** JSON is wrapped in SchemaEnvelope
**AND** `$schema` field is "zjj://remove/v1"
**AND** `schema_type` field is "single"
**AND** payload contains `name` and `message` fields
**AND** exit code is 0

```rust
#[test]
fn test_hp007_json_output_has_correct_schema() {
    // Implementation
}
```

---

### HP-008: Removal with missing workspace (Type 1 orphan)

**GIVEN** an initialized ZJJ repository with session "orphan-session"
**AND** session's workspace directory was externally deleted
**WHEN** user runs `zjj remove orphan-session`
**THEN** database record is deleted
**AND** NO error is returned (workspace already gone)
**AND** exit code is 0
**AND** output contains "(workspace was already gone)"

```rust
#[test]
fn test_hp008_remove_succeeds_when_workspace_already_missing() {
    // Implementation
}
```

---

### HP-009: Concurrent removal (idempotent ENOENT handling)

**GIVEN** an initialized ZJJ repository with session "concurrent-test"
**AND** two processes simultaneously remove the session
**WHEN** both processes run `zjj remove concurrent-test`
**THEN** both processes succeed
**AND** at least one process sees "already removed" message
**AND** NO orphaned resources remain
**AND** both processes exit with code 0

```rust
#[test]
fn test_hp009_concurrent_removal_is_safe() {
    // Implementation (requires multiprocessing)
}
```

---

### HP-010: Multiple removals of same session (idempotent)

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove retry-test --idempotent` three times consecutively
**THEN** all three invocations succeed
**AND** first invocation removes the session
**AND** subsequent invocations output "already removed"
**AND** all exit codes are 0

```rust
#[test]
fn test_hp010_multiple_idempotent_removals_all_succeed() {
    // Implementation
}
```

---

## Error Path Tests (EP)

### EP-001: Session not found (non-idempotent mode)

**GIVEN** an initialized ZJJ repository with NO session "missing"
**WHEN** user runs `zjj remove missing`
**THEN** command fails
**AND** exit code is 2 (NOT_FOUND)
**AND** error message contains "not found"
**AND** error message suggests using `--idempotent`
**AND** no confirmation prompt was shown

```rust
#[test]
fn test_ep001_remove_nonexistent_session_fails() {
    // Implementation
}
```

---

### EP-002: Invalid session name format

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove 123invalid` (starts with number)
**THEN** command fails
**AND** exit code is 1 (VALIDATION_ERROR)
**AND** error message explains naming rules
**AND** error shows valid example

```rust
#[test]
fn test_ep002_invalid_session_name_format() {
    // Implementation
}
```

---

### EP-003: Empty session name

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove ""`
**THEN** command fails
**AND** exit code is 1 (VALIDATION_ERROR)
**AND** error message indicates name cannot be empty

```rust
#[test]
fn test_ep003_empty_session_name_rejected() {
    // Implementation
}
```

---

### EP-004: ZJJ not initialized

**GIVEN** a JJ repository WITHOUT ZJJ initialized
**WHEN** user runs `zjj remove test-session`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error message suggests running `zjj init`

```rust
#[test]
fn test_ep004_remove_fails_when_not_initialized() {
    // Implementation
}
```

---

### EP-005: Workspace deletion permission denied

**GIVEN** an initialized ZJJ repository with session "protected-session"
**AND** workspace directory has read-only permissions
**WHEN** user runs `zjj remove protected-session`
**THEN** command fails
**AND** exit code is 3 (IO_ERROR)
**AND** session is marked as "removal_failed" in database
**AND** error message includes permission denied
**AND** workspace still exists (not deleted)

```rust
#[test]
fn test_ep005_workspace_permission_denied_marks_failed() {
    // Implementation
}
```

---

### EP-006: Database write failure during deletion

**GIVEN** an initialized ZJJ repository with session "db-fail-test"
**AND** database file becomes read-only after workspace deletion
**WHEN** user runs `zjj remove db-fail-test`
**THEN** workspace is deleted
**AND** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error is marked CRITICAL
**AND** error message indicates manual cleanup may be needed

```rust
#[test]
fn test_ep006_database_failure_after_workspace_delete() {
    // Implementation
}
```

---

### EP-007: JJ command not found (with --merge)

**GIVEN** an initialized ZJJ repository with session "merge-test"
**AND** JJ binary is NOT in PATH
**WHEN** user runs `zjj remove merge-test --merge`
**THEN** command fails
**AND** exit code is 4 (JJ_COMMAND_ERROR)
**AND** error message indicates JJ not installed
**AND** error suggests installing JJ
**AND** session is NOT removed

```rust
#[test]
fn test_ep007_jj_not_found_with_merge_flag() {
    // Implementation
}
```

---

### EP-008: JJ merge fails (conflicts)

**GIVEN** an initialized ZJJ repository with session having merge conflicts
**WHEN** user runs `zjj remove conflict-session --merge`
**THEN** command fails
**AND** exit code is 4 (JJ_COMMAND_ERROR)
**AND** error message includes JJ's stderr
**AND** session is NOT removed
**AND** workspace still exists

```rust
#[test]
fn test_ep008_jj_merge_conflict_fails() {
    // Implementation
}
```

---

### EP-009: JJ workspace forget fails (real error, not "not found")

**GIVEN** an initialized ZJJ repository with session "jj-fail-test"
**AND** JJ is in a state that causes real forget failure
**WHEN** user runs `zjj remove jj-fail-test`
**THEN** command fails
**AND** exit code is 3 (IO_ERROR mapped to WorkspaceInaccessible)
**AND** workspace is NOT deleted (orphan prevention)
**AND** error message indicates JJ forget failed

```rust
#[test]
fn test_ep009_jj_forget_failure_prevents_workspace_deletion() {
    // Implementation
}
```

---

### EP-010: Dry-run with non-existent session

**GIVEN** an initialized ZJJ repository with NO session "dry-missing"
**WHEN** user runs `zjj remove dry-missing --dry-run`
**THEN** command fails
**AND** exit code is 2 (NOT_FOUND)
**AND** dry-run doesn't bypass existence check

```rust
#[test]
fn test_ep010_dry_run_fails_on_nonexistent_session() {
    // Implementation
}
```

---

### EP-011: Special characters in session name

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove "test;rm -rf /"`
**THEN** command fails validation
**AND** exit code is 1 (VALIDATION_ERROR)
**AND** special characters are rejected

```rust
#[test]
fn test_ep011_special_chars_in_name_rejected() {
    // Implementation
}
```

---

### EP-012: Session name with newlines

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove "test\nsession"`
**THEN** command fails validation
**AND** exit code is 1 (VALIDATION_ERROR)

```rust
#[test]
fn test_ep012_newlines_in_name_rejected() {
    // Implementation
}
```

---

### EP-013: Very long session name

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove "aaaa...aaaa"` (1000 characters)
**THEN** command fails validation OR succeeds with truncation
**AND** exit code is 1 if validation fails

```rust
#[test]
fn test_ep013_excessively_long_name_rejected() {
    // Implementation
}
```

---

### EP-014: Workspace is a file (not directory)

**GIVEN** an initialized ZJJ repository with session pointing to a file
**WHEN** user runs `zjj remove file-session`
**THEN** command fails
**AND** exit code is 3 (IO_ERROR)
**AND** error indicates workspace is not a directory

```rust
#[test]
fn test_ep014_workspace_is_file_not_directory() {
    // Implementation
}
```

---

### EP-015: Workspace is symlink

**GIVEN** an initialized ZJJ repository with session workspace as symlink
**WHEN** user runs `zjj remove symlink-session`
**THEN** command succeeds OR fails with clear error
**AND** symlink handling is documented

```rust
#[test]
fn test_ep015_workspace_is_symlink() {
    // Implementation
}
```

---

### EP-016: Database lock contention

**GIVEN** an initialized ZJJ repository with session "locked-session"
**AND** database is locked by another process
**WHEN** user runs `zjj remove locked-session`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error indicates lock contention

```rust
#[test]
fn test_ep016_database_lock_contention() {
    // Implementation
}
```

---

### EP-017: Corrupted database record

**GIVEN** an initialized ZJJ repository with corrupted session record
**WHEN** user runs `zjj remove corrupted-session`
**THEN** command fails
**AND** exit code is 3 (DATABASE_ERROR)
**AND** error indicates corruption

```rust
#[test]
fn test_ep017_corrupted_database_record() {
    // Implementation
}
```

---

### EP-018: Workspace path contains Unicode characters

**GIVEN** an initialized ZJJ repository with session "unicode-テスト"
**AND** workspace path contains Unicode characters
**WHEN** user runs `zjj remove unicode-テスト`
**THEN** command succeeds OR fails with clear error
**AND** Unicode is handled correctly

```rust
#[test]
fn test_ep018_unicode_in_workspace_path() {
    // Implementation
}
```

---

### EP-019: Zellij not installed (inside Zellij session)

**GIVEN** an initialized ZJJ repository with session "no-zellij"
**AND** running inside Zellij but Zellij binary removed
**WHEN** user runs `zjj remove no-zellij`
**THEN** command succeeds (Zellij closure is non-critical)
**AND** warning logged for Zellij failure
**AND** exit code is 0

```rust
#[test]
fn test_ep019_zellij_not_installed_is_non_critical() {
    // Implementation
}
```

---

### EP-020: Multiple errors (workspace fails + database fails)

**GIVEN** an initialized ZJJ repository with session "multi-fail"
**AND** both workspace and database operations will fail
**WHEN** user runs `zjj remove multi-fail`
**THEN** command fails
**AND** first error is reported (workspace failure)
**AND** session marked as "removal_failed" in database
**AND** exit code is 3 (IO_ERROR)

```rust
#[test]
fn test_ep020_cascading_failures_report_first_error() {
    // Implementation
}
```

---

### EP-021: Idempotent mode with workspace deletion failure

**GIVEN** an initialized ZJJ repository with session "idem-fail"
**AND** workspace deletion will fail
**WHEN** user runs `zjj remove idem-fail --idempotent`
**THEN** command fails (idempotent doesn't bypass all errors)
**AND** exit code is 3 (IO_ERROR)
**AND** session marked as "removal_failed"

```rust
#[test]
fn test_ep021_idempotent_doesnt_suppress_real_errors() {
    // Implementation
}
```

---

### EP-022: Merge with no main branch

**GIVEN** an initialized ZJJ repository without main branch
**AND** session "no-main" exists
**WHEN** user runs `zjj remove no-main --merge`
**THEN** command fails
**AND** exit code is 4 (JJ_COMMAND_ERROR)
**AND** error indicates main branch missing

```rust
#[test]
fn test_ep022_merge_without_main_branch() {
    // Implementation
}
```

---

### EP-023: Merge with no changes to squash

**GIVEN** an initialized ZJJ repository with session having no changes
**WHEN** user runs `zjj remove no-changes --merge`
**THEN** command succeeds (no changes is OK)
**AND** exit code is 0
**AND** session is removed

```rust
#[test]
fn test_ep023_merge_with_no_changes_succeeds() {
    // Implementation
}
```

---

### EP-024: Keep-branch flag behavior

**GIVEN** an initialized ZJJ repository with session "keep-branch-test"
**WHEN** user runs `zjj remove keep-branch-test --keep-branch`
**THEN** command succeeds
**AND** session is removed from database
**AND** workspace is deleted
**AND** JJ branch is preserved (document behavior)

```rust
#[test]
fn test_ep024_keep_branch_preserves_jj_branch() {
    // Implementation
}
```

---

### EP-025: JSON output with error

**GIVEN** an initialized ZJJ repository with NO session "json-error"
**WHEN** user runs `zjj remove json-error --json`
**THEN** command fails
**AND** exit code is 2 (NOT_FOUND)
**AND** stderr contains valid JSON error
**AND** JSON error follows schema with error fields

```rust
#[test]
fn test_ep025_json_error_output_format() {
    // Implementation
}
```

---

## Edge Case Tests (EC)

### EC-001: Removal immediately after creation

**GIVEN** an initialized ZJJ repository
**WHEN** user creates session then immediately removes it
**THEN** both operations succeed
**AND** no race conditions occur

```rust
#[test]
fn test_ec001_immediate_removal_after_creation() {
    // Implementation
}
```

---

### EC-002: Removing currently active session

**GIVEN** an initialized ZJJ repository with active session "active"
**AND** user is currently inside the session's workspace
**WHEN** user runs `zjj remove active`
**THEN** command succeeds OR fails with clear error
**AND** behavior is documented

```rust
#[test]
fn test_ec002_remove_currently_active_session() {
    // Implementation
}
```

---

### EC-003: Workspace path with trailing slash

**GIVEN** an initialized ZJJ repository with session having path with trailing "/"
**WHEN** user runs `zjj remove slash-session`
**THEN** command succeeds
**AND** path normalization handled correctly

```rust
#[test]
fn test_ec003_workspace_path_with_trailing_slash() {
    // Implementation
}
```

---

### EC-004: Workspace path with relative components

**GIVEN** an initialized ZJJ repository with session path containing "../"
**WHEN** user runs `zjj remove relative-path-session`
**THEN** command succeeds OR validation rejects path
**AND** path traversal is prevented

```rust
#[test]
fn test_ec004_workspace_path_with_relative_components() {
    // Implementation
}
```

---

### EC-005: Empty workspace directory

**GIVEN** an initialized ZJJ repository with session having empty workspace
**WHEN** user runs `zjj remove empty-workspace`
**THEN** command succeeds
**AND** empty directory is deleted

```rust
#[test]
fn test_ec005_empty_workspace_directory() {
    // Implementation
}
```

---

### EC-006: Workspace with many files

**GIVEN** an initialized ZJJ repository with session having 10,000 files
**WHEN** user runs `zjj remove large-workspace`
**THEN** command succeeds
**AND** all files are deleted
**AND** operation completes in reasonable time

```rust
#[test]
fn test_ec006_large_workspace_deletion() {
    // Implementation
}
```

---

### EC-007: Workspace with deep directory structure

**GIVEN** an initialized ZJJ repository with session having 100 nested directories
**WHEN** user runs `zjj remove deep-workspace`
**THEN** command succeeds
**AND** entire tree is deleted

```rust
#[test]
fn test_ec007_deep_directory_structure() {
    // Implementation
}
```

---

### EC-008: Session name same as JJ command

**GIVEN** an initialized ZJJ repository with session named "status"
**WHEN** user runs `zjj remove status`
**THEN** command succeeds
**AND** no ambiguity with JJ commands

```rust
#[test]
fn test_ec008_session_name_conflicts_with_jj_command() {
    // Implementation
}
```

---

### EC-009: Session with non-ASCII characters

**GIVEN** an initialized ZJJ repository with session "café"
**WHEN** user runs `zjj remove café`
**THEN** command succeeds
**AND** Unicode handled correctly

```rust
#[test]
fn test_ec009_non_ascii_session_name() {
    // Implementation
}
```

---

### EC-010: Multiple spaces in session name

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove "test  session"` (multiple spaces)
**THEN** command fails validation
**AND** spaces are rejected

```rust
#[test]
fn test_ec010_multiple_spaces_in_name() {
    // Implementation
}
```

---

### EC-011: Leading/trailing whitespace in name

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove "  test  "` (with spaces)
**THEN** command fails validation
**AND** whitespace is rejected

```rust
#[test]
fn test_ec011_whitespace_trimmed_or_rejected() {
    // Implementation
}
```

---

### EC-012: Case sensitivity in session name

**GIVEN** an initialized ZJJ repository with session "MySession"
**WHEN** user runs `zjj remove mysession` (different case)
**THEN** command fails
**AND** case-sensitive matching is enforced

```rust
#[test]
fn test_ec012_case_sensitive_session_names() {
    // Implementation
}
```

---

### EC-013: Session with dash in name

**GIVEN** an initialized ZJJ repository with session "my-session"
**WHEN** user runs `zjj remove my-session`
**THEN** command succeeds
**AND** dashes are valid

```rust
#[test]
fn test_ec013_session_name_with_dash() {
    // Implementation
}
```

---

### EC-014: Session with underscore in name

**GIVEN** an initialized ZJJ repository with session "my_session"
**WHEN** user runs `zjj remove my_session`
**THEN** command succeeds
**AND** underscores are valid

```rust
#[test]
fn test_ec014_session_name_with_underscore() {
    // Implementation
}
```

---

### EC-015: Very common session name

**GIVEN** an initialized ZJJ repository with 100 sessions
**AND** one named "test" exists
**WHEN** user runs `zjj remove test`
**THEN** correct session is removed
**AND** no ambiguity with other sessions

```rust
#[test]
fn test_ec015_common_session_name_unambiguous() {
    // Implementation
}
```

---

## Contract Verification Tests (CV)

### CV-001: Verify precondition - database accessible

**GIVEN** an initialized ZJJ repository
**AND** database file is readable
**WHEN** running any remove operation
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
**WHEN** user runs `zjj remove test`
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

### CV-003: Verify precondition - session name valid

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove valid-session-123`
**THEN** name validation passes
**AND** operation proceeds

```rust
#[test]
fn test_cv003_valid_session_name_precondition() {
    // Implementation
}
```

---

### CV-004: Verify precondition violation - invalid session name

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove 123invalid`
**THEN** name validation fails
**AND** Error::ValidationError returned
**AND** exit code is 1

```rust
#[test]
fn test_cv004_invalid_name_violates_precondition() {
    // Implementation
}
```

---

### CV-005: Verify postcondition - session removed from database

**GIVEN** an initialized ZJJ repository with session "verify-removal"
**WHEN** user runs `zjj remove verify-removal`
**THEN** `db.get("verify-removal").await` returns `None`
**AND** session is not in database

```rust
#[test]
fn test_cv005_session_removed_from_database_postcondition() {
    // Implementation
}
```

---

### CV-006: Verify postcondition - workspace directory deleted

**GIVEN** an initialized ZJJ repository with session and workspace
**WHEN** user runs `zjj remove workspace-test`
**THEN** workspace directory no longer exists
**AND** `Path::new(&workspace_path).exists()` is false

```rust
#[test]
fn test_cv006_workspace_deleted_postcondition() {
    // Implementation
}
```

---

### CV-007: Verify postcondition - JSON output format

**GIVEN** an initialized ZJJ repository with session
**WHEN** user runs `zjj remove json-test --json`
**THEN** output is valid JSON
**AND** SchemaEnvelope wrapper present
**AND** `$schema` field follows `zjj://remove/v1` pattern
**AND** `schema_type` is "single"

```rust
#[test]
fn test_cv007_json_output_format_postcondition() {
    // Implementation
}
```

---

### CV-008: Verify invariant - no orphaned workspaces

**GIVEN** an initialized ZJJ repository
**AND** a session with workspace exists
**WHEN** removal fails at any point
**THEN** workspace is never deleted without database record
**AND** NO Type 2 orphans created
**AND** database has "removal_failed" marker if workspace deletion failed

```rust
#[test]
fn test_cv008_no_orphaned_workspaces_invariant() {
    // Implementation
}
```

---

### CV-009: Verify invariant - idempotent ENOENT handling

**GIVEN** an initialized ZJJ repository with session
**AND** workspace externally deleted
**WHEN** user runs `zjj remove idem-test`
**THEN** ENOENT error is handled idempotently
**AND** database record is cleaned up
**AND** operation succeeds

```rust
#[test]
fn test_cv009_idempotent_enoent_invariant() {
    // Implementation
}
```

---

### CV-010: Verify invariant - force flag no-op

**GIVEN** an initialized ZJJ repository with session
**WHEN** user runs `zjj remove force-test --force`
**THEN** behavior is identical to `zjj remove force-test`
**AND** no confirmation prompt occurs in either case
**AND** both succeed

```rust
#[test]
fn test_cv010_force_flag_no_op_invariant() {
    // Implementation
}
```

---

### CV-011: Verify invariant - no interactive prompting

**GIVEN** an initialized ZJJ repository with session
**WHEN** user runs `zjj remove no-prompt-test`
**THEN** NO stdin reading occurs
**AND** NO "y/N" prompt displayed
**AND** removal executes immediately

```rust
#[test]
fn test_cv011_no_interactive_prompting_invariant() {
    // Implementation (requires stdin mocking)
}
```

---

### CV-012: Verify error taxonomy - NotFound exit code

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove nonexistent`
**THEN** error is `Error::NotFound`
**AND** exit code is 2
**AND** error code is "NOT_FOUND"

```rust
#[test]
fn test_cv012_error_taxonomy_not_found() {
    // Implementation
}
```

---

### CV-013: Verify error taxonomy - ValidationError exit code

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove ""`
**THEN** error is `Error::ValidationError`
**AND** exit code is 1
**AND** error code is "VALIDATION_ERROR"

```rust
#[test]
fn test_cv013_error_taxonomy_validation_error() {
    // Implementation
}
```

---

### CV-014: Verify error taxonomy - IoError exit code

**GIVEN** an initialized ZJJ repository with read-only workspace
**WHEN** user runs `zjj remove readonly-test`
**THEN** error is `Error::IoError` or `RemoveError::WorkspaceRemovalFailed`
**AND** exit code is 3
**AND** error code is "IO_ERROR"

```rust
#[test]
fn test_cv014_error_taxonomy_io_error() {
    // Implementation
}
```

---

### CV-015: Verify error taxonomy - JjCommandError exit code

**GIVEN** an initialized ZJJ repository
**AND** JJ not in PATH
**WHEN** user runs `zjj remove jj-fail --merge`
**THEN** error is `Error::JjCommandError`
**AND** exit code is 4
**AND** error code is "JJ_COMMAND_ERROR"

```rust
#[test]
fn test_cv015_error_taxonomy_jj_command_error() {
    // Implementation
}
```

---

### CV-016: Verify idempotent mode precondition relaxation

**GIVEN** an initialized ZJJ repository
**WHEN** user runs `zjj remove missing --idempotent`
**THEN** precondition "session must exist" is relaxed
**AND** operation succeeds
**AND** output indicates "already removed"

```rust
#[test]
fn test_cv016_idempotent_relaxes_precondition() {
    // Implementation
}
```

---

### CV-017: Verify dry-run mode no changes

**GIVEN** an initialized ZJJ repository with session
**WHEN** user runs `zjj remove dry-test --dry-run`
**THEN** NO filesystem changes occur
**AND** NO database changes occur
**AND** exit code is 0

```rust
#[test]
fn test_cv017_dry_run_no_changes_invariant() {
    // Implementation
}
```

---

### CV-018: Verify merge mode postcondition

**GIVEN** an initialized ZJJ repository with session containing changes
**WHEN** user runs `zjj remove merge-test --merge`
**THEN** changes are squash-merged to main
**AND** workspace is deleted
**AND** session is removed from database
**AND** exit code is 0

```rust
#[test]
fn test_cv018_merge_mode_postcondition() {
    // Implementation
}
```

---

### CV-019: Verify Zellij tab closure is non-critical

**GIVEN** an initialized ZJJ repository with session
**AND** Zellij tab closure will fail
**WHEN** user runs `zjj remove zellij-fail`
**THEN** removal still succeeds
**AND** warning is logged
**AND** exit code is 0

```rust
#[test]
fn test_cv019_zellij_failure_non_critical() {
    // Implementation
}
```

---

### CV-020: Verify backwards compatibility - force flag accepted

**GIVEN** an initialized ZJJ repository with session
**WHEN** user runs `zjj remove compat-test -f`
**THEN** command succeeds
**AND** flag is accepted (no "unknown argument" error)
**AND** behavior is identical to without flag

```rust
#[test]
fn test_cv020_backwards_compatibility_force_flag() {
    // Implementation
}
```

---

## Test Execution Order

### Phase 1: Contract Verification (CV-001 to CV-020)
- Run first to verify contract fundamentals
- All MUST pass before proceeding

### Phase 2: Happy Path (HP-001 to HP-010)
- Basic functionality tests
- All MUST pass

### Phase 3: Error Path (EP-001 to EP-025)
- Error handling tests
- All MUST pass

### Phase 4: Edge Cases (EC-001 to EC-015)
- Boundary condition tests
- All SHOULD pass

## Success Criteria

- **P0 (Critical):** All CV tests pass (contract verification)
- **P0 (Critical):** All HP tests pass (happy path)
- **P0 (Critical):** All EP tests pass (error handling)
- **P1 (High):** All EC tests pass (edge cases)
- **Coverage:** 100% of contract specification tested

## Test Metrics

- Total tests: 65
- Critical (P0): 55 tests
- High priority (P1): 10 tests
- Estimated execution time: 5-10 minutes (serial)

---

**Test Plan Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
