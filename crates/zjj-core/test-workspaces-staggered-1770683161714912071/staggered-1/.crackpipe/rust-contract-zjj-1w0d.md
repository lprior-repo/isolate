# Rust Contract Specification: Lock Non-Existent Session (zjj-1w0d)

**Generated**: 2026-02-08 07:15:00 UTC
**Bead**: zjj-1w0d
**Title**: locking: Fix lock non-existent session succeeds
**Issue Type**: Bug fix (data integrity)

---

## Problem Statement

**Reported Issue**: Locks can be acquired for sessions that don't exist in the database.

**Example**:
```bash
# No session named "nonexistent" exists
$ zjj list
Sessions: 0

# But we can still lock it!
$ zjj lock nonexistent --agent-id agent-1
✓ Locked: nonexistent

# This creates an orphaned lock
$ zjj lock list
Session: nonexistent, Holder: agent-1
```

**Impact**:
- Orphaned locks accumulate
- Lock table polluted with invalid references
- Cannot reason about lock validity
- Foreign key violations possible

**Root Cause**:
The `lock()` function in `locks.rs` does NOT validate session existence:
```rust
pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
    // No check if session exists!

    sqlx::query("INSERT INTO session_locks (...) VALUES (...)")
        .execute(&self.db)
        .await?;

    Ok(LockResponse { ... })
}
```

---

## Module Structure

**Primary File**: `crates/zjj-core/src/coordination/locks.rs`

**Problematic Function**:
```rust
pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
    // Missing: Session existence check
}
```

**Related Files**:
- `crates/zjj/src/commands/lock/mod.rs` - CLI handler
- `crates/zjj/src/db.rs` - Session database operations
- `crates/zjj/src/commands/lock/tests.rs` - Lock tests

---

## Public API

**Current Signature**:
```rust
pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse>
```

**Required Behavior**:
1. Check session exists BEFORE creating lock
2. Return `SessionNotFound` error if session doesn't exist
3. Use atomic check-and-lock (transaction)
4. Cascading delete: removing session MUST auto-release locks

---

## Type Changes

**New Error Required**:
```rust
pub enum Error {
    SessionNotFound {
        session: String,
    },
    // ... existing errors
}
```

**Exit Code Mapping**: Exit code 4 for session not found

---

## Database Schema Changes

**Add Foreign Key**:
```sql
-- Current schema (broken)
CREATE TABLE IF NOT EXISTS session_locks (
    lock_id TEXT PRIMARY KEY,
    session TEXT NOT NULL UNIQUE,  -- No foreign key!
    agent_id TEXT NOT NULL,
    acquired_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
)

-- Fixed schema (with foreign key)
CREATE TABLE IF NOT EXISTS session_locks (
    lock_id TEXT PRIMARY KEY,
    session TEXT NOT NULL UNIQUE,
    agent_id TEXT NOT NULL,
    acquired_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (session) REFERENCES sessions(name) ON DELETE CASCADE
)
```

**Note**: The `sessions` table needs a primary key on `name` for foreign key to work.

---

## CLI Changes

**No CLI argument changes** - behavior fix only.

**Expected Behavior**:
```bash
# Lock non-existent session
$ zjj lock nonexistent --agent-id agent-1
Error: Session 'nonexistent' not found
$ echo $?
4

# Lock existing session
$ zjj lock existing-session --agent-id agent-1
✓ Locked: existing-session

# Delete session -> auto-releases locks
$ zjj remove existing-session
✓ Removed: existing-session
$ zjj lock list
# (no locks for existing-session)
```

---

## Error Types

**New Error**:
```rust
SessionNotFound {
    session: String,
    hint: String,  // "Use 'zjj list' to see all sessions"
}
```

---

## Performance Constraints

**Lock Acquisition**: < 50ms even with session existence check
- Query is indexed (session.name)

**Cascading Delete**: Automatic via foreign key
- No extra code needed

---

## Testing Requirements

### Session Validation Tests (Critical):

```rust
#[tokio::test]
async fn lock_nonexistent_session_returns_error() {
    let pool = test_pool().await.unwrap();
    let mgr = LockManager::new(pool);
    mgr.init().await.unwrap();

    // Try to lock non-existent session
    let result = mgr.lock("nonexistent-session", "agent-1").await;

    assert!(result.is_err());

    match result.unwrap_err() {
        Error::SessionNotFound { session, .. } => {
            assert_eq!(session, "nonexistent-session");
        }
        other => panic!("Expected SessionNotFound, got {:?}", other),
    }

    // Verify no lock was created
    let locks = mgr.get_all_locks().await.unwrap();
    assert!(locks.is_empty(), "No lock should be created for non-existent session");
}

#[tokio::test]
async fn lock_existing_session_succeeds() {
    let pool = test_pool().await.unwrap();
    let mgr = LockManager::new(pool.clone());
    mgr.init().await.unwrap();

    // Create session first
    sqlx::query("INSERT INTO sessions (name, status, workspace_path) VALUES (?, ?, ?)")
        .bind("existing-session")
        .bind("active")
        .bind("/workspace")
        .execute(&pool)
        .await
        .unwrap();

    // Now lock should succeed
    let result = mgr.lock("existing-session", "agent-1").await;

    assert!(result.is_ok());

    let locks = mgr.get_all_locks().await.unwrap();
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].session, "existing-session");
}

#[tokio::test]
async fn lock_session_then_delete_session_auto_releases_lock() {
    let pool = test_pool().await.unwrap();
    let mgr = LockManager::new(pool.clone());
    mgr.init().await.unwrap();

    // Create session
    sqlx::query("INSERT INTO sessions (name, status, workspace_path) VALUES (?, ?, ?)")
        .bind("temp-session")
        .bind("active")
        .bind("/workspace")
        .execute(&pool)
        .await
        .unwrap();

    // Lock the session
    mgr.lock("temp-session", "agent-1").await.unwrap();

    // Verify lock exists
    let locks = mgr.get_all_locks().await.unwrap();
    assert_eq!(locks.len(), 1);

    // Delete the session
    sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind("temp-session")
        .execute(&pool)
        .await
        .unwrap();

    // Lock should be auto-released via CASCADE
    let locks = mgr.get_all_locks().await.unwrap();
    assert!(locks.is_empty(), "Lock should be auto-released when session deleted");
}

#[tokio::test]
async fn lock_deleted_session_returns_error() {
    let pool = test_pool().await.unwrap();
    let mgr = LockManager::new(pool);
    mgr.init().await.unwrap();

    // Create, lock, then delete session
    sqlx::query("INSERT INTO sessions (name, status, workspace_path) VALUES (?, ?, ?)")
        .bind("delete-me")
        .bind("active")
        .bind("/workspace")
        .execute(&pool)
        .await
        .unwrap();

    mgr.lock("delete-me", "agent-1").await.unwrap();

    sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind("delete-me")
        .execute(&pool)
        .await
        .unwrap();

    // Trying to lock again should fail
    let result = mgr.lock("delete-me", "agent-2").await;
    assert!(result.is_err());
    assert!(matches!(result, Err(Error::SessionNotFound { .. })));
}
```

### Concurrency Tests:

```rust
#[tokio::test]
async fn concurrent_lock_and_delete_handled_correctly() {
    let pool = test_pool().await.unwrap();
    let mgr = LockManager::new(pool.clone());
    mgr.init().await.unwrap();

    // Create session
    sqlx::query("INSERT INTO sessions (name, status, workspace_path) VALUES (?, ?, ?)")
        .bind("race-session")
        .bind("active")
        .bind("/workspace")
        .execute(&pool)
        .await
        .unwrap();

    // Spawn concurrent lock and delete
    let mgr_clone = mgr.clone();
    let lock_task = tokio::spawn(async move {
        mgr_clone.lock("race-session", "agent-1").await
    });

    let delete_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        sqlx::query("DELETE FROM sessions WHERE name = ?")
            .bind("race-session")
            .execute(&pool)
            .await
    });

    let (lock_result, _) = tokio::join!(lock_task, delete_task);

    // Lock might succeed or fail, but shouldn't create orphaned lock
    match lock_result.unwrap() {
        Ok(_) => {},  // Lock won the race
        Err(Error::SessionNotFound { .. }) => {},  // Delete won the race
        other => panic!("Unexpected result: {:?}", other),
    }

    // No orphaned locks should exist
    let locks = mgr.get_all_locks().await.unwrap();
    if locks.iter().any(|l| l.session == "race-session") {
        // If lock exists, session should also exist
        let session_exists: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM sessions WHERE name = ?)")
            .bind("race-session")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert!(session_exists, "Lock for race-session exists but session doesn't!");
    }
}
```

---

## Implementation Checklist

- [ ] Add `SessionNotFound` error variant
- [ ] Add session existence check in `lock()`
- [ ] Add foreign key to `session_locks` table
- [ ] Ensure `sessions.name` is indexed/unique
- [ ] Add cascading delete support
- [ ] Add unit tests for validation
- [ ] Add concurrency tests
- [ ] Update error messages
- [ ] Verify exit codes

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let exists = sqlx::query("SELECT 1 FROM sessions WHERE name = ?")
    .bind(session)
    .fetch_one(&pool)
    .await
    .unwrap();

// ✅ REQUIRED
let exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM sessions WHERE name = ?")
    .bind(session)
    .fetch_optional(&pool)
    .await
    .map_err(|e| Error::DatabaseError(e.to_string()))?;
```

---

## Success Criteria

1. Lock on non-existent session returns `SessionNotFound` error
2. Exit code 4 for non-existent session
3. Lock on existing session succeeds
4. Deleting session auto-releases all locks
5. No orphaned locks possible
6. All tests pass

---

## Verification Steps

Before closing bead:

```bash
# 1. Test lock non-existent session
zjj lock nonexistent --agent-id agent-1
echo $?
# Should be 4

# 2. Test lock existing session
zjj add test-session
zjj lock test-session --agent-id agent-1
# Should succeed

# 3. Test cascading delete
zjj remove test-session
zjj lock list
# Should show no locks for test-session

# 4. Run tests
moon run :test lock

# 5. Full CI
moon run :ci
```

---

## Related Beads

- zjj-26pf: Fix session status never updates
- zjj-1nyz: Ensure consistent session counting
- zjj-2qj5: Create lock tables during init

---

**Contract Status**: Ready for Implementation

**Estimated Resolution Time**: 1.5 hours (add validation + foreign key + tests)

**Risk Level**: Medium (database schema change)
