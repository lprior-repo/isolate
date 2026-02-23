# CLI Handlers Functional Refactoring - Summary

## What Was Done

I successfully refactored the CLI handlers module following Scott Wlaschin's DDD refactor principles and functional Rust best practices.

## Files Modified

### 1. Created New Domain Types Module
**File**: `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/domain.rs` (612 lines)

**Purpose**: Provides semantic newtypes that make illegal states unrepresentable

**Key Types**:
- `SessionName` - Validated session names (non-empty, alphanumeric/-/_, max 100 chars)
- `BeadId` - Validated bead IDs (bd- prefix, alphanumeric suffix, max 50 chars)
- `AgentId` - Validated agent IDs (non-empty, alphanumeric/-/_, max 100 chars)
- `WorkspaceName` - Validated workspace names (no path separators, max 255 chars)
- `QueueId` - Validated queue entry IDs (positive integers)
- `Priority` - Validated priority values (0-10)
- `QueueAction` - Enum-based state machine for queue operations

**Key Features**:
- All types implement `FromStr` for parsing
- All types implement `Display` for formatting
- All types implement `AsRef<str>` for easy string access
- Comprehensive unit tests for each type
- Zero-cost abstraction (newtypes compile away)

### 2. Refactored Queue Handler
**File**: `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/queue.rs`

**Changes**:
- Replaced boolean flags with `QueueAction` enum (makes illegal states unrepresentable)
- Added `parse_queue_action()` function (parse at boundaries, validate once)
- Added `queue_action_to_options()` bridge function (maintains compatibility)
- All handlers now use validated domain types
- Removed `unwrap()` and `expect()` calls
- Added strict lints at top of file

**Before**:
```rust
let add = sub_m.get_one::<String>("add").cloned();
let bead_id = sub_m.get_one::<String>("bead").cloned();
let priority = sub_m.get_one::<i32>("priority").copied().unwrap_or(5);
// Could be invalid! No validation until later.
```

**After**:
```rust
let session = SessionName::from_str(add_str)
    .map_err(|e| anyhow::anyhow!("Invalid session name: {}", e))?;
let bead = matches.get_one::<String>("bead")
    .map(|s| BeadId::from_str(s))
    .transpose()
    .map_err(|e| anyhow::anyhow!("Invalid bead ID: {}", e))?;
// Guaranteed valid! All validation at the boundary.
```

### 3. Refactored Stack Handler
**File**: `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/stack.rs`

**Changes**:
- Refactored `build_stack_trees()` to use functional patterns:
  - Used `partition()` instead of manual filtering
  - Used `into_group_map()` from itertools instead of mutation
  - Eliminated mutable variables in core logic
- Refactored `stack_node_to_output_stack()` to use `try_fold()` instead of for loop
- Replaced `add_children_to_stack()` with `add_node_to_stack()` using `try_fold()`
- Added strict lints at top of file

**Before**:
```rust
fn build_stack_trees(entries: &[QueueEntry]) -> Vec<StackNode> {
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut roots: Vec<String> = Vec::new();

    for entry in entries {
        entry_map.insert(entry.workspace.clone(), entry);
        // ...mutation...
    }
}
```

**After**:
```rust
fn build_stack_trees(entries: &[QueueEntry]) -> Vec<StackNode> {
    let entry_map: HashMap<_, _> = entries
        .iter()
        .map(|entry| (entry.workspace.clone(), entry))
        .collect();

    let (with_parent, without_parent) = entries
        .iter()
        .partition(|entry| entry.parent_workspace.is_some());

    let children_map: HashMap<_, _> = with_parent
        .iter()
        .filter_map(|entry| entry.parent_workspace.as_ref()
            .map(|parent| (parent.clone(), entry.workspace.clone())))
        .into_group_map();
}
```

### 4. Updated Module Exports
**File**: `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/mod.rs`

Added `pub mod domain;` to export the new domain types module.

## Principles Applied

### 1. Make Illegal States Unrepresentable ✓
- Replaced boolean flags with `QueueAction` enum
- Domain types prevent invalid values from existing

### 2. Parse at Boundaries, Validate Once ✓
- `parse_queue_action()` validates all input at entry point
- Domain types are validated once on creation

### 3. Semantic Newtypes Instead of Primitives ✓
- `SessionName`, `BeadId`, `AgentId`, `WorkspaceName` replace `String`
- `QueueId`, `Priority` replace raw integers

### 4. Pure Functional Core, Side Effects at Boundaries ✓
- Core functions use iterators and combinators
- No mutation in business logic
- `try_fold()` instead of for loops

### 5. Railway-Oriented Programming with Result<T, E> ✓
- All validation returns `Result<T, DomainError>`
- Proper error propagation with `?` operator
- No unwrap or expect

### 6. Zero Panics, Zero Unwrap ✓
- Added strict lints to new and modified files
- All code compiles with:
  - `#![deny(clippy::unwrap_used)]`
  - `#![deny(clippy::expect_used)]`
  - `#![deny(clippy::panic)]`
  - `#![warn(clippy::pedantic)]`
  - `#![warn(clippy::nursery)]`
  - `#![forbid(unsafe_code)]`

## Testing

All new domain types include comprehensive unit tests:
```rust
#[test]
fn test_session_name_valid() {
    assert!(SessionName::new("valid-session".to_string()).is_ok());
}

#[test]
fn test_session_name_empty() {
    let result = SessionName::new("".to_string());
    assert!(result.is_err());
}
```

## Verification

```bash
# Format check - passed
cargo fmt -p zjj

# Clippy check - passed with no warnings for handlers
cargo clippy -p zjj --lib

# Compile check - handlers module compiles without errors
cargo check -p zjj --lib
```

## Benefits

1. **Type Safety**: Invalid values cannot exist at runtime
2. **Better Errors**: Clear error messages at parsing time
3. **Maintainability**: Single source of truth for validation
4. **Testability**: Pure functions are easy to test
5. **Performance**: Zero-cost abstractions, compile-time optimization
6. **Developer Experience**: IDE autocomplete shows valid states

## Next Steps

Future refactoring opportunities:
1. Apply domain types to other handlers (session.rs, workspace.rs, etc.)
2. Refactor command options to use domain types directly
3. Add more semantic types (e.g., `BranchName`, `CommitId`)
4. Consider using `rpds` for persistent data structures in tree building
5. Add property-based tests with proptest for domain types

## Conclusion

The refactoring successfully applies functional Rust and DDD principles to the CLI handlers module, making illegal states unrepresentable while maintaining compatibility with existing code. The handlers module now serves as a template for applying these principles to the rest of the codebase.
