# Code Review Checklist for ZJJ Pull Requests

Comprehensive checklist for reviewing code changes in the ZJJ codebase following Domain-Driven Design (DDD) principles and functional Rust patterns.

## Table of Contents

1. [DDD Principles Compliance](#1-ddd-principles-compliance)
2. [Functional Rust Patterns](#2-functional-rust-patterns)
3. [Error Handling](#3-error-handling)
4. [Test Coverage](#4-test-coverage)
5. [Documentation](#5-documentation)
6. [Performance Considerations](#6-performance-considerations)
7. [Security Considerations](#7-security-considerations)
8. [Common Pitfalls](#8-common-pitfalls)
9. [Automated Checks](#9-automated-checks)
10. [Review Severity Levels](#10-review-severity-levels)

---

## 1. DDD Principles Compliance

### 1.1 Bounded Contexts

**Check items:**
- [ ] Code respects module boundaries (domain, coordination, cli_contracts, etc.)
- [ ] No direct dependencies from domain layer on infrastructure/shell layers
- [ ] Clear interfaces between contexts (no leaking internals)
- [ ] Domain types are not bypassed with raw primitives

**Good:**
```rust
// In cli/handlers/session.rs (shell layer)
use zjj_core::domain::SessionName;

pub async fn create_handler(name: String) -> Result<()> {
    // Parse at boundary
    let session_name = SessionName::parse(name)?;
    // Pass validated type to core
    core.create_session(session_name).await
}
```

**Bad:**
```rust
// In core/domain/session.rs
pub async fn create_session(&self, name: String) {  // Primitive obsession!
    // Validation scattered throughout
    if name.is_empty() || name.len() > 63 {
        return Err(...);
    }
}
```

### 1.2 Aggregates and Invariants

**Check items:**
- [ ] Aggregate roots protect their invariants
- [ ] No direct access to internal aggregate state from outside
- [ ] State transitions are explicit and validated
- [ ] Related entities are accessed through the aggregate root

**Good:**
```rust
impl Session {
    // Enforce invariant: closed_at required when status is Closed
    pub fn close(self, closed_at: DateTime<Utc>) -> Result<Self, SessionError> {
        match self.status {
            SessionStatus::Open => Ok(Self {
                status: SessionStatus::Closed { at: closed_at },
                // ... rest of state
            }),
            SessionStatus::Closed { .. } => Err(SessionError::AlreadyClosed),
            _ => Err(SessionError::InvalidTransition),
        }
    }
}
```

**Bad:**
```rust
pub struct Session {
    pub status: String,  // String-based state!
    pub closed_at: Option<String>,  // Option encoding state - invalid states possible!
}
```

### 1.3 Value Objects

**Check items:**
- [ ] Domain concepts use semantic newtypes (not raw primitives)
- [ ] Value objects are immutable
- [ ] Equality is by value, not identity
- [ ] Validation happens at construction (parse-once pattern)

**Good:**
```rust
// Semantic newtype with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionName(String);

impl SessionName {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }
}
```

**Bad:**
```rust
// Primitive obsession
pub struct Session {
    pub name: String,  // Should be SessionName
    pub agent_id: Option<String>,  // Should be Option<AgentId>
}
```

### 1.4 Domain Events

**Check items:**
- [ ] State changes produce domain events where appropriate
- [ ] Events are immutable facts
- [ ] Events have clear, domain-relevant names
- [ ] Event handlers are separate from domain logic

**Good:**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    Created { id: SessionId, name: SessionName },
    Closed { id: SessionId, at: DateTime<Utc> },
    Failed { id: SessionId, reason: String },
}

impl Session {
    pub fn close(mut self) -> Result<(Self, Vec<SessionEvent>), SessionError> {
        // ... validation ...
        self.status = SessionStatus::Closed;
        let event = SessionEvent::Closed { id: self.id, at: Utc::now() };
        Ok((self, vec![event]))
    }
}
```

### 1.5 Repository Pattern

**Check items:**
- [ ] Domain logic doesn't depend on storage details
- [ ] Repositories are defined in domain, implemented in infrastructure
- [ ] Repository methods return domain types, not database entities
- [ ] No SQL/IO logic in domain layer

**Good:**
```rust
// In domain/repository.rs (trait)
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get(&self, name: &SessionName) -> Result<Option<Session>, RepositoryError>;
    async fn save(&self, session: &Session) -> Result<(), RepositoryError>;
}

// In infrastructure/sqlite_session_repo.rs (implementation)
pub struct SqliteSessionRepository {
    db: Arc<SqlitePool>,
}
```

---

## 2. Functional Rust Patterns

### 2.1 Zero Unwrap / Zero Panic

**Check items:**
- [ ] No `unwrap()`, `expect()`, `unwrap_or()`, `unwrap_or_else()`, `unwrap_or_default()`
- [ ] No `panic!()`, `todo!()`, `unimplemented!()`
- [ ] File header includes lint denies: `#![deny(clippy::unwrap_used)]`
- [ ] All fallible operations return `Result<T, E>`

**Good:**
```rust
fn process_item(item: Option<Item>) -> Result<ProcessedItem, Error> {
    item.ok_or_else(|| Error::NotFound("item not found".into()))
        .and_then(|i| i.validate())
        .map(|i| i.process())
}

// Or using match
fn process_item(item: Option<Item>) -> Result<ProcessedItem, Error> {
    match item {
        None => Err(Error::NotFound("item not found".into())),
        Some(i) => i.validate().map(|i| i.process()),
    }
}
```

**Bad:**
```rust
fn process_item(item: Option<Item>) -> ProcessedItem {
    let i = item.expect("item must exist");  // VIOLATION
    i.validate().unwrap()  // VIOLATION
}
```

### 2.2 Pure Functions

**Check items:**
- [ ] Core domain logic is pure (no I/O, no global state)
- [ ] Functions are deterministic (same input = same output)
- [ ] No hidden side effects in pure functions
- [ ] I/O only in shell/handler layer

**Good:**
```rust
// Pure function in core
fn calculate_session_status(
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    timeout: Duration,
) -> SessionStatus {
    let idle = Utc::now().signed_duration_since(last_activity);
    if idle > timeout {
        SessionStatus::TimedOut
    } else {
        SessionStatus::Active
    }
}

// I/O in shell
async fn get_session_status(repo: &SessionRepository, id: SessionId) -> Result<SessionStatus> {
    let session = repo.get(&id).await?;
    Ok(calculate_session_status(session.created_at, session.last_activity, Duration::hours(24)))
}
```

**Bad:**
```rust
// I/O mixed with domain logic - NOT PURE
fn calculate_session_status(repo: &SessionRepository, id: SessionId) -> SessionStatus {
    let session = repo.get(&id).await.unwrap();  // I/O in pure function!
    // ... calculation ...
}
```

### 2.3 Immutability

**Check items:**
- [ ] Default to immutable (`let` not `let mut`)
- [ ] Use `iter()` instead of `iter_mut()` where possible
- [ ] Use functional combinators (`map`, `filter`, `fold`) instead of loops with mutation
- [ ] If mutation is necessary, isolate it in the smallest possible scope

**Good:**
```rust
use itertools::Itertools;

fn process_items(items: Vec<Item>) -> Vec<ProcessedItem> {
    items
        .into_iter()
        .map(|item| item.validate())
        .filter_map(Result::ok)
        .map(|item| item.process())
        .collect()
}

// Using fold for accumulation
fn sum_valid_items(items: Vec<Item>) -> u32 {
    items.iter()
        .filter_map(|item| item.value().ok())
        .fold(0, |acc, val| acc + val)
}
```

**Bad:**
```rust
fn process_items(items: Vec<Item>) -> Vec<ProcessedItem> {
    let mut result = Vec::new();  // Unnecessary mutation
    for item in items {
        if let Ok(valid) = item.validate() {
            result.push(valid.process());
        }
    }
    result
}
```

### 2.4 Iterator Pipelines

**Check items:**
- [ ] Use `itertools` for complex iterator operations
- [ ] Prefer combinators over imperative loops
- [ ] Use `tap` for pipeline observation/debugging
- [ ] Use `fold` and `scan` for stateful transformations

**Good:**
```rust
use itertools::Itertools;
use tap::Pipe;

fn process_sessions(sessions: Vec<Session>) -> Vec<SessionSummary> {
    sessions
        .into_iter()
        .filter(|s| s.status() != SessionStatus::Closed)
        .map(|s| s.to_summary())
        .tap(|summaries| println!("Processing {} summaries", summaries.len()))
        .sorted_by_key(|s| s.created_at)
        .collect_vec()
}
```

**Bad:**
```rust
fn process_sessions(sessions: Vec<Session>) -> Vec<SessionSummary> {
    let mut summaries = Vec::new();
    for session in sessions {
        if session.status() != SessionStatus::Closed {
            summaries.push(session.to_summary());
        }
    }
    summaries.sort_by_key(|s| s.created_at);
    summaries
}
```

### 2.5 Type-Level Guarantees

**Check items:**
- [ ] Use newtypes to prevent mixing incompatible values
- [ ] Use enums to make illegal states unrepresentable
- [ ] Use PhantomData for type-level tracking
- [ ] Leverage the type system for compile-time validation

**Good:**
```rust
// Newtype prevents mixing incompatible IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

// Enum makes invalid states impossible
pub enum ClaimState {
    Unclaimed,
    Claimed {
        agent: AgentId,
        claimed_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    Expired {
        previous_agent: AgentId,
        expired_at: DateTime<Utc>,
    },
}
```

**Bad:**
```rust
// Can accidentally mix these IDs
pub fn get_session(id: u64) -> Session { /* ... */ }
pub fn get_task(id: u64) -> Task { /* ... */ }

// Invalid state possible
pub struct TaskInfo {
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claim_expires_at: Option<DateTime<Utc>>,
    // Can have claimed_by=Some but claimed_at=None - invalid!
}
```

---

## 3. Error Handling

### 3.1 Error Type Selection

**Check items:**
- [ ] Core domain uses `thiserror` for domain errors
- [ ] Shell/handler layer uses `anyhow` with context
- [ ] Error variants are exhaustive and domain-specific
- [ ] Errors preserve context for debugging

**Good:**
```rust
// In domain layer - use thiserror
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session '{0}' not found")]
    NotFound(SessionName),

    #[error("session '{0}' already exists")]
    AlreadyExists(SessionName),

    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: SessionStatus, to: SessionStatus },
}

// In shell layer - use anyhow with context
async fn handle_create(name: String) -> anyhow::Result<()> {
    let session_name = SessionName::parse(name)
        .context("failed to parse session name from user input")?;
    // ...
}
```

**Bad:**
```rust
// In domain layer - NOT using thiserror
pub fn create_session(name: &str) -> Result<Session> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("name cannot be empty"));  // Wrong error type!
    }
    // ...
}
```

### 3.2 Error Conversion

**Check items:**
- [ ] `From` impls for error conversions
- [ ] Use `?` operator for propagation
- [ ] Use `.map_err()` for context enrichment
- [ ] Use `.context()` in shell layer for additional information

**Good:**
```rust
impl From<IdentifierError> for SessionError {
    fn from(err: IdentifierError) -> Self {
        match err {
            IdentifierError::Empty => SessionError::InvalidName("name cannot be empty".into()),
            IdentifierError::TooLong { max, .. } => {
                SessionError::InvalidName(format!("name exceeds {max} characters"))
            }
            _ => SessionError::InvalidName(err.to_string()),
        }
    }
}

// Usage with ?
impl Session {
    pub fn new(name: SessionName) -> Result<Self, SessionError> {
        // IdentifierError automatically converted to SessionError
        let validated = name.validate()?;
        Ok(Self { name: validated, .. })
    }
}
```

**Bad:**
```rust
// Manual error conversion everywhere
pub fn new(name: SessionName) -> Result<Self, SessionError> {
    let validated = name.validate().map_err(|e| {
        SessionError::InvalidName(format!("validation failed: {}", e))  // Repetitive
    })?;
    Ok(Self { name: validated, .. })
}
```

### 3.3 Railway-Oriented Programming

**Check items:**
- [ ] Use `Result` chaining with `and_then`, `map`, `or_else`
- [ ] Early return on error with `?`
- [ ] Separate happy path from error handling
- [ ] No nested `if` chains for error handling

**Good:**
```rust
fn create_session_from_input(
    name_str: &str,
    workspace_path: &str,
) -> Result<Session, SessionError> {
    let name = SessionName::parse(name_str)?;  // Early exit on error
    let workspace = Workspace::load(workspace_path)?;
    let id = SessionId::new()?;
    let session = Session::new_root(id, name, workspace.path)?;
    Ok(session)
}
```

**Bad:**
```rust
fn create_session_from_input(
    name_str: &str,
    workspace_path: &str,
) -> Result<Session, SessionError> {
    let name = match SessionName::parse(name_str) {
        Ok(n) => n,
        Err(e) => return Err(e.into()),
    };

    let workspace = match Workspace::load(workspace_path) {
        Ok(w) => w,
        Err(e) => return Err(e.into()),
    };

    // Deeply nested error handling
    if let Some(id) = SessionId::new() {
        if let Ok(session) = Session::new_root(id, name, workspace.path) {
            Ok(session)
        } else {
            Err(SessionError::CreationFailed)
        }
    } else {
        Err(SessionError::InvalidId)
    }
}
```

### 3.4 Error Messages

**Check items:**
- [ ] Error messages are clear and actionable
- [ ] Include relevant values in error messages
- [ ] Use structured error types, not just strings
- [ ] Provide hints for resolution where appropriate

**Good:**
```rust
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session '{name}' not found")]
    NotFound { name: SessionName },

    #[error("session '{name}' already exists (created at {created_at})")]
    AlreadyExists { name: SessionName, created_at: DateTime<Utc> },

    #[error("invalid session name '{name}': {reason}")]
    InvalidName { name: String, reason: String },

    #[error("cannot close session: status is {current:?}, expected {expected:?}")]
    InvalidClose { current: SessionStatus, expected: SessionStatus },
}
```

**Bad:**
```rust
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("failed")]
    Failed,  // Not actionable!

    #[error("error: {0}")]
    Error(String),  // String-based, not structured

    #[error("session error")]
    SessionError,  // No context!
}
```

---

## 4. Test Coverage

### 4.1 Unit Tests

**Check items:**
- [ ] Domain logic has comprehensive unit tests
- [ ] Tests cover happy path and error paths
- [ ] Tests use domain types, not raw primitives
- [ ] Tests are deterministic (no randomness, no external state)

**Good:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::parse("my-session").is_ok());
        assert!(SessionName::parse("Feature_Auth").is_ok());
        assert!(SessionName::parse("a").is_ok());
    }

    #[test]
    fn test_session_name_invalid_empty() {
        let result = SessionName::parse("");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[test]
    fn test_session_name_invalid_too_long() {
        let long_name = "a".repeat(64);
        let result = SessionName::parse(&long_name);
        assert!(matches!(result, Err(IdentifierError::TooLong { max: 63, .. })));
    }

    #[test]
    fn test_session_close_valid_transition() {
        let session = Session::new_root(id, name, path);
        let result = session.close(Utc::now());
        assert!(result.is_ok());
        assert!(matches!(result?.status, SessionStatus::Closed { .. }));
    }
}
```

**Bad:**
```rust
#[test]
fn test_session() {  // Too vague!
    let session = Session::new("test".to_string());  // Using raw string
    assert!(session.is_valid());  // What does this test?
}
```

### 4.2 Property-Based Tests

**Check items:**
- [ ] Critical invariants tested with proptest
- [ ] Strategies cover valid and invalid inputs
- [ ] Tests verify roundtrip properties
- [ ] Tests verify boundary conditions

**Good:**
```rust
proptest! {
    #[test]
    fn test_session_name_roundtrip(name in valid_session_name_strategy()) {
        let parsed = SessionName::parse(name).unwrap();
        let displayed = parsed.to_string();
        let reparsed = SessionName::parse(&displayed).unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_session_name_never_empty(input in "[a-zA-Z0-9_-]*") {
        if input.is_empty() || input.trim().is_empty() {
            assert!(SessionName::parse(&input).is_err());
        }
    }

    #[test]
    fn test_session_name_max_length(name in "[a-zA-Z][a-zA-Z0-9_-]{0,100}") {
        let result = SessionName::parse(&name);
        if name.len() > 63 {
            assert!(result.is_err());
        } else if !name.is_empty() {
            assert!(result.is_ok());
        }
    }
}
```

### 4.3 Integration Tests

**Check items:**
- [ ] Handler/shell layer has integration tests
- [ ] Tests verify end-to-end flows
- [ ] Tests use test fixtures, not production data
- [ ] Tests clean up after themselves

**Good:**
```rust
#[tokio::test]
async fn test_create_session_workflow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let repo = SqliteSessionRepository::new(&db_path).await.unwrap();

    let name = SessionName::parse("test-session").unwrap();
    let workspace = WorkspaceName::parse("test-workspace").unwrap();

    let result = create_session_use_case(&repo, &name, &workspace).await;

    assert!(result.is_ok());
    let session = repo.get(&name).await.unwrap().unwrap();
    assert_eq!(session.name(), &name);
}
```

### 4.4 Test Quality

**Check items:**
- [ ] Tests are readable and maintainable
- [ ] Tests have descriptive names
- [ ] Tests are independent (no shared state)
- [ ] Tests are fast (avoid unnecessary I/O or sleeps)

**Good:**
```rust
#[test]
fn test_session_close_when_already_closed_returns_error() {
    let mut session = create_test_session();
    session.close(Utc::now()).unwrap();

    let result = session.close(Utc::now());
    assert!(matches!(result, Err(SessionError::AlreadyClosed)));
}
```

**Bad:**
```rust
#[test]
fn test_it() {  // Not descriptive
    let mut s = Session::new();
    s.close();
    assert!(s.close().is_err());
}
```

---

## 5. Documentation

### 5.1 Code Documentation

**Check items:**
- [ ] Public types have module-level docs
- [ ] Public functions have doc comments
- [ ] Complex algorithms have explanatory comments
- [ ] Error conditions are documented

**Good:**
```rust
//! Domain aggregates for session management.
//!
//! This module defines the `Session` aggregate root and its related value objects.
//! Sessions represent isolated development environments with associated workspaces.
//!
//! # State Machine
//!
//! Sessions follow this state transition diagram:
//!
//! ```text
//!     Creating
//!        ↓
//!      Active ←→ Paused
//!        ↓
//!   Completed / Failed
//! ```

/// A session representing an isolated development environment.
///
/// # Invariants
///
/// - A session always has a unique ID and name
/// - Closed sessions have a `closed_at` timestamp
/// - Child sessions must have a parent session
///
/// # Examples
///
/// ```rust
/// use zjj_core::domain::{Session, SessionName, SessionId};
///
/// let name = SessionName::parse("my-session")?;
/// let id = SessionId::new()?;
/// let session = Session::new_root(id, name, workspace_path)?;
/// ```
#[derive(Debug, Clone)]
pub struct Session {
    // ...
}
```

**Bad:**
```rust
// Session struct
pub struct Session {
    pub id: u64,
    pub name: String,
    pub status: String,  // No docs on what values are valid
}
```

### 5.2 API Documentation

**Check items:**
- [ ] Functions document parameters and return values
- [ ] Errors are documented with `# Errors` sections
- [ ] Panics are documented with `# Panics` sections (if any)
- [ ] Examples are provided and tested

**Good:**
```rust
impl Session {
    /// Close the session, marking it as completed.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::AlreadyClosed` if the session is already closed.
    /// Returns `SessionError::InvalidTransition` if the current state doesn't allow closing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zjj_core::domain::Session;
    /// # let mut session = Session::new_root(/* ... */);
    /// let closed_at = Utc::now();
    /// match session.close(closed_at) {
    ///     Ok(s) => println!("Session closed at {:?}", s.closed_at()),
    ///     Err(SessionError::AlreadyClosed) => println!("Already closed"),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn close(self, closed_at: DateTime<Utc>) -> Result<Self, SessionError> {
        // ...
    }
}
```

### 5.3 Architectural Documentation

**Check items:**
- [ ] Complex modules have README.md files
- [ ] Design decisions are documented in ADRs
- [ ] Diagrams are provided for complex flows
- [ ] Migration guides are provided for breaking changes

**Good:**
```
crates/zjj-core/src/domain/
├── README.md           # Explains DDD patterns used
├── mod.rs              # Public API documentation
├── identifiers.rs      # Value objects
├── aggregates/
│   ├── README.md       # Aggregate pattern explanation
│   ├── session.rs
│   └── workspace.rs
└── events.rs           # Domain events
```

---

## 6. Performance Considerations

### 6.1 Allocation Patterns

**Check items:**
- [ ] Avoid unnecessary allocations in hot paths
- [ ] Use `&str` instead of `String` where ownership not needed
- [ ] Use `Cow<str>` for conditional ownership
- [ ] Consider arena allocation for many short-lived objects

**Good:**
```rust
// Accept &str, avoid allocation
fn validate_name(name: &str) -> Result<(), IdentifierError> {
    if name.is_empty() {
        return Err(IdentifierError::Empty);
    }
    // ...
}

// Return Cow<str> for conditional allocation
fn format_name(name: &str) -> Cow<'_, str> {
    if name.contains(' ') {
        Cow::Owned(name.replace(' ', "-"))
    } else {
        Cow::Borrowed(name)
    }
}
```

**Bad:**
```rust
// Unnecessary allocation
fn validate_name(name: String) -> Result<(), IdentifierError> {
    if name.is_empty() {
        return Err(IdentifierError::Empty);
    }
    // ...
}
```

### 6.2 I/O Patterns

**Check items:**
- [ ] Batch database operations where possible
- [ ] Use streaming for large datasets
- [ ] Avoid N+1 query problems
- [ ] Use connection pooling

**Good:**
```rust
// Batch insert
async fn create_sessions_batch(
    repo: &SessionRepository,
    sessions: Vec<Session>,
) -> Result<(), RepositoryError> {
    repo.save_batch(&sessions).await?;  // Single transaction
    Ok(())
}

// Streaming for large datasets
async fn list_all_sessions(
    repo: &SessionRepository,
) -> Result<impl Stream<Item = Session>, RepositoryError> {
    repo.stream_all().await
}
```

**Bad:**
```rust
// N+1 problem
async fn create_sessions(
    repo: &SessionRepository,
    sessions: Vec<Session>,
) -> Result<(), RepositoryError> {
    for session in sessions {
        repo.save(&session).await?;  // N transactions!
    }
    Ok(())
}
```

### 6.3 Concurrency

**Check items:**
- [ ] Use async/await correctly
- [ ] Avoid blocking operations in async contexts
- [ ] Use channels for communication between tasks
- [ ] Lock-free data structures where appropriate

**Good:**
```rust
use tokio::sync::mpsc;
use futures_util::{StreamExt, TryStreamExt};

async fn process_sessions_concurrently(
    sessions: Vec<Session>,
) -> Result<Vec<ProcessedSession>> {
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn workers
    for _ in 0..num_cpus::get() {
        let mut rx_clone = rx.clone();
        tokio::spawn(async move {
            while let Some(session) = rx_clone.recv().await {
                let processed = process_session(session).await;
                // Send result
            }
        });
    }

    // Send work
    for session in sessions {
        tx.send(session).await?;
    }

    // Collect results
    // ...
}
```

---

## 7. Security Considerations

### 7.1 Input Validation

**Check items:**
- [ ] All user input is validated at boundaries
- [ ] Path traversal attacks are prevented
- [ ] SQL injection is prevented (use parameterized queries)
- [ ] Command injection is prevented

**Good:**
```rust
// Validate at boundary
pub async fn handle_create(name: String, workspace_path: String) -> Result<()> {
    // Parse and validate
    let session_name = SessionName::parse(&name)
        .map_err(|e| anyhow::anyhow!("invalid session name: {}", e))?;

    // Validate path - prevent traversal
    let workspace = AbsolutePath::parse(&workspace_path)
        .map_err(|e| anyhow::anyhow!("invalid workspace path: {}", e))?;

    // Use parameterized query (sqlx does this automatically)
    repo.save(&session).await?;
    Ok(())
}
```

**Bad:**
```rust
// No validation - security risk!
pub async fn handle_create(name: String, workspace_path: String) -> Result<()> {
    // Could contain path traversal: "../../../etc/passwd"
    let workspace = Workspace::load(&workspace_path)?;

    // SQL injection risk!
    sqlx::query(&format!("INSERT INTO sessions (name) VALUES ('{}')", name))
        .execute(&pool)
        .await?;
    Ok(())
}
```

### 7.2 Secrets Management

**Check items:**
- [ ] No secrets in code or git history
- [ ] Secrets loaded from environment or secure stores
- [ ] Secrets not logged or exposed in errors
- [ ] API tokens and credentials are validated

**Good:**
```rust
// Load from environment
pub struct ApiConfig {
    api_key: String,
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key = std::env::var("ZJJ_API_KEY")
            .map_err(|_| ConfigError::MissingApiKey)?;

        if api_key.is_empty() {
            return Err(ConfigError::InvalidApiKey("empty".into()));
        }

        Ok(Self { api_key })
    }
}

// Don't log secrets
impl std::fmt::Debug for ApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiConfig")
            .field("api_key", &"***REDACTED***")
            .finish()
    }
}
```

**Bad:**
```rust
// Hardcoded secret!
pub const API_KEY: &str = "sk-1234567890abcdef";

// Logged in error messages
Err(anyhow::anyhow!("API request failed with key: {}", api_key))
```

### 7.3 File System Operations

**Check items:**
- [ ] Validate file paths are within expected directories
- [ ] Check for symlink attacks
- [ ] Use secure file permissions
- [ ] Clean up temporary files

**Good:**
```rust
use std::path::Path;

fn is_within_parent(path: &Path, parent: &Path) -> bool {
    path.canonicalize()
        .ok()
        .map(|p| p.starts_with(parent.canonicalize().ok().as_ref().unwrap()))
        .unwrap_or(false)
}

fn load_workspace(path: &Path, allowed_base: &Path) -> Result<Workspace, Error> {
    if !is_within_parent(path, allowed_base) {
        return Err(Error::PathTraversal(
            "workspace path outside allowed directory".into(),
        ));
    }

    // Safe to proceed
    Workspace::load(path)
}
```

---

## 8. Common Pitfalls

### 8.1 String Handling

**Pitfall:** Using `String` everywhere instead of semantic types

**Detection:**
```rust
// Look for function signatures like this:
pub fn process(name: String, id: String, status: String)
```

**Fix:**
```rust
// Use semantic types
pub fn process(name: SessionName, id: SessionId, status: SessionStatus)
```

### 8.2 Option for State

**Pitfall:** Using `Option<T>` to encode state machines

**Detection:**
```rust
pub struct Task {
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    // Can have claimed_by=Some but claimed_at=None - invalid!
}
```

**Fix:**
```rust
pub enum ClaimState {
    Unclaimed,
    Claimed {
        agent: AgentId,
        claimed_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
}
```

### 8.3 Boolean Flags

**Pitfall:** Multiple boolean flags making invalid states possible

**Detection:**
```rust
#[allow(clippy::struct_excessive_bools)]
pub struct QueueOptions {
    pub list: bool,
    pub process: bool,
    pub next: bool,
    pub stats: bool,
    // Can have list=true AND process=true - invalid!
}
```

**Fix:**
```rust
pub enum QueueCommand {
    List,
    Process,
    Next,
    Stats,
    // Mutually exclusive by construction
}
```

### 8.4 Error Swallowing

**Pitfall:** Silently ignoring errors

**Detection:**
```rust
let _ = session.close();  // Error ignored!
let result = some_operation().ok();  // Error lost!
```

**Fix:**
```rust
session.close()
    .map_err(|e| eprintln!("Warning: failed to close session: {}", e))?;

// Or use ?
session.close()?;
```

### 8.5 Unnecessary Mutation

**Pitfall:** Using `mut` when immutability would work

**Detection:**
```rust
fn process(items: Vec<Item>) -> Vec<Item> {
    let mut result = Vec::new();
    for item in items {
        let processed = item.transform();
        result.push(processed);
    }
    result
}
```

**Fix:**
```rust
fn process(items: Vec<Item>) -> Vec<Item> {
    items.into_iter()
        .map(|item| item.transform())
        .collect()
}
```

### 8.6 Repeated Validation

**Pitfall:** Validating the same data multiple times

**Detection:**
```rust
// In handler
validate_name(&name)?;
create_session(&name)?;

// In create_session
validate_name(&name)?;  // Already validated!
```

**Fix:**
```rust
// In handler - parse once
let session_name = SessionName::parse(name)?;

// In core - accept validated type
fn create_session(name: &SessionName) -> Result<Session> {
    // No validation needed!
}
```

---

## 9. Automated Checks

### Pre-commit Checklist

Run these before committing:

```bash
# Format code
moon run :fmt-fix

# Run linter
moon run :check

# Run quick tests
moon run :quick

# Check for unwrap/panic usage
cargo clippy --all-targets -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
```

### Pre-merge Checklist

Run these before creating PR:

```bash
# Full test suite
moon run :test

# Full CI pipeline
moon run :ci

# Check documentation
cargo doc --no-deps --all-features

# Audit dependencies
cargo audit
```

### CI Pipeline Checklist

Ensure CI includes:

```yaml
# .github/workflows/ci.yml
steps:
  - name: Format check
    run: moon run :check

  - name: Clippy
    run: cargo clippy --all-targets -- -D warnings

  - name: Tests
    run: moon run :test

  - name: Property tests
    run: cargo test --test '*_properties*

  - name: Documentation
    run: cargo doc --no-deps

  - name: Security audit
    run: cargo audit
```

### Automated Lint Rules

Ensure these are in `.clippy.toml`:

```toml
# Deny unwrap/panic in production code
warn-on-all-wildcard-imports = true
disallow-methods = ["unwrap", "expect", "panic", "todo", "unimplemented"]
```

---

## 10. Review Severity Levels

### CRITICAL (Must Fix)

- Blocks merge
- Security vulnerabilities
- Data loss bugs
- Breaking DDD principles
- Using unwrap/panic in production code

### HIGH (Should Fix)

- Significant performance issues
- Missing error handling
- Test coverage < 80%
- Poor error messages
- Violating functional patterns

### MEDIUM (Consider Fixing)

- Minor performance optimizations
- Missing documentation
- Code duplication
- Inconsistent naming
- Weak type safety

### LOW (Nice to Have)

- Minor style improvements
- Extra test cases
- Better comments
- Refactoring opportunities

### REVIEW TEMPLATE

```markdown
## Review Summary

- [ ] CRITICAL: ___ items
- [ ] HIGH: ___ items
- [ ] MEDIUM: ___ items
- [ ] LOW: ___ items

### DDD Compliance

- [ ] Bounded contexts respected
- [ ] Aggregates protect invariants
- [ ] Value objects used correctly
- [ ] Domain events produced where appropriate

### Functional Rust

- [ ] No unwrap/panic in production code
- [ ] Pure functions in domain layer
- [ ] Immutable by default
- [ ] Iterator pipelines preferred

### Error Handling

- [ ] Domain errors use thiserror
- [ ] Shell errors use anyhow with context
- [ ] Error messages are actionable
- [ ] No error swallowing

### Test Coverage

- [ ] Unit tests for domain logic
- [ ] Property tests for invariants
- [ ] Integration tests for workflows
- [ ] Tests are deterministic

### Documentation

- [ ] Public APIs documented
- [ ] Complex logic explained
- [ ] Examples provided
- [ ] Error conditions documented

### Performance

- [ ] No unnecessary allocations
- [ ] I/O batched where appropriate
- [ ] Async used correctly
- [ ] No N+1 problems

### Security

- [ ] Input validation at boundaries
- [ ] No path traversal vulnerabilities
- [ ] Secrets not hardcoded
- [ ] SQL injection prevented

### Automated Checks

- [ ] `moon run :fmt-fix` passes
- [ ] `moon run :check` passes
- [ ] `moon run :test` passes
- [ ] `moon run :ci` passes
```

---

## Quick Reference

### File Header Template

Every Rust file should start with:

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

### Error Handling Decision Tree

```
Is this in the domain layer?
├─ Yes → Use thiserror with domain-specific errors
└─ No → Is this in the shell/handler layer?
    ├─ Yes → Use anyhow with .context()
    └─ No → Is this a value object?
        ├─ Yes → Return type-specific error (e.g., IdentifierError)
        └─ No → Use RepositoryError from repository layer
```

### Type Selection Guide

```
Need to represent a name/ID?
├─ Yes → Use semantic newtype (SessionName, AgentId)
│         with validation at construction

Need to represent state?
├─ Yes → Use enum (not bool or String)
│         Makes illegal states unrepresentable

Need optional data?
├─ Yes → Is it encoding state?
│   ├─ Yes → Use enum variants
│   └─ No → Use Option<T>
```

---

**Remember:**
- The compiler is your friend - let it catch errors at compile time
- Make illegal states unrepresentable
- Parse at boundaries, validate once
- Pure functions in core, I/O in shell
- Zero unwrap, zero panic, zero unsafe
- Tests are code - treat them with the same quality standards

**Questions?** See:
- [AGENTS.md](AGENTS.md) - Mandatory rules
- [ERROR_HANDLING_GUIDE.md](ERROR_HANDLING_GUIDE.md) - Error handling patterns
- [DDD_REFACTORING_REPORT.md](DDD_REFACTORING_REPORT.md) - DDD refactoring examples
- [CLI_CONTRACTS_REFACTORING.md](CLI_CONTRACTS_REFACTORING.md) - Contract patterns
