# Contract Specification: bd-1z6 - Delete interactive confirmation from remove command

**Bead ID:** bd-1z6
**Title:** Delete interactive confirmation from remove command
**Status:** Design Contract
**Version:** 1.0

## Overview

This contract defines the behavior changes for removing interactive confirmation from the `zjj remove` command. The command currently prompts for user confirmation before removing a session unless `--force` is specified. This change makes the command **non-interactive by default**, removing the confirmation prompt entirely.

### Key Behavioral Change

**Before:** `zjj remove <session>` prompts for confirmation ("Remove session 'X' and its workspace? [y/N]")
**After:** `zjj remove <session>` removes immediately without prompting

The `--force` flag remains supported for backwards compatibility but becomes a no-op (it already skipped confirmation).

## Function Signature

```rust
pub async fn run_with_options(name: &str, options: &RemoveOptions) -> Result<()>
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

2. **Session Name Valid**
   - `name` must be a non-empty string
   - `name` must match pattern: `^[a-zA-Z][a-zA-Z0-9_-]*$`
   - **Violation:** Returns `Error::ValidationError` (exit code 1)

3. **JJ Repository Available**
   - Must be inside a JJ repository (or `--idempotent` with missing session)
   - JJ binary must be available in PATH (if `--merge` or jj_forget is used)
   - **Violation:** Returns `Error::JjCommandError` (exit code 4)

### Mode-Specific Preconditions

4. **Non-Idempotent Mode (`!options.idempotent`)**
   - Session MUST exist in database
   - **Violation:** Returns `Error::NotFound` (exit code 2)

5. **Idempotent Mode (`options.idempotent == true`)**
   - Session MAY exist or not exist (both states valid)
   - If session doesn't exist, returns success with "already removed" message

6. **Dry Run Mode (`options.dry_run == true`)**
   - Session MUST exist in database (even with `--idempotent`)
   - **Violation:** Returns `Error::NotFound` (exit code 2)

7. **Merge Mode (`options.merge == true`)**
   - JJ repository must have a `main` branch
   - Session's JJ workspace must be mergeable to main
   - **Violation:** Returns `Error::JjCommandError` (exit code 4)

## Postconditions

### Success Postconditions (MUST hold after successful execution)

1. **Session Removed from Database**
   - Session record is deleted from session database
   - `db.get(name).await` returns `None`

2. **Workspace Directory Deleted**
   - Workspace directory at `session.workspace_path` is removed
   - `Path::new(&session.workspace_path).exists()` returns `false`

3. **JJ Workspace Forgotten** (if jj_forget enabled)
   - `jj workspace list` no longer includes the session's workspace
   - Or returns "not found" error (idempotent success)

4. **Zellij Tab Closed** (if running inside Zellij)
   - Zellij tab with name `session.zellij_tab` is closed
   - **Non-critical:** Failure doesn't cause operation failure (warning only)

5. **Output Produced**
   - Human-readable message printed to stdout
   - OR JSON output if `options.format.is_json()`
   - Output includes session name and status message

### Special Case Postconditions

6. **Dry Run Mode Success**
   - NO changes made to database or filesystem
   - Output prefixed with "DRY-RUN:"
   - Exit code 0

7. **Idempotent Mode - Session Already Missing**
   - NO changes made (nothing to do)
   - Output message: "Session 'X' already removed"
   - Exit code 0

8. **Idempotent Mode - Workspace Already Gone**
   - Database record deleted
   - NO filesystem operation (workspace already absent)
   - Output message includes "(workspace was already gone)"
   - Exit code 0

9. **Merge Mode Success**
   - Session's changes squash-merged to `main` branch
   - JJ workspace forgotten before deletion
   - Then normal removal proceeds

## Invariants

### Always True (during and after execution)

1. **Atomic Cleanup Guarantee**
   - If workspace deletion fails, session marked as `"removal_failed"` in database
   - If database deletion fails (after workspace deleted), critical error logged
   - **Never:** Orphaned workspaces without database records (Type 2 orphans prevented)

2. **Idempotent ENOENT Handling**
   - `tokio::fs::remove_dir_all` with `NotFound` error succeeds (continues cleanup)
   - JJ forget with "no such workspace" error succeeds (continues cleanup)
   - **Never:** Failures due to concurrent removal by another process

3. **Force Flag No-Op**
   - `options.force` has NO behavioral effect (confirmation removed)
   - Flag accepted for backwards compatibility
   - **Always:** Removal proceeds immediately whether force=true or force=false

4. **No Interactive Prompting**
   - Function NEVER reads from stdin
   - NO "y/N" prompt printed
   - **Always:** Executes immediately or returns error

5. **Output Format Consistency**
   - JSON output always wrapped in `SchemaEnvelope`
   - `$schema` field follows pattern: `zjj://remove/v1`
   - `schema_type` field is `"single"`
   - **Always:** Valid JSON when `--json` flag present

## Error Taxonomy

### Exhaustive Error Variants (with semantic exit codes)

```rust
pub enum RemoveError {
    // === Exit Code 2: Not Found Errors ===

    /// Session not found in database (non-idempotent mode)
    SessionNotFound {
        session: String,
        hint: String,  // "Use --idempotent to succeed when session is missing"
    },

    // === Exit Code 3: System Errors ===

    /// Workspace path invalid or inaccessible
    WorkspaceInaccessible {
        path: String,
        reason: String,  // "Directory does not exist", "Permission denied", etc.
    },

    /// Workspace directory removal failed
    WorkspaceRemovalFailed {
        path: String,
        source: std::io::Error,
        recovery: String,  // "Session marked as 'removal_failed' in database"
    },

    /// Database deletion failed (after workspace deleted - critical)
    DatabaseDeletionFailed {
        name: String,
        source: zjj_core::Error,
        severity: String,  // "CRITICAL: Workspace deleted, manual cleanup required"
    },

    // === Exit Code 4: External Command Errors ===

    /// JJ command failed (merge, forget, etc.)
    JjCommandError {
        operation: String,  // "squash", "workspace forget", etc.
        source: String,
        is_not_found: bool,  // true if JJ not installed
        hint: String,  // Installation or troubleshooting guidance
    },

    /// Zellij tab closure failed (non-critical, logged as warning)
    ZellijTabCloseFailed {
        tab: String,
        source: anyhow::Error,
        severity: String,  // "WARNING: Non-critical, cleanup continues"
    },
}
```

### Error Propagation Mapping

```rust
// Database layer errors
impl From<zjj_core::Error> for RemoveError {
    fn from(err: zjj_core::Error) -> Self {
        match err {
            Error::NotFound(msg) => RemoveError::SessionNotFound {
                session: msg,
                hint: "Use --idempotent to succeed when session is missing".into(),
            },
            Error::DatabaseError(msg) => RemoveError::DatabaseDeletionFailed {
                name: "<session>".into(),
                source: err,
                severity: "CRITICAL: Manual cleanup required".into(),
            },
            Error::IoError(msg) => RemoveError::WorkspaceInaccessible {
                path: "<path>".into(),
                reason: msg,
            },
            _ => RemoveError::JjCommandError { /* ... */ },
        }
    }
}
```

## Function Signatures (All Fallible Operations)

### Core Removal Function

```rust
/// Remove a session atomically with comprehensive error handling
///
/// # Arguments
/// * `name` - Session name to remove
/// * `options` - Removal options (force, merge, idempotent, dry_run, format)
///
/// # Returns
/// * `Ok(())` - Session removed successfully
/// * `Err(Error::NotFound)` - Session not found (non-idempotent mode)
/// * `Err(Error::IoError)` - Filesystem operation failed
/// * `Err(Error::DatabaseError)` - Database operation failed
/// * `Err(Error::JjCommandError)` - JJ command failed
pub async fn run_with_options(
    name: &str,
    options: &RemoveOptions,
) -> Result<()> {
    // Implementation...
}

/// Result of atomic cleanup operation
#[derive(Debug, Clone)]
pub struct RemoveResult {
    /// Whether the session was actually removed
    pub removed: bool,
}

/// Atomic session cleanup with idempotent error handling
///
/// # Errors
/// * `RemoveError::WorkspaceInaccessible` - Workspace path invalid
/// * `RemoveError::WorkspaceRemovalFailed` - Deletion failed
/// * `RemoveError::DatabaseDeletionFailed` - DB delete failed (critical)
/// * `RemoveError::ZellijTabCloseFailed` - Non-critical, logged only
pub async fn cleanup_session_atomically(
    db: &SessionDb,
    session: &Session,
    jj_forget: bool,
) -> Result<RemoveResult, RemoveError> {
    // Implementation...
}
```

### Validation Functions

```rust
/// Validate session name format
///
/// # Returns
/// * `Ok(())` - Name is valid
/// * `Err(Error::ValidationError)` - Name invalid
fn validate_session_name(name: &str) -> Result<()> {
    // Implementation...
}

/// Check if session exists (for non-idempotent mode)
///
/// # Returns
/// * `Ok(true)` - Session exists
/// * `Ok(false)` - Session doesn't exist
/// * `Err(Error::DatabaseError)` - Database query failed
async fn session_exists(db: &SessionDb, name: &str) -> Result<bool> {
    // Implementation...
}
```

### Helper Functions (all Result-returning)

```rust
/// Squash-merge session changes to main branch
///
/// # Errors
/// * `Error::JjCommandError` - JJ squash failed
fn merge_to_main(name: &str, workspace_path: &str) -> Result<()> {
    // Implementation...
}

/// Close Zellij tab (non-critical operation)
///
/// # Returns
/// * `Ok(())` - Tab closed successfully
/// * `Err(RemoveError::ZellijTabCloseFailed)` - Failed to close (warning only)
async fn close_zellij_tab(tab_name: &str) -> Result<(), RemoveError> {
    // Implementation...
}

/// Run JJ workspace forget command with idempotent error handling
///
/// # Returns
/// * `Ok(())` - Workspace forgotten or already absent
/// * `Err(RemoveError::WorkspaceInaccessible)` - Real error (not "not found")
async fn jj_workspace_forget(name: &str) -> Result<(), RemoveError> {
    // Implementation...
}
```

## Behavioral Change Summary

### Removed Behavior

1. **No Confirmation Prompt**
   - Function `confirm_removal()` should be **removed entirely**
   - NO stdin reading
   - NO "y/N" prompt
   - Code path at lines 93-107 in `/home/lewis/src/zjj/crates/zjj/src/commands/remove.rs` deleted

2. **Force Flag Becomes No-Op**
   - `--force` flag still accepted (backwards compatibility)
   - Condition `if !options.force` becomes **always true**
   - Pre_remove hooks still skipped when `--force` is set (retained behavior)

### Retained Behavior

1. **All other flags work identically**
   - `--merge`: Still squash-merges to main before removal
   - `--idempotent`: Still succeeds when session already missing
   - `--dry-run`: Still previews without executing
   - `--json`: Still outputs JSON

2. **Error handling unchanged**
   - Same error taxonomy
   - Same exit codes
   - Same recovery hints

3. **Atomic cleanup guarantees retained**
   - Workspace deletion before database deletion
   - Idempotent ENOENT handling
   - Orphan prevention

## Migration Notes

### Breaking Changes

1. **Scripts relying on confirmation prompt will break**
   - Automated scripts that expected "y/N" prompt will now execute immediately
   - Scripts that auto-answered "yes" will work unchanged (confirmation removed)
   - Scripts that auto-answered "no" will **break** (removal now proceeds)

### Backwards Compatibility

1. **`--force` flag retained as no-op**
   - Existing invocations with `-f` or `--force` continue to work
   - Flag has no effect (already skipped confirmation)

2. **All other flags unchanged**
   - `--merge`, `--idempotent`, `--dry-run`, `--json` work identically

### Testing Strategy

See `/home/lewis/src/zjj/contracts/bd-1z6-martin-fowler-tests.md` for comprehensive test plan covering:

- Happy path: Non-interactive removal succeeds
- Error path: Each failure mode tested
- Edge cases: Idempotent, dry-run, merge modes
- Contract verification: Precondition/postcondition validation

---

**Contract Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
