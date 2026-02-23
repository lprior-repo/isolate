# Coordination Module DDD Refactor Summary

## Overview
Applied Scott Wlaschin's Domain-Driven Design refactoring principles to the coordination module in `/home/lewis/src/zjj/crates/zjj-core/src/coordination/`.

## Changes Made

### 1. Created Domain Primitive Types (`domain_types.rs`)

**Problem**: Primitive obsession - using raw `String`, `i64`, `str` for domain concepts throughout the codebase.

**Solution**: Created semantic newtypes with validation:
- `QueueEntryId` - wraps `i64` for queue entry IDs, ensures positive values
- `WorkspaceName` - wraps `String` for workspace identifiers, ensures non-empty
- `AgentId` - wraps `String` for agent identifiers, ensures non-empty
- `BeadId` - wraps `String` for bead identifiers, ensures non-empty
- `DedupeKey` - wraps `String` for deduplication keys, ensures non-empty
- `Priority` - wraps `i32` for queue priority, provides `high()`, `low()`, `default()` constants

**Benefits**:
- Type safety - cannot accidentally pass an `AgentId` where a `WorkspaceName` is expected
- Validation at boundaries - empty strings rejected at construction time
- Self-documenting code - `WorkspaceName` is more descriptive than `String`
- Zero runtime overhead - all newtypes use `#[repr(transparent)]` via serde

### 2. Refactored Pure Queue to be Truly Functional (`pure_queue.rs`)

**Problems**:
- Used `mut` variables and `get_mut()` despite being labeled "pure"
- Used `rpds` which has awkward API for persistent data structures
- Had mutation in functions that should return new instances

**Solutions**:
- Switched from `rpds` to `im` crate for persistent collections
  - `im::Vector` for entries - O(log32 n) updates with structural sharing
  - `im::HashMap` for workspace index - immutable hash map
  - `im::HashSet` for consistency checking - immutable hash set
- Removed all `mut` variables where possible
- All operations now return new `PureQueue` instances
- Added helper trait `WithDedupeOpt` to avoid mutation in builder pattern

**Key Improvements**:
```rust
// Before: mutation in "pure" function
let mut new_queue = self.clone();
new_queue.entries.push_back(entry);
new_queue.lock_holder = Some(agent_id.to_string());

// After: purely functional
let new_entries = self.entries.push_back(entry);
Ok(Self {
    entries: new_entries,
    lock_holder: Some(agent_id.to_string()),
    // ... other fields cloned or updated functionally
})
```

### 3. Enhanced Error Types

**Improvements**:
- Added `LockHeldByOther { holder, requester }` variant to `PureQueueError`
- All errors use `thiserror` for consistent display formatting
- Error variants are exhaustive and type-safe

### 4. Zero Unwrap/Panic Enforcement

All refactored files include:
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

### 5. Updated Module Exports

Modified `coordination/mod.rs` to export:
- `domain_types` module
- All domain primitive types: `AgentId`, `BeadId`, `DedupeKey`, `DomainError`, `Priority`, `QueueEntryId`, `WorkspaceName`

## Remaining Work

### High Priority
1. **Replace `unwrap()`/`expect()` in test code** - Found in:
   - `queue_submission.rs` (lines 892, 899, 926, 927)
   - `queue.rs` (multiple panic assertions in tests)
   - `worker_lifecycle.rs` (multiple `.expect()` calls)
   - `conflict_resolutions.rs` (line 618)
   - `locks.rs` (line 923)
   - `queue_status.rs` (line 768)

2. **Use domain types throughout the codebase**:
   - Replace `String` workspace identifiers with `WorkspaceName`
   - Replace `i64` queue IDs with `QueueEntryId`
   - Replace `String` agent IDs with `AgentId`

3. **Refactor queue.rs to use pure core**:
   - Separate domain logic from infrastructure layer
   - Use `PureQueue` for in-memory operations
   - DB operations should be thin adapters around pure functions

### Medium Priority
4. **Fix `im` vs `rpds` inconsistency** - Currently using `im` in pure_queue but `rpds` is still in dependencies
   - Choose one and standardize, or add rationale for using different crates

5. **Add property-based tests** using the new pure queue:
   - Test queue invariants under random operations
   - Verify consistency properties hold

6. **State machine refactoring**:
   - Make illegal states unrepresentable using enums instead of Option combinations
   - Example: `ClaimState` enum with variants `Unclaimed` and `Claimed { agent: AgentId }`

### Low Priority
7. **Documentation**:
   - Add module-level documentation explaining the pure/impure split
   - Document when to use domain types vs primitives

8. **Performance review**:
   - Benchmark pure queue operations
   - Consider lazy evaluation for expensive operations

## Files Modified

1. `/home/lewis/src/zjj/crates/zjj-core/src/coordination/domain_types.rs` (NEW)
2. `/home/lewis/src/zjj/crates/zjj-core/src/coordination/pure_queue.rs` (REFACTORED)
3. `/home/lewis/src/zjj/crates/zjj-core/src/coordination/mod.rs` (UPDATED)

## Principles Applied

1. ✅ **Make illegal states unrepresentable** - Domain types prevent empty strings, negative IDs
2. ✅ **Parse at boundaries, validate once** - Validation in type constructors
3. ✅ **Use semantic newtypes** - Created 6 domain primitive types
4. ✅ **Pure functional core** - Removed mutation from `PureQueue`
5. ✅ **Railway-oriented programming** - All operations return `Result<T, E>`
6. ✅ **Zero panics, zero unwrap** - Enforced with lints in refactored code

## Testing Status

- ✅ Code compiles with `cargo check -p zjj-core`
- ⏳ Full test suite blocked by pre-existing compilation errors in `output/types.rs`
- ✅ Pure queue module includes comprehensive unit tests

## Next Steps

1. Fix pre-existing compilation errors in output/types.rs
2. Replace unwrap/panic in test code with Result-based assertions
3. Gradually migrate queue.rs to use new domain types
4. Add property-based tests for queue invariants
