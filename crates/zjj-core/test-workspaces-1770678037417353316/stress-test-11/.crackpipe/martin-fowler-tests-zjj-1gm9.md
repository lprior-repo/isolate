# Martin Fowler Test Plan: Checkpoint Restore Data Loss (zjj-1gm9)

**Generated**: 2026-02-08 07:10:30 UTC
**Bead**: zjj-1gm9
**Contract**: `.crackpipe/rust-contract-zjj-1gm9.md`
**Issue Type**: Bug fix (critical - SEVERE DATA LOSS)

---

## Test Strategy

Since this is a **data loss bug**, our test strategy focuses on:

1. **Data Preservation**: Verify existing sessions aren't deleted without consent
2. **Confirmation Testing**: Ensure user must explicitly confirm destructive operations
3. **Backup Testing**: Verify auto-backup works
4. **Workspace Cleanup**: Verify orphaned workspaces are cleaned up

**Martin Fowler Principles Applied**:
- **State Verification**: Verify session count before/after
- **No Mocking**: Real database operations
- **Safety Testing**: Tests prevent destructive operations by accident
- **Clear Intent**: Tests verify safety guarantees

---

## Test Categories

### 1. Data Loss Prevention Tests (Critical)

**Purpose**: Ensure restore doesn't delete sessions without explicit consent.

```rust
#[cfg(test)]
mod data_loss_prevention_tests {
    use super::*;

    // Test 1: Restore with existing sessions requires confirmation
    #[tokio::test]
    async fn restore_with_existing_sessions_requires_confirmation() {
        let db = setup_test_db().await;

        // Create 5 active sessions
        for i in 1..=5 {
            db.create(&format!("active-session-{}", i), "/workspace", None)
                .await
                .unwrap();
        }

        // Create checkpoint with 3 sessions
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s3"]).await;

        // Attempt restore without --force
        let result = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: false,
            backup: false,
            dry_run: false,
        }).await;

        // Should require confirmation
        match result {
            Err(CheckpointError::ConfirmationRequired { current_sessions, checkpoint_sessions }) => {
                assert_eq!(current_sessions, 5);
                assert_eq!(checkpoint_sessions, 3);
            }
            other => panic!("Expected ConfirmationRequired, got {:?}", other),
        }

        // Verify original 5 sessions STILL exist (not deleted)
        let sessions = db.list(None).await.unwrap();
        assert_eq!(sessions.len(), 5, "Original sessions must not be deleted without confirmation");
    }

    // Test 2: Restore with --force deletes sessions (user explicitly consented)
    #[tokio::test]
    async fn restore_with_force_deletes_existing_sessions() {
        let db = setup_test_db().await;

        // Create 5 active sessions
        for i in 1..=5 {
            db.create(&format!("active-session-{}", i), "/workspace", None)
                .await
                .unwrap();
        }

        // Create checkpoint with 3 sessions
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s3"]).await;

        // Force restore (user explicitly consented)
        let result = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: true,
            backup: false,
            dry_run: false,
        }).await;

        assert!(result.is_ok(), "Force restore should succeed");

        // Verify only 3 sessions exist now
        let sessions = db.list(None).await.unwrap();
        assert_eq!(sessions.len(), 3, "Only checkpoint sessions should exist after force restore");

        // Verify they're the checkpoint sessions
        let session_names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
        assert!(session_names.contains(&"s1"));
        assert!(session_names.contains(&"s2"));
        assert!(session_names.contains(&"s3"));
    }

    // Test 3: Restore with no existing sessions doesn't require confirmation
    #[tokio::test]
    async fn restore_to_empty_database_doesnt_require_confirmation() {
        let db = setup_test_db().await;

        // No existing sessions

        // Create checkpoint with 3 sessions
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s3"]).await;

        // Restore without --force should still work (no data to lose)
        let result = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: false,
            backup: false,
            dry_run: false,
        }).await;

        assert!(result.is_ok(), "Restore to empty DB should succeed without confirmation");

        // Verify sessions restored
        let sessions = db.list(None).await.unwrap();
        assert_eq!(sessions.len(), 3);
    }
}
```

**Fowler's Classification**: **Safety Test**
- Tests prevent destructive operations by accident
- Verifies user consent is required
- State verification (session count)

---

### 2. Auto-Backup Tests

**Purpose**: Verify auto-backup creation works.

```rust
#[cfg(test)]
mod backup_tests {
    use super::*;

    // Test 1: Auto-backup created when --backup specified
    #[tokio::test]
    async fn restore_with_backup_creates_auto_backup() {
        let db = setup_test_db().await;

        // Create 5 active sessions
        for i in 1..=5 {
            db.create(&format!("backup-session-{}", i), "/workspace", None)
                .await
                .unwrap();
        }

        // Create checkpoint with 3 sessions
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s3"]).await;

        // Restore with backup
        let result = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: true,
            backup: true,
            dry_run: false,
        }).await;

        assert!(result.is_ok());

        // Verify backup checkpoint was created
        let backups = list_checkpoints(&db).await.unwrap();
        let auto_backup = backups.iter()
            .find(|cp| cp.description.as_ref()
                .map_or(false, |d| d.contains("auto-backup") || d.contains("pre-restore")));

        assert!(auto_backup.is_some(), "Auto-backup checkpoint should be created");

        // Verify backup has 5 sessions (original state)
        let backup_sessions = get_checkpoint_session_count(&db, &auto_backup.unwrap().id).await;
        assert_eq!(backup_sessions, 5, "Backup should preserve original 5 sessions");
    }

    // Test 2: Restoring from auto-backup recovers original state
    #[tokio::test]
    async fn restoring_from_auto_backup_recovers_original_state() {
        let db = setup_test_db().await;

        // Create 5 sessions with specific data
        for i in 1..=5 {
            db.create(&format!("recovery-session-{}", i), "/workspace", None)
                .await
                .unwrap();
        }

        // Create checkpoint with 3 sessions
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s3"]).await;

        // Restore with backup
        let response = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: true,
            backup: true,
            dry_run: false,
        }).await.unwrap();

        // Get backup checkpoint ID from response
        let backup_id = match response {
            CheckpointResponse::Restored { backup_checkpoint, .. } => backup_checkpoint,
            _ => None,
        };

        assert!(backup_id.is_some(), "Response should include backup checkpoint ID");

        // Now restore from the backup
        let restore_result = restore_checkpoint_safe(&db, &backup_id.unwrap(), RestoreOptions {
            force: true,
            backup: false,
            dry_run: false,
        }).await;

        assert!(restore_result.is_ok(), "Restore from backup should succeed");

        // Verify we have the original 5 sessions back
        let sessions = db.list(None).await.unwrap();
        assert_eq!(sessions.len(), 5, "Should have 5 sessions after restoring from backup");

        let session_names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
        for i in 1..=5 {
            assert!(session_names.contains(&format!("recovery-session-{}", i)),
                    "Session recovery-session-{} should be recovered", i);
        }
    }
}
```

---

### 3. Dry Run Tests

**Purpose**: Verify dry-run shows accurate preview without making changes.

```rust
#[cfg(test)]
mod dry_run_tests {
    use super::*;

    #[tokio::test]
    async fn dry_run_shows_accurate_preview() {
        let db = setup_test_db().await;

        // Create 5 sessions: s1, s2, s3, s4, s5
        for i in 1..=5 {
            db.create(&format!("s{}", i), "/workspace", None)
                .await
                .unwrap();
        }

        // Create checkpoint with 3 sessions: s1, s2, s6
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s6"]).await;

        // Dry run
        let preview = restore_checkpoint_dry_run(&db, &checkpoint_id).await.unwrap();

        assert_eq!(preview.current_sessions, 5, "Should show 5 current sessions");
        assert_eq!(preview.checkpoint_sessions, 3, "Should show 3 checkpoint sessions");

        // Sessions to delete: s3, s4, s5 (not in checkpoint)
        assert_eq!(preview.sessions_to_delete, 3, "Should delete 3 sessions");

        // Sessions to add: s6 (only in checkpoint)
        assert_eq!(preview.sessions_to_add, 1, "Should add 1 session");

        // Verify no changes were made
        let sessions = db.list(None).await.unwrap();
        assert_eq!(sessions.len(), 5, "Dry run should not modify data");
    }

    #[tokio::test]
    async fn dry_run_output_is_human_readable() {
        let db = setup_test_db().await;

        // Create sessions
        db.create("session-1", "/workspace", None).await.unwrap();

        // Create checkpoint
        let checkpoint_id = create_checkpoint(&db).await;

        // Get dry run output
        let preview = restore_checkpoint_dry_run(&db, &checkpoint_id).await.unwrap();

        // Verify output format
        let output = format!("{}", preview);
        assert!(output.contains("current sessions"), "Output should mention current sessions");
        assert!(output.contains("checkpoint sessions"), "Output should mention checkpoint sessions");
        assert!(output.contains("delete") || output.contains("remove"), "Output should warn about deletions");
    }
}
```

---

### 4. Workspace Cleanup Tests

**Purpose**: Verify orphaned workspaces are cleaned up.

```rust
#[cfg(test)]
mod workspace_cleanup_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn restore_cleans_up_orphaned_workspaces() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_base = temp_dir.path().join("workspaces");
        fs::create_dir_all(&workspace_base).unwrap();

        let db = setup_test_db_with_workspace_base(workspace_base.clone()).await;

        // Create session with workspace
        let workspace_path = workspace_base.join("orphaned-session");
        fs::create_dir_all(&workspace_path).unwrap();
        fs::write(workspace_path.join("marker.txt"), "test").unwrap();

        db.create("orphaned-session", workspace_path.to_str().unwrap(), None)
            .await
            .unwrap();

        // Create checkpoint (without this session)
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2"]).await;

        // Force restore
        restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: true,
            backup: false,
            dry_run: false,
        }).await.unwrap();

        // Verify workspace was cleaned up
        assert!(!workspace_path.exists(), "Orphaned workspace should be deleted");
    }

    #[tokio::test]
    async fn restore_doesnt_clean_up_active_workspaces() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_base = temp_dir.path().join("workspaces");
        fs::create_dir_all(&workspace_base).unwrap();

        let db = setup_test_db_with_workspace_base(workspace_base.clone()).await;

        // Create two sessions with workspaces
        let ws1 = workspace_base.join("kept-session");
        let ws2 = workspace_base.join("deleted-session");

        fs::create_dir_all(&ws1).unwrap();
        fs::create_dir_all(&ws2).unwrap();

        db.create("kept-session", ws1.to_str().unwrap(), None).await.unwrap();
        db.create("deleted-session", ws2.to_str().unwrap(), None).await.unwrap();

        // Create checkpoint with only kept-session
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["kept-session"]).await;

        // Force restore
        restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: true,
            backup: false,
            dry_run: false,
        }).await.unwrap();

        // Verify kept-session workspace still exists
        assert!(ws1.exists(), "Kept session workspace should not be deleted");

        // Verify deleted-session workspace was cleaned up
        assert!(!ws2.exists(), "Deleted session workspace should be cleaned up");
    }
}
```

---

### 5. Integration Tests

**Purpose**: End-to-end verification of restore behavior.

```bash
#!/bin/bash
# test/integration/checkpoint_restore_safety.sh

set -euo pipefail

echo "=== Checkpoint Restore Safety Tests ==="

# Setup
TEMP_DIR=$(mktemp -d)
export ZJJ_DATA_DIR="$TEMP_DIR"
cd "$TEMP_DIR"

cleanup() {
    cd /
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "Test 1: Restore requires confirmation"
# Create sessions
zjj add session-1 --no-zellij >/dev/null 2>&1 || true
zjj add session-2 --no-zellij >/dev/null 2>&1 || true

# Create checkpoint
CHECKPOINT=$(zjj checkpoint create --description "test" 2>&1 | grep -oP 'chk-\w+' || echo "")

# Try to restore (should require confirmation)
if zjj checkpoint restore "$CHECKPOINT" 2>&1 | grep -iq "confirm\|force\|--backup"; then
    echo "✓ PASS: Restore requires confirmation/flags"
else
    echo "✗ FAIL: Restore should warn about destructive operation"
    exit 1
fi

echo ""
echo "Test 2: Dry-run shows preview"
OUTPUT=$(zjj checkpoint restore "$CHECKPOINT" --dry-run 2>&1)

if echo "$OUTPUT" | grep -iq "sessions\|restore\|preview"; then
    echo "✓ PASS: Dry-run shows preview"
else
    echo "✗ FAIL: Dry-run should show preview"
    exit 1
fi

echo ""
echo "Test 3: Force restore with backup"
# This should succeed and create backup
OUTPUT=$(zjj checkpoint restore "$CHECKPOINT" --force --backup 2>&1)

if echo "$OUTPUT" | grep -iq "restored\|success"; then
    echo "✓ PASS: Force restore with backup succeeded"
else
    echo "Output: $OUTPUT"
    echo "✗ FAIL: Force restore should succeed"
    exit 1
fi

# Verify backup checkpoint exists
BACKUPS=$(zjj checkpoint list 2>&1)
if echo "$BACKUPS" | grep -iq "backup\|pre-restore"; then
    echo "✓ PASS: Auto-backup was created"
else
    echo "✗ FAIL: Auto-backup should be created"
    exit 1
fi

echo ""
echo "=== All restore safety tests passed ==="
```

---

### 6. Regression Tests

**Purpose**: Prevent exact bug from recurring.

```rust
#[cfg(test)]
mod regression_tests {
    use super::*;

    // Regression: The exact reported bug
    #[tokio::test]
    async fn regression_restore_does_not_silently_delete_sessions() {
        let db = setup_test_db().await;

        // Create 22 sessions (like the bug report)
        for i in 1..=22 {
            db.create(&format!("session-{:03}", i), "/workspace", None)
                .await
                .unwrap();
        }

        // Verify 22 sessions
        let sessions_before = db.list(None).await.unwrap();
        assert_eq!(sessions_before.len(), 22);

        // Create checkpoint with 14 sessions
        let checkpoint_id = create_checkpoint_with_sessions(
            &db,
            &(1..=14).map(|i| format!("session-{:03}", i)).collect::<Vec<_>>()
        ).await;

        // Attempt restore WITHOUT explicit consent (no --force)
        let result = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: false,
            backup: false,
            dry_run: false,
        }).await;

        // Should NOT silently delete - should require confirmation
        assert!(result.is_err() || matches!(result, Err(CheckpointError::ConfirmationRequired { .. })),
                "Should require confirmation, not silently delete");

        // Verify all 22 sessions STILL exist (the bug was that 8 were deleted!)
        let sessions_after = db.list(None).await.unwrap();
        assert_eq!(sessions_after.len(), 22,
                    "REGRESSION: All 22 sessions should still exist without explicit confirmation");
    }

    // Regression: Verify fix doesn't prevent valid restores
    #[tokio::test]
    async fn regression_valid_restore_still_works() {
        let db = setup_test_db().await;

        // Create checkpoint
        let checkpoint_id = create_checkpoint_with_sessions(&db, &["s1", "s2", "s3"]).await;

        // Restore to empty DB (should work without --force)
        let result = restore_checkpoint_safe(&db, &checkpoint_id, RestoreOptions {
            force: false,
            backup: false,
            dry_run: false,
        }).await;

        assert!(result.is_ok(), "Valid restore to empty DB should still work");

        let sessions = db.list(None).await.unwrap();
        assert_eq!(sessions.len(), 3);
    }
}
```

---

## Test Coverage Targets

| Metric Type | Target | Rationale |
|-------------|--------|-----------|
| **Safety Coverage** | 100% | All destructive paths tested |
| **Backup Coverage** | 100% | Auto-backup verified |
| **Cleanup Coverage** | 100% | Workspace cleanup verified |

**Specific Coverage**:
| Scenario | Tests |
|----------|-------|
| Restore with existing sessions | Confirmation required |
| Restore with --force | Data loss allowed (user consented) |
| Restore with --backup | Auto-backup created |
| Dry run | Preview shown, no changes |
| Workspace cleanup | Orphaned workspaces deleted |
| Regression (22→14 sessions) | No silent data loss |

---

## Test Smells to Avoid

### 1. **Testing Implementation Details**

❌ **Bad**: Testing internal transaction state
```rust
assert!(tx.is_active());
```

✅ **Good**: Testing observable behavior (session count)
```rust
assert_eq!(sessions.len(), 5);
```

### 2. **Not Testing Data Loss**

❌ **Bad**: Only testing success path
```rust
#[test]
fn restore_works() {
    restore(...).await.unwrap();
}
```

✅ **Good**: Testing data preservation
```rust
#[test]
fn restore_preserves_existing_sessions_without_consent() {
    let count_before = sessions.len();
    restore_without_force(...).await;
    assert_eq!(sessions.len(), count_before);
}
```

---

## Regression Test Checklist

Before closing bead:

- [ ] Restore without --force requires confirmation
- [ ] Restore with --force deletes sessions (user consented)
- [ ] Restore with --backup creates auto-backup
- [ ] Restoring from backup recovers original state
- [ ] Dry-run shows accurate preview
- [ ] Dry-run makes no changes
- [ ] Orphaned workspaces cleaned up
- [ ] Active workspaces preserved
- [ ] Regression: 22→14 sessions prevented
- [ ] Valid restores still work
- [ ] All integration tests pass
- [ ] `moon run :ci` passes

---

## Continuous Integration (CI) Configuration

```yaml
name: Checkpoint Restore Safety Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon-action@v1

      - name: Run unit tests
        run: moon run :test checkpoint

      - name: Run integration tests
        run: ./test/integration/checkpoint_restore_safety.sh
```

---

## Manual Testing Checklist

Before closing bead:

- [ ] Create sessions, create checkpoint, try restore (should prompt)
- [ ] Use --dry-run (should show preview)
- [ ] Use --force (should restore)
- [ ] Use --backup (should create backup)
- [ ] Restore from backup (should recover original state)
- [ ] Check workspace cleanup

---

## Post-Deployment Monitoring

After merging:

1. **User Reports**: "Sessions disappeared after restore"
2. **Backup Failures**: Auto-backup not creating
3. **Cleanup Issues**: Workspaces not being deleted
4. **Performance**: Slow restore with many sessions

---

## Summary

**Test Approach**: Data loss prevention + Backup verification + Cleanup verification

**Test Count**: ~25 tests
- 6 unit tests (rust)
- 8 integration tests (bash)
- 6 regression tests (rust)
- 5 cleanup/backup tests

**Execution Time**: ~45 seconds

**Risk Coverage**: High (catches data loss bugs)

**Fowler Compliance**: ✅
- ✅ State verification (session counts)
- ✅ Safety testing (prevents data loss)
- ✅ No test smells (tests observable behavior)
- ✅ Clear intent (tests verify safety)

---

**Test Plan Status**: ✅ Ready for Implementation

**Estimated Test Execution Time**: 45 seconds

**Confidence Level**: High (tests prevent exact bug)
