# ZJJ Glossary

A comprehensive glossary of domain, technical, and DDD terms used throughout the ZJJ codebase.

## Table of Contents

- [DDD Terms](#ddd-terms)
- [Functional Rust Terms](#functional-rust-terms)
- [ZJJ-Specific Terms](#zjj-specific-terms)
- [Architecture Terms](#architecture-terms)
- [Error Handling Terms](#error-handling-terms)

---

## DDD Terms

### Aggregate

**Definition:** A cluster of domain objects that can be treated as a single unit. The aggregate root is the only entry point for accessing the internal objects.

**Example in codebase:**
```rust
// Session is an aggregate root
use zjj_core::domain::Session;

let session = Session::new(session_name, workspace_path)?;
// Internal state accessed only through Session methods
let branch = session.branch(); // Not session.branch directly
```

**Related terms:** Aggregate Root, Consistency Boundary, Domain Model

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/aggregates/`

---

### Aggregate Root

**Definition:** The root entity of an aggregate that controls access to all other objects within the aggregate. External references can only point to the aggregate root.

**Example in codebase:**
```rust
use zjj_core::domain::{Session, Workspace, Bead, QueueEntry};

// These are aggregate roots - external code references them
let session = Session::new(name, workspace)?;
let workspace = Workspace::create(name)?;
let bead = Bead::create(title, description)?;
```

**Related terms:** Aggregate, Entity, Consistency Boundary

---

### Bounded Context

**Definition:** A distinct part of the domain logic where particular terms and rules apply consistently. The ZJJ codebase is organized into bounded contexts like `domain`, `coordination`, `beads`, and `cli_contracts`.

**Example in codebase:**
```rust
// Domain bounded context - pure business logic
use zjj_core::domain::Session;

// Coordination bounded context - distributed operations
use zjj_core::coordination::MergeQueue;

// CLI contracts bounded context - input/output validation
use zjj_core::cli_contracts::SessionContracts;
```

**Related terms:** Ubiquitous Language, Context Mapping, Domain

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/`

---

### Domain Event

**Definition:** An event that represents something that happened in the domain. Events are immutable and contain all information needed to reconstruct what happened.

**Example in codebase:**
```rust
use zjj_core::domain::events::{SessionCreatedEvent, SessionCompletedEvent};

// Event raised when a session is created
let event = SessionCreatedEvent {
    metadata: EventMetadata {
        aggregate_id: session_id.clone(),
        occurred_at: Utc::now(),
    },
    session_name: session_name.clone(),
    workspace: workspace.clone(),
};

// Event raised when a session completes
let event = SessionCompletedEvent {
    metadata: EventMetadata { ... },
    session_id,
    duration,
};
```

**Related terms:** Event Sourcing, Event Store, Aggregate

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/events.rs`

---

### Entity

**Definition:** An object defined by its identity rather than its attributes. Two entities with the same attributes are still different if their identities differ.

**Example in codebase:**
```rust
use zjj_core::domain::Session;

let session1 = Session::new("session-1", workspace)?;
let session2 = Session::new("session-2", workspace)?;

// Different identities, even if attributes are the same
assert_ne!(session1.id(), session2.id());
```

**Related terms:** Value Object, Aggregate, Identity

---

### Factory

**Definition:** A pattern for complex object creation that encapsulates the logic and validates invariants at construction time.

**Example in codebase:**
```rust
use zjj_core::domain::{Session, SessionBuilder};

// Builder pattern as factory
let session = SessionBuilder::new()
    .with_name(session_name)?
    .with_workspace(workspace_path)?
    .with_parent(parent_session)?
    .build()?;
```

**Related terms:** Builder Pattern, Constructor, Validation

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/builders.rs`

---

### Invariant

**Definition:** A rule that must always be true for a domain object or aggregate. Invariants are enforced at aggregate boundaries.

**Example in codebase:**
```rust
use zjj_core::cli_contracts::{Invariant, Contract};

// Session contracts define invariants
impl Contract<CreateSessionInput, Session> for SessionContracts {
    fn invariants(_input: &CreateSessionInput) -> Vec<Invariant> {
        vec![
            Invariant::new(
                "session_name_unique",
                "Session name must be unique",
                |s| { /* verify uniqueness */ }
            ),
        ]
    }
}
```

**Related terms:** Aggregate, Business Rule, Consistency Boundary

---

### Repository

**Definition:** A pattern for abstracting persistence logic. Repositories provide collection-like interfaces for accessing domain objects.

**Example in codebase:**
```rust
use zjj_core::domain::{SessionRepository, RepositoryResult};

#[async_trait]
pub trait SessionRepository {
    async fn get(&self, id: &SessionId) -> RepositoryResult<Option<Session>>;
    async fn save(&self, session: &Session) -> RepositoryResult<()>;
    async fn delete(&self, id: &SessionId) -> RepositoryResult<()>;
}
```

**Related terms:** Persistence Abstraction, Data Access Layer, Aggregate

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/repository.rs`

---

### Ubiquitous Language

**Definition:** A shared, rigorous language used by both developers and domain experts. The code uses the same terminology as the domain.

**Example in codebase:**
```rust
// Code uses domain terminology, not technical terms
use zjj_core::domain::{Session, Workspace, Bead, QueueEntry, Agent};

// NOT: use zjj_core::{Record, Table, Row, Field}
```

**Related terms:** Bounded Context, Domain Model, DDD

---

### Value Object

**Definition:** An object defined by its attributes rather than its identity. Two value objects with the same attributes are considered equal.

**Example in codebase:**
```rust
use zjj_core::beads::domain::{Title, Description, Priority};

// Value objects - equality by value
let title1 = Title::new("Fix bug")?;
let title2 = Title::new("Fix bug")?;

assert_eq!(title1, title2); // Same values = equal

// Priority is a value object
let p1 = Priority::P0;
let p2 = Priority::P0;
assert_eq!(p1, p2);
```

**Related terms:** Entity, Immutable, Newtype

---

## Functional Rust Terms

### Newtype Pattern

**Definition:** A Rust idiom where a single-field tuple struct wraps a primitive type to provide type safety and domain semantics.

**Example in codebase:**
```rust
// Instead of passing raw strings
use zjj_core::domain::{SessionName, AgentId, BeadId};

// Parse at boundary - validates once
let name = SessionName::parse("my-session")?;
let agent = AgentId::parse("agent-123")?;

// Use throughout domain - no further validation needed
create_session(&name, &agent)?;
```

**Related terms:** Semantic Types, Type Safety, Parse-Once Pattern

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`

---

### Zero Unwrap

**Definition:** A strict code quality rule forbidding the use of `unwrap()`, `expect()`, and similar panic-prone methods. All errors must be handled explicitly using `Result<T, E>`.

**Example in codebase:**
```rust
// BANNED - will panic on None/Err
// let name = some_option.unwrap();

// REQUIRED - proper error handling
use std::ops::ControlFlow;

match some_option {
    Some(name) => process(name),
    None => return Err(SessionError::NotFound),
}

// Or use combinators
some_option
    .ok_or_else(|| SessionError::NotFound)?
    .pipe(|name| process(name))?;
```

**Related terms:** Railway-Oriented Programming, Error Handling, Result Type

**See also:** File headers contain `#![deny(clippy::unwrap_used)]`

---

### Railway-Oriented Programming

**Definition:** A functional programming pattern where operations are chained like railway tracks. Success continues down the "track" (via `?` operator), while errors switch to a separate "error track".

**Example in codebase:**
```rust
use anyhow::Result;

fn process_workflow() -> Result<Output> {
    // Each operation returns Result
    // Success continues, error short-circuits
    let name = parse_session_name(input)?;
    let workspace = find_workspace(&name)?;
    let session = create_session(&name, &workspace)?;
    let result = save_session(&session)?;

    Ok(result)
}
```

**Related terms:** Result Type, Error Propagation, Combinators

---

### Pure Function

**Definition:** A function that always produces the same output for the same input and has no side effects (no I/O, no mutation of external state).

**Example in codebase:**
```rust
// Pure function - deterministic, no side effects
use zjj_core::domain::queue;

fn calculate_queue_position(priority: Priority, submitted_at: DateTime<Utc>) -> usize {
    // Same inputs always produce same output
    // No I/O, no external state mutation
    priority_score(priority) + time_score(submitted_at)
}

// Impure function (shell layer) - has side effects
async fn save_to_database(queue: &QueueEntry) -> Result<()> {
    db.execute(query).await?; // Side effect: I/O
    Ok(())
}
```

**Related terms:** Functional Core, Imperative Shell, Deterministic

---

### Immutable by Default

**Definition:** A design principle where data structures are not modified after creation. Instead of mutating existing values, new values are created with the desired changes.

**Example in codebase:**
```rust
use rpds::Vector; // Persistent data structure

// Create new queue, don't mutate old one
let new_queue = queue.push_back(entry);

// NOT: queue.push_back(entry) // Mutates

// State transitions return new state
let new_state = current_state.transition_to(State::Active)?;

// NOT: current_state.state = State::Active // Mutates
```

**Related terms:** Persistent Data Structures, Pure Functions, Value Objects

---

### Parse-Once Pattern

**Definition:** Validate and parse input at system boundaries (CLI, API), then use validated types throughout the core. Validation happens exactly once.

**Example in codebase:**
```rust
// Shell layer - parse once
use zjj_core::domain::SessionName;

fn cli_handler(raw_name: String) -> Result<()> {
    // Parse and validate at boundary
    let name = SessionName::parse(raw_name)?;

    // Core receives already-validated type
    core::create_session(&name)?;
    Ok(())
}

// Core layer - no validation needed
fn create_session(name: &SessionName) -> Result<Session> {
    // Name is already validated
    Ok(Session::new(name.clone())?)
}
```

**Related terms:** Semantic Types, Newtype Pattern, Boundary Validation

---

### Semantic Newtype

**Definition:** A newtype wrapper that carries domain meaning beyond just the wrapped value. The type name conveys intent and documents the code.

**Example in codebase:**
```rust
// Semantic - clear intent
use zjj_core::domain::{SessionName, WorkspaceName};

fn create_session(name: &SessionName, workspace: &WorkspaceName) { ... }

// NOT semantic - unclear what strings represent
fn create_session(name: &str, workspace: &str) { ... }
```

**Related terms:** Newtype Pattern, Type Safety, Self-Documenting Code

---

### Combinators

**Definition:** Functions that combine other functions to build complex behavior. In Rust, methods like `map`, `and_then`, `filter`, and `fold` are combinators.

**Example in codebase:**
```rust
use itertools::Itertools;

// Combinator pipeline - no loops, no mutation
fn process_queue(entries: Vec<QueueEntry>) -> Vec<QueueEntry> {
    entries
        .into_iter()
        .filter(|e| e.is_processable())        // Filter
        .map(|e| e.priority_sort_key())        // Transform
        .sorted()                               // Sort
        .map(|key| find_entry(key))            // Transform
        .collect()                              // Collect
}
```

**Related terms:** Iterator Pipelines, Functional Programming, Higher-Order Functions

**See also:** `itertools` crate usage throughout codebase

---

### Phantom Types

**Definition:** A technique where type parameters are used solely for compile-time type checking without any runtime value. Used to enforce invariants at compile time.

**Example in codebase:**
```rust
// State machines can use phantom types
struct Session<State> {
    _phantom: PhantomData<State>,
    name: SessionName,
}

struct Creating;
struct Active;
struct Completed;

impl Session<Creating> {
    fn start(self) -> Session<Active> {
        // State transition enforced by types
        Session { _phantom: PhantomData, name: self.name }
    }
}
```

**Related terms:** Type Safety, Type-State Pattern, Compile-Time Guarantees

---

## ZJJ-Specific Terms

### Session

**Definition:** A parallel workspace session managed by ZJJ. Sessions track work in specific branches, can be nested (stacked), and represent development contexts.

**Example in codebase:**
```rust
use zjj_core::domain::Session;
use zjj_core::domain::session::BranchState;

let session = Session::new(session_name, workspace_path)?;

match session.branch() {
    BranchState::Detached => println!("Not on any branch"),
    BranchState::OnBranch { name } => println!("On branch: {name}"),
}
```

**Related terms:** Workspace, Stack, Parent Session, Branch State

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/aggregates/session.rs`

---

### Workspace

**Definition:** A JJ (Jujutsu) repository root directory where development work happens. Workspaces can be nested to form stacks.

**Example in codebase:**
```rust
use zjj_core::domain::{Workspace, WorkspaceName, WorkspaceState};

let workspace = Workspace::create(WorkspaceName::parse("my-workspace")?)?;

match workspace.state() {
    WorkspaceState::Ready => println!("Ready for use"),
    WorkspaceState::Active => println!("Currently active"),
    WorkspaceState::Removing => println!("Being removed"),
}
```

**Related terms:** Session, Stack, Repository Root

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/aggregates/workspace.rs`

---

### Bead

**Definition:** An issue or task in the beads issue tracker system. Beads have IDs prefixed with `bd-` and track state, priority, and relationships.

**Example in codebase:**
```rust
use zjj_core::beads::{Issue, IssueState, Priority};
use zjj_core::domain::BeadId;

let bead = Issue::builder()
    .id(BeadId::parse("bd-123")?)
    .title(Title::new("Fix authentication bug")?)
    .state(IssueState::Open)
    .priority(Priority::P1)
    .build()?;
```

**Related terms:** Issue, Task, BeadId, IssueState

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/beads/`

---

### Queue (Merge Queue)

**Definition:** A distributed work queue for managing workspace processing. Entries have priorities, can be claimed by agents, and are processed in order.

**Example in codebase:**
```rust
use zjj_core::coordination::{MergeQueue, QueueEntry};
use zjj_core::domain::{WorkspaceName, Priority};

// Submit work to queue
let request = QueueSubmissionRequest {
    workspace: WorkspaceName::parse("my-workspace")?,
    bead_id: Some(bead_id),
    priority: Priority::new(1)?, // Lower = higher priority
};

let response = submit_to_queue(&repo, &request)?;
```

**Related terms:** Queue Entry, Claim, Priority, Deduplication, Train

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs`

---

### Queue Entry

**Definition:** An item in the merge queue representing a workspace waiting to be processed. Entries have claim states and priorities.

**Example in codebase:**
```rust
use zjj_core::coordination::QueueEntry;
use zjj_core::domain::queue::ClaimState;

match entry.claim_state() {
    ClaimState::Unclaimed => println!("Available for processing"),
    ClaimState::Claimed { agent, expires_at } => {
        println!("Claimed by {} until {}", agent, expires_at);
    }
    ClaimState::Expired { previous_agent } => {
        println!("Claim expired for {}", previous_agent);
    }
}
```

**Related terms:** Queue, Claim State, Priority, BeadId

---

### Claim State

**Definition:** Represents whether a queue entry is available, currently claimed by an agent, or has an expired claim. Makes invalid states unrepresentable.

**Example in codebase:**
```rust
use zjj_core::domain::queue::ClaimState;
use chrono::{DateTime, Utc};

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

**Related terms:** Queue Entry, AgentId, Expiration, Make Illegal States Unrepresentable

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/queue.rs`

---

### Train (Merge Train)

**Definition:** An ordered sequence of queue entries processed together. Entries are sorted by priority and processed through quality gates.

**Example in codebase:**
```rust
use zjj_core::coordination::{TrainProcessor, TrainConfig};

let processor = TrainProcessor::new(TrainConfig::default());

let result = processor.process_train(
    &repo,
    &entries,
    &agent_id,
)?;

match result {
    TrainResult::Success => println!("All entries processed"),
    TrainResult::Partial { failed, succeeded } => {
        println!("{} failed, {} succeeded", failed, succeeded);
    }
    TrainResult::Failure(err) => println!("Train failed: {}", err),
}
```

**Related terms:** Queue, Quality Gate, Priority, Processing Step

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/train.rs`

---

### Stack

**Definition:** A nested hierarchy of workspaces where child workspaces are stacked on top of parent workspaces. Stacks have roots and calculated depths.

**Example in codebase:**
```rust
use zjj_core::coordination::{find_stack_root, calculate_stack_depth};

let root = find_stack_root(&repo, &workspace)?;
let depth = calculate_stack_depth(&repo, &workspace)?;

println!("Stack root: {:?}, depth: {}", root, depth);
```

**Related terms:** Workspace, Session, Nested Workspace, Stack Root

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/stack_depth.rs`

---

### Agent

**Definition:** An autonomous worker (Claude, Cursor, Aider, Copilot) that can claim queue entries and perform work. Agents register, send heartbeats, and process entries.

**Example in codebase:**
```rust
use zjj_core::domain::{AgentId, AgentState};

// Agent registers itself
let agent_id = AgentId::parse("claude-opus-4")?;
repository.register_agent(&agent_id, AgentState::Active).await?;

// Agent claims work
let entry = queue.claim_next_entry(&agent_id, timeout_seconds)?;
```

**Related terms:** AgentId, AgentState, Claim, Heartbeat, Queue

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/agent.rs`

---

### KIRK Contract

**Definition:** Design-by-contract pattern for CLI objects. KIRK stands for Known preconditions, Invariants, Return guarantees, and Known postconditions.

**Example in codebase:**
```rust
use zjj_core::cli_contracts::{Contract, SessionContracts};

// Contract defines the rules for session operations
impl Contract<CreateSessionInput, Session> for SessionContracts {
    fn preconditions(input: &CreateSessionInput) -> Result<(), ContractError> {
        // Verify name is valid, workspace exists, etc.
        Ok(())
    }

    fn invariants(input: &CreateSessionInput) -> Vec<Invariant> {
        // Properties that must always be true
        vec![Invariant::documented("name_unique", "Session names must be unique")]
    }

    fn postconditions(input: &CreateSessionInput, result: &Session) -> Result<(), ContractError> {
        // Verify session was created correctly
        Ok(())
    }
}
```

**Related terms:** Design by Contract, Preconditions, Postconditions, Invariants, CLI Object

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/cli_contracts/mod.rs`

---

### JSONL Output

**Definition:** JSON Lines format where each line is a complete, valid JSON object. Used for AI-first control plane design.

**Example in codebase:**
```rust
use zjj_core::output::{emit_stdout, OutputLine, Session};

let session = Session { ... };
let output = OutputLine::Session(session);
emit_stdout(output)?;

// Emits: {"type":"session","session_id":"...","name":"...","state":"active"}
```

**Related terms:** OutputLine, AI-First CLI, Machine-Readable Output

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/output/mod.rs`

---

### Deduplication Key

**Definition:** A computed key that identifies duplicate queue entries for the same workspace, preventing redundant work.

**Example in codebase:**
```rust
use zjj_core::coordination::{compute_dedupe_key, extract_workspace_identity};

let identity = extract_workspace_identity(&workspace)?;
let dedupe_key = compute_dedupe_key(&identity)?;

// Same workspace = same dedupe key
// Prevents duplicate queue entries
```

**Related terms:** Queue, Queue Entry, Workspace Identity, Submission

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue_submission.rs`

---

### Branch State

**Definition:** Represents whether a session is on a git branch or in detached HEAD state. Makes the Option<String> pattern explicit.

**Example in codebase:**
```rust
use zjj_core::domain::session::BranchState;

pub enum BranchState {
    Detached,
    OnBranch { name: String },
}

// Usage - match handles all cases explicitly
match session.branch() {
    BranchState::Detached => println!("Detached HEAD"),
    BranchState::OnBranch { name } => println!("On branch: {}", name),
}
```

**Related terms:** Session, Parent State, Make Illegal States Unrepresentable

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/session.rs`

---

### Parent State

**Definition:** Represents whether a session is a root (no parent) or a child of another session. Replaces Option<String> for parent_session.

**Example in codebase:**
```rust
use zjj_core::domain::session::ParentState;

pub enum ParentState {
    Root,
    ChildOf { parent: SessionName },
}

// Usage - explicit about hierarchy
match session.parent() {
    ParentState::Root => println!("Root session"),
    ParentState::ChildOf { parent } => println!("Child of: {}", parent),
}
```

**Related terms:** Session, Branch State, Stack, Hierarchy

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/session.rs`

---

## Architecture Terms

### Functional Core, Imperative Shell

**Definition:** An architectural pattern separating pure business logic (core) from I/O and side effects (shell). The core is pure, testable, and deterministic; the shell handles async, I/O, and external APIs.

**Example in codebase:**
```rust
// Core - pure function, no I/O
mod core {
    use super::domain::*;

    pub fn calculate_next_entry(entries: &[QueueEntry]) -> Option<QueueEntry> {
        entries
            .iter()
            .filter(|e| e.is_claimable())
            .min_by_key(|e| e.priority())
            .cloned()
    }
}

// Shell - async, I/O, delegates to core
mod shell {
    use super::core::*;

    pub async fn claim_next_entry(repo: &QueueRepo) -> Result<Option<QueueEntry>> {
        let entries = repo.load_all().await?;  // I/O
        Ok(calculate_next_entry(&entries))     // Pure core logic
    }
}
```

**Related terms:** Pure Function, Side Effect, Business Logic, Infrastructure

**See also:** Project architecture documentation

---

### Shell Layer

**Definition:** The outer layer of the application that handles I/O, async operations, external APIs, and delegates to the functional core for business logic. Uses `anyhow` for error handling.

**Example in codebase:**
```rust
use anyhow::{Result, Context};

// Shell layer - async, I/O, anyhow errors
async fn load_session(path: &Path) -> Result<Session> {
    let data = tokio::fs::read(path)
        .await
        .context("Failed to read session file")?;

    let session: Session = serde_json::from_slice(&data)
        .context("Failed to parse session")?;

    Ok(session)
}
```

**Related terms:** Functional Core, Imperative Shell, Boundary, anyhow

---

### Core Layer

**Definition:** The inner layer containing pure business logic. No I/O, no async, deterministic. Uses `thiserror` for domain errors.

**Example in codebase:**
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(SessionName),
}

// Core - pure, sync, deterministic
pub fn find_session(sessions: &[Session], name: &SessionName) -> Result<&Session, SessionError> {
    sessions
        .iter()
        .find(|s| s.name() == name)
        .ok_or_else(|| SessionError::NotFound(name.clone()))
}
```

**Related terms:** Functional Core, Domain Logic, thiserror, Pure Function

---

### Boundary

**Definition:** The interface between layers (e.g., shell and core) or between bounded contexts. Input is parsed and validated at boundaries, then passed as trusted types to inner layers.

**Example in codebase:**
```rust
// Boundary - parse once, validate once
use zjj_core::domain::SessionName;

async fn http_handler(raw_name: String) -> Result<()> {
    // Parse at boundary
    let name = SessionName::parse(raw_name)
        .map_err(|e| anyhow::anyhow!("Invalid name: {}", e))?;

    // Core receives validated type
    core::create_session(&name)?;
    Ok(())
}
```

**Related terms:** Parse-Once Pattern, Shell Layer, Core Layer, Input Validation

---

### Quality Gate

**Definition:** A validation step in the merge train pipeline that must pass before processing continues. Failed quality gates stop the train.

**Example in codebase:**
```rust
use zjj_core::coordination::{QualityGate, TrainStep};

pub enum QualityGate {
    PreMerge,
    PostMerge,
    PrePush,
}

let step = TrainStep {
    kind: TrainStepKind::QualityGate(QualityGate::PreMerge),
    status: TrainStepStatus::Passed,
    // ...
};
```

**Related terms:** Train, Merge Pipeline, Validation, Train Step

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/train.rs`

---

### Persistence Abstraction

**Definition:** Hiding implementation details of data storage behind traits, allowing the core logic to be independent of storage technology.

**Example in codebase:**
```rust
use zjj_core::domain::SessionRepository;

// Core depends on trait, not concrete implementation
pub struct SessionService<R: SessionRepository> {
    repo: R,
}

impl<R: SessionRepository> SessionService<R> {
    async fn get_session(&self, id: &SessionId) -> Result<Session> {
        // Doesn't care if R is SQLite, PostgreSQL, in-memory, etc.
        self.repo.get(id).await?
            .ok_or_else(|| SessionError::NotFound(id.clone()))
    }
}
```

**Related terms:** Repository Pattern, Dependency Injection, Trait Object

---

### Conflict Resolution

**Definition:** The process of detecting and resolving merge conflicts during workspace processing. Resolutions are stored for reuse.

**Example in codebase:**
```rust
use zjj_core::coordination::conflict_resolutions_entities::ConflictResolution;

let resolution = ConflictResolution {
    id: resolution_id,
    decider: agent_id.clone(),
    resolved_at: Utc::now(),
    conflict_type: ConflictType::Content,
    resolution_strategy: ResolutionStrategy::AcceptOurs,
    // ...
};

insert_conflict_resolution(&repo, &resolution).await?;
```

**Related terms:** Merge, Train, Workspace, Conflict Type, Resolution Strategy

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/conflict_resolutions.rs`

---

### Lock Manager

**Definition:** Distributed locking mechanism for mutual exclusion in critical sections. Locks have owners, timeouts, and automatic expiration.

**Example in codebase:**
```rust
use zjj_core::coordination::{LockManager, LockInfo};

let manager = LockManager::new(repo.clone());

// Acquire lock
let response = manager.acquire("session-123", &agent_id, 60).await?;

match response {
    LockResponse::Acquired => println!("Lock acquired"),
    LockResponse::AlreadyLocked(LockInfo { owner, expires_at }) => {
        println!("Locked by {} until {}", owner, expires_at);
    }
}
```

**Related terms:** Distributed Lock, Mutual Exclusion, Expiration, Critical Section

**See also:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/locks.rs`

---

## Error Handling Terms

### Domain Error

**Definition:** An error that represents a valid business scenario (not exceptional). Domain errors use `thiserror` and are part of the domain model.

**Example in codebase:**
```rust
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("session '{0}' not found")]
    NotFound(SessionName),

    #[error("session '{0}' already exists")]
    AlreadyExists(SessionName),

    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: SessionState, to: SessionState },
}
```

**Related terms:** thiserror, Expected Error, Business Rule, Aggregate

---

### Boundary Error

**Definition:** An error that occurs at system boundaries (I/O, parsing, external APIs). Boundary errors use `anyhow` and include context.

**Example in codebase:**
```rust
use anyhow::{Result, Context};

async fn load_config(path: &Path) -> Result<Config> {
    let data = tokio::fs::read(path)
        .await
        .context(format!("Failed to read config from {}", path.display()))?;

    let config: Config = serde_json::from_slice(&data)
        .context("Failed to parse config JSON")?;

    Ok(config)
}
```

**Related terms:** anyhow, Context, Shell Layer, I/O Error

---

### Result Type

**Definition:** Rust's built-in type for fallible operations. `Result<T, E>` represents either success (`Ok(T)`) or failure (`Err(E)`).

**Example in codebase:**
```rust
use anyhow::Result;

fn create_session(name: &str) -> Result<Session> {
    if name.is_empty() {
        // Failure case
        return Err(anyhow::anyhow!("Session name cannot be empty"));
    }

    // Success case
    Ok(Session::new(name)?)
}
```

**Related terms:** Railway-Oriented Programming, Error Propagation, ?

---

### Error Propagation

**Definition:** The mechanism of passing errors up the call stack using the `?` operator, which automatically returns early on `Err`.

**Example in codebase:**
```rust
use anyhow::Result;

async fn process_workflow() -> Result<Output> {
    // Each ? propagates errors upward
    let name = parse_name(input)?;
    let workspace = find_workspace(&name)?;
    let session = create_session(&name, &workspace)?;
    let result = save_session(&session)?;

    Ok(result)
}
// Any error in the chain short-circuits and returns early
```

**Related terms:** Railway-Oriented Programming, ? Operator, Result Type

---

### Error Context

**Definition:** Additional information added to errors to explain where and why they occurred. Provided by `.context()` in `anyhow`.

**Example in codebase:**
```rust
use anyhow::{Context, Result};

async fn load_user(id: u32) -> Result<User> {
    repo.find_by_id(id)
        .await
        .context(format!("Failed to load user with id {}", id))?
        .context("User not found or database error")
}
```

**Related terms:** anyhow, Boundary Error, Error Chain

---

### Invariant Violation

**Definition:** An error indicating that a business rule (invariant) was broken. Used in contract checking.

**Example in codebase:**
```rust
use zjj_core::cli_contracts::{ContractError, Invariant};

if !verify_invariant(result) {
    return Err(ContractError::InvariantViolation {
        name: "session_name_unique",
        description: "Session name must be unique",
    });
}
```

**Related terms:** Invariant, Contract, Business Rule, KIRK

---

### Precondition Failed

**Definition:** An error indicating that required preconditions for an operation were not met. Used in contract checking.

**Example in codebase:**
```rust
use zjj_core::cli_contracts::{ContractError, require_precondition, Precondition};

let precondition = Precondition::new(
    "workspace_exists",
    "Workspace must exist before creating session"
);

require_precondition(
    workspace_exists,
    &precondition
)?;
```

**Related terms:** Precondition, Contract, KIRK, Validation

---

### Postcondition Failed

**Definition:** An error indicating that expected postconditions after an operation were not satisfied. Used in contract checking.

**Example in codebase:**
```rust
use zjj_core::cli_contracts::{ContractError, require_postcondition, Postcondition};

let postcondition = Postcondition::new(
    "session_created",
    "Session must exist in storage after creation"
);

require_postcondition(
    session_was_persisted,
    &postcondition
)?;
```

**Related terms:** Postcondition, Contract, KIRK, Verification

---

## Cross-Reference by Category

### Domain Modeling
- Aggregate, Aggregate Root
- Entity, Value Object
- Domain Event
- Factory, Repository
- Bounded Context, Ubiquitous Language
- Invariant

### Type Safety
- Newtype Pattern, Semantic Newtype
- Parse-Once Pattern
- Phantom Types
- Make Illegal States Unrepresentable

### Functional Programming
- Pure Function
- Railway-Oriented Programming
- Immutable by Default
- Combinators
- Zero Unwrap

### ZJJ Domain
- Session, Workspace
- Bead, Issue
- Queue, Queue Entry, Claim State
- Train, Quality Gate
- Stack, Agent
- Branch State, Parent State

### Architecture
- Functional Core, Imperative Shell
- Shell Layer, Core Layer, Boundary
- Persistence Abstraction
- Conflict Resolution, Lock Manager

### Error Handling
- Domain Error, Boundary Error
- Result Type, Error Propagation
- Error Context
- Invariant Violation
- Precondition Failed, Postcondition Failed

### CLI/Contracts
- KIRK Contract
- JSONL Output
- Deduplication Key

---

## File References

### Domain Layer
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs` - Domain module overview
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs` - Semantic newtypes
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/aggregates/` - Aggregate roots
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/events.rs` - Domain events
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/repository.rs` - Repository traits

### Coordination Layer
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/mod.rs` - Coordination overview
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs` - Merge queue
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/train.rs` - Train processing

### CLI Contracts
- `/home/lewis/src/zjj/crates/zjj-core/src/cli_contracts/mod.rs` - KIRK contracts
- `/home/lewis/src/zjj/crates/zjj-core/src/cli_contracts/domain_types.rs` - CLI domain types

### Output
- `/home/lewis/src/zjj/crates/zjj-core/src/output/mod.rs` - Output types

### Beads
- `/home/lewis/src/zjj/crates/zjj-core/src/beads/domain.rs` - Beads domain types
- `/home/lewis/src/zjj/crates/zjj-core/src/beads/issue.rs` - Issue aggregate

---

## External References

### DDD Books
- *Domain-Driven Design* by Eric Evans
- *Domain Modeling Made Functional* by Scott Wlaschin

### Rust Patterns
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Error Handling Best Practices](https://doc.rust-lang.org/book/ch09-00-error-handling.html)

### Functional Programming
- [Railway-Oriented Programming](https://fsharpforfunandprofit.com/posts/recipe-part2/)
- [Property-Based Testing](https://hypothesis.works/)

---

## Contribute

To add new terms to this glossary:

1. Add the term in the appropriate section
2. Include:
   - Clear definition
   - Example usage from codebase
   - Related terms
   - File references
3. Update cross-references
4. Maintain alphabetical order within sections

---

*Generated for the ZJJ project - AI-first CLI for distributed development*
