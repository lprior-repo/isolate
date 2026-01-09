# Implementation Plan: zjj-9nb - SQLite State Store

**Bead**: zjj-9nb
**Status**: In Progress
**Approach**: Test-Driven Development (TDD)

## Overview

Implement persistent session state storage using SQLite with enhanced schema, status management, and recovery capabilities. This refactors the existing `db.rs` implementation to meet all bead requirements.

## Current State Analysis

### Existing Code
- `crates/zjj/src/db.rs` - Basic SessionDb with simple schema
- `crates/zjj/src/session.rs` - Session struct with validation
- Uses `anyhow::Result` (needs migration to `zjj_core::Result`)
- Simple schema: name, workspace_path, zellij_tab, created_at
- Basic CRUD operations
- Tests contain unwraps (violation of zero-unwrap rule)

### What Needs to Change
1. **Error handling**: Migrate from `anyhow` to `zjj_core::Result`
2. **Schema**: Enhanced with status, metadata, timestamps, indexes
3. **Session model**: Add status enum, additional fields
4. **API**: Match bead specification exactly
5. **Tests**: Rewrite with zero-unwrap pattern
6. **Thread safety**: Add Arc<Mutex<Connection>> for concurrency

## Architecture Decisions

### Error Handling
- **Decision**: Keep `zjj_core::Error` lightweight, no rusqlite dependency
- **Implementation**: Convert `rusqlite::Error` to `Error::DatabaseError(String)` in `db.rs`
- **Rationale**: Prevents dependency bloat in core library

### Connection Management
- **Decision**: Use `Arc<Mutex<Connection>>` instead of r2d2 connection pool
- **Rationale**:
  - Simpler for CLI tool usage patterns
  - Fewer dependencies
  - Adequate for concurrent operations
  - Meets thread-safety requirement
  - Can upgrade later if needed

### Status Transitions
- **Decision**: Define valid transitions but don't enforce at DB layer
- **Implementation**: Provide `validate_status_transition()` helper function
- **Rationale**: Separation of concerns - DB validates data, business logic validates behavior

### Recovery Strategy
- **Decision**: `rebuild_from_sessions()` takes discovered sessions as parameter
- **Rationale**: DB layer shouldn't know about JJ commands; workspace discovery is external concern

## Schema Design

```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,
    metadata TEXT  -- JSON blob for extensibility
);

CREATE INDEX idx_status ON sessions(status);
CREATE INDEX idx_name ON sessions(name);

CREATE TRIGGER update_timestamp
AFTER UPDATE ON sessions
FOR EACH ROW
BEGIN
    UPDATE sessions SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;
```

## Type Definitions

### SessionStatus Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Creating,
    Active,
    Paused,
    Completed,
    Failed,
}
```

### Enhanced Session Struct
```rust
pub struct Session {
    pub id: Option<i64>,           // None for new sessions
    pub name: String,
    pub status: SessionStatus,
    pub workspace_path: String,
    pub zellij_tab: String,        // Computed: "jjz:{name}"
    pub branch: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub last_synced: Option<u64>,
    pub metadata: Option<serde_json::Value>,
}
```

### SessionUpdate Struct
```rust
pub struct SessionUpdate {
    pub status: Option<SessionStatus>,
    pub branch: Option<String>,
    pub last_synced: Option<u64>,
    pub metadata: Option<serde_json::Value>,
}
```

## API Specification

### SessionDb Methods
```rust
impl SessionDb {
    pub fn open(path: &Path) -> Result<Self>
    pub fn create(&self, name: &str, workspace_path: &str) -> Result<Session>
    pub fn get(&self, name: &str) -> Result<Option<Session>>
    pub fn update(&self, name: &str, update: SessionUpdate) -> Result<()>
    pub fn delete(&self, name: &str) -> Result<bool>
    pub fn list(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>>
    pub fn rebuild_from_sessions(&self, sessions: Vec<Session>) -> Result<()>
}
```

### Helper Functions
```rust
pub fn validate_status_transition(from: SessionStatus, to: SessionStatus) -> Result<()>
```

## Test-Driven Development Plan

### Phase 1: Write Failing Tests (RED)

#### Test File Structure
All tests in `crates/zjj/src/db.rs` under `#[cfg(test)] mod tests`

#### Test Categories

**1. Schema Tests**
- `test_schema_has_all_columns()` - Verify all 9 columns exist
- `test_schema_has_indexes()` - Verify idx_status and idx_name
- `test_schema_has_status_check_constraint()` - Verify only 5 valid statuses
- `test_schema_has_unique_name_constraint()` - Verify UNIQUE on name
- `test_schema_has_trigger()` - Verify update_timestamp trigger exists

**2. CRUD Operations**
- `test_create_session_sets_status_creating()` - New session has Creating status
- `test_create_session_generates_id()` - Auto-increment ID works
- `test_create_session_sets_timestamps()` - created_at and updated_at set
- `test_get_session_by_name_exists()` - Returns Some(session)
- `test_get_session_by_name_missing()` - Returns None
- `test_update_session_status()` - Changes status, updates updated_at
- `test_update_session_branch()` - Updates optional branch field
- `test_update_session_metadata()` - Updates JSON metadata
- `test_delete_session_exists()` - Returns true, removes record
- `test_delete_session_missing()` - Returns false

**3. List Operations**
- `test_list_all_sessions()` - Returns all sessions
- `test_list_sessions_by_status_active()` - Filters active only
- `test_list_sessions_by_status_empty()` - No matches returns empty vec
- `test_list_sessions_ordered_by_created()` - Verifies ordering

**4. Error Handling**
- `test_create_duplicate_name_fails()` - UNIQUE constraint error
- `test_create_invalid_status_fails()` - CHECK constraint error
- `test_update_nonexistent_session_succeeds()` - UPDATE with no matches is ok
- `test_database_corruption_detected()` - Invalid DB returns error

**5. Status Transitions**
- `test_validate_transition_creating_to_active()` - Valid transition
- `test_validate_transition_creating_to_failed()` - Valid transition
- `test_validate_transition_active_to_paused()` - Valid transition
- `test_validate_transition_invalid()` - Invalid transition fails

**6. Timestamps**
- `test_updated_at_changes_on_update()` - Trigger works
- `test_created_at_immutable()` - Doesn't change on update
- `test_timestamps_are_unix_epoch()` - Verify format

**7. Concurrency**
- `test_concurrent_creates()` - Multiple threads creating different sessions
- `test_concurrent_reads()` - Multiple threads reading same session
- `test_concurrent_create_same_name()` - One succeeds, one fails with UNIQUE error

**8. Recovery**
- `test_rebuild_from_sessions_drops_old_data()` - Clears existing sessions
- `test_rebuild_from_sessions_inserts_new()` - Populates with provided sessions
- `test_rebuild_creates_fresh_schema()` - Schema recreated correctly

### Phase 2: Extend Error Types (RED)
- Add `DatabaseError(String)` to `zjj_core::Error`
- Update Display implementation
- Add test for database error formatting

### Phase 3: Implement SessionStatus (GREEN)
- Define enum with 5 variants
- Implement Display, FromStr traits
- Implement Serialize, Deserialize
- Add conversion methods

### Phase 4: Update Session Struct (GREEN)
- Add new fields
- Update constructor
- Update validation
- Fix compilation errors in existing code

### Phase 5: Implement Enhanced SessionDb (GREEN)
- Rewrite `open()` with enhanced schema
- Implement `create()` with status management
- Implement `update()` with SessionUpdate
- Implement `list()` with filtering
- Implement `rebuild_from_sessions()`
- Add `Arc<Mutex<Connection>>` wrapper

### Phase 6: Verify Tests Pass (GREEN)
- Run `moon run :test`
- Fix any failing tests
- Verify all acceptance criteria met

### Phase 7: Refactor (REFACTOR)
- Extract common test utilities
- Add documentation comments
- Review error messages
- Optimize queries if needed

### Phase 8: Integration Check
- Verify existing commands still compile
- Update any broken callers
- Check that CLI still works

## Zero-Unwrap Test Patterns

### Pattern 1: Test Functions Return Result
```rust
#[test]
fn test_operation() -> Result<()> {
    let db = SessionDb::open(&temp_path())?;
    let session = db.create("test", "/path")?;
    assert_eq!(session.name, "test");
    Ok(())
}
```

### Pattern 2: Assert on Result
```rust
#[test]
fn test_error_case() {
    let result = operation_that_fails();
    assert!(result.is_err());

    // Check specific error
    match result {
        Err(Error::DatabaseError(msg)) => assert!(msg.contains("UNIQUE")),
        _ => panic!("Expected DatabaseError"),
    }
}
```

### Pattern 3: Use unwrap_or for Verification
```rust
#[test]
fn test_retrieval() {
    let session = db.get("test").unwrap_or(None);
    assert!(session.is_some());
    assert_eq!(session.unwrap_or_default().name, "test");
}
```

## Acceptance Criteria Mapping

- [x] Claimed bead zjj-9nb
- [ ] Creates .jjz/state.db with schema → `test_schema_has_all_columns`
- [ ] CRUD operations for sessions → Multiple CRUD tests
- [ ] Status transitions: creating → active, failed on error → `test_validate_transition_*`
- [ ] Timestamps auto-updated → `test_updated_at_changes_on_update`
- [ ] Recovery from corruption → `test_rebuild_from_sessions_*`
- [ ] Thread-safe with connection management → `test_concurrent_*`

## Success Criteria

1. All tests pass with `moon run :test`
2. Zero unwraps/panics/expects in implementation
3. Zero clippy warnings
4. All bead acceptance criteria met
5. Documentation comments complete
6. Existing commands still compile and work

## Notes

- Keep changes minimal and focused
- Follow existing code style
- Maintain compatibility with existing Session usage
- Document breaking changes
- Add migration path if API changes affect callers
