# DDD Refactoring Quick Start

## What Was Done

Created semantic newtype wrappers in `/home/lewis/src/zjj/crates/zjj-core/src/domain/` to make illegal states unrepresentable.

## Key Files

### Domain Types (Created)

```rust
// Validated identifiers - use these instead of String/str
use zjj_core::domain::{
    SessionName,    // Validated session name (1-63 chars, starts with letter)
    AgentId,        // Validated agent ID (1-128 chars)
    WorkspaceName,  // Validated workspace name (no path separators)
    TaskId,         // Validated task ID (bd-{hex} format)
    BeadId,         // Alias for TaskId
};

// State enums - use these instead of Option<String>
use zjj_core::domain::session::{
    BranchState,    // Detached | OnBranch { name }
    ParentState,    // Root | ChildOf { parent }
};

use zjj_core::domain::queue::{
    ClaimState,     // Unclaimed | Claimed { ... } | Expired { ... }
    QueueCommand,   // List | Process | Next | Add { ... } | ...
};
```

### Usage Example

```rust
// OLD (primitive obsession) - can represent invalid states
async fn create_session(&self, name: &str, ...) -> Result<Session> {
    validate_session_name(name)?;  // Validation scattered
    // ...
}

// NEW (semantic newtype) - invalid states impossible
async fn create_session(&self, name: &SessionName, ...) -> Result<Session, SessionError> {
    // Already validated! No checks needed.
    // ...
}

// Parse at boundary (shell layer)
let name = SessionName::parse(raw_input)?;  // Validates once
create_session(&name, ...).await?;           // Core trusts it
```

## Testing

```bash
# Run domain tests
cargo test -p zjj-core --lib domain

# Run all tests (check for regressions)
cargo test -p zjj
```

## Next Phase

Update existing types to use the new domain types:
1. `Session` struct - use `BranchState`, `ParentState`
2. `QueueOptions` - replace with `QueueCommand` enum
3. Function signatures - accept `&SessionName` instead of `&str`

See `/home/lewis/src/zjj/DDD_REFACTORING_REPORT.md` for full details.
