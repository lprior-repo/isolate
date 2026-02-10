# Rust Contract Specification: Checkpoint Restore Data Loss (zjj-1gm9)

**Generated**: 2026-02-08 07:10:00 UTC
**Bead**: zjj-1gm9
**Title**: checkpoint: Fix checkpoint restore data loss
**Issue Type**: Bug fix (critical - SEVERE DATA LOSS)

---

## Problem Statement

**Reported Issue**: Restoring from checkpoint deletes existing sessions and creates orphaned workspaces.

**Example**:
```bash
# Before restore: 22 active sessions
$ zjj checkpoint list
Sessions: 22

$ zjj checkpoint restore chk-abc123
Restored to checkpoint: chk-abc123

# After restore: Only 14 sessions - lost 8!
$ zjj checkpoint list
Sessions: 14
```

**Impact**:
- SEVERE DATA LOSS
- Sessions deleted without warning
- Orphaned workspaces accumulate
- Cannot safely restore checkpoints

**Root Cause**:
The `restore_checkpoint()` function in `checkpoint.rs` uses:
```rust
sqlx::query("DELETE FROM sessions")
    .execute(&mut *tx)
    .await?;
```

This deletes ALL existing sessions before restoring, with no:
1. Warning to user
2. Backup creation
3. Workspace cleanup
4. Confirmation prompt

---

## Module Structure

**Primary File**: `crates/zjj/src/commands/checkpoint/mod.rs`

**Problematic Function**:
```rust
async fn restore_checkpoint(db: &SessionDb, checkpoint_id: &str) -> Result<CheckpointResponse> {
    // ...
    let mut tx = pool.begin().await?;

    // DESTRUCTIVE: Deletes all existing sessions!
    sqlx::query("DELETE FROM sessions")
        .execute(&mut *tx)
        .await?;

    // Then restore from checkpoint
    // ...
}
```

**Related Files**:
- `crates/zjj/src/db.rs` - Session database operations
- `crates/zjj/src/session.rs` - Session management

---

## Public API

**Current Signature**:
```rust
pub async fn run(args: &CheckpointArgs) -> Result<()>
```

**Required Changes**: Add safety checks before destructive restore.

---

## Type Changes

**New Restore Options**:
```rust
pub struct RestoreOptions {
    pub checkpoint_id: String,
    pub force: bool,           // Required for destructive restore
    pub backup: bool,          // Create backup before restore
    pub dry_run: bool,         // Preview what would happen
}
```

**New Response**:
```rust
pub enum RestoreResponse {
    Preview {
        current_sessions: usize,
        checkpoint_sessions: usize,
        sessions_to_delete: usize,
        sessions_to_add: usize,
    },
    Restored {
        checkpoint_id: String,
        sessions_restored: usize,
        backup_checkpoint: Option<String>,  // Auto-created backup
    },
    ConfirmationRequired {
        message: String,
    },
}
```

---

## CLI Changes

**Add Restore Flags**:
```bash
zjj checkpoint restore chk-abc123
# -> If sessions exist: show confirmation prompt

zjj checkpoint restore chk-abc123 --dry-run
# -> Show preview: "Will delete X sessions, restore Y sessions"

zjj checkpoint restore chk-abc123 --force
# -> Skip confirmation, create backup

zjj checkpoint restore chk-abc123 --backup
# -> Create checkpoint backup before restore
```

---

## Error Types

**New Error Required**:
```rust
pub enum CheckpointError {
    ConfirmationRequired {
        current_sessions: usize,
        checkpoint_sessions: usize,
    },
    NoBackupCreated(String),
    RestoreFailed(String),
    // ...
}
```

**Exit Code**: Exit code 4 for confirmation required (user must use --force)

---

## Performance Constraints

**Backup Creation**: < 5 seconds for 100 sessions
- Checkpoint creation is fast (just copying records)

**Restore Speed**: < 10 seconds for 100 sessions
- Transaction-based restore is efficient

---

## Testing Requirements

See `.crackpipe/martin-fowler-tests-zjj-1gm9.md` for detailed test plan.

Key tests:
- Restore with existing sessions requires confirmation
- Restore with --force deletes sessions (user consented)
- Restore with --backup creates auto-backup
- Dry-run shows accurate preview
- Workspace cleanup happens automatically

---

## Migration Guide

**Behavior Changes**:
1. `zjj checkpoint restore <id>` now requires confirmation if sessions exist
2. Use `--force` to skip confirmation
3. Use `--backup` to auto-create backup
4. Use `--dry-run` to preview changes

**Script Compatibility**:
```bash
# Old scripts will fail
zjj checkpoint restore chk-abc123  # Now prompts!

# Update to:
zjj checkpoint restore chk-abc123 --force --backup
```

---

## Implementation Checklist

- [ ] Add `--force` flag to restore CLI
- [ ] Add `--backup` flag to restore CLI
- [ ] Add `--dry-run` flag to restore CLI
- [ ] Implement confirmation prompt
- [ ] Implement auto-backup creation
- [ ] Implement workspace cleanup
- [ ] Add dry-run preview
- [ ] Add data loss prevention tests
- [ ] Add workspace cleanup tests
- [ ] Update documentation

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let checkpoint = get_checkpoint(&db, id).await.unwrap();

// ✅ REQUIRED
let checkpoint = get_checkpoint(&db, id).await?
    .ok_or_else(|| CheckpointError::NotFound(id.to_string()))?;
```

---

## Success Criteria

1. Restore without --force requires confirmation
2. Auto-backup created when --backup specified
3. Workspace cleanup happens automatically
4. Dry-run shows accurate preview
5. No data loss without explicit user intent
6. All tests pass

---

## Verification Steps

Before closing bead:

```bash
# 1. Test confirmation prompt
zjj checkpoint restore chk-abc123
# Should show confirmation

# 2. Test dry run
zjj checkpoint restore chk-abc123 --dry-run
# Should show preview

# 3. Test force restore with backup
zjj checkpoint restore chk-abc123 --force --backup
# Should create backup and restore

# 4. Verify backup exists
zjj checkpoint list
# Should show auto-backup checkpoint

# 5. Run tests
moon run :test checkpoint

# 6. Full CI
moon run :ci
```

---

## Related Beads

- zjj-26pf: Fix session status never updates
- zjj-1w0d: Fix lock non-existent session succeeds
- zjj-27jw: Fix state corruption after 50+ operations

---

**Contract Status**: Ready for Implementation

**Estimated Resolution Time**: 2 hours (add flags + tests)

**Risk Level**: Medium (changes user-visible behavior)
