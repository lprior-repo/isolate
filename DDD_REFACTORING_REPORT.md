# DDD Refactoring Report: Commands Module

## Executive Summary

Applied Scott Wlaschin's Domain-Driven Design refactoring principles to the ZJJ commands module. Created semantic newtype wrappers to make illegal states unrepresentable and established parse-once validation patterns at boundaries.

---

## Phase 1: Semantic Newtypes for Identifiers (COMPLETED)

### Created Files

1. **`/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs`**
   - Domain module entry point
   - Re-exports identifier types

2. **`/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`**
   - `SessionName` - Validated session name (1-63 chars, starts with letter)
   - `AgentId` - Validated agent ID (1-128 chars, alphanumeric + symbols)
   - `WorkspaceName` - Validated workspace name (no path separators)
   - `TaskId` - Validated task ID (bd-{hex} format)
   - `BeadId` - Alias for TaskId
   - `IdError` - Domain errors for validation failures

3. **`/home/lewis/src/zjj/crates/zjj-core/src/domain/agent.rs`**
   - `AgentState` enum (Active, Idle, Offline, Error)
   - `AgentInfo` struct with last_seen timestamp

4. **`/home/lewis/src/zjj/crates/zjj-core/src/domain/session.rs`**
   - `BranchState` enum - Replaces `Option<String>` for branch
   - `ParentState` enum - Replaces `Option<String>` for parent_session

5. **`/home/lewis/src/zjj/crates/zjj-core/src/domain/workspace.rs`**
   - `WorkspaceState` enum (Creating, Ready, Active, Cleaning, Removed)
   - `WorkspaceInfo` struct

6. **`/home/lewis/src/zjj/crates/zjj-core/src/domain/queue.rs`**
   - `ClaimState` enum - Replaces Option fields for claimed_by/claimed_at
   - `QueueCommand` enum - Replaces boolean flags in QueueOptions

---

## Identified DDD Violations

### 1. Primitive Obsession (HIGH PRIORITY)

**Locations**: Throughout all command files

**Issue**: String/str used as identifiers without validation
- Session names: `name: &str`
- Agent IDs: `agent_id: Option<String>`
- Workspace names: `workspace: &str`
- Task IDs: passed as raw strings

**Impact**:
- Invalid states can propagate through the system
- Validation scattered throughout code
- Type errors only caught at runtime

**Solution**: Use semantic newtypes (Phase 1 - DONE)
```rust
// Before:
pub async fn create_session(&self, name: &str, ...)

// After:
pub async fn create_session(&self, name: &SessionName, ...)
```

### 2. Boolean Flags for State Decisions (HIGH PRIORITY)

**Location**: `QueueOptions`, `SessionCommandOptions`

**Issue**: Structs with 10+ boolean flags
```rust
#[allow(clippy::struct_excessive_bools)]
pub struct QueueOptions {
    pub list: bool,
    pub process: bool,
    pub next: bool,
    pub stats: bool,
    pub status_id: Option<i64>,
    pub retry: Option<i64>,
    pub cancel: Option<i64>,
    pub reclaim_stale: Option<i64>,
    // ... more flags
}
```

**Solution**: Use enums (Phase 1 - DONE)
```rust
pub enum QueueCommand {
    List,
    Process,
    Next,
    Stats,
    ShowStatus { workspace: WorkspaceName },
    Add { workspace: WorkspaceName, bead: Option<BeadId> },
    // ...
}
```

### 3. Option Fields Encoding State Machines (MEDIUM PRIORITY)

**Locations**: `Session`, `TaskInfo`, queue entries

**Issue**: `Option<String>` fields that should be explicit states
```rust
pub struct Session {
    pub branch: Option<String>,  // Should be BranchState
    pub parent_session: Option<String>,  // Should be ParentState
}

pub struct TaskInfo {
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claim_expires_at: Option<DateTime<Utc>>,
    // Should be ClaimState
}
```

**Solution**: Use explicit state enums (Phase 1 - DONE)
```rust
pub enum BranchState {
    Detached,
    OnBranch { name: String },
}

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

### 4. Repeated Validation (MEDIUM PRIORITY)

**Locations**: `config.rs`, `session_command.rs`, `queue.rs`

**Issue**: Validation called at boundaries but strings still passed around
```rust
// In multiple places:
validate_session_name(name)?;
// Then pass raw string anyway
db.create(name, workspace)
```

**Solution**: Parse-once pattern (NEXT PHASE)
```rust
// Shell layer - parse once
let name = SessionName::parse(raw_name)?;

// Core layer - accept only validated type
db.create(&name, &workspace)
```

### 5. Opaque Error Strings (LOW PRIORITY)

**Locations**: Error handling across all modules

**Issue**: `anyhow::Error` used at boundaries with string messages
```rust
anyhow::anyhow!("Session '{name}' not found")
```

**Solution**: Domain errors with `thiserror`
```rust
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session '{0}' not found")]
    NotFound(SessionName),

    #[error("session '{0}' already exists")]
    AlreadyExists(SessionName),
}
```

---

## Refactoring Plan

### Phase 1: Semantic Newtypes ✅ COMPLETED

- [x] Create `SessionName` with validation
- [x] Create `AgentId` with validation
- [x] Create `WorkspaceName` with validation
- [x] Create `TaskId` / `BeadId` with validation
- [x] Create `BranchState` enum
- [x] Create `ParentState` enum
- [x] Create `ClaimState` enum
- [x] Create `QueueCommand` enum

### Phase 2: Update Core Types (NEXT)

**Files to modify**:
1. `crates/zjj/src/session.rs`
   - Replace `branch: Option<String>` with `BranchState`
   - Replace `parent_session: Option<String>` with `ParentState`

2. `crates/zjj-core/src/coordination/queue.rs`
   - Replace claimed_by/claimed_at options with `ClaimState`

3. `crates/zjj-core/src/types.rs`
   - Add new domain state types

### Phase 3: Update Core Business Logic

**Files to modify**:
1. `crates/zjj/src/commands/session_command.rs`
   - Accept `&SessionName` instead of `&str`
   - Return domain-specific errors

2. `crates/zjj/src/commands/queue.rs`
   - Accept `QueueCommand` instead of `QueueOptions`
   - Remove `#[allow(clippy::struct_excessive_bools)]`

3. `crates/zjj/src/commands/task.rs`
   - Accept `&TaskId` instead of `&str`

### Phase 4: Update Shell Layer (Parsing)

**Files to modify**:
1. `crates/zjj/src/cli/handlers/*.rs`
   - Parse `SessionName::parse()` at CLI boundary
   - Parse `AgentId::parse()` from environment
   - Parse `WorkspaceName::parse()` from args

2. `crates/zjj/src/commands/mod.rs`
   - Convert `QueueOptions` to `QueueCommand`

### Phase 5: Add Domain Errors

**Files to create**:
1. `crates/zjj-core/src/domain/errors.rs`
   - `SessionError`
   - `QueueError`
   - `TaskError`

2. Update imports across all command files

---

## Examples of Refactored Code

### Before: Primitive Obsession

```rust
// In session_command.rs
pub async fn create_session(
    &self,
    name: &str,  // Could be any string!
    workspace_path: &str,
    parent: Option<&str>,
    agent_id: Option<&str>,
) -> Result<Session> {
    // Validation scattered
    validate_session_name(name)?;

    if let Some(p) = parent {
        if self.db.get(p).await?.is_none() {
            return Err(anyhow::anyhow!("Parent session '{p}' not found"));
        }
    }

    // ... rest of logic
}
```

### After: Semantic Newtypes

```rust
// In session_command.rs
pub async fn create_session(
    &self,
    name: &SessionName,  // Already validated!
    workspace_path: &WorkspacePath,
    parent: Option<&SessionName>,  // Already validated!
    agent_id: Option<&AgentId>,  // Already validated!
) -> Result<Session, SessionError> {
    // No validation needed - done at boundary
    // Business logic only

    if let Some(p) = parent {
        if self.db.get(p).await?.is_none() {
            return Err(SessionError::ParentNotFound(p.clone()));
        }
    }

    // ... rest of logic
}
```

### Before: Boolean Flags

```rust
// In queue.rs
#[allow(clippy::struct_excessive_bools)]
pub struct QueueOptions {
    pub list: bool,
    pub process: bool,
    pub next: bool,
    // ... 10 more booleans
}

// Run logic checks multiple booleans
if options.list {
    handle_list()
} else if options.process {
    handle_process()
} // ... many more branches
```

### After: Enum Command

```rust
// In queue.rs
pub enum QueueCommand {
    List,
    Process,
    Next,
    // ... specific variants
}

// Run logic uses pattern matching
match command {
    QueueCommand::List => handle_list(),
    QueueCommand::Process => handle_process(),
    QueueCommand::Next => handle_next(),
    // ... exhaustive match
}
```

### Before: Option Fields for State

```rust
// In session.rs
pub struct Session {
    pub branch: Option<String>,
    pub parent_session: Option<String>,
}

// Usage requires checking both Option and parsing string
if let Some(branch) = session.branch {
    println!("On branch: {branch}");
} else {
    println!("Detached");
}
```

### After: Explicit State Enum

```rust
// In session.rs
pub struct Session {
    pub branch: BranchState,
    pub parent_session: ParentState,
}

// Usage uses match, exhaustive handling
match session.branch {
    BranchState::Detached => println!("Detached"),
    BranchState::OnBranch { name } => println!("On branch: {name}"),
}
```

---

## Testing

### Unit Tests Added

All identifier types include comprehensive tests:
- Valid input acceptance
- Invalid input rejection with proper errors
- Edge cases (empty, too long, special characters)
- Serialization/deserialization
- Display formatting

Run tests with:
```bash
cargo test -p zjj-core --lib domain
```

---

## Next Steps

1. **Immediate**: Run full test suite to ensure no regressions
   ```bash
   cargo test -p zjj
   ```

2. **Phase 2**: Update core types to use new domain types
   - Modify `Session` struct
   - Modify `QueueEntry` struct
   - Modify `TaskInfo` struct

3. **Phase 3**: Update core business logic
   - Change function signatures to accept domain types
   - Update return types to use domain errors

4. **Phase 4**: Update shell layer
   - Parse at CLI boundaries
   - Convert to domain types before passing to core

5. **Phase 5**: Add domain errors
   - Create comprehensive error types
   - Replace anyhow::Error in core

---

## Benefits

1. **Type Safety**: Invalid states cannot be represented
2. **Parse Once**: Validation at boundaries, trusted in core
3. **Self-Documenting**: `SessionName` conveys intent vs `&str`
4. **Better Errors**: Domain-specific errors with context
5. **Easier Refactoring**: Compiler guides changes
6. **Zero Runtime Overhead**: Newtypes compile away

---

## File Reference

### Created Files
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/agent.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/session.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/workspace.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/queue.rs`

### Modified Files
- `/home/lewis/src/zjj/crates/zjj-core/src/lib.rs` (added `pub mod domain`)

### To Be Modified (Next Phases)
- `crates/zjj/src/session.rs`
- `crates/zjj/src/commands/session_command.rs`
- `crates/zjj/src/commands/queue.rs`
- `crates/zjj/src/commands/task.rs`
- `crates/zjj/src/commands/config.rs`
- Various CLI handler files

---

## References

- Scott Wlaschin's "Domain Modeling Made Functional"
- "Type-Driven Development with Idris"
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- ThisError documentation: https://docs.rs/thiserror/
