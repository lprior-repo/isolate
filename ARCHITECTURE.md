# ZJJ Architecture Overview

## Table of Contents

1. [System Overview](#system-overview)
2. [Design Philosophy](#design-philosophy)
3. [Layer Architecture](#layer-architecture)
4. [Module Structure](#module-structure)
5. [Data Flow](#data-flow)
6. [Key Design Decisions](#key-design-decisions)
7. [Dependencies](#dependencies)
8. [Extension Points](#extension-points)
9. [Architecture Diagrams](#architecture-diagrams)

---

## System Overview

ZJJ is a **parallel workspace isolation and queue coordination system** built on top of JJ (Jujutsu). It enables multiple humans or AI agents to work on the same repository concurrently without conflicts.

### Core Purpose

- **Isolation**: Each workstream gets its own JJ workspace
- **Coordination**: Queue system manages concurrent access and merge ordering
- **Recovery**: Database-backed state with corruption recovery

### Binary Structure

```
zjj/
├── crates/zjj/          # CLI binary (shell layer)
└── crates/zjj-core/     # Core library (functional core)
```

---

## Design Philosophy

ZJJ follows two fundamental design principles:

### 1. Functional Rust

**Zero Unwrap Law** (compiler-enforced):
- No `unwrap()`, `expect()`, `panic()`, `todo!()`, `unimplemented!()`
- All fallible operations return `Result<T, E>`
- Railway-oriented error propagation with `?` operator

**Immutability by Default**:
- Prefer `let` over `let mut`
- Use persistent data structures (`rpds`)
- Iterator pipelines over loops (`itertools`)

**Pure Functions**:
- Core logic: deterministic, no I/O, no global state
- Shell layer: handles I/O, async, external APIs

### 2. Domain-Driven Design (DDD)

**Bounded Contexts**:
- Each module is a clear boundary
- Explicit interfaces between contexts
- Ubiquitous language in code

**Aggregates**:
- Cluster entities and value objects
- Enforce invariants at aggregate root
- `Session`, `Bead`, `QueueEntry`, `Workspace`

**Value Objects**:
- Immutable types for domain concepts
- Equality by value, not identity
- Semantic newtypes with validation

**Repository Pattern**:
- Abstract persistence behind traits
- Domain doesn't know storage details
- Database operations isolated in shell layer

---

## Layer Architecture

ZJJ follows the **Functional Core, Imperative Shell** pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                      SHELL LAYER (zjj)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   CLI Parser │  │  I/O Handlers│  │   Database   │      │
│  │   (clap)     │  │  (async/tokio)│  │  (sqlx)      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │              │
│         └──────────────────┼──────────────────┘              │
│                            │                                 │
└────────────────────────────┼─────────────────────────────────┘
                             │
                        ┌────▼─────┐
                        │  KIRK    │  ← Design-by-Contract
                        │Contracts │     (preconditions,
                        └────┬─────┘      postconditions,
                             │           invariants)
┌────────────────────────────┼─────────────────────────────────┐
│                            │                                 │
│         ┌──────────────────▼──────────────────┐              │
│         │         CORE LAYER (zjj-core)       │              │
│         │  ┌────────────────────────────┐     │              │
│         │  │  Domain Primitives (DDD)   │     │              │
│         │  │  - Semantic Newtypes       │     │              │
│         │  │  - Aggregates              │     │              │
│         │  │  - Value Objects           │     │              │
│         │  └────────────────────────────┘     │              │
│         │  ┌────────────────────────────┐     │              │
│         │  │  Business Logic (Pure)     │     │              │
│         │  │  - State Transitions       │     │              │
│         │  │  - Validation              │     │              │
│         │  │  - Coordination            │     │              │
│         │  └────────────────────────────┘     │              │
│         │  ┌────────────────────────────┐     │              │
│         │  │  Output Types (JSONL)      │     │              │
│         │  │  - AI-First Control Plane  │     │              │
│         │  └────────────────────────────┘     │              │
│         └─────────────────────────────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

#### Shell Layer (`crates/zjj/`)

**Responsibilities**:
- Parse CLI arguments with `clap`
- Handle all I/O operations
- Manage async runtime with `tokio`
- Execute external commands (JJ)
- Write to database with `sqlx`
- Emit JSONL output

**Key Modules**:
- `src/cli/` - CLI parser and handlers
- `src/commands/` - Command implementations
- `src/db.rs` - Database operations
- `src/session.rs` - Session state management

**Error Handling**:
- Uses `anyhow::Result<T>` for boundary errors
- Context with `.context()` method
- Converts domain errors to human-readable messages

#### Core Layer (`crates/zjj-core/`)

**Responsibilities**:
- Pure business logic (no I/O)
- Domain primitives and types
- State transition logic
- Validation and invariants
- Coordination algorithms

**Key Modules**:
- `src/domain/` - DDD aggregates and value objects
- `src/coordination/` - Queue and train processing
- `src/beads/` - Issue tracking domain
- `src/output/` - JSONL output types
- `src/cli_contracts/` - KIRK design-by-contract

**Error Handling**:
- Uses `thiserror` for domain errors
- `Result<T, DomainError>` throughout
- No panics, explicit error cases

---

## Module Structure

### Crates

```
zjj/
├── Cargo.toml                    # Workspace configuration
├── ARCHITECTURE.md               # This document
├── AGENTS.md                     # Agent development guidelines
└── crates/
    ├── zjj/                      # CLI binary
    │   ├── src/
    │   │   ├── main.rs           # Entry point
    │   │   ├── cli/              # CLI parsing and handlers
    │   │   ├── commands/         # Command implementations
    │   │   ├── db.rs             # Database wrapper
    │   │   └── session.rs        # Session management
    │   └── Cargo.toml
    │
    └── zjj-core/                 # Core library
        ├── src/
        │   ├── domain/           # DDD primitives
        │   ├── coordination/     # Queue/train logic
        │   ├── beads/            # Issue tracking
        │   ├── output/           # JSONL output types
        │   ├── cli_contracts/    # KIRK contracts
        │   ├── jj/               # JJ integration
        │   └── lib.rs            # Public API
        └── Cargo.toml
```

### Core Modules

#### `domain/` - Domain Primitives

**Purpose**: Semantic types that make illegal states unrepresentable

**Key Types**:
- **Identifiers**: `SessionName`, `AgentId`, `WorkspaceName`, `TaskId`, `BeadId`
- **Aggregates**: `Session`, `Bead`, `QueueEntry`, `Workspace`
- **State Enums**: `BranchState`, `ParentState`, `ClaimState`, `WorkspaceState`
- **Events**: `SessionCreated`, `SessionCompleted`, `SessionFailed`

**Design Pattern**: Parse-once validation at boundaries
```rust
// Shell layer - parse once
let name = SessionName::parse(raw_name)?;

// Core layer - accept only validated type
fn create_session(name: &SessionName) -> Result<Session, SessionError>
```

#### `coordination/` - Queue and Train Processing

**Purpose**: Multi-agent coordination and merge ordering

**Key Components**:
- **Pure Queue**: Immutable queue operations (add, remove, claim)
- **Merge Queue**: SQLite-backed queue with state machine
- **Train Processing**: Ordered merge execution with quality gates
- **Lock Manager**: File-based locks for exclusion
- **Conflict Resolution**: Track and resolve merge conflicts

**State Machine**:
```
Pending → Claimed → Processing → {Done | Failed | Retry}
              ↓
           Expired
```

#### `beads/` - Issue Tracking Domain

**Purpose**: Track beads/issues and tie them to workspaces

**Key Types**:
- `Issue` - Aggregate root with state transitions
- `IssueState` - Open, InProgress, Blocked, Closed
- `IssueId`, `Title`, `Description` - Semantic newtypes

**Invariants**:
- Closed issues must have `closed_at` timestamp
- State transitions are validated
- Dependency graph tracking

#### `output/` - JSONL Output Types

**Purpose**: AI-first control plane with streaming JSONL

**Key Types**:
- `OutputLine` - Self-describing JSON objects
- `Session`, `QueueEntry`, `Stack`, `Train` - Domain output types
- `Action`, `Plan`, `Result` - Operation types
- `Error`, `Warning`, `Recovery` - Diagnostic types

**Design Principles**:
- Every line is a complete JSON object
- Type field indicates structure
- No human-readable formatting
- Streaming-friendly (emit one line at a time)

#### `cli_contracts/` - KIRK Design-by-Contract

**Purpose**: Runtime contract verification for all CLI objects

**KIRK Pattern**:
- **K**nown preconditions: What must be true before
- **I**nvariants: What must always remain true
- **R**eturn guarantees: What the operation guarantees
- **K**nown postconditions: What will be true after

**Contracts for 8 CLI Objects**:
- `TaskContracts` - Beads task management
- `SessionContracts` - Parallel workspace sessions
- `QueueContracts` - Merge train operations
- `StackContracts` - Session stacking
- `AgentContracts` - Agent coordination
- `StatusContracts` - Status reporting
- `ConfigContracts` - Configuration management
- `DoctorContracts` - System diagnostics

---

## Data Flow

### Command Execution Flow

```
User Input
    │
    ▼
┌──────────────┐
│ CLI Parser   │ (clap)
└──────┬───────┘
       │
       ▼
┌──────────────┐      Parse-once validation
│ Shell Layer  │ ─────────────────────────┐
│ (zjj)        │                          │
└──────┬───────┘                          │
       │                                  │
       │  Check contracts                 │
       │  (preconditions)                 ▼
       │    ┌──────────────────────────────────┐
       │    │  KIRK Contracts                  │
       │    │  - Verify preconditions          │
       │    │  - Check invariants              │
       │    └──────────────────────────────────┘
       │    │
       ▼    ▼
┌──────────────────────────────────────────────┐
│ Core Layer (zjj-core)                        │
│  ┌────────────────────────────────────────┐  │
│  │ Domain Logic (Pure Functions)          │  │
│  │ - State transitions                    │  │
│  │ - Validation                           │  │
│  │ - Business rules                       │  │
│  └────────────────────────────────────────┘  │
└──────────────┬───────────────────────────────┘
               │
               │  Return Result<T, E>
               ▼
┌──────────────┐      Verify contracts
│ Shell Layer  │ ─────────────────────────┐
│ (zjj)        │                          │
└──────┬───────┘                          │
       │                                  ▼
       │    ┌──────────────────────────────────┐
       │    │  KIRK Contracts                  │
       │    │  - Verify postconditions         │
       │    │  - Check invariants still hold   │
       │    └──────────────────────────────────┘
       │    │
       ▼    ▼
  ┌────────────────┐
  │  I/O Handlers  │
  │  - Database    │
  │  - File system │
  │  - External    │
  │    processes   │
  └────┬───────────┘
       │
       ▼
  ┌────────────────┐
  │  Output        │
  │  - JSONL       │
  │  - Human       │
  │    readable    │
  └────────────────┘
```

### Example: Creating a Session

```rust
// 1. SHELL: Parse CLI arguments
let args = AddArgs::parse();

// 2. SHELL: Parse-once validation
let name = SessionName::parse(&args.name)?;
let workspace = WorkspaceName::parse(&args.workspace)?;

// 3. SHELL: Check preconditions (KIRK)
SessionContracts::preconditions(&args)?;

// 4. SHELL: I/O operations
let db = get_session_db().await?;

// 5. CORE: Business logic (pure)
let session = SessionBuilder::new()
    .with_name(&name)
    .with_workspace(&workspace)
    .build()?;

// 6. SHELL: Persist to database
db.insert(&session).await?;

// 7. SHELL: Verify postconditions (KIRK)
SessionContracts::postconditions(&args, &session)?;

// 8. SHELL: Emit output
emit_stdout(&session).await?;
```

### Error Flow

```
Error occurs in core
    │
    ▼
Domain Error (thiserror)
    │
    │  Convert to boundary error
    ▼
anyhow::Error (with context)
    │
    │  Add helpful context
    ▼
Human-readable message
    │
    ▼
JSONL error output
```

---

## Key Design Decisions

### 1. SQLite for State Storage

**Rationale**:
- Single-file database, easy to manage
- ACID transactions for consistency
- Embedded, no external dependencies
- SQLx for compile-time checked queries

**Recovery Policy**:
- `silent` - Auto-recover without warning
- `warn` - Show warning, then recover (default)
- `fail-fast` - Fail immediately on corruption

### 2. Async/Await with Tokio

**Rationale**:
- Non-blocking database operations
- Responsive CLI during long operations
- Better resource utilization
- Futures combinators for pipelines

**Trade-off**: Core functions are `async fn` even if pure
**Mitigation**: Keep core logic stateless, use `await` only at boundaries

### 3. Semantic Newtypes with Validation

**Rationale**:
- Make illegal states unrepresentable
- Validate once at boundaries
- Self-documenting code
- Compiler guides correct usage

**Example**:
```rust
// Instead of:
fn create(name: &str) -> Result<Session>

// Use:
fn create(name: &SessionName) -> Result<Session, SessionError>
```

### 4. JSONL Output for AI-First Design

**Rationale**:
- Each line is a complete JSON object
- Streaming-friendly (no large JSON arrays)
- Easy to parse line-by-line
- Self-describing with `type` field

**Example**:
```jsonl
{"type":"session","name":"auth-refactor","status":"active","workspace":"auth-refactor"}
{"type":"warning","code":"sync_needed","message":"Workspace is behind main"}
{"type":"result","kind":"success","output":"Session created"}
```

### 5. KIRK Contracts for Reliability

**Rationale**:
- Catch bugs at runtime (debug builds)
- Document assumptions explicitly
- Verify invariants automatically
- Guide refactoring with confidence

**Example**:
```rust
impl Contract<CreateSessionInput, Session> for SessionContracts {
    fn preconditions(input: &CreateSessionInput) -> Result<(), ContractError> {
        require_precondition(
            !input.name.is_empty(),
            &Precondition::new("name_not_empty", "Session name must not be empty")
        )
    }

    fn postconditions(input: &CreateSessionInput, result: &Session) -> Result<(), ContractError> {
        require_postcondition(
            result.name.as_str() == input.name,
            &Postcondition::new("name_matches", "Session name matches input")
        )
    }
}
```

---

## Dependencies

### Core Dependencies (zjj-core)

| Library | Purpose | Pattern |
|---------|---------|---------|
| `thiserror` | Domain errors | Core error types |
| `anyhow` | Boundary errors | Shell error context |
| `itertools` | Iterator pipelines | `map().filter().collect()` |
| `tap` | Pipeline observation | `pipe().tap()` |
| `rpds` | Persistent data | Immutable collections |
| `futures-util` | Async combinators | `StreamExt`, `TryStreamExt` |
| `sqlx` | Database | Compile-time checked queries |
| `tokio` | Async runtime | `async fn`, `.await` |
| `serde` | Serialization | JSONL output |
| `strum` | Enum utilities | `Display`, `FromStr` derives |

### Shell Dependencies (zjj)

| Library | Purpose | Pattern |
|---------|---------|---------|
| `clap` | CLI parsing | Derive macros |
| `anyhow` | Boundary errors | `.context()` |
| `sqlx` | Database | Async operations |
| `tokio` | Async runtime | `#[tokio::main]` |
| `which` | Command detection | Find binaries in PATH |
| `directories` | XDG paths | Config locations |

### Dependency Graph

```
zjj (binary)
 │
 ├─→ zjj-core (library)
 │   ├─→ thiserror
 │   ├─→ anyhow
 │   ├─→ itertools
 │   ├─→ tap
 │   ├─→ rpds
 │   ├─→ futures-util
 │   ├─→ sqlx
 │   ├─→ tokio
 │   └─→ serde
 │
 ├─→ clap
 ├─→ sqlx
 ├─→ tokio
 ├─→ which
 └─→ directories
```

---

## Extension Points

### 1. Adding a New Command

**Steps**:

1. Define CLI args in `src/cli/commands.rs`
2. Create handler in `src/cli/handlers/`
3. Implement logic in `src/commands/`
4. Add contracts in `zjj-core/src/cli_contracts/`
5. Add output types in `zjj-core/src/output/`

**Example**:
```rust
// 1. CLI args
#[Args)]
pub struct MyCommand {
    pub name: String,
}

// 2. Handler
pub async fn handle_my_command(args: MyCommand) -> Result<()> {
    let name = SessionName::parse(&args.name)?;
    let db = get_session_db().await?;
    commands::my_command(&db, &name).await?;
    Ok(())
}

// 3. Command
pub async fn my_command(db: &SessionDb, name: &SessionName) -> Result<()> {
    // Pure business logic
}

// 4. Contracts
impl Contract<MyInput, MyOutput> for MyCommandContracts {
    // Preconditions, invariants, postconditions
}
```

### 2. Adding a New Domain Type

**Steps**:

1. Create newtype in `zjj-core/src/domain/identifiers.rs`
2. Add validation with `parse()` method
3. Add `thiserror` variant for validation errors
4. Implement `Display`, `FromStr`, `Serialize`, `Deserialize`
5. Add unit tests for valid/invalid inputs

**Example**:
```rust
// identifiers.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MyId(String);

impl MyId {
    pub fn parse(s: impl AsRef<str>) -> Result<Self, IdError> {
        let s = s.as_ref();
        if s.len() < 3 {
            return Err(IdError::TooShort);
        }
        Ok(MyId(s.to_string()))
    }
}

impl Display for MyId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
```

### 3. Adding a New Output Type

**Steps**:

1. Add type to `zjj-core/src/output/types.rs`
2. Add semantic newtypes to `domain_types.rs`
3. Implement `Serialize` for JSONL output
4. Add `OutputLine` variant
5. Document in output format spec

### 4. Adding a New Quality Gate

**Steps**:

1. Define gate in `zjj-core/src/coordination/train.rs`
2. Add to `QualityGate` enum
3. Implement check logic
4. Add to train processor
5. Add tests

### 5. Adding a New Contract

**Steps**:

1. Create module in `zjj-core/src/cli_contracts/`
2. Implement `Contract<T, R>` trait
3. Define preconditions, invariants, postconditions
4. Add helper functions
5. Wire up in CLI handlers

---

## Architecture Diagrams

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER / AGENT                            │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                          ZJJ CLI                                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Command Parser                          │  │
│  │                   (clap derive)                            │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                    CLI Handlers                            │  │
│  │  - Parse-once validation                                   │  │
│  │  - Check contracts (KIRK)                                  │  │
│  │  - Call command logic                                      │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                    Command Layer                           │  │
│  │  - Business logic orchestration                            │  │
│  │  - I/O coordination                                        │  │
│  │  - Error handling                                          │  │
│  └─────┬─────────────────────┬───────────────────────────────┘  │
│        │                     │                                  │
└────────┼─────────────────────┼──────────────────────────────────┘
         │                     │
         ▼                     ▼
┌──────────────────┐   ┌──────────────────────┐
│   zjj-core       │   │  External Systems    │
│  ┌────────────┐  │   │  ┌────────────────┐  │
│  │  Domain    │  │   │  │  JJ (git)      │  │
│  │  Types     │  │   │  │  File System   │  │
│  │            │  │   │  └────────────────┘  │
│  └────────────┘  │   └──────────────────────┘
│  ┌────────────┐  │   └──────────────────────┘
│  │ Business   │  │
│  │ Logic      │  │
│  │ (Pure)     │  │
│  └────────────┘  │
│  ┌────────────┐  │
│  │ Output     │  │
│  │ Types      │  │
│  └─────┬──────┘  │
└────────┼──────────┘
         │
         ▼
┌──────────────────┐
│  SQLite Database │
│  (.zjj/state.db) │
└──────────────────┘
```

### Session Lifecycle

```
          ┌─────────┐
          │  START  │
          └────┬────┘
               │
               ▼
        ┌──────────────┐
        │   zjj add    │  Create new session
        └──────┬───────┘
               │
               ▼
        ┌──────────────┐
        │   Creating   │  Create JJ workspace
        └──────┬───────┘
               │
               ▼
        ┌──────────────┐
        │    Ready     │  Ready for work
        └──────┬───────┘
               │
               ▼
        ┌──────────────┐
        │    Active    │  User/Agent working
        └──────┬───────┘
               │
               ▼
      ┌────────┴────────┐
      │                 │
      ▼                 ▼
┌───────────┐     ┌───────────┐
│   Done    │     │  Remove   │
│ (merge)   │     │ (delete)  │
└───────────┘     └───────────┘
```

### Queue Processing Flow

```
┌────────────────────────────────────────────────────────────┐
│                     WORK SUBMISSION                         │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                │
│  │ Agent 1 │    │ Agent 2 │    │ Agent 3 │                │
│  └────┬────┘    └────┬────┘    └────┬────┘                │
│       │              │              │                      │
│       └──────────────┼──────────────┘                      │
│                      ▼                                     │
│              ┌─────────────────┐                           │
│              │   Add to Queue  │                           │
│              └────────┬────────┘                           │
└──────────────────────────┼─────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────┐
│                    QUEUE (SQLite)                           │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐          │
│  │ PND │ │ PND │ │ PND │ │ CLM │ │ PRC │ │ DON │          │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘          │
│   Pending        Claimed Processing  Done                  │
└──────────────────────────┼─────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────┐
│                    TRAIN PROCESSOR                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  1. Calculate priority order                        │   │
│  │  2. Filter processable entries                      │   │
│  │  3. Execute quality gates                           │   │
│  │  4. Rebase onto main                                │   │
│  │  5. Run tests (Moon gates)                          │   │
│  │  6. Merge to main                                   │   │
│  │  7. Clean up workspace                             │   │
│  └─────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────┐
│                      RESULT HANDLING                         │
│  ┌───────────┐    ┌───────────┐    ┌───────────┐          │
│  │  Success  │    │   Retry   │    │   Failed  │          │
│  │   (Done)  │    │  (Queue)  │    │  (Log)    │          │
│  └───────────┘    └───────────┘    └───────────┘          │
└────────────────────────────────────────────────────────────┘
```

### Stack (Session Stacking) Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     MAIN BRANCH                              │
│                      (root)                                  │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Session A  │  (depth 1)
                    └──────┬──────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
    ┌───────────┐     ┌───────────┐    ┌───────────┐
    │Session A1 │     │Session A2 │    │Session A3 │  (depth 2)
    └─────┬─────┘     └─────┬─────┘    └─────┬─────┘
          │                 │                │
          └─────────────────┼────────────────┘
                           ▼
                    ┌─────────────┐
                    │  Session A2a│  (depth 3)
                    └─────────────┘

State Machine:
┌──────────┐    push    ┌──────────┐
│  Stacked  │ ────────▶ │  Active  │
└──────────┘            └────┬─────┘
     ▲                       │
     │                       │ pop/done
     │                       ▼
     │                  ┌──────────┐
     └──────────────────│  Stacked  │
                        └──────────┘
```

---

## Best Practices for Contributors

### When Adding Features

1. **Use Semantic Newtypes**: Define domain types for all identifiers
2. **Follow DDD**: Model aggregates, value objects, domain events
3. **Pure Core**: Keep core logic free of I/O and global state
4. **Railway Errors**: Use `Result<T, E>` throughout, no unwraps
5. **KIRK Contracts**: Define preconditions, invariants, postconditions
6. **JSONL Output**: Emit structured output for AI consumption
7. **Test Coverage**: Add unit tests for domain logic
8. **Documentation**: Document invariants and state transitions

### When Refactoring

1. **Start from Domain**: Identify aggregates and value objects first
2. **Parse Once**: Move validation to boundaries
3. **Make Illegal States Unrepresentable**: Use enums instead of Option/bool
4. **Keep Core Pure**: Isolate I/O in shell layer
5. **Use Iterator Pipelines**: Prefer combinators over loops
6. **Add Contracts**: Document assumptions with KIRK

### When Error Handling

1. **Core**: Use `thiserror` for domain errors
2. **Shell**: Use `anyhow` with `.context()` for boundary errors
3. **Never Panic**: Use `Result` for all fallible operations
4. **Semantic Errors**: Error types should convey domain meaning
5. **Recovery**: Provide recovery actions when possible

---

## References

- [AGENTS.md](AGENTS.md) - Agent development guidelines
- [README.md](README.md) - Project overview and quick start
- Scott Wlaschin's "Domain Modeling Made Functional"
- "Type-Driven Development with Idris"
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Functional Core, Imperative Shell: https://www.sitepoint.com/functional-core-imperative-shell/
