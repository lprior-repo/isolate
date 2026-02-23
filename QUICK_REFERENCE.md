# ZJJ Domain Layer - Quick Reference Card

> Single-page cheat sheet for zjj domain types. Keep this open while coding.

---

## IDENTIFIER TYPES

### Parse Pattern (All Identifiers)
```rust
// All identifiers use this pattern:
let name = SessionName::parse("my-session")?;
let id = AgentId::parse("agent-123")?;

// Access methods:
name.as_str()              // &str
name.into_string()         // String
name.to_string()           // String (via Display)

// Traits impl: TryFrom<String>, TryFrom<&str>, FromStr, Display, AsRef<str>
```

### SessionName
- **Pattern**: `parse()` - trims whitespace, then validates
- **Rules**: Start with letter, 1-63 chars, alphanumeric/hyphen/underscore
- **Error**: `SessionNameError` (alias of `IdentifierError`)

### AgentId
- **Rules**: 1-128 chars, alphanumeric/hyphen/underscore/dot/colon
- **Error**: `AgentIdError`
- **Special**: `AgentId::from_process()` for default

### WorkspaceName
- **Rules**: 1-255 chars, no path separators or null bytes
- **Error**: `WorkspaceNameError`

### TaskId / BeadId
- **Rules**: Must start with `bd-` prefix, followed by hex
- **Examples**: `bd-abc123`, `bd-ABC123DEF456`
- **Error**: `TaskIdError` / `BeadIdError`
- **Note**: `BeadId` is type alias of `TaskId`

### SessionId
- **Rules**: Non-empty, ASCII only
- **Error**: `SessionIdError`

### AbsolutePath
- **Rules**: Must be absolute (starts with `/` on Unix), no null bytes
- **Methods**: `to_path_buf()`, `exists()`, `display()`
- **Error**: `AbsolutePathError`

### QueueEntryId
- **Type**: `i64` wrapper, must be positive (> 0)
- **Constructor**: `QueueEntryId::new(42)?`
- **Access**: `id.value()` -> i64
- **Error**: `QueueEntryIdError`

---

## STATE ENUMS

### AgentState
```rust
pub enum AgentState {
    Active,
    Idle,
    Offline,
    Error,
}
// Methods: is_active(), is_offline(), can_transition_to(), valid_transitions()
// Valid: Idle <-> Active, Any -> Offline/Error, Offline -> Idle
```

### WorkspaceState
```rust
pub enum WorkspaceState {
    Creating,
    Ready,
    Active,
    Cleaning,
    Removed,  // Terminal
}
// Methods: is_active(), is_ready(), is_removed(), is_terminal(), can_transition_to()
// Valid: Creating->Ready/Removed, Ready->Active/Cleaning/Removed, Active->Cleaning/Removed
```

### BranchState (Session)
```rust
pub enum BranchState {
    Detached,
    OnBranch { name: String },
}
// Methods: branch_name() -> Option<&str>, is_detached(), can_transition_to()
// Valid: Detached<->OnBranch, OnBranch->OnBranch (switch)
```

### ParentState (Session)
```rust
pub enum ParentState {
    Root,
    ChildOf { parent: SessionName },
}
// Methods: parent_name() -> Option<&SessionName>, is_root(), is_child()
// Valid: Root->ChildOf (creation), ChildOf->ChildOf (adoption), never to Root
```

### ClaimState (Queue)
```rust
pub enum ClaimState {
    Unclaimed,
    Claimed { agent: AgentId, claimed_at: DateTime<Utc>, expires_at: DateTime<Utc> },
    Expired { previous_agent: AgentId, expired_at: DateTime<Utc> },
}
// Methods: is_claimed(), is_unclaimed(), holder() -> Option<&AgentId>, can_transition_to()
// Valid: Unclaimed->Claimed, Claimed->Expired/Unclaimed, Expired->Unclaimed
```

### IssueState (Beads)
```rust
pub enum IssueState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },  // Timestamp REQUIRED
}
// Methods: is_active(), is_blocked(), is_closed(), closed_at() -> Option<DateTime<Utc>>
```

### QueueCommand
```rust
pub enum QueueCommand {
    List, Process, Next, Stats,
    ShowStatus { workspace: WorkspaceName },
    Add { workspace, bead, priority, agent },
    Remove { workspace },
    Retry { entry_id }, Cancel { entry_id },
    ReclaimStale { threshold_secs },
    ShowById { entry_id },
}
```

---

## VALUE OBJECTS

### Priority (Beads)
```rust
pub enum Priority { P0, P1, P2, P3, P4 }
// Lower number = higher priority
// Methods: Priority::from_u32(n), to_u32()
```

### IssueType (Beads)
```rust
pub enum IssueType {
    Bug, Feature, Task, Epic, Chore, MergeRequest,
}
// Strum: EnumString, Display (lowercase)
```

### Title (Beads)
```rust
pub struct Title(String);
// Constructor: Title::new("title")? - trims, non-empty, max 200 chars
// Methods: as_str(), into_inner()
```

### Description (Beads)
```rust
pub struct Description(String);
// Constructor: Description::new("desc")? - max 10,000 chars
// Methods: as_str(), into_inner()
```

### Labels (Beads)
```rust
pub struct Labels(Vec<String>);
// Constructor: Labels::new(vec!["label".into()])?
// Limits: max 20 labels, 50 chars each
// Methods: iter(), contains(), len(), is_empty(), add(), remove()
```

### DependsOn / BlockedBy (Beads)
```rust
pub struct DependsOn(Vec<IssueId>);
pub struct BlockedBy(Vec<IssueId>);
// Constructor: DependsOn::new(vec!["bd-abc".into()])?
// Limits: max 50 dependencies/blockers
// Methods: iter(), contains(), len(), is_empty()
```

---

## ERROR TYPES

### IdentifierError (Unified)
```rust
pub enum IdentifierError {
    Empty,
    TooLong { max, actual },
    InvalidCharacters { details },
    InvalidFormat { details },
    InvalidStart { expected },
    InvalidPrefix { prefix, value },
    InvalidHex { value },
    NotAbsolutePath { value },
    NullBytesInPath,
    NotAscii { value },
    ContainsPathSeparators,
}
// Type aliases: SessionNameError, AgentIdError, WorkspaceNameError, etc.
// Helper methods: IdentifierError::empty(), too_long(), invalid_characters(), etc.
```

### DomainError (Beads)
```rust
pub enum DomainError {
    EmptyId, InvalidIdPattern(String),
    EmptyTitle, TitleTooLong { max, got },
    DescriptionTooLong { max },
    InvalidDatetime(String),
    NotFound(String), DuplicateId(String),
    InvalidStateTransition { from, to },
    ClosedWithoutTimestamp,
    InvalidFilter(String),
}
```

### ContractError (CLI Contracts)
```rust
pub enum ContractError {
    PreconditionFailed { name, description },
    InvariantViolation { name, description },
    PostconditionFailed { name, description },
    Multiple(String),
    InvalidInput { field, reason },
    InvalidStateTransition { from, to },
    NotFound { resource_type, identifier },
    InvalidOperationForState { operation, resource_type, state },
    ConcurrentModification { description },
}
// Helpers: ContractError::invalid_input(), invalid_transition(), not_found(), combine()
```

### Aggregate Errors
- `SessionError` - Session aggregate violations
- `WorkspaceError` - Workspace aggregate violations
- `QueueEntryError` - Queue entry aggregate violations
- `BeadError` - Bead aggregate violations

---

## KEY IMPORTS

### Domain Layer
```rust
// Identifiers
use zjj_core::domain::{
    SessionName, AgentId, WorkspaceName, TaskId, BeadId,
    SessionId, AbsolutePath, QueueEntryId,
    IdentifierError, SessionNameError, AgentIdError,
};

// State types
use zjj_core::domain::{
    AgentState, WorkspaceState, BranchState, ParentState,
    ClaimState, IssueState,
};

// Value objects (beads)
use zjj_core::beads::{
    Title, Description, Priority, IssueType,
    Labels, DependsOn, BlockedBy, DomainError,
};

// Aggregates
use zjj_core::domain::{
    Session, Workspace, QueueEntry, Bead,
    SessionBuilder, WorkspaceBuilder,
};

// Events
use zjj_core::domain::{DomainEvent, EventMetadata, StoredEvent};

// CLI contracts
use zjj_core::cli_contracts::ContractError;
```

### Functional Core 6
```rust
use itertools::{Itertools, iproduct};  // Iterator pipelines
use tap::Pipe;                         // Pipeline ergonomics
use rpds::{Vector, HashTrieMap};       // Persistent state
use thiserror::Error;                  // Domain errors (core)
use anyhow::{Result, Context};         // Boundary errors (shell)
use futures_util::{StreamExt, TryStreamExt};  // Async streams
```

---

## COMMON PATTERNS

### Parse-Validate-Use (Identifiers)
```rust
// 1. Parse at boundary
let name = SessionName::parse(raw_input)?;

// 2. Use safely (no unwrap needed)
println!("Session: {}", name);

// 3. Convert back if needed
let s: String = name.into();
```

### State Transition Validation
```rust
// Check before transition
if !current_state.can_transition_to(&target_state) {
    return Err(ContractError::invalid_transition(current_state, target_state));
}

// Then apply
let new_state = target_state;
```

### Builder Pattern (Aggregates)
```rust
let session = SessionBuilder::new()
    .name("my-session")?
    .base("/path")?
    .branch(Some("main".into()))?
    .build()?;
```

### Error Conversion (Domain -> Boundary)
```rust
use anyhow::Context;

// Core returns DomainError
fn validate(input: &str) -> Result<&str, DomainError> { ... }

// Shell converts with context
async fn load(path: &str) -> anyhow::Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .context(format!("failed to read {path}"))?
        .pipe(|s| validate(s)?)
        .context("validation failed")
}
```

### Iterator Pipelines (itertools)
```rust
use itertools::Itertools;

let result: Vec<_> = items
    .iter()
    .map(|x| x.trim())
    .filter(|x| !x.is_empty())
    .unique()
    .sorted()
    .collect();
```

---

## SERIALIZATION NOTES

### serde (All Identifiers)
- All identifiers: `#[serde(try_from = "String")]`
- Validated on deserialize, returns error if invalid
- State enums: `#[serde(rename_all = "snake_case")]` or `"lowercase"`

### chrono (DateTime)
```rust
use chrono::{DateTime, Utc};

// Serialize as RFC3339 by default
#[serde(rename = "created_at")]
pub created_at: DateTime<Utc>,
```

---

## QUICK CHEATSHEET

### Creating Identifiers
| Type | Constructor | Example |
|------|-------------|---------|
| SessionName | `parse("name")` | Must start with letter |
| AgentId | `parse("id")` | Alphanumeric + `-_.:` |
| WorkspaceName | `parse("name")` | No path separators |
| TaskId/BeadId | `parse("bd-123")` | Must have `bd-` prefix |
| AbsolutePath | `parse("/path")` | Must start with `/` |
| QueueEntryId | `new(42)` | Must be positive i64 |

### State Queries
| Type | Active Check | Terminal Check |
|------|--------------|----------------|
| AgentState | `is_active()` | none |
| WorkspaceState | `is_ready()` | `is_terminal()` |
| IssueState | `is_active()` | `is_closed()` |
| ClaimState | `is_claimed()` | none |

### Error Helpers
| Error Type | Helper | Example |
|------------|--------|---------|
| ContractError | `invalid_input(field, reason)` | `ContractError::invalid_input("name", "empty")` |
| ContractError | `not_found(type, id)` | `ContractError::not_found("Session", "foo")` |
| IdentifierError | `empty()` | `IdentifierError::empty()` |
| IdentifierError | `too_long(max, actual)` | `IdentifierError::too_long(63, 100)` |

---

## FILE HEADERS (Required)

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

---

## FUNCTIONAL PRINCIPLES

### Zero Unwrap
- AVOID: `unwrap()`, `expect()`, `unwrap_or()`
- USE: `match`, `if let`, `map()`, `and_then()`, `ok_or_else()`

### Zero Mut
- AVOID: `let mut`, `mut` parameters
- USE: `fold()`, `scan()`, `collect()`, `rpds` persistent collections

### Zero Panics
- AVOID: `panic!()`, `todo!()`, `unimplemented!()`
- USE: `Result<T, E>`, proper error propagation with `?`

---

*Generated for zjj domain layer - Single source of truth: domain/ modules*
