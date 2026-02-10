# Design by Contract Specification
**Bead**: zjj-3ltb - "database: Fix session-workspace desynchronization"
**Architect**: architect-2
**Date**: 2026-02-08

## Executive Summary

The remove command has a critical flaw: it performs cleanup operations sequentially without transactional guarantees. If any step fails, the system can be left in an inconsistent state with orphaned workspaces or dangling session records. This contract defines atomic cleanup requirements to prevent resource leaks.

---

## Current State Analysis

**File**: `crates/zjj/src/commands/remove.rs` (lines 79-94)

**Existing Flow**:
```rust
// Step 1: Forget from JJ (may fail)
run_command("jj", &["workspace", "forget", name]).await?;

// Step 2: Remove directory (may fail)
tokio::fs::remove_dir_all(workspace_path).await?;

// Step 3: Delete from database (may fail)
db.delete(name).await?;
```

**Problem Scenarios**:
1. **JJ forget fails, directory removal succeeds**: Workspace deleted but JJ still tracks it → JJ confusion
2. **Directory removal fails, DB delete succeeds**: Session record gone but workspace remains → Orphaned workspace
3. **JJ forget succeeds, DB delete fails**: JJ doesn't track workspace but database thinks it exists → Desynchronization
4. **Partial directory removal**: Some files deleted, others remain → Corrupted workspace

---

## Module: Remove Command
**File**: `crates/zjj/src/commands/remove.rs`

### Contract: Atomic Session Removal

**Function**: `run_with_options()`

**Preconditions**:
1. `name` must reference an existing session in database
2. Session must have a valid `workspace_path` in database
3. User must have confirmed removal (or `--force` flag set)
4. If `--merge` flag set, workspace must be mergeable to main

**Postconditions**:
1. **All-or-nothing**: Either ALL cleanup succeeds OR NONE succeeds
2. **Atomic state**: System is never in a partially-removed state
3. **Consistent guarantees**:
   - If session record exists in DB → workspace directory exists on disk
   - If workspace directory exists → session record exists in DB
   - If JJ tracks workspace → session record exists in DB

**Invariants**:
- No `unwrap()`, `expect()`, `panic!()` patterns
- All operations return `Result` types
- Proper error propagation with context
- Cleanup rollback on failure

---

### Contract: Cleanup Transaction

**New Function**: `cleanup_session_atomically()`

**Signature**:
```rust
async fn cleanup_session_atomically(
    db: &SessionDb,
    session: &Session,
    options: &RemoveOptions,
) -> Result<RemoveResult, RemoveError>
```

**Transaction Phases**:

#### Phase 1: Pre-Cleanup Validation
```rust
// Verify all resources exist and are accessible
validate_session_state(&session)?;
```

**Preconditions**:
- Session exists in database
- Workspace path is valid and accessible
- If inside Zellij, tab exists (optional)

**Postconditions**:
- Returns `Ok(ValidationResult)` if all checks pass
- Returns `Err(RemoveError::ValidationFailed)` if any check fails

---

#### Phase 2: Resource Teardown

**Step 2a: Close Zellij Tab** (optional, non-critical)
```rust
if is_inside_zellij() {
    close_zellij_tab(&session.zellij_tab).await
        .unwrap_or_else(|e| tracing::warn!("Failed to close Zellij tab: {e}"));
}
```

**Contract**:
- **Failure mode**: Non-critical, log warning and continue
- **Rationale**: Tab closure is UI cleanup, not data integrity

---

**Step 2b: Forget from JJ** (critical, may fail)
```rust
let jj_result = run_command("jj", &["workspace", "forget", name]).await;
```

**Contract**:
- **Failure mode**: Critical, must rollback or abort
- **Rollback**: Cannot rollback (JJ is external state)
- **Decision**: If this fails, log error but continue (workspace will be orphaned in JJ but deleted locally)
- **Rationale**: Local cleanup more important than JJ state synchronization

**Error Handling**:
```rust
if let Err(e) = jj_result {
    tracing::warn!("JJ workspace forget failed: {e}");
    tracing::warn!("Workspace will be deleted locally but JJ may still track it");
    // Continue with local cleanup
}
```

---

**Step 2c: Remove Workspace Directory** (critical, may fail)
```rust
tokio::fs::remove_dir_all(workspace_path).await
    .map_err(|e| RemoveError::WorkspaceRemovalFailed {
        path: workspace_path.clone(),
        source: e,
    })?;
```

**Contract**:
- **Failure mode**: Critical, must preserve session record
- **Rollback**: Keep session record in database with "corrupted" flag
- **Rationale**: If directory deletion fails, user needs manual cleanup path

**Error Handling**:
```rust
if let Err(e) = directory_removal_result {
    // Mark session as "removal_failed" in database
    db.mark_removal_failed(name, &e.to_string()).await?;
    return Err(e);
}
```

---

**Step 2d: Delete from Database** (critical, may fail)
```rust
db.delete(name).await
    .map_err(|e| RemoveError::DatabaseDeletionFailed {
        name: name.to_string(),
        source: e,
    })?;
```

**Contract**:
- **Failure mode**: Critical, workspace already deleted
- **Rollback**: Cannot rollback (directory already gone)
- **Decision**: Log critical error, exit with error but don't crash
- **Rationale**: Worst case = orphaned workspace (user can manually clean up)

---

### Contract: Error States and Recovery

**Error Type**: `RemoveError`

```rust
pub enum RemoveError {
    /// Session not found in database
    SessionNotFound { name: String },

    /// Workspace path invalid or inaccessible
    WorkspaceInaccessible { path: String, reason: String },

    /// Workspace directory removal failed (session preserved)
    WorkspaceRemovalFailed {
        path: String,
        source: io::Error,
    },

    /// Database deletion failed (workspace already deleted)
    DatabaseDeletionFailed {
        name: String,
        source: sqlx::Error,
    },

    /// JJ workspace forget failed (non-critical)
    JjForgetFailed { name: String, source: io::Error },

    /// Zellij tab closure failed (non-critical)
    ZellijTabCloseFailed { tab: String, source: io::Error },
}
```

**Recovery Strategies**:

1. **SessionNotFound**: Exit with error code 2 (not found)
2. **WorkspaceInaccessible**: Exit with error code 3 (permission/I/O error)
3. **WorkspaceRemovalFailed**:
   - Mark session as `removal_failed` in database
   - Store error message in session metadata
   - Exit with error code 3
   - User can retry removal or manually clean up
4. **DatabaseDeletionFailed**:
   - Log critical error
   - Exit with error code 3
   - Workspace directory already deleted
   - User must manually delete session record
5. **JjForgetFailed**:
   - Log warning
   - Continue with local cleanup
   - JJ may show stale workspace entry
6. **ZellijTabCloseFailed**:
   - Log warning
   - Continue with cleanup
   - User can manually close tab

---

### Contract: Database Schema Updates

**New Columns Needed**:

```sql
-- Add to sessions table
ALTER TABLE sessions ADD COLUMN removal_status TEXT DEFAULT NULL;
ALTER TABLE sessions ADD COLUMN removal_error TEXT DEFAULT NULL;
ALTER TABLE sessions ADD COLUMN removal_attempted_at INTEGER DEFAULT NULL;
```

**Removal Status Values**:
- `NULL`: Normal operation
- `"pending"`: Removal in progress
- `"failed"`: Removal failed, manual cleanup needed
- `"orphaned"`: Workspace exists but session record missing (detected during cleanup)

**New Database Methods**:

```rust
impl SessionDb {
    /// Mark session as failed removal
    pub async fn mark_removal_failed(
        &self,
        name: &str,
        error: &str,
    ) -> Result<(), DbError>;

    /// Find orphaned workspaces (workspace exists but no session)
    pub async fn find_orphaned_workspaces(&self)
        -> Result<Vec<String>, DbError>;

    /// Cleanup orphaned session records (session exists but no workspace)
    pub async fn cleanup_orphaned_sessions(&self)
        -> Result<usize, DbError>;
}
```

---

### Contract: Orphaned Workspace Detection

**New Command**: `zjj doctor --cleanup-orphaned`

**Function**: `cleanup_orphaned_resources()`

**Signature**:
```rust
pub async fn cleanup_orphaned_resources(
    db: &SessionDb,
    options: &CleanupOptions,
) -> Result<CleanupReport, CleanupError>
```

**Detection Logic**:

1. **Find Type 1 Orphans**: Session in DB, workspace missing
   ```rust
   // Query database for all sessions
   let sessions = db.list_all().await?;
   for session in sessions {
       let workspace_exists = Path::new(&session.workspace_path).exists();
       if !workspace_exists {
           // Type 1 orphan: Session record exists, workspace gone
           report.type1_orphans.push(session.name.clone());
       }
   }
   ```

2. **Find Type 2 Orphans**: Workspace exists, no session in DB
   ```rust
   // Scan workspace directory for workspace folders
   for workspace_entry in fs::read_dir(workspaces_dir)? {
       let workspace_name = workspace_entry.file_name();
       let session = db.get(&workspace_name).await?;
       if session.is_none() {
           // Type 2 orphan: Workspace exists, no session record
           report.type2_orphans.push(workspace_name);
       }
   }
   ```

**Cleanup Strategy**:

1. **Type 1 Orphans** (session without workspace):
   - Delete session record from database
   - Log removal
   - Safe to auto-cleanup

2. **Type 2 Orphans** (workspace without session):
   - Prompt user for confirmation
   - Offer to:
     - Delete workspace directory
     - Create session record for existing workspace
   - Default: Don't auto-delete (user data safety)

---

## Module: Session Database
**File**: `crates/zjj/src/db.rs` (or wherever SessionDb is defined)

### Contract: Session Lifecycle Invariants

**Invariant 1**: Session Existence
```rust
// For any session name N:
db.exists(N) == workspace_directory_exists(N)
```

**Invariant 2**: Workspace Path Validity
```rust
// For any session S:
Path::new(&S.workspace_path).is_dir() == true
```

**Invariant 3**: JJ Synchronization (best-effort)
```rust
// For any session S:
jj_workspace_exists(S.name) == db.exists(S.name)  // May be false during failures
```

**Maintenance**: These invariants must hold after ALL operations:
- `create()`
- `delete()`
- `update()`
- Orphaned cleanup

---

## Module: Integration Tests

### Contract: Test Scenarios

**Test 1: Atomic Removal Success**
- Given: Valid session with workspace
- When: Remove called
- Then: Session deleted AND workspace deleted
- And: No orphaned resources

**Test 2: Workspace Removal Failure**
- Given: Valid session with read-only workspace
- When: Remove called
- Then: Session marked as "removal_failed"
- And: Workspace still exists (read-only protected)
- And: Error message includes reason

**Test 3: Database Deletion Failure**
- Given: Valid session, database connection closes mid-transaction
- When: Remove called (after workspace deleted)
- Then: Workspace deleted
- And: Error logged
- And: Type 2 orphan exists (manual cleanup needed)

**Test 4: JJ Forget Failure**
- Given: Valid session, JJ command fails
- When: Remove called
- Then: Workspace deleted
- And: Session deleted
- And: Warning logged about JJ state

**Test 5: Orphaned Workspace Detection**
- Given: Type 1 and Type 2 orphans exist
- When: `zjj doctor --cleanup-orphaned` called
- Then: Detects both orphan types
- And: Reports counts
- And: Offers cleanup options

---

## Acceptance Criteria

**Functional**:
- [ ] Remove command is atomic (all-or-nothing)
- [ ] Failed removals mark session with error state
- [ ] Orphaned workspace detection implemented
- [ ] Doctor command can cleanup orphans
- [ ] All error paths tested

**Quality**:
- [ ] Zero unwrap/expect/panic patterns
- [ ] All errors provide context
- [ ] Proper Result propagation
- [ ] Test coverage > 85% for remove module

**Data Integrity**:
- [ ] No orphaned workspaces created in normal operation
- [ ] Failed removals are recoverable
- [ ] Orphan detection works reliably
- [ ] Cleanup commands safe (confirm before destructive actions)

---

## Implementation Phases

### Phase 1: Add Removal Status Tracking
**Estimated**: 1 hour
- Add `removal_status`, `removal_error` columns to sessions table
- Implement `mark_removal_failed()` method
- Update delete() to check status

### Phase 2: Refactor Remove Command
**Estimated**: 2 hours
- Extract cleanup logic to `cleanup_session_atomically()`
- Add error handling for each phase
- Implement rollback/abort logic
- Add tracing logs for each phase

### Phase 3: Implement Orphan Detection
**Estimated**: 2 hours
- Implement `find_orphaned_workspaces()`
- Implement `cleanup_orphaned_sessions()`
- Add doctor command integration

### Phase 4: Testing
**Estimated**: 2 hours
- Write unit tests for error paths
- Write integration tests for orphan cleanup
- Manual testing with simulated failures

**Total Estimated Time**: 7 hours

---

## Migration Path

### For Existing Users
1. Deploy database migration (add new columns)
2. Run orphan detection to find existing issues
3. Prompt users to run `zjj doctor --cleanup-orphaned`
4. Document manual cleanup steps in user guide

### Backward Compatibility
- Existing remove command behavior preserved (except error handling)
- No breaking changes to CLI flags
- Database migration is additive only

---

## Recommended Approach

**Priority 1**: Fix remove command atomicity (Phase 1-2)
- Prevents new orphaned workspaces
- Critical for data integrity

**Priority 2**: Implement orphan detection (Phase 3)
- Helps clean up existing issues
- Provides recovery path

**Priority 3**: Enhanced error messages and logging
- Improves user experience
- Aids debugging

**Order**: Sequential (Phase 1 → Phase 2 → Phase 3 → Phase 4)
