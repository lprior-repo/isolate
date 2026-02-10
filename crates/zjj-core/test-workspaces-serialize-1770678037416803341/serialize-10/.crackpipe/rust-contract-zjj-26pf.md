# Rust Contract Specification: Session Status Never Updates (zjj-26pf)

**Generated**: 2026-02-08 07:20:00 UTC
**Bead**: zjj-26pf
**Title**: Fix session status never updates - sessions remain active forever
**Issue Type**: Bug fix (critical - state machine broken)

---

## Problem Statement

**Reported Issue**: Sessions remain in "active" status forever, even after completion.

**Example**:
```bash
$ zjj add my-session
$ zjj list
my-session  active

# ... do work ...

$ zjj done
✓ Workspace 'my-session' completed

# Check status - STILL ACTIVE!
$ zjj list
my-session  active  # WRONG! Should be 'completed' or 'merged'
```

**Impact**:
- Cannot track actual session state
- Status meaningless
- Automation cannot determine completion
- UI shows incorrect information

**Root Cause**:
The `done` command in `done.rs` sets `session_updated: false` but doesn't actually update the session status:
```rust
// Line 193 in done.rs
Ok(DoneOutput {
    workspace_name,
    // ...
    session_updated: false,  // NEVER SET TO TRUE!
    // ...
})
```

The session status is never updated from "active" to "completed" or "merged".

---

## Module Structure

**Primary File**: `crates/zjj/src/commands/done/mod.rs`

**Problematic Function**:
```rust
async fn execute_done(...) -> Result<DoneOutput, DoneError> {
    // ...
    // Phase 8: Update bead status - works!
    update_bead_status(bead, "closed", bead_repo).await?;

    // But session status - NOT UPDATED!
    // No call to update session status in database

    Ok(DoneOutput {
        session_updated: false,  // ALWAYS FALSE!
        // ...
    })
}
```

**Related Files**:
- `crates/zjj/src/db.rs` - Session database operations
- `crates/zjj/src/commands/status.rs` - Status display

---

## Public API

**Current Behavior**: Session status never changes after creation.

**Required Behavior**:
1. When `zjj done` completes successfully, update session status to "completed"
2. When merged to main, update session status to "merged"
3. When session is removed, update status to "deleted"
4. Status transitions must be atomic with the operation

---

## Type Changes

**Session Status Enum** (should already exist, needs to be used):
```rust
pub enum SessionStatus {
    Active,      // Initial state when created
    Paused,      // User paused work
    Completed,   // Work done, ready to merge
    Merged,      // Merged to main
    Deleted,     // Session removed
}
```

**DoneOutput Update**:
```rust
pub struct DoneOutput {
    // ...
    pub session_updated: bool,  // Should be TRUE when status updated
    pub new_status: Option<String>,  // NEW: Track what status we set
    // ...
}
```

---

## CLI Changes

**No CLI argument changes** - behavior fix only.

**Expected Behavior**:
```bash
# Before fix
$ zjj done
✓ Workspace 'my-session' completed
$ zjj list
my-session  active  # WRONG

# After fix
$ zjj done
✓ Workspace 'my-session' completed
$ zjj list
my-session  completed  # CORRECT
```

---

## Error Types

**No new errors needed** - this is a missing feature, not an error path.

---

## Performance Constraints

**Status Update**: < 100ms
- Simple UPDATE query on sessions table

**Atomic with Merge**: Must happen in same transaction as workspace forget

---

## Testing Requirements

### Status Update Tests (Critical):

```rust
#[tokio::test]
async fn done_command_updates_session_status_to_completed() {
    let (db, temp_dir) = setup_test_env().await;

    // Create session
    db.create("test-session", &temp_dir.path().join("workspace"), None)
        .await
        .unwrap();

    // Verify initial status
    let session = db.get("test-session").await.unwrap().unwrap();
    assert_eq!(session.status, SessionStatus::Active);

    // Run done command
    let executor = RealJjExecutor::new();
    let mut bead_repo = RealBeadRepository::new(temp_dir.path().to_path_buf());
    let filesystem = RealFileSystem::new();

    let options = DoneOptions {
        keep_workspace: true,
        no_keep: false,
        // ...
        ..Default::default()
    };

    let result = execute_done(&options, &executor, &mut bead_repo, &filesystem).await;

    assert!(result.is_ok());

    // Verify session status was updated
    let session = db.get("test-session").await.unwrap().unwrap();
    assert_eq!(session.status, SessionStatus::Completed,
               "Session status should be 'Completed' after done");

    // Verify DoneOutput reflects the update
    assert!(result.unwrap().session_updated,
            "session_updated should be true");
}

#[tokio::test]
async fn done_with_merge_updates_status_to_merged() {
    let (db, temp_dir) = setup_test_env().await;

    // Create session
    db.create("merge-session", &temp_dir.path().join("workspace"), None)
        .await
        .unwrap();

    // Run done with merge
    let options = DoneOptions {
        keep_workspace: false,  // Don't keep - triggers merge
        no_keep: false,
        ..Default::default()
    };

    // ... execute done ...

    // Verify status is Merged
    let session = db.get("merge-session").await.unwrap().unwrap();
    assert_eq!(session.status, SessionStatus::Merged);
}

#[tokio::test]
async fn remove_session_updates_status_to_deleted() {
    let db = setup_test_db().await;

    // Create session
    db.create("delete-me", "/workspace", None).await.unwrap();

    // Remove session
    db.remove("delete-me").await.unwrap();

    // Verify status updated (not just deleted from DB)
    // Option A: Record is soft-deleted with status=Deleted
    // Option B: Record is hard-deleted but status updated first
    // Implementation choice
}

#[tokio::test]
async fn status_query_shows_correct_current_state() {
    let db = setup_test_db().await;

    // Create session
    db.create("status-session", "/workspace", None).await.unwrap();

    // Initially active
    let session = db.get("status-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Active));

    // Update to completed
    db.update_status("status-session", SessionStatus::Completed).await.unwrap();

    // Verify new status
    let session = db.get("status-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Completed));
}
```

### State Machine Tests:

```rust
#[tokio::test]
async fn session_status_transitions_follow_valid_state_machine() {
    let db = setup_test_db().await;

    // Create session - starts as Active
    db.create("state-session", "/workspace", None).await.unwrap();
    let session = db.get("state-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Active));

    // Pause -> Paused
    db.update_status("state-session", SessionStatus::Paused).await.unwrap();
    let session = db.get("state-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Paused));

    // Resume -> Active
    db.update_status("state-session", SessionStatus::Active).await.unwrap();
    let session = db.get("state-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Active));

    // Complete -> Completed
    db.update_status("state-session", SessionStatus::Completed).await.unwrap();
    let session = db.get("state-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Completed));

    // Merge -> Merged
    db.update_status("state-session", SessionStatus::Merged).await.unwrap();
    let session = db.get("state-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Merged));

    // Delete -> Deleted
    db.update_status("state-session", SessionStatus::Deleted).await.unwrap();
    let session = db.get("state-session").await.unwrap().unwrap();
    assert!(matches!(session.status, SessionStatus::Deleted));
}

#[tokio::test]
async fn session_status_list_shows_correct_states() {
    let db = setup_test_db().await;

    // Create sessions with different states
    db.create("active-1", "/w1", None).await.unwrap();
    db.create("active-2", "/w2", None).await.unwrap();
    db.create("paused-1", "/w3", None).await.unwrap();
    db.create("completed-1", "/w4", None).await.unwrap();

    // Set different statuses
    db.update_status("paused-1", SessionStatus::Paused).await.unwrap();
    db.update_status("completed-1", SessionStatus::Completed).await.unwrap();

    // List all sessions
    let sessions = db.list(None).await.unwrap();

    // Verify counts
    let active_count = sessions.iter().filter(|s| matches!(s.status, SessionStatus::Active)).count();
    let paused_count = sessions.iter().filter(|s| matches!(s.status, SessionStatus::Paused)).count();
    let completed_count = sessions.iter().filter(|s| matches!(s.status, SessionStatus::Completed)).count();

    assert_eq!(active_count, 2);
    assert_eq!(paused_count, 1);
    assert_eq!(completed_count, 1);
}
```

---

## Implementation Checklist

- [ ] Add `update_status()` method to SessionDb
- [ ] Update `done.rs` to call `update_status()` after successful completion
- [ ] Update `remove` command to set status to Deleted (or soft-delete)
- [ ] Update `merge` logic to set status to Merged
- [ ] Set `session_updated: true` in DoneOutput when status updated
- [ ] Add status transition tests
- [ ] Add state machine tests
- [ ] Verify `zjj list` shows correct statuses

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
db.update_status(session, status).await.unwrap();

// ✅ REQUIRED
db.update_status(session, status).await
    .map_err(|e| DoneError::InvalidState {
        reason: format!("Failed to update session status: {e}")
    })?;
```

---

## Success Criteria

1. `zjj done` updates session status to "Completed"
2. Merging updates session status to "Merged"
3. Removing updates session status to "Deleted"
4. `zjj list` shows correct statuses
5. Status transitions are atomic with operations
6. All tests pass

---

## Verification Steps

Before closing bead:

```bash
# 1. Create session
zjj add test-session
zjj list | grep test-session
# Should show: test-session  active

# 2. Complete work
zjj done
zjj list | grep test-session
# Should show: test-session  completed

# 3. Verify status persists
zjj list
# Status should remain completed

# 4. Run tests
moon run :test done

# 5. Full CI
moon run :ci
```

---

## Related Beads

- zjj-1gm9: Fix checkpoint restore data loss
- zjj-1w0d: Fix lock non-existent session succeeds
- zjj-1nyz: Ensure consistent session counting

---

**Contract Status**: Ready for Implementation

**Estimated Resolution Time**: 1.5 hours (add update_status + tests)

**Risk Level**: Low (adding missing functionality)
