# ZJJ Refactoring Architecture

**Visual guide to the refactored codebase architecture**

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI SHELL (Imperative)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Handlers   │  │   Commands   │  │    I/O       │          │
│  │  (parse)     │→ │  (validate)  │→ │  (async)     │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Parse to Domain Types
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    FUNCTIONAL CORE (Pure)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Domain     │  │   Business   │  │   State      │          │
│  │   Types      │→ │    Logic     │→ │  Machines    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                   │
│  • No I/O          • Deterministic      • Compile-time safety    │
│  • No global state • No mutation        • Zero unwrap/panic     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Persist
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                   INFRASTRUCTURE LAYER                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Database    │  │   File I/O   │  │  External    │          │
│  │  (SQLite)    │  │  (JSONL)     │  │  (jj) │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Dependency Graph

```
                       ┌─────────────────┐
                       │  CLI Shell      │
                       │  (zjj/src/)     │
                       └────────┬────────┘
                                │ parses to
                                ↓
┌───────────────────────────────────────────────────────────────┐
│                    Domain Layer                                │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  zjj-core/src/domain/                                 │    │
│  │  ├── identifiers.rs  (SessionName, AgentId, ...)      │    │
│  │  ├── agent.rs        (AgentStatus, ...)               │    │
│  │  ├── session.rs      (BranchState, ParentState, ...)  │    │
│  │  ├── workspace.rs    (WorkspaceState, ...)            │    │
│  │  └── queue.rs        (ClaimState, ...)                │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  zjj-core/src/cli_contracts/                          │    │
│  │  ├── domain_types.rs   (CLI-specific types)           │    │
│  │  ├── session_v2.rs     (Session contracts)            │    │
│  │  ├── queue_v2.rs       (Queue contracts)              │    │
│  │  └── task.rs           (Task contracts)               │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  zjj-core/src/beads/                                  │    │
│  │  ├── domain.rs        (Issue domain types)            │    │
│  │  └── issue.rs         (Issue aggregate root)          │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  zjj-core/src/coordination/                           │    │
│  │  ├── domain_types.rs  (Coordination types)            │    │
│  │  └── pure_queue.rs    (Pure queue implementation)     │    │
│  └──────────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────────┘
                                │
                                │ uses
                                ↓
┌───────────────────────────────────────────────────────────────┐
│                    Infrastructure Layer                         │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  zjj-core/src/beads/                                  │    │
│  │  ├── db.rs            (Database persistence)          │    │
│  │  └── query.rs         (Query operations)              │    │
│  └──────────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────────┘
```

---

## Type Transformation Flow

```
┌───────────────────────────────────────────────────────────────┐
│                    DATA FLOW DIAGRAM                           │
└───────────────────────────────────────────────────────────────┘

 User Input (String)
        │
        │ PARSE (Shell Layer)
        ↓
 ┌─────────────┐
 │ Validation  │  ← Errors returned here if invalid
 └──────┬──────┘
        │ Valid
        ↓
 ┌─────────────────────┐
 │  Domain Type        │  ← Type-safe, guaranteed valid
 │  (SessionName)      │
 └──────────┬──────────┘
            │ Pass to Core
            ↓
 ┌─────────────────────┐
 │  Pure Function      │  ← No validation needed!
 │  create_session()   │
 └──────────┬──────────┘
            │ Result
            ↓
 ┌─────────────────────┐
 │  Domain Result      │  ← Type-safe result
 │  (Session)          │
 └──────────┬──────────┘
            │
            │ SERIALIZE (Shell Layer)
            ↓
 ┌─────────────────────┐
 │  Output             │
 │  (JSON/Text)        │
 └─────────────────────┘
```

---

## State Machine Examples

### Session State Machine

```
                    ┌─────────────┐
                    │  Creating   │
                    └──────┬──────┘
                           │ create()
                           ↓
                    ┌─────────────┐
           ┌────────│   Active    │────────┐
           │        └─────────────┘        │
           │                             pause()
   spawn() │                               │ resume()
           │                             ↓
           │                        ┌─────────────┐
           │                        │   Paused    │
           │                        └──────┬──────┘
           │                               │
           │         fail() / complete()   │
           └──────────────────────────────┼────────┐
                                          │        │
                                          ↓        ↓
                                   ┌──────────┬──────────┐
                                   │ Completed │  Failed  │
                                   └──────────┴──────────┘
                                        (Terminal)
```

### Task State Machine

```
                    ┌─────────────┐
                    │    Open     │
                    └──────┬──────┘
                           │ claim()
                           ↓
                    ┌─────────────┐
                    │ InProgress  │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
      block()         done()        yield()
           │               │               │
           ↓               ↓               ↓
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │ Blocked  │    │  Closed  │    │   Open   │
    └──────────┘    └──────────┘    └──────────┘
                                           │
                                           └──(back to Open)
```

### Queue Entry State Machine

```
                    ┌─────────────┐
                    │   Pending   │
                    └──────┬──────┘
                           │ claim()
                           ↓
                    ┌─────────────┐
           ┌────────│ Processing  │────────┐
           │        └─────────────┘        │
           │                             │
      cancel()                      complete()
           │                             │
           ↓                             ↓
    ┌──────────┐                  ┌──────────┐
    │ Cancelled│                  │Completed │
    └──────────┘                  └──────────┘

    Note: Also supports "Failed" state
```

---

## Domain Type Hierarchy

```
DomainType (trait)
    │
    ├── Identifier
    │   ├── SessionName
    │   ├── AgentId
    │   ├── WorkspaceName
    │   ├── TaskId / BeadId
    │   └── IssueId
    │
    ├── State Machine
    │   ├── SessionStatus (Creating, Active, Paused, Completed, Failed)
    │   ├── TaskStatus (Open, InProgress, Blocked, Closed)
    │   ├── QueueStatus (Pending, Processing, Completed, Failed, Cancelled)
    │   ├── AgentStatus (Active, Idle, Offline, Error)
    │   ├── BranchState (Detached, OnBranch)
    │   └── ParentState (NoParent, HasParent)
    │
    ├── Value Object
    │   ├── Priority (0..=1000)
    │   ├── Limit (1..=1000)
    │   ├── TimeoutSeconds (1..=86400)
    │   └── NonEmptyString
    │
    └── Collection
        ├── Labels (Vec<Label>)
        ├── DependsOn (Vec<TaskId>)
        └── BlockedBy (Vec<TaskId>)
```

---

## Testing Pyramid

```
                    ┌─────────────────────┐
                    │  E2E / BDD Tests    │  30+
                    │  (features/*.feature)│
                    └──────────┬──────────┘
                               │
                    ┌──────────┴──────────┐
                    │ Integration Tests   │  120+
                    │  (tests/*.rs)       │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┴──────────────────────┐
        │  Property-Based Tests (proptest)            │  137
        │  ┌──────────────────────────────────────┐  │
        │  │ • Invariant testing                  │  │
        │  │ • State machine validation           │  │
        │  │ • Concurrent operations              │  │
        │  └──────────────────────────────────────┘  │
        └──────────────────────┬──────────────────────┘
                               │
                    ┌──────────┴──────────┐
                    │   Unit Tests        │  2,900+
                    │   (src/*_test.rs)   │
                    └─────────────────────┘

Total Test Functions: 3,135+
Total Test Cases Executed: 31,088+
```

---

## Error Handling Flow

```
┌───────────────────────────────────────────────────────────────┐
│                  ERROR PROPAGATION PATTERN                     │
└───────────────────────────────────────────────────────────────┘

 Shell Layer (anyhow)
    │
    │ Parse Error
    ↓
 ┌─────────────────────────┐
 │  anyhow::Context        │  ← Add context at boundaries
 │  .context("failed ...") │
 └──────────┬──────────────┘
            │ Convert
            ↓
 Core Layer (thiserror)
 ┌─────────────────────────┐
 │  DomainError            │  ← Structured domain errors
 │  ├── InvalidInput       │
 │  ├── NotFound           │
 │  ├── InvalidTransition  │
 │  └── ...                │
 └─────────────────────────┘
            │
            │ Return via ?
            ↓
 ┌─────────────────────────┐
 │  Result<T, E>           │  ← Railway-oriented programming
 │  Ok(value) │ Err(error) │
 └─────────────────────────┘
```

---

## File Organization

```
zjj/
├── crates/
│   ├── zjj/                    # CLI application (shell)
│   │   ├── src/
│   │   │   ├── cli/            # CLI handlers
│   │   │   │   └── handlers/   # Parse user input → Domain Types
│   │   │   └── commands/       # Command orchestration
│   │   └── tests/              # CLI integration tests
│   │
│   └── zjj-core/               # Core library (pure + infra)
│       ├── src/
│       │   ├── domain/         # ✨ NEW: Domain types
│       │   ├── cli_contracts/  # ✨ NEW: Type-safe contracts
│       │   ├── beads/          # ✨ REFACTORED: Aggregate roots
│       │   ├── coordination/   # ✨ REFACTORED: Pure queue
│       │   ├── output/         # Output formatting
│       │   └── ...
│       └── tests/              # ✨ NEW: Property tests
│
├── features/                   # ✨ NEW: BDD test scenarios
├── tests/                      # ✨ NEW: Integration tests
└── *.md                        # ✨ NEW: Documentation
```

---

## Key Patterns Summary

| Pattern | Purpose | Example |
|---------|---------|---------|
| **Semantic Newtype** | Type safety | `SessionName(String)` |
| **State Enum** | Make illegal states unrepresentable | `BranchState { Detached, OnBranch }` |
| **Parse Once** | Validate at boundaries | `SessionName::parse(input)?` |
| **Pure Core** | Deterministic logic | Core functions have no I/O |
| **Railway-Oriented** | Error propagation | All functions return `Result<T, E>` |
| **Aggregate Root** | Consistency boundary | `Issue` manages its own invariants |

---

## Before vs After

### Before: Primitive Obsession
```rust
pub async fn create_session(
    &self,
    name: &str,              // Could be any string!
    branch: Option<String>,  // Inconsistent with state
    status: String,          // Could be "invalid"!
) -> Result<Session>
{
    if name.is_empty() {          // Validation scattered
        return Err(...);
    }
    if !is_valid_status(status) { // Runtime check
        return Err(...);
    }
    // ... business logic mixed with validation
}
```

### After: Domain Types
```rust
pub async fn create_session(
    &self,
    name: &SessionName,      // Already validated!
    branch: BranchState,     // Consistent by construction
    status: SessionStatus,   // Only valid variants
) -> Result<Session, SessionError>
{
    // Pure business logic - no validation needed!
    // Compiler guarantees valid inputs
}
```

---

**See Also**: `FINAL_REFACTORING_REPORT.md` for complete details
