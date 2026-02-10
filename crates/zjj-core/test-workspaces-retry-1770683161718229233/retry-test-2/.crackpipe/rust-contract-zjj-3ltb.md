# Contract Specification: Fix Session-Workspace Desynchronization (zjj-3ltb)

## Context
- **Feature**: Fix database bug causing orphaned workspaces
- **Bead**: zjj-3ltb - "database: Fix session-workspace desynchronization"
- **Issue Type**: Bug fix (data integrity, resource leak)
- **Severity**: Priority 3 (high)
- **Estimated Time**: 120 minutes

## Problem Statement
**Root Cause**: Session records are deleted from the database without cleaning up their corresponding workspace directories. This creates orphaned workspaces that accumulate over time, causing:
- Resource leaks (disk space)
- Inconsistent state between database and filesystem
- User confusion (workspace exists but no session record)

**Discovery**: Found by Agent #5 during code review

## Domain Terms
- **Session**: A record in the database tracking a workspace (session_name, workspace_path, status)
- **Workspace**: A JJ repository directory on disk (typically `~/.local/share/zjj/workspaces/<name>`)
- **Orphaned Workspace**: A workspace directory with no corresponding session record
- **Orphaned Session**: A session record with no corresponding workspace directory
- **Type 1 Orphan**: Session exists, workspace deleted (external deletion)
- **Type 2 Orphan**: Workspace exists, session deleted (bug - this is what we're fixing)
- **Atomic Operation**: Both database record AND filesystem directory succeed/fail together

## Assumptions
1. Database is SQLite (via `bead-kv` crate)
2. Workspaces are stored in `~/.local/share/zjj/workspaces/`
3. Session records have a `workspace_path` field
4. Filesystem operations can fail (permissions, locks, disk full)
5. Database operations can fail (locked, corrupted, I/O error)
6. JJ workspace forget uses `jj workspace forget` command
7. Zellij tab closure uses `zellij action close-pane` or similar

## Open Questions
1. **Should removal be atomic?** YES - either both delete or neither deletes
2. **What order for deletion?** Workspace first, then database (safer if DB delete fails)
3. **Should we track removal status?** YES - add `removal_status` field to sessions
4. **How to handle partial failures?** Mark session as `removal_failed`, retry later
5. **Should doctor command auto-cleanup?** YES - with `--cleanup-orphaned` flag

## Preconditions

### For `remove_session(name: &str)`:
- Session record must exist in database
- Session must not be currently active (not in use)
- Workspace path must be valid and accessible
- JJ workspace must be forgotten first (separate operation)
- Zellij tab must be closed first (separate operation)

### For `find_orphaned_workspaces()`:
- Database must be accessible
- Workspaces directory must be readable
- No concurrent removal operations running

### For `cleanup_orphaned_sessions()`:
- User confirmation required (unless `--force` flag)
- No active sessions using the workspaces

## Postconditions

### For `remove_session(name: &str)`:
**Success Path**:
- Session record deleted from database
- Workspace directory deleted from filesystem
- No orphaned resources created
- Returns `Ok(RemoveOutput { removed: true, cleanup_count: 0 })`

**Failure Path** (any operation fails):
- Session record marked with `removal_status: "failed"`
- Error message describes what failed and why
- Workspace directory state depends on failure point:
  - Workspace deletion failed: workspace still exists
  - Database deletion failed: workspace deleted, session still exists
- Returns `Err(RemoveError)` with appropriate variant

### For `find_orphaned_workspaces()`:
- Returns list of session names with missing workspaces (Type 1)
- Returns list of workspace paths with missing sessions (Type 2)
- No state changes (read-only operation)

### For `cleanup_orphaned_sessions()`:
- Type 1 orphans: Session records deleted
- Type 2 orphans: Workspace directories deleted
- Returns count of cleaned resources
- Doctor log updated with cleanup details

## Invariants

### Database Invariants:
1. **Session-Workspace Consistency**: Every session record must have a corresponding workspace directory
2. **No Orphaned Sessions**: If workspace deleted, session must be deleted
3. **No Orphaned Workspaces**: If session deleted, workspace must be deleted
4. **Removal Status Tracking**: Failed removals must be marked

### Filesystem Invariants:
1. **Workspace Path Validity**: All workspace paths must be absolute and under `~/.local/share/zjj/workspaces/`
2. **Workspace Directory Contents**: Each workspace must be a valid JJ repository (contains `.jj` directory)

### Operation Invariants:
1. **Atomic Removal**: `remove_session()` either succeeds completely or fails gracefully
2. **Error Recovery**: Failed removals can be retried after fixing underlying issue
3. **No Data Loss**: User data in workspaces must not be deleted without explicit action

## Error Taxonomy

### RemoveError (New Enum)
```rust
pub enum RemoveError {
    /// Session not found in database
    SessionNotFound {
        session_name: String,
    },

    /// Session is currently active (in use)
    SessionIsActive {
        session_name: String,
        current_status: String,
    },

    /// Workspace directory not found
    WorkspaceNotFound {
        workspace_path: PathBuf,
    },

    /// Workspace deletion failed (permissions, locks, etc.)
    WorkspaceDeletionFailed {
        workspace_path: PathBuf,
        reason: String,
    },

    /// Database operation failed
    DatabaseError {
        operation: String,
        reason: String,
    },

    /// JJ workspace forget failed
    JjForgetFailed {
        session_name: String,
        reason: String,
    },

    /// Zellij tab closure failed
    ZellijCloseFailed {
        tab_name: String,
        reason: String,
    },

    /// Concurrent modification detected
    ConcurrentModification {
        session_name: String,
    },

    /// Invalid workspace path (outside allowed directory)
    InvalidWorkspacePath {
        workspace_path: PathBuf,
    },
}
```

### Error Handling Strategy
- **Zero Panic**: All errors return `Result<T, RemoveError>`
- **Error Recovery**: Mark failed removals in database for retry
- **User Communication**: Clear error messages explain what failed and how to fix

## Contract Signatures

### Core Removal Function
```rust
/// Remove a session and its workspace atomically
///
/// # Contract
/// - Precondition: Session exists and is not active
/// - Postcondition: Session and workspace both deleted, OR error with state preserved
/// - Invariant: No orphaned resources created
pub async fn remove_session(
    session_name: &str,
    options: &RemoveOptions,
) -> Result<RemoveOutput, RemoveError>
{
    // Implementation in builder phase
}
```

### Orphan Detection Functions
```rust
/// Find all orphaned workspaces (Type 1 and Type 2)
///
/// # Contract
/// - Precondition: Database and filesystem accessible
/// - Postcondition: Returns all orphans without modifying state
/// - Invariant: Read-only operation
pub async fn find_orphaned_workspaces(
    db: &SessionDb,
) -> Result<OrphanReport, RemoveError>
{
    // Implementation in builder phase
}

/// Clean up orphaned resources
///
/// # Contract
/// - Precondition: User confirmed (or --force flag)
/// - Postcondition: No orphans remain
/// - Invariant: Only orphans are deleted, active sessions untouched
pub async fn cleanup_orphaned_sessions(
    db: &SessionDb,
    options: &CleanupOptions,
) -> Result<CleanupOutput, RemoveError>
{
    // Implementation in builder phase
}
```

### Database Schema Changes
```rust
/// Session record with removal tracking
pub struct Session {
    pub name: String,
    pub workspace_path: PathBuf,
    pub status: String,  // "active", "completed", etc.
    pub removal_status: Option<String>,  // NEW: None, "failed", "pending"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Output Types
```rust
/// Result of remove operation
pub struct RemoveOutput {
    pub removed: bool,
    pub cleanup_count: usize,  // Orphans cleaned up during removal
    pub workspace_deleted: bool,
    pub session_deleted: bool,
}

/// Orphan detection report
pub struct OrphanReport {
    pub type1_orphans: Vec<String>,  // Sessions without workspaces
    pub type2_orphans: Vec<PathBuf>,  // Workspaces without sessions
    pub total_orphan_count: usize,
}

/// Cleanup result
pub struct CleanupOutput {
    pub sessions_removed: usize,   // Type 1 orphans cleaned
    pub workspaces_removed: usize, // Type 2 orphans cleaned
    pub total_cleaned: usize,
}
```

## Algorithm Specifications

### Removal Algorithm (Atomic)
```rust
// Pseudocode for atomic removal
async fn remove_session(name: &str) -> Result<RemoveOutput, RemoveError> {
    // Phase 1: Validation
    let session = db.get(name).await?
        .ok_or(RemoveError::SessionNotFound { session_name: name })?;

    if session.is_active() {
        return Err(RemoveError::SessionIsActive { ... });
    }

    // Phase 2: Workspace deletion (delete filesystem first)
    fs::remove_dir_all(&session.workspace_path)
        .await
        .map_err(|e| RemoveError::WorkspaceDeletionFailed { ... })?;

    // Phase 3: Database deletion (if this fails, workspace already deleted)
    db.delete(name).await
        .map_err(|e| {
            // Mark session as removal_failed for recovery
            let _ = db.mark_removal_failed(name, &e.to_string()).await;
            RemoveError::DatabaseError { ... }
        })?;

    Ok(RemoveOutput {
        removed: true,
        workspace_deleted: true,
        session_deleted: true,
        cleanup_count: 0,
    })
}
```

### Orphan Detection Algorithm
```rust
async fn find_orphaned_workspaces(db: &SessionDb) -> Result<OrphanReport, RemoveError> {
    let mut type1_orphans = Vec::new();
    let mut type2_orphans = Vec::new();

    // Type 1: Sessions without workspaces
    for session in db.list_all().await? {
        if !session.workspace_path.exists() {
            type1_orphans.push(session.name.clone());
        }
    }

    // Type 2: Workspaces without sessions
    for entry in fs::read_dir(workspaces_dir()).await? {
        let workspace_path = entry?.path();
        let session_name = workspace_path.file_name().unwrap().to_str().unwrap();

        if db.get(session_name).await?.is_none() {
            type2_orphans.push(workspace_path);
        }
    }

    Ok(OrphanReport {
        type1_orphans,
        type2_orphans,
        total_orphan_count: type1_orphans.len() + type2_orphans.len(),
    })
}
```

## Doctor Command Integration

### New Doctor Subcommand
```bash
zjj doctor --cleanup-orphaned    # Interactive cleanup
zjj doctor --cleanup-orphaned --force    # Auto-cleanup without confirmation
zjj doctor --cleanup-orphaned --dry-run  # Show what would be cleaned
```

### Doctor Output Format
```
Doctor Report - 2026-02-08

Orphaned Resources Found:
  Type 1 (Sessions without workspaces): 2
    - session-123 (workspace deleted externally)
    - session-456 (workspace deleted externally)

  Type 2 (Workspaces without sessions): 1
    - ~/.local/share/zjj/workspaces/orphan-workspace (BUG: session not deleted)

Recommendations:
  - Run 'zjj doctor --cleanup-orphaned' to clean up
  - Type 2 orphans indicate a bug - please report
```

## Non-goals

### Out of Scope (Not Part of This Fix):
1. **JJ workspace management**: `jj workspace forget` is a separate concern
2. **Zellij tab management**: Tab closure is handled separately
3. **Workspace migration**: Moving workspaces to different locations
4. **Backup/restore**: No snapshot or backup functionality
5. **Performance optimization**: Focus on correctness, not speed
6. **Distributed locking**: Single-machine only (no multi-node)
7. **Automatic cleanup on startup**: Cleanup is manual (doctor command)

### Explicitly NOT Doing:
- Adding `#[allow]` attributes to suppress warnings
- Using `unwrap()`, `expect()`, or `panic!()` in production code
- Mocking filesystem or database in tests (use real resources)
- Changing existing API surface (only extending)
- Breaking backward compatibility

## Testing Strategy

### Unit Tests (Required)
- `remove_session_deletes_workspace_and_record` - Happy path
- `remove_when_workspace_deletion_fails_marks_session_failed` - Error path
- `remove_when_database_deletion_fails_leaves_workspace_deleted` - Partial failure
- `find_orphaned_workspaces_detects_type1_orphans` - Detection
- `find_orphaned_workspaces_detects_type2_orphans` - Detection
- `cleanup_orphaned_sessions_removes_type1_orphans` - Cleanup
- `cleanup_orphaned_sessions_removes_type2_orphans` - Cleanup
- `retry_remove_after_failed_removal_succeeds` - Recovery

### Integration Tests (Required)
- CLI remove command end-to-end
- CLI doctor command orphan detection
- CLI doctor command cleanup with confirmation

### Concurrency Tests (Required)
- `concurrent_remove_of_different_sessions_succeeds` - No race conditions

See `.crackpipe/martin-fowler-tests-zjj-3ltb.md` for complete test plan.

## Verification Checklist

Before marking bead as `ready-qa`:
- [ ] All error variants tested
- [ ] Atomic removal verified (both delete or neither)
- [ ] Orphan detection finds both Type 1 and Type 2
- [ ] Cleanup removes both types of orphans
- [ ] Doctor command works interactively and with `--force`
- [ ] Failed removals marked in database for recovery
- [ ] No `unwrap()`, `expect()`, or `panic!()` in production code
- [ ] All functions return `Result<T, Error>`
- [ ] Error messages are clear and actionable
- [ ] Tests use real filesystem (no mocks)
- [ ] `moon run :test` passes
- [ ] `moon run :quick` passes (no clippy warnings)

## Related Code

- Session database: `crates/zjj-core/src/session.rs` (may need creation)
- Remove command: `crates/zjj/src/commands/remove/mod.rs`
- Doctor command: `crates/zjj/src/commands/doctor.rs`
- Bead database: `crates/bead-kv/src/store.rs`

## Dependencies

- `tokio` - Async runtime
- `sqlx` or `bead-kv` - Database operations
- `tempfile` - Test fixtures
- `serde` - Serialization for error reporting

---

**Contract Status**: ✅ Ready for Builder

**Estimated Implementation Time**: 2 hours

**Confidence Level**: High (clear invariants, well-understood problem)

**Next Step**: Implement `remove_session()`, `find_orphaned_workspaces()`, and `cleanup_orphaned_sessions()` following this contract.
