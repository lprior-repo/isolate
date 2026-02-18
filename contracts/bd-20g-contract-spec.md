# Contract Specification: bd-20g - Delete interactive confirmation from clean command

**Bead ID:** bd-20g
**Title:** Delete interactive confirmation from clean command
**Status:** Design Contract
**Version:** 1.0

## Overview

This contract defines the behavior changes for removing interactive confirmation from the `zjj clean` command. The command currently prompts for user confirmation before removing stale sessions unless `--force` is specified. This change makes the command **non-interactive by default**, removing the confirmation prompt entirely.

### Key Behavioral Change

**Before:** `zjj clean` prompts for confirmation ("Remove these sessions? [y/N]") when stale sessions are found
**After:** `zjj clean` removes stale sessions immediately without prompting

The `--force` flag remains supported for backwards compatibility but becomes a no-op (it already skipped confirmation).

## Function Signature

```rust
pub async fn run_with_options(options: &CleanOptions) -> Result<()>
```

### Return Type

```rust
Result<()> // Success or Error::*
```

## Preconditions

### Global Preconditions (MUST hold before execution)

1. **Session Database Accessible**
   - Session database must be initialized (`zjj init` has been run)
   - Database file must be readable and writable
   - **Violation:** Returns `Error::DatabaseError` (exit code 3)

2. **Not in Periodic Mode**
   - `options.periodic` must be `false` (periodic mode has different behavior)
   - **Violation:** Returns `Result::Ok(())` after running periodic daemon

### Mode-Specific Preconditions

3. **Dry Run Mode (`options.dry_run == true`)**
   - No changes made to filesystem or database
   - Output indicates dry-run mode
   - Exit code 0

4. **Force Mode (`options.force == true`)**
   - **NO behavioral effect** (confirmation removed)
   - Flag accepted for backwards compatibility only
   - Proceeds with cleanup immediately

## Postconditions

### Success Postconditions (MUST hold after successful execution)

1. **Stale Sessions Identified**
   - All sessions with missing workspace directories are identified
   - Sessions are filtered by checking `tokio::fs::try_exists(&session.workspace_path)`

2. **Stale Sessions Removed from Database**
   - All stale session records are deleted from session database
   - `db.get(&session.name).await` returns `None` for each stale session

3. **Output Produced**
   - Human-readable message printed to stdout
   - OR JSON output if `options.format.is_json()`
   - Output includes count of stale sessions and list of names

4. **No Workspace Deletion**
   - Clean command only removes database records
   - Does NOT delete workspace directories (they're already missing)
   - This differs from `remove` command which deletes workspaces

### Special Case Postconditions

5. **No Stale Sessions Found**
   - Output message: "No stale sessions found"
   - Exit code 0
   - No database changes

6. **Dry Run Mode Success**
   - NO changes made to database
   - Output prefixed with "Found X stale session(s) (dry-run, no changes made):"
   - Exit code 0
   - Lists stale sessions that would be removed

7. **Periodic Mode**
   - Runs background daemon indefinitely
   - Checks for stale sessions every 1 hour
   - Uses age threshold (default 2 hours)
   - **Not affected by this change** (already non-interactive)

## Invariants

### Always True (during and after execution)

1. **No Interactive Prompting**
   - Function NEVER reads from stdin
   - NO "y/N" prompt printed to stderr
   - **Always:** Executes cleanup immediately or returns error
   - **Deleted:** The `confirm_removal()` function (lines 204-222)

2. **Force Flag No-Op**
   - `options.force` has NO behavioral effect (confirmation removed)
   - Flag accepted for backwards compatibility
   - **Always:** Cleanup proceeds immediately whether force=true or force=false

3. **Output Format Consistency**
   - JSON output always wrapped in `SchemaEnvelope`
   - `$schema` field follows pattern: `zjj://clean/v1`
   - `schema_type` field is `"single"`
   - **Always:** Valid JSON when `--json` flag present

4. **Atomic Database Operations**
   - Each session deletion is independent
   - Failure to delete one session doesn't prevent deletion of others
   - Errors during deletion are accumulated

5. **Stale Detection Consistency**
   - Stale = workspace directory doesn't exist
   - Detection uses `tokio::fs::try_exists()` which handles errors
   - **Never:** A session with existing workspace is removed

## Error Taxonomy

### Exhaustive Error Variants (with semantic exit codes)

```rust
pub enum CleanError {
    // === Exit Code 3: System Errors ===

    /// Database operation failed
    DatabaseError {
        operation: String,  // "list", "delete", etc.
        source: zjj_core::Error,
        recovery: String,  // "Check database permissions"
    },

    /// Workspace path verification failed
    WorkspaceVerificationError {
        session: String,
        path: String,
        reason: String,  // "Permission denied", "Not a directory", etc.
    },

    /// Session listing failed
    SessionListError {
        source: zjj_core::Error,
        hint: String,  // "Check database integrity"
    },
}
```

### Error Propagation Mapping

```rust
// Database layer errors
impl From<zjj_core::Error> for CleanError {
    fn from(err: zjj_core::Error) -> Self {
        match err {
            Error::DatabaseError(msg) => CleanError::DatabaseError {
                operation: "unknown".into(),
                source: err,
                recovery: "Check database file permissions".into(),
            },
            Error::IoError(msg) => CleanError::WorkspaceVerificationError {
                session: "<unknown>".into(),
                path: "<unknown>".into(),
                reason: msg,
            },
            _ => CleanError::DatabaseError {
                operation: "unknown".into(),
                source: err,
                recovery: "Check system state".into(),
            },
        }
    }
}
```

## Function Signatures (All Fallible Operations)

### Core Clean Function

```rust
/// Run clean command with options
///
/// # Arguments
/// * `options` - Clean options (force, dry_run, format, periodic, age_threshold)
///
/// # Returns
/// * `Ok(())` - Clean completed successfully
/// * `Err(CleanError::DatabaseError)` - Database operation failed
/// * `Err(CleanError::WorkspaceVerificationError)` - Workspace check failed
/// * `Err(CleanError::SessionListError)` - Listing sessions failed
pub async fn run_with_options(
    options: &CleanOptions,
) -> Result<()> {
    // Implementation...
}

/// Options for the clean command
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// Skip confirmation prompt (NO-OP after this change)
    pub force: bool,
    /// List stale sessions without removing
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
    /// Run periodic cleanup daemon (1hr interval)
    pub periodic: bool,
    /// Age threshold for periodic cleanup (seconds, default 7200 = 2hr)
    pub age_threshold: Option<u64>,
}

/// Output for clean command in JSON mode
#[derive(Debug, Clone, Serialize)]
pub struct CleanOutput {
    pub stale_count: usize,
    pub removed_count: usize,
    pub stale_sessions: Vec<String>,
}
```

### Helper Functions (all Result-returning)

```rust
/// Check if workspace directory exists
///
/// # Returns
/// * `Ok(true)` - Directory exists
/// * `Ok(false)` - Directory doesn't exist (stale session)
/// * `Err(CleanError::WorkspaceVerificationError)` - Real error (permission denied)
async fn verify_workspace_exists(
    path: &str,
    session_name: &str,
) -> Result<bool, CleanError> {
    // Implementation...
}

/// Delete stale session from database
///
/// # Errors
/// * `CleanError::DatabaseError` - Deletion failed
async fn delete_stale_session(
    db: &SessionDb,
    session_name: &str,
) -> Result<bool, CleanError> {
    // Implementation...
}
```

## Behavioral Change Summary

### Removed Behavior

1. **No Confirmation Prompt**
   - Function `confirm_removal()` should be **removed entirely**
   - NO stdin reading
   - NO "y/N" prompt printed to stderr
   - Code path at lines 87-91 in `/home/lewis/src/zjj/crates/zjj/src/commands/clean/mod.rs` deleted
   - Condition `if !options.force && !confirm_removal(&stale_names)?` removed

2. **Force Flag Becomes No-Op**
   - `--force` flag still accepted (backwards compatibility)
   - Condition `if !options.force` becomes **irrelevant**
   - Cleanup always proceeds immediately

3. **Output Cancelled Function Obsolete**
   - Function `output_cancelled()` (lines 158-173) becomes **dead code**
   - Can be removed or kept for backwards compatibility with JSON output format

### Retained Behavior

1. **All other flags work identically**
   - `--dry-run`: Still previews without executing
   - `--json`: Still outputs JSON with SchemaEnvelope
   - `--periodic`: Still runs background daemon
   - `--age-threshold`: Still configures periodic cleanup age

2. **Error handling unchanged**
   - Same error taxonomy
   - Same exit codes
   - Same recovery hints

3. **Stale detection logic retained**
   - Workspace existence check using `tokio::fs::try_exists()`
   - Functional stream-based filtering
   - Atomic database deletions

4. **Periodic mode unchanged**
   - Already non-interactive
   - Not affected by this change

## Migration Notes

### Breaking Changes

1. **Scripts relying on confirmation prompt will break**
   - Automated scripts that expected "y/N" prompt will now execute cleanup immediately
   - Scripts that auto-answered "yes" will work unchanged (confirmation removed)
   - Scripts that auto-answered "no" will **break** (cleanup now proceeds)

2. **Use --dry-run for preview**
   - Scripts that need to preview before cleanup should use `--dry-run` flag
   - Dry-run mode lists stale sessions without removing them

### Backwards Compatibility

1. **`--force` flag retained as no-op**
   - Existing invocations with `-f` or `--force` continue to work
   - Flag has no effect (already skipped confirmation)

2. **All other flags unchanged**
   - `--dry-run`, `--json`, `--periodic`, `--age-threshold` work identically

### Testing Strategy

See `/home/lewis/src/zjj/contracts/bd-20g-martin-fowler-tests.md` for comprehensive test plan covering:

- Happy path: Non-interactive cleanup succeeds
- Error path: Each failure mode tested
- Edge cases: Dry-run, periodic modes, empty stale list
- Contract verification: Precondition/postcondition validation
- Invariant verification: No stdin reading, force flag no-op

## Code Changes Required

### File: `/home/lewis/src/zjj/crates/zjj/src/commands/clean/mod.rs`

**Delete lines 87-91:**
```rust
// DELETED:
// if !options.force && !confirm_removal(&stale_names)? {
//     output_cancelled(&stale_names, options.format);
//     return Ok(());
// }
```

**Delete lines 199-222 (confirm_removal function):**
```rust
// DELETED ENTIRE FUNCTION:
// fn confirm_removal(stale_names: &[String]) -> Result<bool> {
//     ...
// }
```

**Optional: Delete lines 158-173 (output_cancelled function):**
```rust
// OPTIONAL - Dead code after change:
// fn output_cancelled(stale_names: &[String], format: OutputFormat) {
//     ...
// }
```

**Update line 43 documentation:**
```rust
// BEFORE: "4. Handle dry-run or interactive confirmation"
// AFTER: "4. Handle dry-run mode"
```

**No other code changes required.**

---

**Contract Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
