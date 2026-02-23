# Domain Types Guide

**Canonical reference for developers working with the zjj domain layer**

This guide provides comprehensive documentation for all domain types in the zjj codebase. These types follow Domain-Driven Design (DDD) principles to make illegal states unrepresentable and ensure type safety throughout the system.

## Table of Contents

1. [Design Principles](#design-principles)
2. [Identifier Types](#identifier-types)
3. [Value Objects](#value-objects)
4. [State Enums](#state-enums)
5. [Aggregates](#aggregates)
6. [Domain Events](#domain-events)
7. [Repository Interfaces](#repository-interfaces)
8. [Common Patterns](#common-patterns)
9. [Error Handling](#error-handling)

---

## Design Principles

The domain layer follows these core principles:

### Parse-at-Boundaries Pattern

All identifier types validate their input on construction:

```rust
// Parse once at the boundary (e.g., CLI input, API request)
let session_name = SessionName::parse("my-session")?;
// session_name is guaranteed valid - no further checks needed
```

### Make Illegal States Unrepresentable

Use enums instead of optional fields:

```rust
// BAD: Option<String> for branch
struct Session {
    branch: Option<String>,  // What does None mean?
}

// GOOD: Enum with explicit states
enum BranchState {
    Detached,
    OnBranch { name: String },
}
```

### Zero Unwrap

The domain layer never uses `unwrap()`, `expect()`, or panics. All fallible operations return `Result<T, E>`.

### Pure Core

Domain logic is pure and deterministic:
- No I/O in domain types
- No global state
- Same input = same output

---

## Identifier Types

All identifiers are defined in `crates/zjj-core/src/domain/identifiers.rs`. This is the **single source of truth** for identifier types.

### Unified Error Type

All identifier validation uses a single `IdentifierError` enum with these variants:

| Variant | Description |
|---------|-------------|
| `Empty` | Identifier is empty or whitespace-only |
| `TooLong { max, actual }` | Exceeds type-specific maximum length |
| `InvalidCharacters { details }` | Contains disallowed characters |
| `InvalidFormat { details }` | Generic format validation error |
| `InvalidStart { expected }` | Doesn't start with required character |
| `InvalidPrefix { prefix, value }` | Missing required prefix (e.g., "bd-") |
| `InvalidHex { value }` | Invalid hexadecimal format |
| `NotAbsolutePath { value }` | Path is not absolute |
| `NullBytesInPath` | Path contains null bytes |
| `NotAscii { value }` | Identifier must be ASCII-only |
| `ContainsPathSeparators` | Identifier contains path separators |

### SessionName

**Purpose:** Human-readable name for sessions

**Validation Rules:**
- Must start with a letter (a-z, A-Z)
- Can contain letters, numbers, hyphens, underscores
- 1-63 characters
- Whitespace is trimmed before validation

**Example:**

```rust
use zjj_core::domain::SessionName;

// Valid
let name = SessionName::parse("my-session")?;
let name = SessionName::parse("  my_session  ")?;  // Trimmed

// Invalid
SessionName::parse("")?;  // Error: Empty
SessionName::parse("123-session")?;  // Error: must start with letter
SessionName::parse("my.session")?;  // Error: invalid characters
```

**Methods:**

```rust
impl SessionName {
    pub const MAX_LENGTH: usize = 63;

    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
}
```

---

### AgentId

**Purpose:** Unique identifier for agents (processes, workers, services)

**Validation Rules:**
- 1-128 characters
- Can contain alphanumeric, hyphen, underscore, dot, colon
- No trimming (whitespace is significant)

**Example:**

```rust
use zjj_core::domain::AgentId;

// Valid
let agent = AgentId::parse("agent-123")?;
let agent = AgentId::parse("worker:prod-1")?;
let agent = AgentId::from_process();  // Auto-generate from PID

// Invalid
AgentId::parse("")?;  // Error: Empty
AgentId::parse("agent/123")?;  // Error: invalid characters
```

**Methods:**

```rust
impl AgentId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
    pub fn from_process() -> Self;  // Generate from process ID
}
```

---

### WorkspaceName

**Purpose:** Name for workspace directories

**Validation Rules:**
- 1-255 characters
- Cannot contain path separators (`/`, `\`)
- Cannot contain null bytes

**Example:**

```rust
use zjj_core::domain::WorkspaceName;

// Valid
let workspace = WorkspaceName::parse("my-workspace")?;
let workspace = WorkspaceName::parse("project_alpha")?;

// Invalid
WorkspaceName::parse("")?;  // Error: Empty
WorkspaceName::parse("my/workspace")?;  // Error: contains path separator
WorkspaceName::parse("my\0workspace")?;  // Error: contains null byte
```

**Methods:**

```rust
impl WorkspaceName {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
}
```

---

### TaskId / BeadId

**Purpose:** Unique identifier for tasks/beads (issues)

**Validation Rules:**
- Must start with `bd-` prefix
- Followed by hexadecimal characters (0-9, a-f, A-F)
- At least one hex character required
- `BeadId` is a type alias for `TaskId`

**Example:**

```rust
use zjj_core::domain::{TaskId, BeadId};

// Valid
let task = TaskId::parse("bd-abc123")?;
let task = TaskId::parse("bd-ABC123DEF456")?;
let bead = BeadId::parse("bd-abc123")?;  // Same as TaskId

// Invalid
TaskId::parse("abc123")?;  // Error: missing bd- prefix
TaskId::parse("bd-xyz")?;  // Error: invalid hex
TaskId::parse("bd-")?;  // Error: no hex characters
```

**Methods:**

```rust
impl TaskId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
}

// BeadId is an alias:
pub type BeadId = TaskId;
```

---

### SessionId

**Purpose:** Unique identifier for sessions (internal/system IDs)

**Validation Rules:**
- Non-empty
- ASCII only
- More lenient than SessionName (for system-generated IDs)

**Example:**

```rust
use zjj_core::domain::SessionId;

// Valid
let id = SessionId::parse("session-abc123")?;
let id = SessionId::parse("sess-123")?;
let id = SessionId::parse("SESSION_ABC")?;

// Invalid
SessionId::parse("")?;  // Error: Empty
SessionId::parse("session-abc-日本語")?;  // Error: non-ASCII
```

**Methods:**

```rust
impl SessionId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
}
```

---

### AbsolutePath

**Purpose:** Validated absolute filesystem path

**Validation Rules:**
- Must be absolute (starts with `/` on Unix, `C:\` or similar on Windows)
- Must not contain null bytes

**Example:**

```rust
use zjj_core::domain::AbsolutePath;

// Valid (Unix)
let path = AbsolutePath::parse("/home/user/workspace")?;
let path = AbsolutePath::parse("/tmp/workspace")?;

// Invalid
AbsolutePath::parse("")?;  // Error: invalid format
AbsolutePath::parse("relative/path")?;  // Error: not absolute
AbsolutePath::parse("/path\0with\0nulls")?;  // Error: null bytes
```

**Methods:**

```rust
impl AbsolutePath {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
    pub fn to_path_buf(&self) -> std::path::PathBuf;
    pub fn exists(&self) -> bool;
    pub fn display(&self) -> impl std::fmt::Display + '_;
}
```

---

### QueueEntryId

**Purpose:** Database auto-increment ID for queue entries

**Validation Rules:**
- Must be positive (> 0)
- Stored as i64

**Example:**

```rust
use zjj_core::domain::QueueEntryId;

// Valid
let id = QueueEntryId::new(1)?;
let id = QueueEntryId::new(42)?;
let id = "123".parse::<QueueEntryId>()?;

// Invalid
QueueEntryId::new(0)?;  // Error: must be positive
QueueEntryId::new(-1)?;  // Error: must be positive
"abc".parse::<QueueEntryId>()?;  // Error: invalid format
```

**Methods:**

```rust
impl QueueEntryId {
    pub fn new(value: i64) -> Result<Self, IdentifierError>;
    pub const fn value(self) -> i64;
}
```

---

## Value Objects

Value objects are immutable types defined by their attributes, not identity.

### DedupeKey

**Location:** `crates/zjj-core/src/coordination/domain_types.rs`

**Purpose:** Deduplication key for queue entries to prevent duplicate work

**Validation Rules:**
- Non-empty string

**Example:**

```rust
use zjj_core::coordination::domain_types::DedupeKey;

let key = DedupeKey::new("workspace-123".to_string())?;
let key = DedupeKey::new_from_str("workspace-123")?;

// Invalid
DedupeKey::new(String::new())?;  // Error: Empty
```

**Methods:**

```rust
impl DedupeKey {
    pub fn new(value: String) -> Result<Self, DomainError>;
    pub fn new_from_str(value: &str) -> Result<Self, DomainError>;
    pub fn as_str(&self) -> &str;
    pub fn into_inner(self) -> String;
}
```

---

### Priority

**Location:** `crates/zjj-core/src/coordination/domain_types.rs`

**Purpose:** Queue priority value (lower = higher priority)

**Validation Rules:**
- No validation (any i32 is valid)
- Provides constants for common priorities

**Example:**

```rust
use zjj_core::coordination::domain_types::Priority;

let p = Priority::new(7);
let p = Priority::default();  // 5
let p = Priority::high();     // 1
let p = Priority::low();      // 10
```

**Methods:**

```rust
impl Priority {
    pub const fn new(value: i32) -> Self;
    pub const fn value(self) -> i32;
    pub const fn default() -> Self;
    pub const fn high() -> Self;
    pub const fn low() -> Self;
}
```

---

### Beads Domain Value Objects

**Location:** `crates/zjj-core/src/beads/domain.rs`

#### IssueId

**Purpose:** Validated issue identifier

**Validation Rules:**
- Non-empty
- 1-100 characters
- Alphanumeric, hyphens, underscores only

**Example:**

```rust
use zjj_core::beads::domain::IssueId;

let id = IssueId::new("ISSUE-123")?;
let id = IssueId::new("bug_fix_2024")?;
```

---

#### Title

**Purpose:** Validated issue title

**Validation Rules:**
- Non-empty (after trimming)
- 1-200 characters

**Example:**

```rust
use zjj_core::beads::domain::Title;

let title = Title::new("Fix the crash on startup")?;
let title = Title::new("  Trimmed title  ")?;  // Trimmed
```

---

#### Description

**Purpose:** Validated issue description (optional)

**Validation Rules:**
- 0-10,000 characters

**Example:**

```rust
use zjj_core::beads::domain::Description;

let desc = Description::new("Detailed description here")?;
let desc = Description::new("")?;  // Empty is OK
```

---

#### Assignee

**Purpose:** Username or email for issue assignment

**Validation Rules:**
- Non-empty
- 1-100 characters

**Example:**

```rust
use zjj_core::beads::domain::Assignee;

let assignee = Assignee::new("user@example.com")?;
let assignee = Assignee::new("john_doe")?;
```

---

#### Labels

**Purpose:** Collection of validated labels

**Validation Rules:**
- Maximum 20 labels
- Maximum 50 characters per label

**Example:**

```rust
use zjj_core::beads::domain::Labels;

let labels = Labels::new(vec![
    "bug".to_string(),
    "critical".to_string(),
])?;

let labels = Labels::empty();
let labels = labels.add("enhancement".to_string())?;
let labels = labels.remove("bug");
```

**Methods:**

```rust
impl Labels {
    pub const MAX_COUNT: usize = 20;
    pub const MAX_LABEL_LENGTH: usize = 50;

    pub fn new(labels: Vec<String>) -> Result<Self, DomainError>;
    pub const fn empty() -> Self;
    pub fn iter(&self) -> impl Iterator<Item = &String>;
    pub fn contains(&self, label: &str) -> bool;
    pub const fn len(&self) -> usize;
    pub const fn is_empty(&self) -> bool;
    pub fn add(&self, label: String) -> Result<Self, DomainError>;
    pub fn remove(&self, label: &str) -> Self;
    pub fn as_slice(&self) -> &[String];
    pub fn to_vec(&self) -> Vec<String>;
}
```

---

#### DependsOn / BlockedBy

**Purpose:** Collections of issue dependencies/blockers

**Validation Rules:**
- Maximum 50 dependencies/blockers

**Example:**

```rust
use zjj_core::beads::domain::{DependsOn, BlockedBy};

let deps = DependsOn::new(vec!["ISSUE-1".to_string(), "ISSUE-2".to_string()])?;
let blockers = BlockedBy::new(vec!["ISSUE-3".to_string()])?;
```

---

## State Enums

State enums make illegal states unrepresentable by encoding state-specific data in enum variants.

### AgentState

**Location:** `crates/zjj-core/src/domain/agent.rs`

**Purpose:** Current state of an agent

**Variants:**
- `Active` - Agent is processing
- `Idle` - Agent is available
- `Offline` - Agent is offline
- `Error` - Agent is in error state

**State Transitions:**

```rust
impl AgentState {
    // Valid transitions:
    // - Idle <-> Active (bidirectional)
    // - Any state -> Offline
    // - Any state -> Error
    // - Offline -> Idle

    pub const fn can_transition_to(self, target: &Self) -> bool;
    pub fn valid_transitions(&self) -> Vec<Self>;
}
```

**Example:**

```rust
use zjj_core::domain::AgentState;

let state = AgentState::Idle;

assert!(state.is_active() == false);
assert!(state.can_transition_to(&AgentState::Active));
assert!(!state.can_transition_to(&AgentState::Idle));  // No self-loops
```

---

### WorkspaceState

**Location:** `crates/zjj-core/src/domain/workspace.rs`

**Purpose:** Lifecycle state of a workspace

**Variants:**
- `Creating` - Workspace is being created
- `Ready` - Workspace ready for use
- `Active` - Workspace is in use
- `Cleaning` - Workspace is being cleaned up
- `Removed` - Workspace has been removed (terminal)

**State Transitions:**

```rust
impl WorkspaceState {
    // Valid transitions:
    // - Creating -> Ready | Removed
    // - Ready -> Active | Cleaning | Removed
    // - Active -> Cleaning | Removed
    // - Cleaning -> Removed
    // - Removed is terminal (no outgoing transitions)

    pub const fn can_transition_to(self, target: &Self) -> bool;
    pub fn valid_transitions(&self) -> Vec<Self>;
    pub const fn is_terminal(&self) -> bool;
}
```

**Example:**

```rust
use zjj_core::domain::WorkspaceState;

let state = WorkspaceState::Ready;

assert!(state.is_ready());
assert!(state.is_active() == false);
assert!(state.can_transition_to(&WorkspaceState::Active));
```

---

### BranchState

**Location:** `crates/zjj-core/src/domain/session.rs`

**Purpose:** Git branch state for a session

**Variants:**
- `Detached` - Session is detached (no branch)
- `OnBranch { name: String }` - Session is on a specific branch

**Example:**

```rust
use zjj_core::domain::session::BranchState;

let state = BranchState::OnBranch {
    name: "main".to_string(),
};

assert_eq!(state.branch_name(), Some("main"));
assert!(!state.is_detached());

// Valid transitions
assert!(state.can_transition_to(&BranchState::Detached));
```

---

### ParentState

**Location:** `crates/zjj-core/src/domain/session.rs`

**Purpose:** Parent relationship for sessions

**Variants:**
- `Root` - Root session (no parent)
- `ChildOf { parent: SessionName }` - Child session with a parent

**Example:**

```rust
use zjj_core::domain::{session::ParentState, SessionName};

let parent = SessionName::parse("parent-session")?;
let state = ParentState::ChildOf {
    parent: parent.clone(),
};

assert!(state.is_child());
assert_eq!(state.parent_name(), Some(&parent));

// Valid transitions: ChildOf can change to another parent
let new_parent = SessionName::parse("new-parent")?;
let new_state = ParentState::ChildOf {
    parent: new_parent,
};
assert!(state.can_transition_to(&new_state));
```

---

### ClaimState

**Location:** `crates/zjj-core/src/domain/queue.rs`

**Purpose:** Claim state for queue entries

**Variants:**
- `Unclaimed` - Entry is not claimed
- `Claimed { agent, claimed_at, expires_at }` - Entry is claimed
- `Expired { previous_agent, expired_at }` - Previous claim expired

**State Transitions:**

```rust
impl ClaimState {
    // Valid transitions:
    // - Unclaimed -> Claimed
    // - Claimed -> Expired | Unclaimed
    // - Expired -> Unclaimed

    pub fn can_transition_to(&self, target: &Self) -> bool;
    pub fn valid_transition_types(&self) -> Vec<&'static str>;
}
```

**Example:**

```rust
use zjj_core::domain::{queue::ClaimState, AgentId};
use chrono::{Utc, Duration};

let agent = AgentId::parse("agent-1")?;
let now = Utc::now();
let expires = now + Duration::seconds(300);

let claimed = ClaimState::Claimed {
    agent: agent.clone(),
    claimed_at: now,
    expires_at: expires,
};

assert!(claimed.is_claimed());
assert_eq!(claimed.holder(), Some(&agent));
```

---

### IssueState

**Location:** `crates/zjj-core/src/beads/domain.rs`

**Purpose:** State of an issue/task

**Variants:**
- `Open` - Issue is open
- `InProgress` - Issue is being worked on
- `Blocked` - Issue is blocked
- `Deferred` - Issue is deferred
- `Closed { closed_at: DateTime<Utc> }` - Issue is closed (requires timestamp!)

**Key Design:** The `Closed` variant *must* include a timestamp, making it impossible to have a closed issue without knowing when it was closed.

**Example:**

```rust
use zjj_core::beads::domain::IssueState;
use chrono::Utc;

let state = IssueState::Open;
assert!(state.is_active());
assert!(!state.is_closed());

let closed = IssueState::Closed {
    closed_at: Utc::now(),
};
assert!(!closed.is_active());
assert!(closed.is_closed());
assert!(closed.closed_at().is_some());
```

---

### QueueCommand

**Location:** `crates/zjj-core/src/domain/queue.rs`

**Purpose:** Commands for queue operations

**Variants:**
- `List` - List queue entries
- `Process` - Process next entry
- `Next` - Get next without processing
- `Stats` - Show statistics
- `ShowStatus { workspace }` - Show workspace status
- `Add { workspace, bead, priority, agent }` - Add to queue
- `Remove { workspace }` - Remove from queue
- `Retry { entry_id }` - Retry failed entry
- `Cancel { entry_id }` - Cancel entry
- `ReclaimStale { threshold_secs }` - Reclaim stale entries
- `ShowById { entry_id }` - Show by entry ID

**Example:**

```rust
use zjj_core::domain::queue::QueueCommand;
use zjj_core::domain::{WorkspaceName, AgentId};

let cmd = QueueCommand::Add {
    workspace: WorkspaceName::parse("my-workspace")?,
    bead: None,
    priority: 1,
    agent: Some(AgentId::parse("agent-1")?),
};

assert_eq!(cmd.name(), "add");
```

---

## Aggregates

Aggregates are clusters of domain objects treated as a unit. They are the roots of consistency boundaries.

### Session

**Location:** `crates/zjj-core/src/domain/repository.rs`

**Purpose:** Represents a development session

**Fields:**

```rust
pub struct Session {
    pub id: SessionId,           // Unique session identifier
    pub name: SessionName,       // Human-readable name
    pub branch: BranchState,     // Git branch state
    pub parent: ParentState,     // Parent relationship
    pub workspace_path: PathBuf, // Absolute path to workspace
}
```

**Methods:**

```rust
impl Session {
    pub fn is_active(&self) -> bool {
        // Active if: not detached AND workspace exists
        !self.branch.is_detached() && self.workspace_path.exists()
    }
}
```

**Example:**

```rust
use zjj_core::domain::repository::Session;
use zjj_core::domain::*;
use std::path::PathBuf;

let session = Session {
    id: SessionId::parse("session-123")?,
    name: SessionName::parse("my-session")?,
    branch: BranchState::OnBranch {
        name: "main".to_string(),
    },
    parent: ParentState::Root,
    workspace_path: PathBuf::from("/home/user/project"),
};

if session.is_active() {
    println!("Session is active on {}", session.name);
}
```

---

### Workspace

**Location:** `crates/zjj-core/src/domain/repository.rs`

**Purpose:** Represents a workspace

**Fields:**

```rust
pub struct Workspace {
    pub name: WorkspaceName,  // Workspace name
    pub path: PathBuf,        // Absolute path
    pub state: WorkspaceState, // Current state
}
```

**Example:**

```rust
use zjj_core::domain::repository::Workspace;
use zjj_core::domain::*;

let workspace = Workspace {
    name: WorkspaceName::parse("my-workspace")?,
    path: PathBuf::from("/tmp/my-workspace"),
    state: WorkspaceState::Ready,
};
```

---

### Bead

**Location:** `crates/zjj-core/src/domain/repository.rs`

**Purpose:** Represents a task/issue (bead)

**Fields:**

```rust
pub struct Bead {
    pub id: BeadId,
    pub title: String,
    pub description: Option<String>,
    pub state: BeadState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

**Associated State:**

```rust
pub enum BeadState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: chrono::DateTime<chrono::Utc> },
}
```

---

### QueueEntry

**Location:** `crates/zjj-core/src/domain/repository.rs`

**Purpose:** Represents an entry in the distributed queue

**Fields:**

```rust
pub struct QueueEntry {
    pub id: i64,                            // Database-generated ID
    pub workspace: WorkspaceName,            // Workspace to process
    pub bead: Option<BeadId>,               // Optional bead to process
    pub priority: i32,                       // Lower = higher priority
    pub claim_state: ClaimState,            // Current claim state
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

---

### Agent

**Location:** `crates/zjj-core/src/domain/repository.rs`

**Purpose:** Represents an agent (worker process)

**Fields:**

```rust
pub struct Agent {
    pub id: AgentId,
    pub state: AgentState,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}
```

---

## Domain Events

**Location:** `crates/zjj-core/src/domain/events.rs`

Domain events represent important business events that have occurred in the system. They are:
- **Immutable** - Cannot be modified after creation
- **Serializable** - Can be persisted and transmitted
- **Timestamped** - Include when they occurred

### Event Types

#### SessionCreatedEvent

```rust
pub struct SessionCreatedEvent {
    pub session_id: String,
    pub session_name: SessionName,
    pub timestamp: DateTime<Utc>,
}
```

#### SessionCompletedEvent

```rust
pub struct SessionCompletedEvent {
    pub session_id: String,
    pub session_name: SessionName,
    pub timestamp: DateTime<Utc>,
}
```

#### SessionFailedEvent

```rust
pub struct SessionFailedEvent {
    pub session_id: String,
    pub session_name: SessionName,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}
```

#### WorkspaceCreatedEvent

```rust
pub struct WorkspaceCreatedEvent {
    pub workspace_name: WorkspaceName,
    pub path: PathBuf,
    pub timestamp: DateTime<Utc>,
}
```

#### WorkspaceRemovedEvent

```rust
pub struct WorkspaceRemovedEvent {
    pub workspace_name: WorkspaceName,
    pub path: PathBuf,
    pub timestamp: DateTime<Utc>,
}
```

#### QueueEntryAddedEvent

```rust
pub struct QueueEntryAddedEvent {
    pub entry_id: i64,
    pub workspace_name: WorkspaceName,
    pub priority: i32,
    pub bead_id: Option<BeadId>,
    pub timestamp: DateTime<Utc>,
}
```

#### QueueEntryClaimedEvent

```rust
pub struct QueueEntryClaimedEvent {
    pub entry_id: i64,
    pub workspace_name: WorkspaceName,
    pub agent: AgentId,
    pub claimed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub timestamp: DateTime<Utc>,
}
```

#### QueueEntryCompletedEvent

```rust
pub struct QueueEntryCompletedEvent {
    pub entry_id: i64,
    pub workspace_name: WorkspaceName,
    pub agent: AgentId,
    pub timestamp: DateTime<Utc>,
}
```

#### BeadCreatedEvent

```rust
pub struct BeadCreatedEvent {
    pub bead_id: BeadId,
    pub title: String,
    pub description: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

#### BeadClosedEvent

```rust
pub struct BeadClosedEvent {
    pub bead_id: BeadId,
    pub closed_at: DateTime<Utc>,
    pub timestamp: DateTime<Utc>,
}
```

### Event Creation

Events are created using factory methods on `DomainEvent`:

```rust
use zjj_core::domain::events::DomainEvent;
use zjj_core::domain::*;
use chrono::Utc;

let event = DomainEvent::session_created(
    "session-123".to_string(),
    SessionName::parse("my-session")?,
    Utc::now(),
);

let event = DomainEvent::workspace_created(
    WorkspaceName::parse("my-workspace")?,
    PathBuf::from("/tmp/workspace"),
    Utc::now(),
);
```

### Event Metadata

```rust
pub struct EventMetadata {
    pub event_number: i64,     // Unique sequence number
    pub stream_id: String,     // Stream identifier
    pub stream_version: i64,   // Stream version
    pub stored_at: DateTime<Utc>,
}

pub struct StoredEvent {
    pub event: DomainEvent,
    pub metadata: EventMetadata,
}
```

---

## Repository Interfaces

**Location:** `crates/zjj-core/src/domain/repository.rs`

Repository traits abstract data access behind interfaces, enabling:
- Dependency injection
- Testing with mocks
- Swappable backends (SQLite, PostgreSQL, in-memory)

### Common Error Type

```rust
pub enum RepositoryError {
    NotFound(String),
    Conflict(String),
    InvalidInput(String),
    StorageError(String),
    NotSupported(String),
    ConcurrentModification(String),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;
```

### SessionRepository

```rust
pub trait SessionRepository: Send + Sync {
    fn load(&self, id: &SessionId) -> RepositoryResult<Session>;
    fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session>;
    fn save(&self, session: &Session) -> RepositoryResult<()>;
    fn delete(&self, id: &SessionId) -> RepositoryResult<()>;
    fn list_all(&self) -> RepositoryResult<Vec<Session>>;
    fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>>;
    fn exists(&self, id: &SessionId) -> RepositoryResult<bool>;
    fn get_current(&self) -> RepositoryResult<Option<Session>>;
    fn set_current(&self, id: &SessionId) -> RepositoryResult<()>;
    fn clear_current(&self) -> RepositoryResult<()>;
}
```

### WorkspaceRepository

```rust
pub trait WorkspaceRepository: Send + Sync {
    fn load(&self, name: &WorkspaceName) -> RepositoryResult<Workspace>;
    fn save(&self, workspace: &Workspace) -> RepositoryResult<()>;
    fn delete(&self, name: &WorkspaceName) -> RepositoryResult<()>;
    fn list_all(&self) -> RepositoryResult<Vec<Workspace>>;
    fn exists(&self, name: &WorkspaceName) -> RepositoryResult<bool>;
}
```

### BeadRepository

```rust
pub trait BeadRepository: Send + Sync {
    fn load(&self, id: &BeadId) -> RepositoryResult<Bead>;
    fn save(&self, bead: &Bead) -> RepositoryResult<()>;
    fn delete(&self, id: &BeadId) -> RepositoryResult<()>;
    fn list_all(&self) -> RepositoryResult<Vec<Bead>>;
    fn list_by_state(&self, state: BeadState) -> RepositoryResult<Vec<Bead>>;
    fn exists(&self, id: &BeadId) -> RepositoryResult<bool>;
}
```

### QueueRepository

```rust
pub trait QueueRepository: Send + Sync {
    fn load(&self, id: i64) -> RepositoryResult<QueueEntry>;
    fn save(&self, entry: &QueueEntry) -> RepositoryResult<()>;
    fn delete(&self, id: i64) -> RepositoryResult<()>;
    fn list_all(&self) -> RepositoryResult<Vec<QueueEntry>>;
    fn list_unclaimed(&self) -> RepositoryResult<Vec<QueueEntry>>;
    fn claim_next(
        &self,
        agent: &AgentId,
        claim_duration_secs: i64,
    ) -> RepositoryResult<Option<QueueEntry>>;
    fn release(&self, id: i64, agent: &AgentId) -> RepositoryResult<()>;
    fn expire_claims(&self, older_than_secs: i64) -> RepositoryResult<usize>;
    fn add_workspace(
        &self,
        workspace: &WorkspaceName,
        bead: Option<&BeadId>,
        priority: i32,
    ) -> RepositoryResult<i64>;
    fn remove_workspace(&self, workspace: &WorkspaceName) -> RepositoryResult<()>;
    fn stats(&self) -> RepositoryResult<QueueStats>;
}

pub struct QueueStats {
    pub total: usize,
    pub unclaimed: usize,
    pub claimed: usize,
    pub expired: usize,
}
```

### AgentRepository

```rust
pub trait AgentRepository: Send + Sync {
    fn load(&self, id: &AgentId) -> RepositoryResult<Agent>;
    fn save(&self, agent: &Agent) -> RepositoryResult<()>;
    fn heartbeat(&self, id: &AgentId) -> RepositoryResult<()>;
    fn list_all(&self) -> RepositoryResult<Vec<Agent>>;
    fn list_active(&self) -> RepositoryResult<Vec<Agent>>;
}
```

---

## Common Patterns

### Pattern 1: Parse at Boundaries

Validate input once at system boundaries (CLI, API, config):

```rust
// BAD: Validate everywhere
fn process_session(name: &str) {
    if name.is_empty() || name.len() > 63 {
        return Err("Invalid name");
    }
    // ...
}

// GOOD: Parse once, use validated type
fn process_session(name: SessionName) -> Result<()> {
    // name is guaranteed valid - no further checks needed
    // ...
}

// At boundary (CLI, API)
let name = SessionName::parse(raw_input)?;
process_session(name)?;
```

### Pattern 2: Enum State Transitions

Use `can_transition_to` for state machine validation:

```rust
use zjj_core::domain::WorkspaceState;

fn change_workspace(
    workspace: &mut Workspace,
    new_state: WorkspaceState,
) -> Result<(), RepositoryError> {
    if !workspace.state.can_transition_to(&new_state) {
        return Err(RepositoryError::InvalidInput(format!(
            "Cannot transition from {:?} to {:?}",
            workspace.state, new_state
        )));
    }
    workspace.state = new_state;
    Ok(())
}
```

### Pattern 3: Repository Pattern

Use dependency injection for testable business logic:

```rust
// Business logic depends on trait
fn get_active_sessions(
    repo: &dyn SessionRepository,
) -> Result<Vec<Session>, RepositoryError> {
    let all = repo.list_all()?;
    Ok(all.into_iter()
        .filter(|s| s.is_active())
        .collect())
}

// Test with mock
struct MockSessionRepo { sessions: Vec<Session> }
impl SessionRepository for MockSessionRepo { /* ... */ }

#[test]
fn test_get_active_sessions() {
    let mock = MockSessionRepo { /* ... */ };
    let active = get_active_sessions(&mock).unwrap();
    assert_eq!(active.len(), 1);
}
```

### Pattern 4: Option Replacement with Enums

Replace `Option<T>` with semantic enums:

```rust
// BAD: Option doesn't convey meaning
struct Session {
    branch: Option<String>,      // What does None mean?
    parent: Option<String>,      // What does None mean?
}

// GOOD: Enums make states explicit
struct Session {
    branch: BranchState,         // Detached or OnBranch
    parent: ParentState,         // Root or ChildOf
}
```

### Pattern 5: Domain Events for Audit

Emit events for all state changes:

```rust
fn close_session(
    repo: &dyn SessionRepository,
    event_bus: &dyn EventBus,
    session_id: &SessionId,
) -> Result<()> {
    let session = repo.load(session_id)?;
    // ... close session logic ...
    repo.save(&session)?;

    // Emit event
    let event = DomainEvent::session_completed(
        session.id.into_string(),
        session.name.clone(),
        Utc::now(),
    );
    event_bus.publish(event)?;

    Ok(())
}
```

---

## Error Handling

### Domain Errors (Core)

Domain errors use `thiserror` for expected failures:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: State, to: State },
}
```

### Repository Errors (Boundary)

Repository errors abstract storage failures:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("entity not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("storage error: {0}")]
    StorageError(String),
}
```

### Error Conversion

Convert between error layers with `?`:

```rust
fn use_session_name(name: &str) -> Result<(), DomainError> {
    let validated = SessionName::parse(name)?;  // IdentifierError -> DomainError
    // ...
    Ok(())
}
```

---

## Best Practices

1. **Always use identifier types** - Never pass raw strings after validation
2. **Validate at boundaries** - Parse once, trust everywhere
3. **Use enums for state** - Make illegal states unrepresentable
4. **Never unwrap** - Always handle errors properly
5. **Emit domain events** - Record all important state changes
6. **Use repository traits** - Enable testing and flexibility
7. **Keep aggregates small** - Focus on consistency boundaries
8. **Document invariants** - Make validation rules explicit

---

## Module Structure

```
crates/zjj-core/src/domain/
├── mod.rs              # Exports all domain types
├── identifiers.rs      # All ID types (SessionName, AgentId, etc.)
├── agent.rs           # Agent state and info
├── session.rs         # Session state types (BranchState, ParentState)
├── workspace.rs       # Workspace state
├── queue.rs           # Queue state (ClaimState, QueueCommand)
├── events.rs          # Domain events
└── repository.rs      # Repository traits and aggregates

crates/zjj-core/src/beads/
└── domain.rs          # Beads-specific domain types

crates/zjj-core/src/coordination/
└── domain_types.rs    # Queue-specific value objects
```

---

## Further Reading

- [Domain-Driven Design](https://domainlanguage.com/ddd/) by Eric Evans
- [Functional Core, Imperative Shell](https://www.destroyallsoftware.com/talks/boundaries) by Gary Bernhardt
- [Making Illegal States Unrepresentable](https://alexkuznetsov.dev/blog/making-illegal-states-unrepresentable/) by Alex Kuznetsov
