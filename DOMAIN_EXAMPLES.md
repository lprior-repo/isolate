# Domain Types - Practical Examples

**Comprehensive usage examples for zjj domain types**

This guide provides copy-pasteable examples for working with all domain types in the zjj codebase. Each example demonstrates real-world patterns with before/after comparisons, common pitfalls, and best practices.

## Table of Contents

1. [Parsing and Validating Identifiers](#1-parsing-and-validating-identifiers)
2. [Working with State Machines](#2-working-with-state-machines)
3. [Using Value Objects](#3-using-value-objects)
4. [Working with Aggregates](#4-working-with-aggregates)
5. [Using Domain Events](#5-using-domain-events)
6. [Error Handling Patterns](#6-error-handling-patterns)

---

## 1. Parsing and Validating Identifiers

### Pattern: Parse at Boundaries

**Before (Wrong):** Validating everywhere

```rust
// BAD: Validation scattered throughout code
fn process_session(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 63 {
        return Err("Invalid session name".into());
    }
    // ... more validation ...
    // ... business logic ...
    Ok(())
}

fn save_session(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 63 {
        return Err("Invalid session name".into());
    }
    // ... save logic ...
    Ok(())
}
```

**After (Correct):** Parse once, use validated type

```rust
use zjj_core::domain::SessionName;

// GOOD: Parse at boundary, use validated type everywhere
fn process_session(name: SessionName) -> Result<()> {
    // name is guaranteed valid - no further checks needed
    println!("Processing session: {}", name);
    Ok(())
}

fn save_session(name: &SessionName) -> Result<()> {
    // Already validated - just use it
    println!("Saving session: {}", name);
    Ok(())
}

// At the boundary (CLI, API, config file loading)
fn handle_user_input(raw_input: &str) -> Result<()> {
    let name = SessionName::parse(raw_input)?;  // Validate once
    process_session(name)?;
    save_session(&name)?;
    Ok(())
}
```

### Example: SessionName Validation

```rust
use zjj_core::domain::SessionName;

// Valid session names
let name1 = SessionName::parse("my-session")?;
let name2 = SessionName::parse("my_session")?;
let name3 = SessionName::parse("session-123")?;

// Whitespace trimming
let name4 = SessionName::parse("  my-session  ")?;
assert_eq!(name4.as_str(), "my-session");  // Trimmed!

// Invalid - returns Err
match SessionName::parse("") {
    Err(e) => println!("Error: {}", e),  // "identifier cannot be empty"
    Ok(_) => panic!("Should have failed"),
}

match SessionName::parse("123-session") {
    Err(e) => println!("Error: {}", e),  // "identifier must start with a letter"
    Ok(_) => panic!("Should have failed"),
}

// Using the validated name
fn use_session_name(name: &SessionName) -> String {
    name.as_str().to_uppercase()
}

assert_eq!(use_session_name(&name1), "MY-SESSION");
```

### Example: Multiple Identifiers

```rust
use zjj_core::domain::{SessionName, AgentId, WorkspaceName, TaskId};

// Parse different identifier types
let session = SessionName::parse("my-session")?;
let agent = AgentId::parse("agent-123")?;
let workspace = WorkspaceName::parse("my-workspace")?;
let task = TaskId::parse("bd-abc123def456")?;

// Auto-generate AgentId from process
let auto_agent = AgentId::from_process();
println!("Agent ID: {}", auto_agent);  // "pid-12345"

// Convert to/from String
let session_string: String = session.clone().into();
let session_again = SessionName::parse(session_string)?;
assert_eq!(session, session_again);

// Use as_str() to borrow without allocating
fn print_name(name: &SessionName) {
    println!("{}", name.as_str());  // No allocation
}
```

### Example: Batch Parsing with Error Collection

```rust
use zjj_core::domain::SessionName;
use std::collections::HashMap;

fn parse_session_names(inputs: Vec<&str>) -> HashMap<String, Result<SessionName, String>> {
    inputs
        .into_iter()
        .map(|input| {
            let result = SessionName::parse(input)
                .map(|name| name.to_string())
                .map_err(|e| e.to_string());
            (input.to_string(), result)
        })
        .collect()
}

let results = parse_session_names(vec![
    "valid-session",
    "123-invalid",
    "",
    "another-valid",
]);

// Process results
for (input, result) in results {
    match result {
        Ok(name) => println!("✓ {}: {}", input, name),
        Err(e) => println!("✗ {}: {}", input, e),
    }
}
```

### Gotchas to Avoid

```rust
// DON'T: Use raw strings after validation boundary
fn bad_function(name: &str) {  // What if validation was skipped?
    // ...
}

// DO: Use validated types
fn good_function(name: &SessionName) {  // Guaranteed valid!
    // ...
}

// DON'T: Validate in business logic
fn bad_process(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err("Invalid".into());
    }
    // ...
}

// DO: Parse once, trust everywhere
fn good_process(name: SessionName) -> Result<()> {
    // Already validated!
    // ...
}
```

---

## 2. Working with State Machines

### Pattern: State Transition Validation

**Before (Wrong):** Boolean flags and manual checks

```rust
// BAD: Boolean state with no validation
struct Workspace {
    is_active: bool,
    is_ready: bool,
    is_cleaning: bool,
    is_removed: bool,
}

impl Workspace {
    fn set_active(&mut self) {
        self.is_active = true;
        // What about other flags? Invalid states possible!
    }
}
```

**After (Correct):** Enum with transition validation

```rust
use zjj_core::domain::WorkspaceState;

// GOOD: Enum makes invalid states unrepresentable
struct Workspace {
    state: WorkspaceState,
}

impl Workspace {
    fn transition_to(&mut self, new_state: WorkspaceState) -> Result<(), String> {
        if !self.state.can_transition_to(&new_state) {
            return Err(format!(
                "Cannot transition from {:?} to {:?}",
                self.state, new_state
            ));
        }
        self.state = new_state;
        Ok(())
    }
}
```

### Example: Workspace State Transitions

```rust
use zjj_core::domain::WorkspaceState;

// Create a workspace in Creating state
let mut workspace_state = WorkspaceState::Creating;

// Valid transitions
assert!(workspace_state.can_transition_to(&WorkspaceState::Ready));
assert!(workspace_state.can_transition_to(&WorkspaceState::Removed));

// Invalid transitions
assert!(!workspace_state.can_transition_to(&WorkspaceState::Active));
assert!(!workspace_state.can_transition_to(&WorkspaceState::Cleaning));

// Perform transition
workspace_state = WorkspaceState::Ready;
assert!(workspace_state.is_ready());

// Get valid transitions from current state
let valid = workspace_state.valid_transitions();
println!("Valid transitions from Ready: {:?}", valid);
// vec![Active, Cleaning, Removed]

// Check terminal states
assert!(!workspace_state.is_terminal());
let removed = WorkspaceState::Removed;
assert!(removed.is_terminal());
```

### Example: Agent State with Bidirectional Transitions

```rust
use zjj_core::domain::AgentState;

let mut agent_state = AgentState::Idle;

// Bidirectional: Idle <-> Active
assert!(agent_state.can_transition_to(&AgentState::Active));
agent_state = AgentState::Active;

assert!(agent_state.can_transition_to(&AgentState::Idle));
agent_state = AgentState::Idle;

// Any state can go to Offline
assert!(agent_state.can_transition_to(&AgentState::Offline));

// Any state can go to Error
assert!(agent_state.can_transition_to(&AgentState::Error));

// Once Offline, can only go to Idle
agent_state = AgentState::Offline;
assert!(agent_state.can_transition_to(&AgentState::Idle));
assert!(!agent_state.can_transition_to(&AgentState::Active));
```

### Example: Claim State with Data

```rust
use zjj_core::domain::{ClaimState, AgentId};
use chrono::{Utc, Duration};

let agent = AgentId::parse("agent-1")?;
let now = Utc::now();
let expires = now + Duration::seconds(300);

// Claim an entry
let claimed = ClaimState::Claimed {
    agent: agent.clone(),
    claimed_at: now,
    expires_at: expires,
};

assert!(claimed.is_claimed());
assert_eq!(claimed.holder(), Some(&agent));

// Valid transition: Claimed -> Expired
let expired = ClaimState::Expired {
    previous_agent: agent.clone(),
    expired_at: expires,
};
assert!(claimed.can_transition_to(&expired));

// Valid transition: Expired -> Unclaimed
let unclaimed = ClaimState::Unclaimed;
assert!(expired.can_transition_to(&unclaimed));

// Check transition types
println!("Valid transitions from Claimed: {:?}", claimed.valid_transition_types());
// ["Expired", "Unclaimed"]
```

### Example: Branch State (No Self-Loops)

```rust
use zjj_core::domain::session::BranchState;

let detached = BranchState::Detached;

// Can switch to branch
let on_main = BranchState::OnBranch {
    name: "main".to_string(),
};
assert!(detached.can_transition_to(&on_main));

// Can switch back to detached
assert!(on_main.can_transition_to(&BranchState::Detached));

// Can switch between branches
let on_feature = BranchState::OnBranch {
    name: "feature".to_string(),
};
assert!(on_main.can_transition_to(&on_feature));

// But Detached -> Detached is NOT valid (no self-loops)
assert!(!detached.can_transition_to(&BranchState::Detached));
```

### Gotchas to Avoid

```rust
// DON'T: Skip transition validation
struct BadWorkspace {
    state: WorkspaceState,
}

impl BadWorkspace {
    fn set_state(&mut self, state: WorkspaceState) {
        self.state = state;  // No validation!
    }
}

// DO: Always validate transitions
struct GoodWorkspace {
    state: WorkspaceState,
}

impl GoodWorkspace {
    fn set_state(&mut self, state: WorkspaceState) -> Result<(), String> {
        if !self.state.can_transition_to(&state) {
            return Err(format!("Invalid transition"));
        }
        self.state = state;
        Ok(())
    }
}

// DON'T: Use Option for state
struct BadSession {
    branch: Option<String>,  // What does None mean?
    closed_at: Option<DateTime<Utc>>,  // Closed but when?
}

// DO: Use enums that make states explicit
struct GoodSession {
    branch: BranchState,  // Detached or OnBranch
    state: IssueState,    // Open, InProgress, or Closed { closed_at }
}
```

---

## 3. Using Value Objects

### Pattern: Immutable Collections

**Before (Wrong):** Mutable vectors

```rust
// BAD: Mutating labels directly
struct Issue {
    labels: Vec<String>,
}

impl Issue {
    fn add_label(&mut self, label: String) {
        self.labels.push(label);  // What about max count?
    }
}
```

**After (Correct):** Return new instances

```rust
use zjj_core::beads::domain::Labels;

// GOOD: Immutable with validation
struct Issue {
    labels: Labels,
}

impl Issue {
    fn add_label(&self, label: String) -> Result<Issue, DomainError> {
        let new_labels = self.labels.add(label)?;  // Validates max count
        Ok(Issue {
            labels: new_labels,
            ..self.clone()
        })
    }
}
```

### Example: Working with Labels

```rust
use zjj_core::beads::domain::Labels;

// Create labels from vector
let labels = Labels::new(vec![
    "bug".to_string(),
    "critical".to_string(),
])?;

assert_eq!(labels.len(), 2);
assert!(labels.contains("bug"));
assert!(!labels.contains("enhancement"));

// Empty labels
let empty = Labels::empty();
assert!(empty.is_empty());

// Add a label (returns new Labels instance)
let with_enhancement = labels.add("enhancement".to_string())?;
assert_eq!(with_enhancement.len(), 3);
assert!(with_enhancement.contains("enhancement"));

// Remove a label (returns new Labels instance)
let without_bug = with_enhancement.remove("bug");
assert_eq!(without_bug.len(), 2);
assert!(!without_bug.contains("bug"));

// Iterate over labels
for label in labels.iter() {
    println!("Label: {}", label);
}

// Convert to slice
let label_slice = labels.as_slice();
assert_eq!(label_slice, &["bug", "critical"]);
```

### Example: Priority Value Object

```rust
use zjj_core::coordination::domain_types::Priority;

// Create priority
let p = Priority::new(7);
assert_eq!(p.value(), 7);

// Use constants
let high = Priority::high();   // 1
let default = Priority::default();  // 5
let low = Priority::low();     // 10

// Ordering works (lower = higher priority)
assert!(high < default);
assert!(default < low);

// Use in collections
use std::collections::BinaryHeap;

let mut heap = BinaryHeap::new();
heap.push(Priority::new(5));
heap.push(Priority::new(1));
heap.push(Priority::new(10));

assert_eq!(heap.pop().map(|p| p.value()), Some(1));  // Highest priority first
```

### Example: DedupeKey

```rust
use zjj_core::coordination::domain_types::DedupeKey;

// Create from String
let key = DedupeKey::new("workspace-123".to_string())?;

// Create from &str
let key2 = DedupeKey::new_from_str("workspace-456")?;

// Access the value
assert_eq!(key.as_str(), "workspace-123");

// Convert to inner String
let inner = key.into_inner();
assert_eq!(inner, "workspace-123");

// Empty key is invalid
assert!(DedupeKey::new(String::new()).is_err());
```

### Example: IssueId, Title, Description

```rust
use zjj_core::beads::domain::{IssueId, Title, Description};

// Create IssueId
let id = IssueId::new("ISSUE-123")?;
assert_eq!(id.as_str(), "ISSUE-123");

// Create Title (trimmed)
let title = Title::new("  Fix the bug  ")?;
assert_eq!(title.as_str(), "Fix the bug");  // Trimmed!

// Title too long
assert!(Title::new("a".repeat(201)).is_err());

// Create Description (optional, can be empty)
let desc1 = Description::new("Detailed description")?;
let desc2 = Description::new("")?;  // Empty is OK
let desc3 = Description::new("   ")?;  // Whitespace is OK

// Convert to/from String
let title_string = title.clone().into_inner();
let title_again = Title::new(title_string)?;
assert_eq!(title.as_str(), title_again.as_str());
```

### Gotchas to Avoid

```rust
// DON'T: Bypass validation
let mut labels = Labels::new(vec!["bug".to_string()])?;
labels.0.push("another".to_string());  // Breaks encapsulation!
// (Note: The actual implementation makes the field private, so this won't compile)

// DO: Use provided methods
let labels = Labels::new(vec!["bug".to_string()])?;
let labels = labels.add("another".to_string())?;

// DON'T: Forget that methods return new instances
let mut labels = Labels::new(vec!["bug".to_string()])?;
labels.add("enhancement".to_string())?;  // Returns new instance, not mutates!
assert_eq!(labels.len(), 1);  // Still 1, not 2!

// DO: Use the returned value
let labels = labels.add("enhancement".to_string())?;
assert_eq!(labels.len(), 2);  // Now it's 2
```

---

## 4. Working with Aggregates

### Pattern: Consistency Boundaries

**Before (Wrong):** Separate concerns without invariants

```rust
// BAD: No consistency boundary
struct Session {
    id: String,
    name: String,
    branch: Option<String>,
    parent: Option<String>,
}

// Can create invalid sessions
let session = Session {
    id: "".to_string(),  // Invalid ID
    name: "123-bad".to_string(),  // Invalid name
    branch: None,
    parent: None,
};
```

**After (Correct):** Aggregate with enforced invariants

```rust
use zjj_core::domain::repository::Session;
use zjj_core::domain::*;
use std::path::PathBuf;

// GOOD: All fields validated, invariants enforced
let session = Session {
    id: SessionId::parse("session-123")?,
    name: SessionName::parse("my-session")?,
    branch: BranchState::OnBranch { name: "main".to_string() },
    parent: ParentState::Root,
    workspace_path: PathBuf::from("/home/user/project"),
};

// Aggregate-level behavior
assert!(session.is_active());
```

### Example: Session Aggregate

```rust
use zjj_core::domain::repository::Session;
use zjj_core::domain::*;
use std::path::PathBuf;

// Create a root session
let session = Session {
    id: SessionId::parse("session-123")?,
    name: SessionName::parse("my-session")?,
    branch: BranchState::OnBranch { name: "main".to_string() },
    parent: ParentState::Root,
    workspace_path: PathBuf::from("/home/user/project"),
};

// Check if active
if session.is_active() {
    println!("Session is active on {}", session.name);
}

// Create a child session
let parent_name = SessionName::parse("parent-session")?;
let child = Session {
    id: SessionId::parse("session-456")?,
    name: SessionName::parse("child-session")?,
    branch: BranchState::Detached,
    parent: ParentState::ChildOf { parent: parent_name },
    workspace_path: PathBuf::from("/home/user/project"),
};

assert!(child.parent.is_child());
assert_eq!(child.parent.parent_name(), Some(&parent_name));
```

### Example: Workspace Aggregate

```rust
use zjj_core::domain::repository::Workspace;
use zjj_core::domain::*;
use std::path::PathBuf;

let workspace = Workspace {
    name: WorkspaceName::parse("my-workspace")?,
    path: PathBuf::from("/tmp/my-workspace"),
    state: WorkspaceState::Ready,
};

// Workspace state transitions
assert!(workspace.state.is_ready());
assert!(!workspace.state.is_active());
assert!(!workspace.state.is_removed());

// Valid state transitions
assert!(workspace.state.can_transition_to(&WorkspaceState::Active));
assert!(workspace.state.can_transition_to(&WorkspaceState::Cleaning));
```

### Example: Queue Entry Aggregate

```rust
use zjj_core::domain::repository::QueueEntry;
use zjj_core::domain::*;
use chrono::Utc;

let entry = QueueEntry {
    id: 42,
    workspace: WorkspaceName::parse("my-workspace")?,
    bead: Some(BeadId::parse("bd-abc123")?),
    priority: 1,
    claim_state: ClaimState::Unclaimed,
    created_at: Utc::now(),
};

// Check claim state
assert!(entry.claim_state.is_unclaimed());
assert!(!entry.claim_state.is_claimed());
assert!(entry.claim_state.holder().is_none());

// Transition to claimed
let agent = AgentId::parse("agent-1")?;
let claimed = ClaimState::Claimed {
    agent: agent.clone(),
    claimed_at: Utc::now(),
    expires_at: Utc::now() + chrono::Duration::seconds(300),
};

assert!(entry.claim_state.can_transition_to(&claimed));
```

### Example: Using Repository Pattern

```rust
use zjj_core::domain::repository::{SessionRepository, RepositoryError, RepositoryResult};
use zjj_core::domain::SessionName;

// Business logic uses trait (testable!)
fn get_active_sessions(
    repo: &dyn SessionRepository,
) -> RepositoryResult<Vec<Session>> {
    let all = repo.list_all()?;
    Ok(all.into_iter()
        .filter(|s| s.is_active())
        .collect())
}

fn rename_session(
    repo: &dyn SessionRepository,
    old_name: &SessionName,
    new_name: &SessionName,
) -> RepositoryResult<()> {
    // Check if old session exists
    let mut session = repo.load_by_name(old_name)?;

    // Check if new name is already taken
    if repo.exists_by_name(new_name)? {
        return Err(RepositoryError::Conflict(format!(
            "Session '{}' already exists",
            new_name
        )));
    }

    // Update name
    session.name = new_name.clone();
    repo.save(&session)?;

    Ok(())
}

// In tests, use mock repository
struct MockSessionRepo {
    sessions: Vec<Session>,
}

impl SessionRepository for MockSessionRepo {
    fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
        self.sessions
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(id.to_string()))
    }

    fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session> {
        self.sessions
            .iter()
            .find(|s| &s.name == name)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(name.to_string()))
    }

    fn save(&self, _session: &Session) -> RepositoryResult<()> {
        Ok(())
    }

    fn delete(&self, _id: &SessionId) -> RepositoryResult<()> {
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<Session>> {
        Ok(self.sessions.clone())
    }

    fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>> {
        let mut sessions = self.sessions.clone();
        sessions.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        Ok(sessions)
    }

    fn exists(&self, _id: &SessionId) -> RepositoryResult<bool> {
        Ok(true)
    }

    fn get_current(&self) -> RepositoryResult<Option<Session>> {
        Ok(self.sessions.first().cloned())
    }

    fn set_current(&self, _id: &SessionId) -> RepositoryResult<()> {
        Ok(())
    }

    fn clear_current(&self) -> RepositoryResult<()> {
        Ok(())
    }

    fn exists_by_name(&self, _name: &SessionName) -> RepositoryResult<bool> {
        Ok(false)
    }
}
```

### Gotchas to Avoid

```rust
// DON'T: Modify aggregate state directly without validation
struct BadWorkspace {
    state: WorkspaceState,
}

impl BadWorkspace {
    fn set_state(&mut self, state: WorkspaceState) {
        self.state = state;  // No validation!
    }
}

// DO: Use methods that enforce invariants
struct GoodWorkspace {
    state: WorkspaceState,
}

impl GoodWorkspace {
    fn transition_to(&mut self, state: WorkspaceState) -> Result<(), String> {
        if !self.state.can_transition_to(&state) {
            return Err("Invalid transition".to_string());
        }
        self.state = state;
        Ok(())
    }
}

// DON'T: Expose internal mutable state
impl BadWorkspace {
    fn state_mut(&mut self) -> &mut WorkspaceState {
        &mut self.state  // Allows external mutation!
    }
}

// DO: Keep state private, provide read-only access
impl GoodWorkspace {
    fn state(&self) -> WorkspaceState {
        self.state
    }
}
```

---

## 5. Using Domain Events

### Pattern: Event Sourcing

**Before (Wrong):** No audit trail

```rust
// BAD: State changes without recording
fn close_session(repo: &dyn SessionRepository, id: &SessionId) -> Result<()> {
    let mut session = repo.load(id)?;
    // ... close session ...
    session.closed = true;
    session.closed_at = Some(Utc::now());
    repo.save(&session)?;
    // No record of what happened!
    Ok(())
}
```

**After (Correct):** Emit events for all changes

```rust
use zjj_core::domain::events::DomainEvent;
use chrono::Utc;

// GOOD: Record state changes as events
fn close_session(
    repo: &dyn SessionRepository,
    event_bus: &dyn EventBus,
    id: &SessionId,
) -> Result<()> {
    let session = repo.load(id)?;

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

### Example: Creating Domain Events

```rust
use zjj_core::domain::events::DomainEvent;
use zjj_core::domain::*;
use chrono::Utc;
use std::path::PathBuf;

// Session created event
let session_event = DomainEvent::session_created(
    "session-123".to_string(),
    SessionName::parse("my-session")?,
    Utc::now(),
);

// Workspace created event
let workspace_event = DomainEvent::workspace_created(
    WorkspaceName::parse("my-workspace")?,
    PathBuf::from("/tmp/workspace"),
    Utc::now(),
);

// Queue entry added event
let queue_event = DomainEvent::queue_entry_added(
    42,
    WorkspaceName::parse("my-workspace")?,
    1,  // priority
    Some(BeadId::parse("bd-abc123")?),
    Utc::now(),
);

// Bead closed event
let bead_event = DomainEvent::bead_closed(
    BeadId::parse("bd-abc123")?,
    Utc::now(),
    Utc::now(),
);
```

### Example: Event Serialization

```rust
use zjj_core::domain::events::{DomainEvent, serialize_event, deserialize_event};

let event = DomainEvent::session_created(
    "session-123".to_string(),
    SessionName::parse("my-session")?,
    Utc::now(),
);

// Serialize to JSON
let json = serialize_event(&event)?;
println!("{}", json);

// Deserialize from JSON
let restored = deserialize_event(&json)?;
assert_eq!(event, restored);
```

### Example: Event Metadata

```rust
use zjj_core::domain::events::{DomainEvent, StoredEvent, EventMetadata};
use chrono::Utc;

let event = DomainEvent::session_created(
    "session-123".to_string(),
    SessionName::parse("my-session")?,
    Utc::now(),
);

let metadata = EventMetadata {
    event_number: 1,
    stream_id: "session-123".to_string(),
    stream_version: 1,
    stored_at: Utc::now(),
};

let stored = StoredEvent::new(event, metadata);

assert_eq!(stored.event_number(), 1);
assert_eq!(stored.stream_id(), "session-123");
assert_eq!(stored.stream_version(), 1);
```

### Example: Processing Events

```rust
use zjj_core::domain::events::DomainEvent;

fn handle_event(event: &DomainEvent) {
    match event {
        DomainEvent::SessionCreated(e) => {
            println!("Session created: {} at {}", e.session_name, e.timestamp);
        }
        DomainEvent::SessionCompleted(e) => {
            println!("Session completed: {} at {}", e.session_name, e.timestamp);
        }
        DomainEvent::SessionFailed(e) => {
            println!("Session failed: {} - Reason: {}", e.session_name, e.reason);
        }
        DomainEvent::WorkspaceCreated(e) => {
            println!("Workspace created: {} at {}", e.workspace_name, e.path.display());
        }
        DomainEvent::QueueEntryClaimed(e) => {
            println!("Entry {} claimed by {} until {}",
                e.entry_id, e.agent, e.expires_at);
        }
        _ => {
            println!("Unhandled event type: {}", event.event_type());
        }
    }
}

// Get event type
let event = DomainEvent::session_created(
    "session-123".to_string(),
    SessionName::parse("my-session")?,
    Utc::now(),
);
assert_eq!(event.event_type(), "session_created");
```

### Example: Event Filtering

```rust
use zjj_core::domain::events::DomainEvent;

fn filter_session_events(events: &[DomainEvent]) -> Vec<&DomainEvent> {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                DomainEvent::SessionCreated(_)
                    | DomainEvent::SessionCompleted(_)
                    | DomainEvent::SessionFailed(_)
            )
        })
        .collect()
}

fn find_events_by_type(events: &[DomainEvent], event_type: &str) -> Vec<&DomainEvent> {
    events
        .iter()
        .filter(|e| e.event_type() == event_type)
        .collect()
}
```

### Gotchas to Avoid

```rust
// DON'T: Modify events after creation
// Events are immutable by design (all fields are pub but you should not mutate)

// DON'T: Create events without proper context
let bad_event = DomainEvent::session_completed(
    "".to_string(),  // Empty session ID
    SessionName::parse("my-session")?,
    Utc::now(),
);

// DO: Include all relevant context
let good_event = DomainEvent::session_completed(
    "session-123".to_string(),
    SessionName::parse("my-session")?,
    Utc::now(),
);

// DON'T: Forget to emit events
fn close_session_bad(repo: &dyn SessionRepository, id: &SessionId) -> Result<()> {
    let session = repo.load(id)?;
    // ... close logic ...
    repo.save(&session)?;
    // No event emitted!
    Ok(())
}

// DO: Always emit events for state changes
fn close_session_good(
    repo: &dyn SessionRepository,
    event_bus: &dyn EventBus,
    id: &SessionId,
) -> Result<()> {
    let session = repo.load(id)?;
    // ... close logic ...
    repo.save(&session)?;

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

## 6. Error Handling Patterns

### Pattern: Domain Errors vs Boundary Errors

**Before (Wrong):** Using String for all errors

```rust
// BAD: String errors lose context
fn parse_session_name(name: &str) -> Result<SessionName, String> {
    if name.is_empty() {
        return Err("Invalid name".to_string());  // What's invalid?
    }
    // ...
}
```

**After (Correct):** Structured errors with thiserror

```rust
use zjj_core::domain::IdentifierError;

// GOOD: Structured error with details
fn parse_session_name(name: &str) -> Result<SessionName, IdentifierError> {
    SessionName::parse(name)  // Returns IdentifierError with context
}

match parse_session_name("") {
    Err(e) => {
        if e.is_empty() {
            println!("Name cannot be empty");
        } else if e.is_too_long() {
            println!("Name is too long");
        }
    }
    Ok(name) => println!("Valid name: {}", name),
}
```

### Example: Working with IdentifierError

```rust
use zjj_core::domain::{SessionName, IdentifierError};

fn handle_session_name(input: &str) -> Result<String, IdentifierError> {
    let name = SessionName::parse(input)?;
    Ok(name.to_string())
}

match handle_session_name("") {
    Err(IdentifierError::Empty) => {
        println!("Error: Session name cannot be empty");
    }
    Err(IdentifierError::InvalidStart { .. }) => {
        println!("Error: Session name must start with a letter");
    }
    Err(IdentifierError::TooLong { max, actual }) => {
        println!("Error: Session name too long ({} chars, max {})", actual, max);
    }
    Err(e) => {
        println!("Error: {}", e);
    }
    Ok(name) => {
        println!("Valid session name: {}", name);
    }
}
```

### Example: Working with DomainError (Beads)

```rust
use zjj_core::beads::domain::{Title, DomainError};

fn create_title(input: &str) -> Result<Title, DomainError> {
    Title::new(input)
}

match create_title("") {
    Err(DomainError::EmptyTitle) => {
        println!("Title cannot be empty");
    }
    Err(DomainError::TitleTooLong { max, got }) => {
        println!("Title too long: {} chars (max {})", got, max);
    }
    Ok(title) => {
        println!("Valid title: {}", title);
    }
}
```

### Example: Working with ContractError

```rust
use zjj_core::cli_contracts::error::ContractError;

fn validate_precondition(session_exists: bool) -> Result<(), ContractError> {
    if !session_exists {
        return Err(ContractError::PreconditionFailed {
            name: "session_exists",
            description: "Session must exist before removal",
        });
    }
    Ok(())
}

// Use helper methods
let error = ContractError::invalid_input("name", "cannot be empty");
let error = ContractError::invalid_transition("completed", "active");
let error = ContractError::not_found("Session", "my-session");
```

### Example: Error Conversion

```rust
use zjj_core::domain::SessionName;
use anyhow::{Result, Context};

// Convert domain errors to anyhow errors with context
fn load_session_config(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .context(format!("Failed to read config from {}", path))?
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty config file"))?
        .parse::<SessionName>()
        .map_err(|e| anyhow::anyhow!("Invalid session name in config: {}", e))
        .map(|name| name.to_string())
}
```

### Example: Error Aggregation

```rust
use zjj_core::domain::SessionName;
use std::collections::HashMap;

fn validate_multiple_names(inputs: Vec<&str>) -> Result<Vec<SessionName>, Vec<String>> {
    let mut valid = Vec::new();
    let mut errors = Vec::new();

    for input in inputs {
        match SessionName::parse(input) {
            Ok(name) => valid.push(name),
            Err(e) => errors.push(format!("'{}': {}", input, e)),
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(valid)
    }
}

let result = validate_multiple_names(vec![
    "valid-session",
    "123-invalid",
    "",
]);

match result {
    Ok(names) => println!("Valid names: {:?}", names),
    Err(errors) => {
        println!("Validation errors:");
        for error in errors {
            println!("  - {}", error);
        }
    }
}
```

### Gotchas to Avoid

```rust
// DON'T: Use unwrap/expect
fn bad_parse(input: &str) -> SessionName {
    SessionName::parse(input).unwrap()  // Panics on invalid input!
}

// DO: Use Result and handle errors
fn good_parse(input: &str) -> Result<SessionName, IdentifierError> {
    SessionName::parse(input)
}

// DON'T: Ignore errors
fn bad_save(name: &SessionName) {
    let _ = save_to_db(name);  // Error discarded!
}

// DO: Propagate errors
fn good_save(name: &SessionName) -> Result<(), DatabaseError> {
    save_to_db(name)?
}

// DON'T: Use String for all errors
fn bad_function() -> Result<(), String> {
    Err("Something went wrong".to_string())  // No context!
}

// DO: Use structured errors
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

fn good_function() -> Result<(), MyError> {
    Err(MyError::NotFound("session-123".to_string()))
}
```

---

## Best Practices Summary

### DO:

1. **Parse at boundaries** - Validate once, trust everywhere
2. **Use validated types** - Never pass raw strings after validation
3. **Validate state transitions** - Use `can_transition_to()` before changing state
4. **Use enums for state** - Make illegal states unrepresentable
5. **Return new instances** - Value objects should be immutable
6. **Emit domain events** - Record all important state changes
7. **Use structured errors** - Leverage thiserror for domain errors
8. **Aggregate invariants** - Keep consistency boundaries in aggregates
9. **Never unwrap** - Always handle errors properly
10. **Use repository traits** - Enable testing with dependency injection

### DON'T:

1. Don't validate in business logic - Parse at boundaries only
2. Don't use Option for state - Use enums instead
3. Don't skip transition validation - Always check `can_transition_to()`
4. Don't mutate value objects - Return new instances
5. Don't forget to emit events - Record all state changes
6. Don't use String for errors - Use structured error types
7. Don't use unwrap/expect - Always handle Result types
8. Don't expose mutable state - Keep aggregate state private
9. Don't break encapsulation - Use provided methods, not direct field access
10. Don't lose error context - Convert errors with `.context()` when crossing boundaries

---

## Further Reading

- [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md) - Comprehensive reference for all domain types
- [Domain-Driven Design](https://domainlanguage.com/ddd/) by Eric Evans
- [Functional Core, Imperative Shell](https://www.destroyallsoftware.com/talks/boundaries) by Gary Bernhardt
- [Making Illegal States Unrepresentable](https://alexkuznetsov.dev/blog/making-illegal-states-unrepresentable/) by Alex Kuznetsov
