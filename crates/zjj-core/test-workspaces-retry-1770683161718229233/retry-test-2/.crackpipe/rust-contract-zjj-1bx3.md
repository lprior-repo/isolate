# Rust Contract Specification: Ghost Sessions After Corruption (zjj-1bx3)

**Generated**: 2026-02-08 06:44:00 UTC
**Bead**: zjj-1bx3
**Title**: database: Fix ghost sessions after corruption
**Issue Type**: Bug fix

---

## Problem Statement

After manual database corruption, "ghost sessions" appear in listings that don't actually exist in the database.

**Current Behavior**:
- Database corruption occurs (e.g., incomplete write, manual edit)
- Sessions list shows entries that don't exist in database
- User tries to interact with ghost session → confusing errors
- No way to clean up these invalid entries

**Expected Behavior**:
- Database should validate session existence before listing
- Invalid/corrupt entries should be filtered out
- Optional: Auto-repair mechanism to clean corruption

**Root Cause**:
Missing validation in session listing - queries return rows without verifying data integrity.

---

## Module Structure

**File**: `crates/bead-kv/src/store.rs` (database operations)

**Changes Required**:
1. Add validation layer to session listing queries
2. Filter out corrupt/invalid session entries
3. Add error recovery mechanism (optional)
4. Add tests for corruption scenarios

---

## Public API Changes

**Before (Vulnerable)**:
```rust
pub async fn list_sessions(&self) -> Result<Vec<Session>> {
    // Just returns all rows, no validation
    let sessions = sqlx::query_as("SELECT * FROM sessions")
        .fetch_all(&self.pool)
        .await?;
    Ok(sessions)
}
```

**After (Resilient)**:
```rust
pub async fn list_sessions(&self) -> Result<Vec<Session>> {
    let sessions = sqlx::query_as("SELECT * FROM sessions")
        .fetch_all(&self.pool)
        .await?;

    // Validate each session before returning
    sessions
        .into_iter()
        .filter_map(|session| {
            match validate_session(&session) {
                Ok(_) => Some(session),
                Err(e) => {
                    log::warn!("Filtering corrupt session: {}", e);
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|session| session.ok_or_else(|| anyhow::anyhow!("Invalid session")))
        .collect()
}
```

---

## Type Changes

### New Validation Function:

```rust
/// Validate session data integrity
fn validate_session(session: &SessionRow) -> Result<(), SessionError> {
    // Check required fields
    if session.name.is_empty() {
        return Err(SessionError::InvalidName("Empty session name".to_string()));
    }

    if session.workspace_path.is_empty() {
        return Err(SessionError::InvalidPath("Empty workspace path".to_string()));
    }

    // Check path exists (optional, may be expensive)
    // if !std::path::Path::new(&session.workspace_path).exists() {
    //     return Err(SessionError::PathNotFound(session.workspace_path.clone()));
    // }

    // Check JSON fields are valid (if any)
    if let Some(metadata) = &session.metadata {
        serde_json::from_str::<serde_json::Value>(metadata)
            .map_err(|_| SessionError::InvalidMetadata)?;
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Invalid session name: {0}")]
    InvalidName(String),

    #[error("Invalid workspace path: {0}")]
    InvalidPath(String),

    #[error("Workspace path not found: {0}")]
    PathNotFound(String),

    #[error("Invalid metadata JSON")]
    InvalidMetadata,
}
```

---

## CLI Changes

**No changes** - this is internal database validation.

However, consider adding verbose mode to show filtered sessions:

```bash
zjj list --verbose
# Output:
# Sessions:
#   feature-x (active)
#   bug-fix
# Filtered 2 corrupt sessions (use --show-corrupt to see)
```

---

## Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Session validation failed: {0}")]
    ValidationFailed(String),

    #[error("Corrupt session filtered: {session_name}")]
    CorruptSession { session_name: String },

    #[error("Database corrupted, {count} invalid sessions found")]
    DatabaseCorrupted { count: usize },
}
```

---

## Performance Constraints

- Validation must not significantly slow down session listing
- Target: <50ms for 100 sessions (including validation)
- Path existence checks are expensive → make optional or skip

---

## Testing Requirements

### Unit Tests Required:

1. **Valid session passes validation**:
   ```rust
   #[test]
   fn validate_session_accepts_valid_data() {
       let session = SessionRow {
           name: "test-session".to_string(),
           workspace_path: "/tmp/workspace".to_string(),
           metadata: Some(r#"{"created":"2024-01-01"}"#.to_string()),
       };

       assert!(validate_session(&session).is_ok());
   }
   ```

2. **Empty session name rejected**:
   ```rust
   #[test]
   fn validate_session_rejects_empty_name() {
       let session = SessionRow {
           name: "".to_string(),
           workspace_path: "/tmp/workspace".to_string(),
           metadata: None,
       };

       assert!(matches!(
           validate_session(&session),
           Err(SessionError::InvalidName(_))
       ));
   }
   ```

3. **Empty workspace path rejected**:
   ```rust
   #[test]
   fn validate_session_rejects_empty_path() {
       let session = SessionRow {
           name: "test".to_string(),
           workspace_path: "".to_string(),
           metadata: None,
       };

       assert!(matches!(
           validate_session(&session),
           Err(SessionError::InvalidPath(_))
       ));
   }
   ```

4. **Invalid metadata JSON rejected**:
   ```rust
   #[test]
   fn validate_session_rejects_invalid_metadata() {
       let session = SessionRow {
           name: "test".to_string(),
           workspace_path: "/tmp/workspace".to_string(),
           metadata: Some("not json".to_string()),
       };

       assert!(matches!(
           validate_session(&session),
           Err(SessionError::InvalidMetadata)
       ));
   }
   ```

5. **List sessions filters corrupt entries**:
   ```rust
   #[tokio::test]
   async fn list_sessions_filters_corrupt_entries() {
       let db = setup_test_db().await;

       // Insert valid session
       db.create("valid-session", "/tmp/workspace").await.unwrap();

       // Insert corrupt session directly (bypass validation)
       sqlx::query("INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp')")
           .execute(&db.pool)
           .await
           .unwrap();

       // List should only return valid session
       let sessions = db.list_sessions().await.unwrap();
       assert_eq!(sessions.len(), 1);
       assert_eq!(sessions[0].name, "valid-session");
   }
   ```

### Integration Tests Required:

1. **Corruption recovery test**:
   ```bash
   #!/bin/bash
   # test/integration/corruption_recovery.sh

   set -euo pipefail

   # Create test database
   TEST_DB=$(mktemp)
   zjj --db "$TEST_DB" add test-session

   # Corrupt database (insert invalid row)
   sqlite3 "$TEST_DB" "INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp')"

   # List should not crash and should filter corrupt entry
   OUTPUT=$(zjj --db "$TEST_DB" list)
   if [[ "$OUTPUT" == *"test-session"* ]] && [[ "$OUTPUT" != *""* ]]; then
       echo "✓ Corrupt sessions filtered"
   else
       echo "✗ Corrupt sessions not filtered properly"
       exit 1
   fi

   rm -f "$TEST_DB"
   ```

2. **Multiple corrupt entries handled**:
   ```bash
   # Insert multiple corrupt entries
   sqlite3 "$TEST_DB" "INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp1')"
   sqlite3 "$TEST_DB" "INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp2')"
   sqlite3 "$TEST_DB" "INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp3')"

   # Should still work
   zjj --db "$TEST_DB" list
   ```

---

## Migration Guide

**No migration needed** - this is a bug fix that improves behavior.

**Users affected**: Those with corrupt databases

**Action**: No action required - corrupt sessions will be silently filtered.

**Optional**: Provide command to clean corrupt sessions:
```bash
zjj cleanup --remove-corrupt
# Removes all invalid session entries from database
```

---

## Implementation Checklist

- [ ] Add `validate_session()` function
- [ ] Update `list_sessions()` to validate each entry
- [ ] Add warning logging for filtered sessions
- [ ] Add unit tests for validation
- [ ] Add integration tests for corruption scenarios
- [ ] Test with manually corrupted database
- [ ] Add `--verbose` flag to show filtered count (optional)
- [ ] Add `cleanup --remove-corrupt` command (optional)
- [ ] Run `moon run :quick` (6-7ms)
- [ ] Run `moon run :test` (all tests pass)
- [ ] Run `moon run :ci` (full pipeline)

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let session = sessions.get(0).unwrap();

// ✅ REQUIRED
sessions
    .into_iter()
    .filter_map(|s| validate_session(&s).ok())
    .collect::<Vec<_>>()
```

**Validation logic should use Result types**:
```rust
fn validate_session(session: &SessionRow) -> Result<(), SessionError> {
    if session.name.is_empty() {
        return Err(SessionError::InvalidName("empty".to_string()));
    }
    Ok(())
}
```

---

## Success Criteria

1. Corrupt session entries don't appear in listings
2. Valid sessions still work normally
3. No crashes when database has corrupt entries
4. Logging shows filtered sessions (in verbose mode)
5. All tests pass, including corruption scenarios

---

## Future Enhancements (Out of Scope)

- Auto-repair: Attempt to fix corrupt entries
- Corruption detection: Periodic database health checks
- Backup/restore: Database snapshot before risky operations
- Consistency check: `zjj doctor --check-db-integrity`

---

**Contract Status**: ✅ Ready for Builder

**Estimated Implementation Time**: 2 hours (validation logic + tests)

**Risk Level**: Medium (database changes, but improves robustness)
