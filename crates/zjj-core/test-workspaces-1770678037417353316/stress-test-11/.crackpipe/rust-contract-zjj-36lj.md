# Contract Specification: Fix `--fix` Flag Misleading Behavior

## Context

- **Feature**: Fix `zjj doctor --fix` command to actually fix issues instead of just reporting them
- **Domain Terms**:
  - `auto_fixable`: Boolean flag on DoctorCheck indicating if issue can be automatically fixed
  - `run_fixes()`: Function that iterates checks and attempts fixes for auto_fixable issues
  - `DoctorFixOutput`: Struct containing fixed issues and unable_to_fix issues
  - `FixResult`: Record of attempted fix with issue name, action taken, and success status
  - `UnfixableIssue`: Issue that couldn't be fixed with reason and suggestion
- **Assumptions**:
  - Current implementation already has fix functions: `fix_orphaned_workspaces()`, `fix_stale_sessions()`, `fix_state_database()`
  - These functions ARE being called when `--fix` is used
  - The bug is that the flag name is misleading because most checks are NOT auto_fixable
  - Users expect `--fix` to attempt fixing everything possible, but it only fixes 3 specific issues
- **Open Questions**:
  1. Should we rename the flag to `--auto-fix` or keep `--fix` and make it more comprehensive?
  2. Should we add more auto-fixable checks? (Which ones?)
  3. Should we add a `--dry-run` flag to preview what would be fixed?
  4. Should we add verbose output showing what fix is being attempted?
  5. What should happen when a fix partially succeeds?

**Decision**: Keep `--fix` flag name but:
- Add verbose output showing what's being fixed
- Add dry-run mode with `--dry-run` or `--what-if`
- Document clearly which checks are auto-fixable
- Add more auto-fixable checks where safe (e.g., create missing directories)

## Current Behavior (BUG)

```rust
// Current flow:
run(format, fix=true) -> run_fixes() -> only fixes issues with auto_fixable=true

// Problem:
// - Only 3 of 13 checks are auto_fixable (State Database, Orphaned Workspaces, Stale Sessions)
// - 10 checks just get reported in "unable_to_fix" list
// - User sees "✗ JJ Installation" with "Requires manual intervention" after running --fix
// - This is misleading: user expected automatic fixing
```

## Desired Behavior (FIX)

```rust
// New flow with --fix:
run(format, fix=true) -> run_fixes() -> {
  1. Print "Attempting to fix auto-fixable issues..."
  2. For each check with auto_fixable=true:
     - Print "Fixing {check.name}..."
     - Attempt fix
     - Print result
  3. For each check with auto_fixable=false:
     - Skip (not print in "unable_to_fix" if status=Pass/Warn)
     - Only print in "unable_to_fix" if status=Fail
  4. Show summary of fixes
}

// With --dry-run:
run(format, fix=true, dry_run=true) -> {
  1. Print "Dry-run mode: would fix the following:"
  2. List what would be fixed (no actual changes)
  3. Exit 0
}
```

## Preconditions

- `format` must be valid (OutputFormat::Json or human-readable)
- `fix` flag must be true
- Checks must have been run first (via `run_all_checks()`)
- For fix functions:
  - Filesystem operations must have required permissions
  - Commands (jj, zellij) must be available if needed
  - Database must be accessible for session operations

## Postconditions

After `run_fixes()` completes:
- All `auto_fixable=true` checks with `status=Fail` have been attempted
- `DoctorFixOutput.fixed` contains all successful fixes
- `DoctorFixOutput.unable_to_fix` contains:
  - Checks with `auto_fixable=false` AND `status=Fail` (manual intervention required)
  - Checks with `auto_fixable=true` BUT fix attempt failed
- If dry-run: no filesystem changes made
- Exit code 0 if all critical issues fixed or none existed
- Exit code 1 if critical issues remain unfixed

## Invariants

- `auto_fixable=true` checks are ALWAYS attempted when `fix=true` (never skipped)
- `auto_fixable=false` checks are NEVER attempted (only reported)
- Fix functions return `Result<String, String>` (Ok=message, Err=reason)
- Original checks data is never modified (fixes operate on system state)
- Fix results are deterministic (same checks → same fixes)

## Error Taxonomy

- `Error::FixFailed(String)` - Fix attempt failed with specific reason
- `Error::PermissionDenied` - Fix requires elevated permissions
- `Error::FixNotAvailable` - No fix implementation exists for this check
- `Error::DryRunViolation` - Attempted to modify state in dry-run mode

## Contract Signatures

### Modified Functions

```rust
/// Run auto-fixes for detected issues.
///
/// # Behavior
/// - Iterates all checks
/// - Attempts fix for each check with auto_fixable=true
/// - Reports unable_to_fix for:
///   - auto_fixable=false AND status=Fail (manual intervention)
///   - auto_fixable=true BUT fix failed
///
/// # Returns
/// - Ok(DoctorFixOutput) with fixes attempted
/// - Err(anyhow::Error) if critical issues remain (exit code 1)
///
/// # Exit Codes
/// - 0: All critical issues fixed or none existed
/// - 1: Critical issues remain unfixed
async fn run_fixes(
    checks: &[DoctorCheck],
    format: OutputFormat,
    dry_run: bool,  // NEW: dry-run mode
    verbose: bool,  // NEW: verbose output
) -> Result<()>;

/// Fix orphaned workspaces.
///
/// # Returns
/// - Ok(String) with message describing what was fixed
/// - Err(String) with reason if fix failed
async fn fix_orphaned_workspaces(
    check: &DoctorCheck,
    dry_run: bool,  // NEW: dry-run mode
) -> Result<String, String>;

/// Fix stale sessions.
///
/// # Returns
/// - Ok(String) with message describing what was fixed
/// - Err(String) with reason if fix failed
async fn fix_stale_sessions(
    check: &DoctorCheck,
    dry_run: bool,  // NEW: dry-run mode
) -> Result<String, String>;

/// Fix state database.
///
/// # Returns
/// - Ok(String) with message describing what was fixed
/// - Err(String) with reason if fix failed
async fn fix_state_database(
    check: &DoctorCheck,
    dry_run: bool,  // NEW: dry-run mode
) -> Result<String, String>;
```

### New Functions

```rust
/// Print what fixes would be attempted (dry-run mode).
///
/// # Returns
/// - Ok(()) after printing dry-run report
/// - Err(Error::DryRunViolation) if fix would modify state
fn show_dry_run_report(checks: &[DoctorCheck]) -> Result<()>;

/// Check if a fix is available for a given check.
///
/// # Returns
/// - true if fix function exists and check has auto_fixable=true
/// - false otherwise
fn has_fix_available(check: &DoctorCheck) -> bool;

/// Get human-readable description of what fix would do.
///
/// # Returns
/// - Some(String) describing the fix
/// - None if no fix available
fn describe_fix(check: &DoctorCheck) -> Option<String>;
```

## Non-goals

- Fixing issues that require user input or decisions (remain manual)
- Fixing issues that require external tools installation (e.g., install JJ)
- Automatic fixes without user consent (always requires --fix flag explicitly)
- Parallel fix execution (fixes run sequentially for safety)
- Undo/rollback of fixes (not supported)
- Fixing issues that are warnings (only fixes failures)
