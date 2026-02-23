# Error Handling Guide

Comprehensive guide to idiomatic, zero-panic error handling in the zjj codebase following Domain-Driven Design (DDD) principles.

## Table of Contents

1. [Core Principles](#core-principles)
2. [Error Type Hierarchy](#error-type-hierarchy)
3. [IdentifierError](#identifiererror)
4. [Aggregate Errors](#aggregate-errors)
5. [RepositoryError](#repositoryerror)
6. [BuilderError](#buildererror)
7. [Error Conversion Patterns](#error-conversion-patterns)
8. [Context Preservation](#context-preservation)
9. [Recovery Strategies](#recovery-strategies)
10. [Testing Error Cases](#testing-error-cases)
11. [Common Pitfalls](#common-pitfalls)
12. [Best Practices](#best-practices)

---

## Core Principles

### Zero-Panic, Zero-Unwrap Philosophy

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
```

**Rules:**
- Never use `unwrap()`, `expect()`, `unwrap_or()`, `unwrap_or_else()`, `unwrap_or_default()`
- Never use `panic!()`, `todo!()`, `unimplemented!()`
- Always use `Result<T, E>` for fallible operations
- Use `match`, `if let`, `map()`, `and_then()`, `?` for error handling

### Railway-Oriented Programming

Treat error handling as a railway track: success continues forward, errors exit early.

```rust
fn process(name_str: &str) -> Result<Session> {
    let name = SessionName::parse(name_str)?;     // Exit on error
    let workspace = Workspace::load(&name)?;      // Exit on error
    let session = Session::new_root(id, name, workspace.path)?;
    Ok(session)                                    // Success continues
}
```

---

## Error Type Hierarchy

```
Layer 1: Infrastructure (Shell)
         └── anyhow::Error (with .context())

Layer 2: Repository Pattern
         └── RepositoryError

Layer 3: Domain Aggregates
         ├── SessionError
         ├── WorkspaceError
         ├── BeadError
         └── QueueEntryError

Layer 4: Value Objects
         └── IdentifierError

Layer 5: Builders
         └── BuilderError
```

### Error Conversion Flow

```
IdentifierError
       ↓ (From impls)
Aggregate Errors (SessionError, BeadError, etc.)
       ↓ (From impls + context)
RepositoryError
       ↓ (with .context())
anyhow::Error (shell layer)
```

---

## IdentifierError

**Location:** `crates/zjj-core/src/domain/identifiers.rs`

**Purpose:** Validation errors for domain identifiers (SessionName, AgentId, TaskId, etc.).

### Error Variants

```rust
pub enum IdentifierError {
    /// Identifier is empty or whitespace-only
    Empty,

    /// Identifier exceeds maximum length
    TooLong { max: usize, actual: usize },

    /// Identifier contains invalid characters
    InvalidCharacters { details: String },

    /// Identifier format is invalid (generic)
    InvalidFormat { details: String },

    /// Identifier must start with a letter
    InvalidStart { expected: char },

    /// Identifier has invalid prefix (e.g., must start with "bd-")
    InvalidPrefix { prefix: &'static str, value: String },

    /// Identifier hex format is invalid
    InvalidHex { value: String },

    /// Path is not absolute
    NotAbsolutePath { value: String },

    /// Path contains null bytes
    NullBytesInPath,

    /// Identifier must be ASCII
    NotAscii { value: String },

    /// Identifier contains path separators
    ContainsPathSeparators,
}
```

### When to Use IdentifierError

**Use when:**
- Parsing and validating domain identifiers at boundaries
- Validating user input for names, IDs, paths
- Creating value objects from raw strings

**Example:**

```rust
use zjj_core::domain::{SessionName, IdentifierError};

fn parse_session_name(input: &str) -> Result<SessionName, IdentifierError> {
    SessionName::parse(input)  // Returns IdentifierError
}

// Usage
match parse_session_name("my-session") {
    Ok(name) => println!("Valid: {}", name),
    Err(IdentifierError::Empty) => eprintln!("Name cannot be empty"),
    Err(IdentifierError::TooLong { max, actual }) => {
        eprintln!("Too long: {} chars (max {})", actual, max)
    }
    Err(IdentifierError::InvalidStart { .. }) => {
        eprintln!("Must start with a letter")
    }
    Err(e) => eprintln!("Invalid: {}", e),
}
```

### Helper Methods

```rust
impl IdentifierError {
    pub const fn empty() -> Self { Self::Empty }
    pub const fn too_long(max: usize, actual: usize) -> Self { Self::TooLong { max, actual } }
    pub fn invalid_characters(details: impl Into<String>) -> Self { /* ... */ }
    pub fn invalid_format(details: impl Into<String>) -> Self { /* ... */ }
    pub const fn invalid_start(expected: char) -> Self { Self::InvalidStart { expected } }
    pub fn invalid_prefix(prefix: &'static str, value: impl Into<String>) -> Self { /* ... */ }
}
```

### Module-Specific Aliases

For backward compatibility and semantic clarity:

```rust
pub type SessionNameError = IdentifierError;
pub type AgentIdError = IdentifierError;
pub type WorkspaceNameError = IdentifierError;
pub type TaskIdError = IdentifierError;
pub type BeadIdError = IdentifierError;
pub type SessionIdError = IdentifierError;
pub type AbsolutePathError = IdentifierError;
```

---

## Aggregate Errors

Aggregate errors represent business rule violations within domain aggregates.

### SessionError

**Location:** `crates/zjj-core/src/domain/aggregates/session.rs`

```rust
pub enum SessionError {
    /// Invalid branch transition
    InvalidBranchTransition { from: BranchState, to: BranchState },

    /// Invalid parent transition
    InvalidParentTransition { from: ParentState, to: ParentState },

    /// Workspace path does not exist
    WorkspaceNotFound(PathBuf),

    /// Session is not active
    NotActive,

    /// Cannot activate session with invalid state
    CannotActivate,

    /// Cannot modify root session parent
    CannotModifyRootParent,

    /// Session name conflicts with existing session
    NameAlreadyExists(SessionName),
}
```

**When to use:**
- Enforcing session state transitions
- Validating session operations
- Checking workspace existence
- Preventing name conflicts

**Example:**

```rust
use zjj_core::domain::aggregates::session::{Session, SessionError};
use zjj_core::domain::session::BranchState;

impl Session {
    pub fn transition_branch(&self, new_branch: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new_branch) {
            return Err(SessionError::InvalidBranchTransition {
                from: self.branch.clone(),
                to: new_branch,
            });
        }

        Ok(Self {
            branch: new_branch,
            ..self.clone()
        })
    }
}
```

### WorkspaceError

```rust
pub enum WorkspaceError {
    /// Invalid state transition
    InvalidStateTransition { from: WorkspaceState, to: WorkspaceState },

    /// Path does not exist
    PathNotFound(PathBuf),

    /// Workspace is not ready
    NotReady(WorkspaceState),

    /// Workspace is not active
    NotActive(WorkspaceState),

    /// Workspace has been removed
    Removed,

    /// Cannot use workspace in current state
    CannotUse(WorkspaceState),

    /// Workspace name already exists
    NameAlreadyExists(WorkspaceName),
}
```

### BeadError

```rust
pub enum BeadError {
    /// Invalid title
    InvalidTitle(String),

    /// Invalid description
    InvalidDescription(String),

    /// Invalid state transition
    InvalidStateTransition { from: BeadState, to: BeadState },

    /// Cannot modify closed bead
    CannotModifyClosed,

    /// Timestamps are not monotonic
    NonMonotonicTimestamps { created_at: DateTime<Utc>, updated_at: DateTime<Utc> },

    /// Title is required
    TitleRequired,

    /// Domain error from beads module
    Domain(#[from] DomainError),
}
```

### QueueEntryError

```rust
pub enum QueueEntryError {
    /// Invalid claim transition
    InvalidClaimTransition { from: ClaimState, to: ClaimState },

    /// Queue entry is not claimed
    NotClaimed,

    /// Queue entry already claimed
    AlreadyClaimed(AgentId),

    /// Not the owner of the claim
    NotOwner { actual: AgentId, expected: AgentId },

    /// Claim has expired
    ClaimExpired,

    /// Invalid expiration time
    InvalidExpiration,

    /// Negative priority
    NegativePriority,

    /// Cannot modify entry in state
    CannotModify(ClaimState),
}
```

---

## RepositoryError

**Location:** `crates/zjj-core/src/domain/repository.rs`

**Purpose:** Errors from repository operations (persistence layer).

### Error Variants

```rust
pub enum RepositoryError {
    /// Entity not found in repository
    NotFound(String),

    /// Conflict with existing data (duplicate, constraint violation)
    Conflict(String),

    /// Invalid input for domain operation
    InvalidInput(String),

    /// Underlying storage failure
    StorageError(String),

    /// Operation not supported by repository
    NotSupported(String),

    /// Concurrent modification conflict
    ConcurrentModification(String),
}
```

### Constructor Methods

```rust
impl RepositoryError {
    pub fn not_found(entity: &str, id: impl Display) -> Self {
        Self::NotFound(format!("{entity} '{id}'"))
    }

    pub fn conflict(reason: impl Into<String>) -> Self {
        Self::Conflict(reason.into())
    }

    pub fn invalid_input(reason: impl Into<String>) -> Self {
        Self::InvalidInput(reason.into())
    }

    pub fn storage_error(reason: impl Into<String>) -> Self {
        Self::StorageError(reason.into())
    }
}
```

### When to Use RepositoryError

**Use when:**
- Implementing repository traits
- Wrapping storage errors (SQLite, file I/O, network)
- Converting domain errors for repository operations
- Handling concurrent modification

**Example:**

```rust
use zjj_core::domain::repository::{SessionRepository, RepositoryError, RepositoryResult};
use zjj_core::domain::{Session, SessionId};

struct SqliteSessionRepo {
    db: Arc<Mutex<Connection>>,
}

impl SessionRepository for SqliteSessionRepo {
    fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
        self.db
            .lock()
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?
            .query_row("SELECT * FROM sessions WHERE id = ?", [id.as_str()], |row| {
                // Parse session from row
                Ok(session)
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    RepositoryError::not_found("session", id)
                }
                other => RepositoryError::StorageError(other.to_string()),
            })
    }

    fn save(&self, session: &Session) -> RepositoryResult<()> {
        // Validate before saving
        session.validate()
            .map_err(|e| e.on_save("session"))?;

        // ... persist to database
        Ok(())
    }
}
```

---

## BuilderError

**Location:** `crates/zjj-core/src/domain/builders.rs`

**Purpose:** Errors from type-safe builder operations.

### Error Variants

```rust
pub enum BuilderError {
    /// Required field not set
    MissingRequired { field: &'static str },

    /// Invalid value provided
    InvalidValue { field: &'static str, reason: String },

    /// Collection overflow
    Overflow { field: &'static str, capacity: usize },

    /// Invalid state transition
    InvalidTransition {
        from: &'static str,
        to: &'static str,
        reason: String,
    },
}
```

### When to Use BuilderError

**Use when:**
- Implementing type-safe builders
- Validating required fields before construction
- Enforcing field capacity limits
- Building complex aggregates

**Example:**

```rust
use zjj_core::domain::builders::{SessionOutputBuilder, BuilderError};

let session = SessionOutputBuilder::new()
    .name("my-session")?
    .status(SessionStatus::Active)
    .state(WorkspaceState::Active)
    .workspace_path("/path/to/workspace")?
    .build()?;
```

---

## Error Conversion Patterns

The codebase uses a hierarchy of `From` implementations to enable ergonomic error propagation with the `?` operator.

### IdentifierError → Aggregate Errors

**Location:** `crates/zjj-core/src/domain/error_conversion.rs`

```rust
impl From<IdentifierError> for SessionError {
    fn from(err: IdentifierError) -> Self {
        match &err {
            IdentifierError::Empty => SessionError::CannotActivate,
            IdentifierError::TooLong { .. } | IdentifierError::InvalidCharacters { .. } => {
                SessionError::CannotActivate
            }
            _ => SessionError::CannotActivate,
        }
    }
}
```

**Usage:**

```rust
use zjj_core::domain::{SessionName, IdentifierError};
use zjj_core::domain::aggregates::session::SessionError;

fn create_session(name_str: &str) -> Result<Session, SessionError> {
    // IdentifierError automatically converts to SessionError via `?`
    let name = SessionName::parse(name_str)?;  // Returns SessionError if parse fails
    Session::new_root(id, name, branch, path)  // May return SessionError
}
```

### Aggregate Errors → RepositoryError

```rust
impl From<SessionError> for RepositoryError {
    fn from(err: SessionError) -> Self {
        match &err {
            SessionError::InvalidBranchTransition { from, to } => {
                RepositoryError::InvalidInput(format!("invalid branch transition: {from:?} -> {to:?}"))
            }
            SessionError::WorkspaceNotFound(path) => {
                RepositoryError::NotFound(format!("workspace not found: {}", path.display()))
            }
            SessionError::NotActive => {
                RepositoryError::InvalidInput("session is not active".into())
            }
            SessionError::NameAlreadyExists(name) => {
                RepositoryError::Conflict(format!("session name already exists: {name}"))
            }
            // ... other variants
        }
    }
}
```

**Usage:**

```rust
use zjj_core::domain::repository::{SessionRepository, RepositoryError};
use zjj_core::domain::aggregates::session::SessionError;

impl SessionRepository for MyRepo {
    fn save(&self, session: &Session) -> RepositoryResult<()> {
        // SessionError automatically converts to RepositoryError
        session.validate()?;

        // ... persist to storage
        Ok(())
    }
}
```

### Extension Traits for Context-Aware Conversion

```rust
pub trait AggregateErrorExt {
    /// Convert to RepositoryError with entity and operation context
    fn in_context(self, entity: &str, operation: &str) -> RepositoryError;

    /// Convert to RepositoryError for load operations
    fn on_load(self, entity: &str) -> RepositoryError;

    /// Convert to RepositoryError for save operations
    fn on_save(self, entity: &str) -> RepositoryError;

    /// Convert to RepositoryError for delete operations
    fn on_delete(self, entity: &str) -> RepositoryError;
}

impl<E> AggregateErrorExt for E
where
    E: IntoRepositoryError,
{
    fn on_save(self, entity: &str) -> RepositoryError {
        self.into_repository_error(entity, "save")
    }

    // ... other methods
}
```

**Usage:**

```rust
session.validate()
    .map_err(|e| e.on_save("session"))?;

workspace.validate()
    .map_err(|e| e.on_load("workspace"))?;

bead.validate()
    .map_err(|e| e.in_context("bead", "update"))?;
```

---

## Context Preservation

### Adding Context to Errors

When errors propagate across layers, preserve and add context about what operation failed.

#### Pattern 1: Using Extension Traits

```rust
use zjj_core::domain::error_conversion::AggregateErrorExt;

fn save_session(session: Session) -> Result<(), RepositoryError> {
    session
        .validate()
        .map_err(|e| e.on_save("session"))?;
    // ... persist
    Ok(())
}
```

#### Pattern 2: Using `IntoRepositoryError`

```rust
use zjj_core::domain::error_conversion::IntoRepositoryError;

fn save_workspace(workspace: Workspace) -> Result<(), RepositoryError> {
    workspace
        .validate()
        .map_err(|e| e.into_repository_error("workspace", "save"))?;
    Ok(())
}
```

#### Pattern 3: Manual Context Addition

```rust
fn load_bead(id: &BeadId) -> Result<Bead, RepositoryError> {
    repo.load(id)
        .map_err(|e| RepositoryError::NotFound(format!(
            "failed to load bead {}: {}",
            id, e
        )))?
}
```

### Context-Aware Error Messages

Good error messages answer:
1. **What** operation failed?
2. **Which** entity was involved?
3. **Why** did it fail?

```rust
// Bad - no context
Err(RepositoryError::NotFound("not found".into()))

// Good - specific context
Err(RepositoryError::NotFound(format!(
    "session 'my-session' not found during load"
)))

// Better - with suggestions
Err(RepositoryError::NotFound(format!(
    "session 'my-session' not found. Available sessions: {}",
    available.join(", ")
)))
```

---

## Recovery Strategies

### Strategy 1: Early Return with `?`

```rust
fn process(name: &str) -> Result<Session> {
    let parsed_name = SessionName::parse(name)?;  // Exit on error
    let workspace = load_workspace(&parsed_name)?;
    Ok(create_session(parsed_name, workspace)?)
}
```

### Strategy 2: Match and Recover

```rust
fn find_or_create_session(name: &str) -> Result<Session> {
    match repo.load_by_name(&SessionName::parse(name)?) {
        Ok(session) => Ok(session),
        Err(RepositoryError::NotFound(_)) => {
            // Recover by creating new session
            create_new_session(name)
        }
        Err(e) => Err(e),  // Propagate other errors
    }
}
```

### Strategy 3: Combinator Chains

```rust
use itertools::Itertools;

fn process_sessions(names: Vec<&str>) -> Result<Vec<Session>> {
    names
        .into_iter()
        .map(|n| SessionName::parse(n))
        .collect::<Result<Vec<_>, _>>()?  // Fail fast on first error
        .into_iter()
        .map(|n| repo.load_by_name(&n))
        .collect::<Result<Vec<_>, _>>()?  // Fail fast on first error
        .into_iter()
        .filter(|s| s.is_active())
        .collect()
}
```

### Strategy 4: Collect All Errors

```rust
fn validate_all(names: &[&str]) -> Vec<Result<SessionName>> {
    names
        .iter()
        .map(|n| SessionName::parse(n))
        .collect()
}

// Usage
let results = validate_all(&["valid", "", "123invalid"]);
for result in results {
    match result {
        Ok(name) => println!("Valid: {}", name),
        Err(e) => eprintln!("Invalid: {}", e),
    }
}
```

### Strategy 5: Fallback Values

```rust
fn get_session_name() -> SessionName {
    std::env::var("SESSION_NAME")
        .ok()
        .and_then(|s| SessionName::parse(s).ok())
        .unwrap_or_else(|| SessionName::parse("default").expect("valid"))
}
```

---

## Testing Error Cases

### Unit Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zjj_core::domain::{SessionName, IdentifierError};

    #[test]
    fn test_empty_name_returns_error() {
        let result = SessionName::parse("");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[test]
    fn test_too_long_name_returns_error() {
        let long_name = "a".repeat(100);
        let result = SessionName::parse(&long_name);
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::TooLong { max: 63, .. })));
    }

    #[test]
    fn test_invalid_start_character_returns_error() {
        let result = SessionName::parse("123session");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::InvalidStart { .. })));
    }

    #[test]
    fn test_valid_name_succeeds() {
        let result = SessionName::parse("valid-session-123");
        assert!(result.is_ok());
    }
}
```

### Property-Based Testing

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_empty_always_fails(s in "") {
            let result = SessionName::parse(&s);
            assert!(matches!(result, Err(IdentifierError::Empty)));
        }

        #[test]
        fn prop_too_long_fails(max_len in 64usize..1000) {
            let long_name = "a".repeat(max_len);
            let result = SessionName::parse(&long_name);
            assert!(matches!(result, Err(IdentifierError::TooLong { .. })));
        }

        #[test]
        fn prop_valid_alphanumeric_passes(
            prefix in "[a-zA-Z]",
            rest in "[a-zA-Z0-9_-]{0,62}"
        ) {
            let name = format!("{}{}", prefix, rest);
            let result = SessionName::parse(&name);
            prop_assert!(result.is_ok());
        }
    }
}
```

### Error Conversion Tests

```rust
#[test]
fn test_identifier_error_converts_to_session_error() {
    let err = IdentifierError::Empty;
    let session_err: SessionError = err.into();
    assert!(matches!(session_err, SessionError::CannotActivate));
}

#[test]
fn test_session_error_converts_to_repository_error() {
    let name = SessionName::parse("test").expect("valid");
    let err = SessionError::NameAlreadyExists(name);
    let repo_err: RepositoryError = err.into();
    assert!(matches!(repo_err, RepositoryError::Conflict(_)));
}
```

### Integration Test Pattern

```rust
#[tokio::test]
async fn test_repository_returns_not_found() {
    let repo = create_test_repo();
    let id = SessionId::parse("nonexistent").expect("valid");

    let result = repo.load(&id);
    assert!(matches!(result, Err(RepositoryError::NotFound(_))));
}

#[tokio::test]
async fn test_repository_handles_conflict() {
    let repo = create_test_repo();
    let name = SessionName::parse("test").expect("valid");

    // First save succeeds
    let session1 = create_test_session(&name);
    repo.save(&session1).expect("save succeeds");

    // Duplicate name fails
    let session2 = create_test_session(&name);
    let result = repo.save(&session2);
    assert!(matches!(result, Err(RepositoryError::Conflict(_))));
}
```

---

## Common Pitfalls

### Pitfall 1: Using `unwrap()` in Tests

```rust
// Bad - even in tests
#[test]
fn test_something() {
    let name = SessionName::parse("test").unwrap();  // COMPILE ERROR
}

// Good - use proper error handling
#[test]
fn test_something() {
    let name = SessionName::parse("test").expect("valid name should parse");
}
```

### Pitfall 2: Ignoring Error Context

```rust
// Bad - loses context
fn load_session(id: &SessionId) -> Result<Session> {
    repo.load(id).map_err(|_| RepositoryError::NotFound("load failed".into()))?
}

// Good - preserves context
fn load_session(id: &SessionId) -> Result<Session> {
    repo.load(id)
        .map_err(|e| RepositoryError::NotFound(format!("failed to load session {}: {}", id, e)))?
}
```

### Pitfall 3: Using String Error Types

```rust
// Bad - stringly-typed errors
fn parse_name(s: &str) -> Result<SessionName, String> {
    if s.is_empty() {
        return Err("name cannot be empty".into());
    }
    // ...
}

// Good - typed errors
fn parse_name(s: &str) -> Result<SessionName, IdentifierError> {
    SessionName::parse(s)
}
```

### Pitfall 4: Panic on Invalid Data

```rust
// Bad - panics on invalid data
fn process(data: &[u8]) -> Session {
    let session: Session = bincode::deserialize(data).unwrap();
    session
}

// Good - returns error
fn process(data: &[u8]) -> Result<Session, BincodeError> {
    let session: Session = bincode::deserialize(data)?;
    Ok(session)
}
```

### Pitfall 5: Not Validating at Boundaries

```rust
// Bad - validates deep in logic
fn create_session(raw_name: String) -> Session {
    if raw_name.is_empty() {
        panic!("name cannot be empty");
    }
    Session { name: raw_name }
}

// Good - validates at boundary
fn create_session(name: SessionName) -> Session {
    Session { name }
}

// Caller validates
let name = SessionName::parse(raw_name)?;
let session = create_session(name);
```

### Pitfall 6: Converting Errors to Strings Too Early

```rust
// Bad - loses error type information
fn load_session(id: &SessionId) -> Result<Session, String> {
    repo.load(id)
        .map_err(|e| e.to_string())?
}

// Good - preserves error type
fn load_session(id: &SessionId) -> Result<Session, RepositoryError> {
    repo.load(id)?
}
```

---

## Best Practices

### 1. Parse at Boundaries

Validate input as soon as it enters the system:

```rust
// API boundary
pub async fn create_session_handler(name: String) -> Result<Json<Session>> {
    let validated_name = SessionName::parse(name)?;  // Validate immediately
    let session = create_session(validated_name).await?;
    Ok(Json(session))
}
```

### 2. Use Domain Types in Signatures

```rust
// Bad - uses primitives
fn save_session(id: &str, name: &str, path: &str) -> Result<()> {
    // ...
}

// Good - uses domain types
fn save_session(
    id: &SessionId,
    name: &SessionName,
    path: &AbsolutePath,
) -> Result<()> {
    // ...
}
```

### 3. Return Specific Errors

```rust
// Bad - generic error
fn load_workspace(path: &Path) -> Result<Workspace> {
    if !path.exists() {
        return Err(anyhow::anyhow!("workspace not found"));
    }
    // ...
}

// Good - specific error
fn load_workspace(path: &AbsolutePath) -> Result<Workspace, WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::PathNotFound(path.to_path_buf()));
    }
    // ...
}
```

### 4. Document Error Conditions

```rust
/// Load a session by ID.
///
/// # Errors
///
/// Returns `NotFound` if no session with the given ID exists.
/// Returns `StorageError` on database/file access failure.
fn load(&self, id: &SessionId) -> RepositoryResult<Session>;
```

### 5. Use Result Type Aliases

```rust
// In repository module
pub type RepositoryResult<T> = Result<T, RepositoryError>;

// In aggregate modules
pub type SessionResult<T> = Result<T, SessionError>;
pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

// Usage
fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
    // ...
}
```

### 6. Implement Display for User-Facing Errors

```rust
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBranchTransition { from, to } => {
                write!(f, "Cannot transition branch from {:?} to {:?}", from, to)
            }
            Self::WorkspaceNotFound(path) => {
                write!(f, "Workspace not found at {}", path.display())
            }
            Self::NameAlreadyExists(name) => {
                write!(f, "Session name '{}' already exists", name)
            }
            // ... other variants
        }
    }
}
```

### 7. Provide Suggestions for Recovery

```rust
impl SessionError {
    /// Returns a human-readable suggestion for fixing the error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::NameAlreadyExists(_) => {
                Some("Use 'zjj list' to see existing sessions or choose a different name".into())
            }
            Self::WorkspaceNotFound(path) => {
                Some(format!("Check that the workspace exists at: {}", path.display()))
            }
            _ => None,
        }
    }

    /// Returns copy-pastable shell commands to resolve the error
    pub fn fix_commands(&self) -> Vec<String> {
        match self {
            Self::WorkspaceNotFound(_) => {
                vec!["zjj list".to_string(), "zjj doctor".to_string()]
            }
            _ => vec![],
        }
    }
}
```

### 8. Test Error Paths

```rust
#[test]
fn test_duplicate_name_returns_conflict() {
    let repo = create_test_repo();
    let name = SessionName::parse("test").expect("valid");

    repo.save(&create_session(&name)).expect("first save succeeds");
    let result = repo.save(&create_session(&name));

    assert!(matches!(result, Err(RepositoryError::Conflict(_))));
}
```

### 9. Use Extension Traits for Ergonomics

```rust
use zjj_core::domain::error_conversion::AggregateErrorExt;

// Instead of
session
    .validate()
    .map_err(|e| RepositoryError::InvalidInput(format!("validation failed: {}", e)))?;

// Use
session.validate().on_save("session")?;
```

### 10. Separate Domain and Infrastructure Errors

```rust
// Domain layer (core) - uses domain errors
impl Session {
    pub fn transition_branch(&self, new: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new) {
            return Err(SessionError::InvalidBranchTransition {
                from: self.branch.clone(),
                to: new,
            });
        }
        Ok(Self { branch: new, ..self.clone() })
    }
}

// Infrastructure layer (shell) - converts to RepositoryError
impl SessionRepository for SqliteRepo {
    fn save(&self, session: &Session) -> RepositoryResult<()> {
        session
            .validate()
            .map_err(|e| e.on_save("session"))?;  // Convert to RepositoryError
        // ... persist
        Ok(())
    }
}
```

---

## Quick Reference

### Error Type Selection Guide

| Situation | Use Error Type |
|-----------|---------------|
| Validating identifiers (names, IDs, paths) | `IdentifierError` |
| Enforcing aggregate invariants | `SessionError`, `WorkspaceError`, `BeadError`, `QueueEntryError` |
| Repository operations (load, save, delete) | `RepositoryError` |
| Type-safe builder validation | `BuilderError` |
| Shell/imperative layer with I/O | `anyhow::Error` with `.context()` |

### Common Conversion Patterns

```rust
// Identifier → Aggregate
let name = SessionName::parse(input)?;  // IdentifierError → SessionError

// Aggregate → Repository
session.validate().on_save("session")?;  // SessionError → RepositoryError

// Repository → anyhow (shell layer)
repo.load(id).context("failed to load session")?;  // RepositoryError → anyhow::Error
```

### Error Propagation Patterns

```rust
// Pattern 1: Early return
fn process() -> Result<T, E> {
    step1()?;
    step2()?;
    step3()
}

// Pattern 2: Match and recover
fn process() -> Result<T, E> {
    match risky_operation() {
        Ok(value) => Ok(process_value(value)),
        Err(Error::NotFound(_)) => Ok(create_default()),
        Err(e) => Err(e),
    }
}

// Pattern 3: Combinator chain
fn process(items: Vec<Input>) -> Result<Output, E> {
    items
        .into_iter()
        .map(validate)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(transform)
        .collect()
}
```

---

## Further Reading

- **Error Conversion Guide:** `ERROR_CONVERSION_GUIDE.md`
- **Unified Error Examples:** `UNIFIED_ERROR_EXAMPLES.md`
- **Domain Types Guide:** `DOMAIN_TYPES_GUIDE.md`
- **Functional Patterns:** `docs/04_FUNCTIONAL_PATTERNS.md`
- **Rust Standards:** `docs/05_RUST_STANDARDS.md`

---

## Summary

Error handling in the zjj codebase follows these core principles:

1. **Zero Panic:** Never use `unwrap()`, `expect()`, or `panic!()`
2. **Typed Errors:** Use specific error types for each layer
3. **Error Hierarchy:** `IdentifierError` → Aggregate Errors → `RepositoryError` → `anyhow::Error`
4. **Context Preservation:** Add context as errors propagate across layers
5. **Railway-Oriented:** Use `?` operator for early return on errors
6. **Parse at Boundaries:** Validate input immediately at system boundaries
7. **Recovery Strategies:** Match on errors to handle specific cases
8. **Test Error Paths:** Write tests for all error conditions

By following these patterns, you ensure that errors convey domain meaning, preserve context, and enable recovery—all while maintaining type safety and zero panics.
