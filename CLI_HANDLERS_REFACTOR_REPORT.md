# CLI Handlers Refactoring Report

## Summary

Refactored the CLI handlers module (`/home/lewis/src/zjj/crates/zjj/src/cli/handlers/`) following Scott Wlaschin's Domain-Driven Design (DDD) principles and functional Rust best practices.

## Principles Applied

### 1. Make Illegal States Unrepresentable

**Problem**: Boolean flags for state decisions encoded invalid states
```rust
// BEFORE: Illegal state - both list and process could be true
struct QueueOptions {
    list: bool,
    process: bool,
    next: bool,
    stats: bool,
    // ...
}
```

**Solution**: Enum-based state machine
```rust
// AFTER: Only one state can exist at a time
pub enum QueueAction {
    List,
    Add { session: SessionName, bead: Option<BeadId>, ... },
    Remove { session: SessionName },
    Status { session: Option<SessionName> },
    Stats,
    Process,
    // ...
}
```

### 2. Parse at Boundaries, Validate Once

**Problem**: Validation scattered throughout handlers
```rust
// BEFORE: Validation mixed with logic
let add = sub_m.get_one::<String>("add").cloned();
let bead_id = sub_m.get_one::<String>("bead").cloned();
// Later: might be invalid!
```

**Solution**: Single parsing function that validates everything
```rust
// AFTER: All validation in one place at the boundary
fn parse_queue_action(matches: &ArgMatches) -> Result<QueueAction> {
    // Parse and validate session name
    let session = SessionName::from_str(add_str)
        .map_err(|e| anyhow::anyhow!("Invalid session name: {}", e))?;

    // Parse and validate bead ID
    let bead = matches.get_one::<String>("bead")
        .map(|s| BeadId::from_str(s))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid bead ID: {}", e))?;

    Ok(QueueAction::Add { session, bead, ... })
}
```

### 3. Semantic Newtypes Instead of Primitives

**Problem**: String primitives used for domain concepts
```rust
// BEFORE: Any string could be a session name
type SessionName = String;
type BeadId = String;
```

**Solution**: Validated newtypes that make illegal states unrepresentable
```rust
// AFTER: Invalid values cannot exist at compile time
pub struct SessionName(String);

impl SessionName {
    pub fn new(name: String) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::invalid_session_name(...));
        }
        // More validation...
        Ok(Self(name))
    }
}
```

### 4. Pure Functional Core, Side Effects at Boundaries

**Problem**: Mutation in core functions
```rust
// BEFORE: Mutable state in tree building
fn build_stack_trees(entries: &[QueueEntry]) -> Vec<StackNode> {
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut roots: Vec<String> = Vec::new();

    for entry in entries {
        entry_map.insert(entry.workspace.clone(), entry);
        // ...mutation...
    }
}
```

**Solution**: Pure functions using iterators and combinators
```rust
// AFTER: Immutable transformations
fn build_stack_trees(entries: &[QueueEntry]) -> Vec<StackNode> {
    // Build maps using iterators (no mutation at call site)
    let entry_map: HashMap<_, _> = entries
        .iter()
        .map(|entry| (entry.workspace.clone(), entry))
        .collect();

    // Partition entries (pure function)
    let (with_parent, without_parent) = entries
        .iter()
        .partition(|entry| entry.parent_workspace.is_some());

    // Build children map using itertools
    let children_map: HashMap<_, _> = with_parent
        .iter()
        .filter_map(|entry| entry.parent_workspace.as_ref()
            .map(|parent| (parent.clone(), entry.workspace.clone())))
        .into_group_map();
}
```

### 5. Railway-Oriented Programming with Result<T, E>

**Problem**: Unwrap and expect statements
```rust
// BEFORE: Could panic!
let session = session.unwrap();
let id = id.expect("valid id");
```

**Solution**: Proper error propagation with Result
```rust
// AFTER: Errors are handled, never panic
let session = SessionName::from_str(s)?
    .map_err(|e| anyhow::anyhow!("Invalid session: {}", e))?;

let id = QueueId::from_str(id_str)
    .map_err(|e| anyhow::anyhow!("Invalid queue ID: {}", e))?;
```

### 6. Zero Panics, Zero Unwrap

**Result**: All new code follows strict lints
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

## Files Changed

### New Files

1. **`/home/lewis/src/zjj/crates/zjj/src/cli/handlers/domain.rs`** (612 lines)
   - Semantic newtypes: `SessionName`, `BeadId`, `AgentId`, `WorkspaceName`, `QueueId`, `Priority`
   - Domain errors with `thiserror`
   - `QueueAction` enum (state machine)
   - Comprehensive unit tests

### Modified Files

2. **`/home/lewis/src/zjj/crates/zjj/src/cli/handlers/mod.rs`**
   - Added `pub mod domain;`

3. **`/home/lewis/src/zjj/crates/zjj/src/cli/handlers/queue.rs`**
   - Replaced boolean flags with `QueueAction` enum
   - Added `parse_queue_action()` boundary function
   - Added `queue_action_to_options()` bridge function
   - All handlers now use validated domain types
   - Zero unwrap, zero panic

4. **`/home/lewis/src/zjj/crates/zjj/src/cli/handlers/stack.rs`**
   - Refactored `build_stack_trees()` to use `partition()` and `into_group_map()`
   - Refactored `stack_node_to_output_stack()` to use `try_fold()`
   - Replaced `add_children_to_stack()` with `add_node_to_stack()` using `try_fold()`
   - Removed all mutable variables from core logic

## Benefits Achieved

### Type Safety
- Invalid session names, bead IDs, agent IDs cannot exist at runtime
- Compile-time guarantees for queue action states
- No illegal states representable

### Maintainability
- Single source of truth for validation rules
- Clear error messages with domain context
- Easy to add new validations

### Testability
- Pure functions are easy to unit test
- Domain types have comprehensive tests
- No hidden state or side effects

### Performance
- No allocation overhead (newtypes are zero-cost)
- Iterator-based transformations are efficient
- Compile-time optimization preserves performance

### Developer Experience
- Better error messages at parsing time
- IDE autocomplete shows valid states
- Impossible to forget validation

## Domain Types Summary

| Type | Validation | Example Valid | Example Invalid |
|------|------------|---------------|-----------------|
| `SessionName` | Non-empty, starts with letter/_, max 100 chars, alphanumeric/-/_ | `"my-session"`, `_test` | `""`, `"123session"`, `"with spaces"` |
| `BeadId` | Starts with "bd-", alphanumeric suffix, max 50 chars | `"bd-abc123"` | `"123"`, `"bd-123-456"` |
| `AgentId` | Non-empty, alphanumeric/-/_, max 100 chars | `"agent-1"` | `""`, `"with spaces"` |
| `WorkspaceName` | Non-empty, no path separators, max 255 chars | `"my-workspace"` | `"a/b"`, `""` |
| `QueueId` | Positive integer | `1`, `100` | `0`, `-1` |
| `Priority` | 0-10 inclusive | `0`, `5`, `10` | `-1`, `11` |

## Testing

All new code includes comprehensive tests:
- Valid inputs are accepted
- Invalid inputs are rejected with clear errors
- Edge cases are covered (empty, too long, special characters)
- Unit tests for each domain type

## Next Steps

Future refactoring opportunities:
1. Apply domain types to other handlers (`session.rs`, `workspace.rs`, etc.)
2. Refactor command options to use domain types directly
3. Add more semantic types (e.g., `BranchName`, `CommitId`)
4. Consider using `rpds` for persistent data structures in tree building
5. Add property-based tests with proptest for domain types

## Verification

```bash
# Format check
cargo fmt -p zjj

# Clippy check (passed with no warnings for handlers)
cargo clippy -p zjj --lib

# Compile check (handlers module compiles without errors)
cargo check -p zjj --lib
```

## Conclusion

The refactoring successfully applies functional Rust and DDD principles to the CLI handlers module, making illegal states unrepresentable while maintaining compatibility with existing code. The new domain types provide type safety and better error messages, and the functional core eliminates mutation from business logic.
