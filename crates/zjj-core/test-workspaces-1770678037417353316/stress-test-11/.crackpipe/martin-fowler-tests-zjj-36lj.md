# Martin Fowler Test Plan: Fix `--fix` Flag Misleading Behavior

## Happy Path Tests

### test_fix_flag_attempts_all_auto_fixable_issues
**Given**: Doctor checks show 3 auto-fixable issues (database, orphaned workspaces, stale sessions)
**When**: User runs `zjj doctor --fix`
**Then**:
- All 3 fix functions are called
- Output shows "Fixing State Database..."
- Output shows "Fixing Orphaned Workspaces..."
- Output shows "Fixing Stale Sessions..."
- DoctorFixOutput.fixed contains 3 entries
- All exit codes are correct

### test_dry_run_flag_shows_fixes_without_modifying
**Given**: Doctor checks show auto-fixable issues
**When**: User runs `zjj doctor --fix --dry-run`
**Then**:
- Output shows "Dry-run mode: would fix the following:"
- Lists issues that would be fixed
- NO filesystem modifications occur
- Exit code is 0
- Fix functions are NOT actually called

### test_fix_flag_verbose_mode_shows_progress
**Given**: Doctor checks show auto-fixable issues
**When**: User runs `zjj doctor --fix --verbose`
**Then**:
- Output shows each fix attempt with details
- Shows "✓ State Database: Deleted corrupted database file"
- Shows "✓ Orphaned Workspaces: Removed 2 orphaned workspace(s)"
- Shows "✓ Stale Sessions: Removed 1 stale session(s)"
- Output includes timing information if verbose

### test_fix_flag_skips_passing_checks
**Given**: Doctor checks show 1 auto-fixable issue + 5 passing checks
**When**: User runs `zjj doctor --fix`
**Then**:
- Only the failing check is fixed
- Passing checks are not mentioned in fix output
- DoctorFixOutput.fixed contains 1 entry
- DoctorFixOutput.unable_to_fix does NOT contain passing checks

### test_fix_flag_reports_failed_fix_attempts
**Given**: Auto-fixable issue but fix function fails (permission denied)
**When**: User runs `zjj doctor --fix`
**Then**:
- Fix function is called (attempt is made)
- Output shows "✗ State Database: Fix failed: permission denied"
- DoctorFixOutput.fixed is empty
- DoctorFixOutput.unable_to_fix contains the issue with reason
- Exit code is 1 (critical issue remains)

### test_fix_flag_exits_zero_when_all_fixed
**Given**: 3 auto-fixable issues, all fixes succeed
**When**: User runs `zjj doctor --fix`
**Then**:
- All fixes attempted and succeed
- DoctorFixOutput.fixed.len() == 3
- DoctorFixOutput.unable_to_fix.is_empty()
- Exit code is 0
- No errors printed

### test_fix_flag_exits_one_when_critical_remain
**Given**: 3 auto-fixable issues, 1 fix succeeds, 2 fail
**When**: User runs `zjj doctor --fix`
**Then**:
- All fixes attempted
- DoctorFixOutput.fixed.len() == 1
- DoctorFixOutput.unable_to_fix.len() == 2
- Exit code is 1
- Error message mentions "2 critical issue(s) remain unfixed"

## Error Path Tests

### test_fix_flag_permission_denied_during_fix
**Given**: State database is corrupted but file is read-only
**When**: User runs `zjj doctor --fix`
**Then**:
- fix_state_database() is called
- Returns Err("permission denied")
- DoctorFixOutput.unable_to_fix contains State Database issue
- Error reason includes "permission denied"
- Suggestion suggests checking file permissions

### test_fix_flag_database_operation_fails
**Given**: Session database is locked/corrupted
**When**: User runs `zjj doctor --fix` for stale sessions
**Then**:
- fix_stale_sessions() is called
- Returns Err("Failed to open DB: database is locked")
- DoctorFixOutput.unable_to_fix contains Stale Sessions issue
- Error suggests retrying or manual cleanup

### test_fix_flag_command_not_found
**Given**: Orphaned workspaces exist but jj command is not available
**When**: User runs `zjj doctor --fix`
**Then**:
- fix_orphaned_workspaces() is called
- Returns Err("jj command not found")
- DoctorFixOutput.unable_to_fix contains Orphaned Workspaces issue
- Suggestion suggests installing jj

### test_fix_flag_partial_fix_with_some_failures
**Given**: 2 auto-fixable issues: database fix succeeds, workspace fix fails
**When**: User runs `zjj doctor --fix`
**Then**:
- Both fix functions are called
- DoctorFixOutput.fixed contains State Database
- DoctorFixOutput.unable_to_fix contains Orphaned Workspaces
- Exit code is 1 (still has failures)
- Output shows both successful and failed fixes

## Edge Case Tests

### test_fix_flag_no_auto_fixable_issues
**Given**: All checks pass or are manual-only (no auto_fixable=true issues)
**When**: User runs `zjj doctor --fix`
**Then**:
- No fix functions are called
- DoctorFixOutput.fixed.is_empty()
- DoctorFixOutput.unable_to_fix may contain manual issues
- Output says "No auto-fixable issues found"
- Exit code is 0 if no critical failures

### test_fix_flag_only_warnings_no_failures
**Given**: Checks show warnings but no failures (auto_fixable=false)
**When**: User runs `zjj doctor --fix`
**Then**:
- No fix functions are called
- Output mentions warnings are not auto-fixed
- DoctorFixOutput.fixed.is_empty()
- Exit code is 0 (warnings don't cause exit 1)

### test_fix_flag_json_output_mode
**Given**: Auto-fixable issues exist
**When**: User runs `zjj doctor --fix --json`
**Then**:
- Output is valid JSON
- Contains DoctorFixOutput structure
- Includes "fixed" array
- Includes "unable_to_fix" array
- No human-readable text in output
- Exit codes follow same rules

### test_fix_flag_json_with_dry_run
**Given**: Auto-fixable issues exist
**When**: User runs `zjj doctor --fix --dry-run --json`
**Then**:
- Output is valid JSON
- Contains "dry_run": true field
- Contains "would_fix" array with descriptions
- No actual fixes performed
- Exit code is 0

### test_fix_flag_concurrent_execution_safety
**Given**: Multiple auto-fixable issues
**When**: User runs `zjj doctor --fix` twice simultaneously
**Then**:
- Second run waits or fails gracefully
- No race conditions in database
- No corruption of fix operations
- Both runs produce consistent results

### test_fix_flag_idempotent_fixes
**Given**: Fix already applied (e.g., database already deleted)
**When**: User runs `zjj doctor --fix` again
**Then**:
- Fix function handles already-fixed state gracefully
- Either: succeeds with "nothing to do" message
- Or: succeeds with idempotent operation (no-op)
- No errors or double-fixing issues

## Contract Verification Tests

### test_precondition_checks_must_exist
**Given**: Fix function is called
**When**: checks slice is empty
**Then**:
- No fix functions are called
- Output says "No checks to fix"
- Exit code is 0

### test_postcondition_auto_fixable_always_attempted
**Given**: Check with auto_fixable=true and status=Fail
**When**: run_fixes() is called
**Then**:
- Corresponding fix function IS called
- Result is recorded in DoctorFixOutput
- Fix is never skipped for auto_fixable=true

### test_postcondition_non_auto_fixable_never_attempted
**Given**: Check with auto_fixable=false and status=Fail
**When**: run_fixes() is called
**Then**:
- NO fix function is called
- Check appears in unable_to_fix if status=Fail
- Check does NOT appear in unable_to_fix if status=Pass/Warn

### test_invariant_fix_results_deterministic
**Given**: Same checks run multiple times
**When**: run_fixes() called with same checks
**Then**:
- Same fixes are attempted each time
- Same results (success/failure)
- DoctorFixOutput is identical across runs
- No randomness or non-determinism

### test_invariant_dry_run_never_modifies_state
**Given**: Any checks with issues
**When**: run_fixes() called with dry_run=true
**Then**:
- Zero filesystem writes
- Zero database modifications
- Zero command executions that modify state
- Only output/print operations occur

### test_invariant_fix_functions_return_result_string
**Given**: Any fix function (fix_orphaned_workspaces, fix_stale_sessions, fix_state_database)
**When**: Fix function is called
**Then**:
- Returns Result<String, String>
- Ok(String) contains success message
- Err(String) contains failure reason
- Never panics, never returns other types

## Given-When-Then Scenarios

### Scenario 1: User runs doctor with fix for first time
**Given**:
- User just cloned zjj repo
- Doctor shows corrupted database
- Doctor shows 2 orphaned workspaces

**When**:
- User runs `zjj doctor --fix`

**Then**:
- Output shows "Attempting to fix auto-fixable issues..."
- Output shows "Fixing State Database..."
- Output shows "✓ State Database: Deleted corrupted database file"
- Output shows "Fixing Orphaned Workspaces..."
- Output shows "✓ Orphaned Workspaces: Removed 2 orphaned workspace(s)"
- Exit code is 0
- Next doctor run shows all checks passing

### Scenario 2: User wants to see what would be fixed
**Given**:
- Doctor shows multiple issues
- User is unsure what --fix will do

**When**:
- User runs `zjj doctor --fix --dry-run`

**Then**:
- Output shows "Dry-run mode: would fix the following:"
- Lists "State Database: Delete corrupted database file"
- Lists "Orphaned Workspaces: Remove 2 orphaned workspace(s)"
- Lists "Stale Sessions: Remove 1 stale session(s)"
- Output says "No changes will be made"
- No files are deleted or modified
- Exit code is 0

### Scenario 3: Fix partially succeeds due to permissions
**Given**:
- Doctor shows corrupted database (user owns it)
- Doctor shows orphaned workspaces (system-owned, can't delete)

**When**:
- User runs `zjj doctor --fix`

**Then**:
- Output shows "Fixing State Database..."
- Output shows "✓ State Database: Deleted corrupted database file"
- Output shows "Fixing Orphaned Workspaces..."
- Output shows "✗ Orphaned Workspaces: Fix failed: permission denied"
- Output shows "Unable to Fix:" section
- Lists Orphaned Workspaces with suggestion
- Exit code is 1 (critical issue remains)
- Error message suggests checking permissions or running manually

### Scenario 4: Doctor with verbose fix output
**Given**:
- Developer is debugging fix behavior
- Doctor shows issues

**When**:
- Developer runs `zjj doctor --fix --verbose`

**Then**:
- Output shows detailed progress
- Shows which fix function is being called
- Shows parameters passed to fix function
- Shows timing information (e.g., "Fix took 0.2s")
- Shows result of each fix attempt
- Easier to debug fix failures

### Scenario 5: JSON output for automation
**Given**:
- CI/CD pipeline wants to auto-fix issues
- Pipeline needs machine-readable output

**When**:
- Pipeline runs `zjj doctor --fix --json`

**Then**:
- Output is valid JSON
- Structure: `{"fixed": [...], "unable_to_fix": [...]}`
- Pipeline can parse and act on results
- Exit code indicates success/failure
- No human-readable text mixed in JSON

## Integration Test: End-to-End Fix Flow

**Test**: `test_e2e_doctor_fix_flow`

**Setup**:
- Create temporary directory
- Initialize JJ repo
- Create corrupted .zjj/state.db file (write invalid data)
- Create orphaned JJ workspace (jj workspace new, delete session DB entry)

**Execute**:
```bash
# Step 1: Run doctor to detect issues
zjj doctor --json > doctor_output.json

# Step 2: Verify issues detected
# - State Database: Fail
# - Orphaned Workspaces: Warn

# Step 3: Run dry-run to see what would be fixed
zjj doctor --fix --dry-run

# Step 4: Verify dry-run output
# - Lists what would be fixed
# - No changes made

# Step 5: Run actual fix
zjj doctor --fix

# Step 6: Verify fix output
# - Shows "Fixed Issues:" section
# - Shows "✓ State Database: Deleted corrupted database file"
# - Shows "✓ Orphaned Workspaces: Removed 1 orphaned workspace(s)"
# - Exit code 0

# Step 7: Run doctor again to verify fixes
zjj doctor

# Step 8: Verify all checks pass
# - State Database: Pass
# - Orphaned Workspaces: Pass
```

**Verify**:
- Dry-run doesn't modify state
- Fix actually resolves issues
- Output is clear and informative
- Exit codes are correct
- No unwrap/expect in production code
- All errors are Result<T, Error>

## Regression Tests

### test_existing_fix_functions_still_work
**Given**: Existing code uses fix_orphaned_workspaces, fix_stale_sessions, fix_state_database
**When**: These functions are called with dry_run=false
**Then**:
- Functions work as before
- Return types unchanged
- No breaking changes to signatures (only added optional dry_run param)

### test_fix_flag_without_dry_run_still_works
**Given**: User runs `zjj doctor --fix` (no dry-run flag)
**When**: Fix logic executes
**Then**:
- Behavior matches current implementation
- Fixes are actually applied
- No changes to core fix logic
- Only output improvements

### test_json_output_structure_unchanged
**Given**: Existing code parses DoctorFixOutput JSON
**When**: User runs `zjj doctor --fix --json`
**Then**:
- JSON structure is backward compatible
- "fixed" array still exists
- "unable_to_fix" array still exists
- No breaking schema changes

### test_exit_codes_unchanged
**Given**: Existing scripts check exit codes
**When**: User runs `zjj doctor --fix`
**Then**:
- Exit 0 if all critical issues fixed
- Exit 1 if critical issues remain
- No new exit codes introduced
- Backward compatible behavior
