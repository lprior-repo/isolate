# Beads Module DDD Refactoring Report

**Date**: 2025-02-23
**Module**: `crates/zjj-core/src/beads/`
**Approach**: Scott Wlaschin's Domain-Driven Design refactoring principles

## Executive Summary

The beads module has been refactored to follow Domain-Driven Design (DDD) principles with a focus on making illegal states unrepresentable through the type system. The refactoring introduces semantic newtypes, proper state encoding via enums, and a clean separation between the pure functional core and imperative shell.

## Architecture Overview

### New Module Structure

```
beads/
├── domain.rs      # Core domain types (newtypes, enums, errors)
├── issue.rs       # Aggregate root with business logic
├── types.rs       # Legacy types (backward compatibility)
├── db.rs          # Database operations (imperative shell)
├── query.rs       # Query and filtering logic
├── analysis.rs    # Analytical operations
└── mod.rs         # Public API re-exports
```

## Key Improvements

### 1. Semantic Newtypes (Primitive Obsession)

**Before**: Raw strings used for all identifiers and text fields
```rust
pub struct BeadIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
}
```

**After**: Validated semantic types with domain rules
```rust
pub struct Issue {
    pub id: IssueId,           // Non-empty, pattern-validated
    pub title: Title,           // Non-empty, length-limited, trimmed
    pub description: Option<Description>,  // Length-limited
    pub assignee: Option<Assignee>,        // Validated
    pub parent: Option<ParentId>,          // IssueId type alias
}

// Examples:
pub struct IssueId(String);        // Validates non-empty, pattern
pub struct Title(String);          // Validates non-empty, length 200
pub struct Description(String);    // Validates length 10k
pub struct Assignee(String);       // Validates non-empty, length 100
```

**Benefits**:
- Cannot construct invalid values at compile time
- Validation happens once at construction (parse at boundaries)
- Self-documenting code through types
- No need for repeated validation checks

### 2. State Encoding (Illegal States Unrepresentable)

**Before**: `closed_at: Option<DateTime>` separate from status enum
```rust
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
}

pub struct BeadIssue {
    pub status: IssueStatus,
    pub closed_at: Option<DateTime<Utc>>,  // Invalid state possible!
}

// Runtime validation required:
if issue.status == IssueStatus::Closed && issue.closed_at.is_none() {
    return Err(BeadsError::ValidationFailed("closed_at must be set".into()));
}
```

**After**: Closed state includes timestamp inline
```rust
pub enum IssueState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },  // Timestamp required!
}

pub struct Issue {
    pub state: IssueState,
    // No separate closed_at field - it's in the state
}

impl IssueState {
    pub const fn closed_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(closed_at),
            _ => None,
        }
    }
}
```

**Benefits**:
- **Impossible** to have a closed issue without a timestamp
- Invalid states are **unrepresentable** at the type level
- No runtime validation needed
- Self-documenting state machine

### 3. Collections with Validation

**Before**: Raw vectors with no validation
```rust
pub struct BeadIssue {
    pub labels: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub blocked_by: Option<Vec<String>>,
}
```

**After**: Semantic collection types with invariants
```rust
pub struct Labels(Vec<String>);        // Max 20, max length 50 each
pub struct DependsOn(Vec<IssueId>);    // Max 50, all valid IDs
pub struct BlockedBy(Vec<IssueId>);    // Max 50, all valid IDs

impl Labels {
    pub const MAX_COUNT: usize = 20;
    pub const MAX_LABEL_LENGTH: usize = 50;

    pub fn new(labels: Vec<String>) -> Result<Self, DomainError> {
        if labels.len() > Self::MAX_COUNT {
            return Err(DomainError::InvalidFilter(
                format!("Cannot have more than {} labels", Self::MAX_COUNT)
            ));
        }
        // ... validate each label length
    }
}
```

**Benefits**:
- Collection size limits enforced
- Element validation on construction
- Cannot add invalid elements after construction

### 4. Structured Domain Errors

**Before**: Opaque string-based errors
```rust
pub enum BeadsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}
```

**After**: Structured errors with specific variants
```rust
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("ID cannot be empty")]
    EmptyId,

    #[error("ID must match pattern: {0}")]
    InvalidIdPattern(String),

    #[error("Title exceeds maximum length of {max} characters (got {got})")]
    TitleTooLong { max: usize, got: usize },

    #[error("Invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: IssueState, to: IssueState },

    #[error("Closed issues must have a closed_at timestamp")]
    ClosedWithoutTimestamp,
}
```

**Benefits**:
- Errors are testable (implement `PartialEq`)
- Errors contain structured data
- Self-documenting error conditions
- Easier to match on specific error cases

### 5. Aggregate Root with Encapsulation

**Before**: Passive data structure with validation elsewhere
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadIssue {
    pub id: String,
    pub title: String,
    // ... all fields public
}
// Validation in separate functions
```

**After**: Active aggregate with business logic
```rust
pub struct Issue {
    pub id: IssueId,
    pub title: Title,
    pub state: IssueState,
    // ... fields are public for serde, but methods enforce invariants
}

impl Issue {
    // Constructors validate input
    pub fn new(id: impl Into<String>, title: impl Into<String>)
        -> Result<Self, DomainError>

    // State transitions with validation
    pub fn transition_to(&mut self, new_state: IssueState)
        -> Result<(), DomainError>

    pub fn close(&mut self)
    pub fn reopen(&mut self) -> Result<(), DomainError>

    // Field updates with validation
    pub fn update_title(&mut self, title: impl Into<String>)
        -> Result<(), DomainError>
    pub fn update_description(&mut self, description: impl Into<String>)
        -> Result<(), DomainError>
}
```

**Benefits**:
- All invariant enforcement in one place
- Methods guide users to correct usage
- Cannot accidentally create invalid states

### 6. Builder Pattern for Complex Construction

```rust
pub struct IssueBuilder { /* ... */ }

impl IssueBuilder {
    pub fn new() -> Self
    pub fn id(self, id: impl Into<String>) -> Self
    pub fn title(self, title: impl Into<String>) -> Self
    pub fn state(self, state: IssueState) -> Self
    // ... other setters
    pub fn build(self) -> Result<Issue, DomainError>
}

// Usage:
let issue = IssueBuilder::new()
    .id("issue-123")
    .title("Fix the bug")
    .state(IssueState::Open)
    .priority(Priority::P1)
    .build()?;
```

**Benefits**:
- Fluent API for construction
- Validation deferred to `build()`
- Optional fields clearly optional
- Easy to extend with new fields

## Functional Core, Imperative Shell

### Pure Functional Core

The `domain` and `issue` modules contain only pure functions:
- No I/O
- No global state
- Deterministic: same input = same output
- All operations return `Result<T, E>`

### Imperative Shell (Preserved)

The `db.rs` module handles side effects:
- Database connections
- File I/O
- External API calls
- Uses `anyhow` for boundary errors with context

## Migration Path

### Phase 1: Add New Types (Complete)
- [x] Create `domain.rs` with semantic newtypes
- [x] Create `issue.rs` with aggregate root
- [x] Export new types alongside legacy types
- [x] Maintain backward compatibility

### Phase 2: Gradual Migration (Next Steps)
- [ ] Update `db.rs` to use new types internally
- [ ] Add conversion functions: `BeadIssue <-> Issue`
- [ ] Update `query.rs` to work with `Issue`
- [ ] Update `analysis.rs` to work with `Issue`

### Phase 3: Deprecate Legacy Types (Future)
- [ ] Mark `BeadIssue`, `IssueStatus` as `#[deprecated]`
- [ ] Provide migration guide
- [ ] Update all callers
- [ ] Remove legacy types

## Code Quality Metrics

### Zero Unwrap/Panic
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]
#![forbid(unsafe_code)]
```

### Result-Based Error Handling
All fallible operations return `Result<T, E>`:
- Construction: `Issue::new() -> Result<Issue, DomainError>`
- Updates: `issue.update_title() -> Result<(), DomainError>`
- Transitions: `issue.transition_to() -> Result<(), DomainError>`

### Railway-Oriented Programming
```rust
pub fn parse_and_validate(input: &str) -> Result<Issue, DomainError> {
    input
        .parse::<IssueId>()?           // Parse at boundary
        .and_then(|id| validate(id)?)  // Validate once
        .map(|id| Issue::new(id)?)     // Use validated value
}
```

## Testing Strategy

### Unit Tests in Domain Module
```rust
#[test]
fn test_issue_id_valid() {
    assert!(IssueId::new("valid-id-123").is_ok());
}

#[test]
fn test_issue_id_invalid() {
    assert!(matches!(IssueId::new(""), Err(DomainError::EmptyId)));
}

#[test]
fn test_issue_state_closed_has_timestamp() {
    let state = IssueState::Closed { closed_at: Utc::now() };
    assert!(state.is_closed());
    assert!(state.closed_at().is_some());
}
```

### Unit Tests in Issue Module
```rust
#[test]
fn test_issue_creation() {
    let issue = Issue::new("test-1", "Test Issue").unwrap();
    assert!(issue.is_active());
    assert!(!issue.is_closed());
}

#[test]
fn test_issue_close() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.close();
    assert!(issue.is_closed());
    assert!(issue.closed_at().is_some());
}
```

## Comparison: Before vs After

### Issue Creation

**Before**:
```rust
let issue = BeadIssue {
    id: "issue-123".to_string(),
    title: "Fix bug".to_string(),
    status: IssueStatus::Open,
    // ... many fields
    created_at: Utc::now(),
    updated_at: Utc::now(),
};
// No validation!
```

**After**:
```rust
let issue = Issue::new("issue-123", "Fix bug")?;
// ^ Validates ID pattern, title non-empty, sets timestamps automatically
```

### Closing an Issue

**Before**:
```rust
issue.status = IssueStatus::Closed;
issue.closed_at = Some(Utc::now());
// ^ Forgot to set closed_at? Runtime error!
```

**After**:
```rust
issue.close();
// ^ Type system ensures closed_at is always set
```

### Checking if Blocked

**Before**:
```rust
fn is_blocked(issue: &BeadIssue) -> bool {
    issue.status == IssueStatus::Blocked
        || issue.blocked_by.as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}
```

**After**:
```rust
issue.is_blocked()  // Encapsulated logic
```

## Principles Applied

### 1. Make Illegal States Unrepresentable
- `IssueState::Closed { closed_at: DateTime }` - cannot be closed without timestamp
- `IssueId(String)` - cannot be empty or invalid pattern
- `Title(String)` - cannot be empty or too long

### 2. Parse at Boundaries, Validate Once
- Input validation at constructors (`IssueId::new`, `Title::new`)
- Internal methods assume valid data
- No repeated validation checks

### 3. Use Semantic Newtypes
- `IssueId` instead of `String` for identifiers
- `Title` instead of `String` for titles
- `Description` instead of `Option<String>` for descriptions

### 4. Pure Functional Core
- Domain types have no I/O
- All functions deterministic
- Error handling via `Result<T, E>`

### 5. Railway-Oriented Programming
- All fallible operations return `Result`
- `?` operator for clean error propagation
- No `.unwrap()` or `.expect()`

### 6. Zero Panics, Zero Unwrap
- Enforced by lints
- All tests pass without panics
- Error handling at every level

## Files Changed

### New Files
- `crates/zjj-core/src/beads/domain.rs` (825 lines) - Domain types
- `crates/zjj-core/src/beads/issue.rs` (533 lines) - Aggregate root

### Modified Files
- `crates/zjj-core/src/beads/mod.rs` - Re-exports, documentation

### Unchanged Files (Preserved)
- `crates/zjj-core/src/beads/types.rs` - Legacy types
- `crates/zjj-core/src/beads/db.rs` - Database operations
- `crates/zjj-core/src/beads/query.rs` - Query logic
- `crates/zjj-core/src/beads/analysis.rs` - Analysis functions

## Backward Compatibility

The refactoring maintains **100% backward compatibility**:

1. Legacy types (`BeadIssue`, `IssueStatus`) remain unchanged
2. All existing functions continue to work
3. New types are opt-in via separate imports
4. Gradual migration path available

## Next Steps

1. **Add conversion functions** between legacy and new types
2. **Update database layer** to use new types internally
3. **Update query layer** to work with `Issue`
4. **Update analysis layer** to work with `Issue`
5. **Add integration tests** for the full flow
6. **Update callers** incrementally
7. **Deprecate legacy types** with clear migration guide

## References

- Scott Wlaschin's "Domain Modeling Made Functional"
- "Thinking with Types" by Sandy Maguire
- "Domain-Driven Design" by Eric Evans
- "Functional Programming in Rust" blog posts

---

**Status**: Phase 1 complete - Domain types and aggregate root implemented
**Tests Passing**: Domain and issue module tests pass
**Compilation**: Beads module compiles cleanly
**Backward Compatible**: Yes
