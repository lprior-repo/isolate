# Error Conversion Guide

This guide explains the error conversion traits and implementations added to improve error handling ergonomics in the domain layer.

## Overview

The `domain::error_conversion` module provides comprehensive error conversions between:

1. **IdentifierError** → Aggregate errors (SessionError, BeadError, etc.)
2. **Aggregate errors** → RepositoryError
3. **BuilderError** → Aggregate and Repository errors
4. **Extension traits** for ergonomic error handling with context

## Conversion Hierarchy

```
IdentifierError (lowest level)
       ↓
Aggregate Errors (SessionError, BeadError, WorkspaceError, QueueEntryError)
       ↓
RepositoryError (highest level - for repository operations)
```

## Usage Examples

### 1. Converting IdentifierError to Aggregate Errors

```rust
use zjj_core::domain::{SessionName, identifiers::IdentifierError};
use zjj_core::domain::aggregates::session::SessionError;

fn create_session(name_str: &str) -> Result<Session, SessionError> {
    // IdentifierError automatically converts to SessionError via `?`
    let name = SessionName::parse(name_str)?;  // Returns SessionError if parse fails

    Session::new_root(id, name, branch, path)  // May return SessionError
}
```

### 2. Using Extension Traits for Explicit Conversion

```rust
use zjj_core::domain::identifiers::IdentifierError;
use zjj_core::domain::error_conversion::IdentifierErrorExt;

fn handle_identifier_error(err: IdentifierError) -> SessionError {
    // Explicit conversion using extension trait
    err.to_session_error()
}
```

### 3. Converting Aggregate Errors to RepositoryError

```rust
use zjj_core::domain::repository::{RepositoryError, SessionRepository};
use zjj_core::domain::error_conversion::AggregateErrorExt;

impl SessionRepository for MySessionRepo {
    fn save(&self, session: &Session) -> RepositoryResult<()> {
        // Domain validation may return SessionError
        session.validate()?
            .in_context("session", "save")  // Add context for RepositoryError
    }
}
```

### 4. Using Context-Preserving Conversion Methods

```rust
use zjj_core::domain::aggregates::session::SessionError;
use zjj_core::domain::error_conversion::IntoRepositoryError;

fn save_session(session: Session) -> Result<(), RepositoryError> {
    session
        .validate()  // Returns SessionError
        .map_err(|e| e.in_context("session", "save"))?;  // Convert with context
    Ok(())
}
```

### 5. Using Convenience Methods for Common Operations

```rust
use zjj_core::domain::aggregates::workspace::WorkspaceError;
use zjj_core::domain::error_conversion::AggregateErrorExt;

fn load_workspace(name: &WorkspaceName) -> Result<Workspace, RepositoryError> {
    workspace
        .validate()  // Returns WorkspaceError
        .map_err(|e| e.on_load("workspace"))?;  // "load" context added automatically
}
```

## Available Conversion Traits

### IdentifierErrorExt

Extension trait for `IdentifierError` providing conversions to all aggregate error types:

- `to_session_error()` → SessionError
- `to_workspace_error()` → WorkspaceError
- `to_bead_error()` → BeadError
- `to_queue_entry_error()` → QueueEntryError

### AggregateErrorExt

Extension trait for all aggregate errors providing context-aware conversions to RepositoryError:

- `in_context(entity, operation)` → RepositoryError
- `on_load(entity)` → RepositoryError (with "load" operation)
- `on_save(entity)` → RepositoryError (with "save" operation)
- `on_delete(entity)` → RepositoryError (with "delete" operation)

### IntoRepositoryError

Trait for converting aggregate errors to RepositoryError with custom context.

## Error Message Examples

### IdentifierError → SessionError

```
Input:  IdentifierError::Empty
Output: SessionError::CannotActivate
```

### WorkspaceError → RepositoryError

```
Input:  WorkspaceError::PathNotFound(PathBuf::from("/test"))
Output: RepositoryError::NotFound("path not found at /test during load of workspace")
```

### QueueEntryError → RepositoryError

```
Input:  QueueEntryError::AlreadyClaimed(AgentId("agent-1"))
Output: RepositoryError::Conflict("queue entry already claimed by agent-1")
```

## Complete Example: Repository Implementation

```rust
use zjj_core::domain::repository::{RepositoryError, RepositoryResult, SessionRepository};
use zjj_core::domain::{Session, SessionId, SessionName};
use zjj_core::domain::error_conversion::AggregateErrorExt;

struct InMemorySessionRepo {
    sessions: std::sync::Arc<std::sync::Mutex<Vec<Session>>>,
}

impl SessionRepository for InMemorySessionRepo {
    fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
        self.sessions
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("session", id))
    }

    fn save(&self, session: &Session) -> RepositoryResult<()> {
        // Validate the session - may return SessionError
        session.validate()
            .map_err(|e| e.on_save("session"))?;

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

        if let Some(pos) = sessions.iter().position(|s| s.id == session.id) {
            sessions[pos] = session.clone();
        } else {
            sessions.push(session.clone());
        }
        Ok(())
    }

    fn delete(&self, id: &SessionId) -> RepositoryResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

        let pos = sessions
            .iter()
            .position(|s| &s.id == id)
            .ok_or_else(|| RepositoryError::not_found("session", id))?;

        sessions.remove(pos);
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<Session>> {
        self.sessions
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))
            .map(|v| v.clone())
    }

    fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session> {
        self.sessions
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?
            .iter()
            .find(|s| &s.name == name)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("session", name))
    }

    fn get_current(&self) -> RepositoryResult<Option<Session>> {
        self.list_all().map(|sessions| sessions.first().cloned())
    }

    fn set_current(&self, id: &SessionId) -> RepositoryResult<()> {
        self.load(id).map(|_| ())
    }

    fn clear_current(&self) -> RepositoryResult<()> {
        Ok(())
    }
}
```

## Benefits

1. **Ergonomic error propagation**: Use `?` operator without manual error conversion
2. **Context preservation**: Error messages include entity and operation context
3. **Type safety**: Compiler ensures all error paths are handled
4. **Clear error messages**: Converted errors explain what went wrong
5. **Zero-unwrap**: No use of `unwrap()`, `expect()`, or `panic!()`

## Files Modified

- `/home/lewis/src/zjj/crates/zjj-core/src/domain/error_conversion.rs` - New module
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs` - Added module exports

## Testing

Run tests for the error conversion module:

```bash
cargo test -p zjj-core --lib domain::error_conversion
```

All error conversions are tested to ensure:
- Correct error type conversion
- Error message preservation
- Context addition in RepositoryError conversions
