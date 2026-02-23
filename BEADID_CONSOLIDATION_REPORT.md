# BeadId/TaskId Consolidation Report

## Executive Summary

Successfully consolidated all `BeadId` and `TaskId` types across the codebase into a **single canonical implementation** following Domain-Driven Design (DDD) principles.

**Decision:** `BeadId` is a **type alias** to `TaskId` - they represent the same domain concept.

## The Problem

Prior to this consolidation, the codebase had **5 different implementations** of `BeadId` with inconsistent validation:

| Module | Implementation | Validation |
|--------|---------------|------------|
| `domain/identifiers.rs` | `pub type BeadId = TaskId` | `bd-{hex}` format |
| `output/domain_types.rs` | Separate struct | Non-empty only |
| `coordination/domain_types.rs` | Separate struct | Non-empty only |
| `cli/handlers/domain.rs` | Separate struct | `bd-{alphanumeric}` |
| `commands/done/newtypes.rs` | Separate struct | Alphanumeric + dash/underscore |

This violated core DDD principles:
- **Single Source of Truth**: Multiple implementations created confusion
- **Don't Repeat Yourself**: Validation logic duplicated
- **Parse at Boundaries**: Inconsistent validation across boundaries

## The Solution

### Canonical Implementation

**Location:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`

```rust
/// A validated task ID (bead ID format)
///
/// Validates bd-{hex} format (e.g., bd-abc123def456)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TaskId(String);

/// A validated bead ID (same as task ID)
///
/// Alias for `TaskId` since beads and tasks use the same ID format.
pub type BeadId = TaskId;
```

### Validation Rules

The canonical implementation enforces:
- **Prefix:** Must start with `bd-`
- **Suffix:** Hexadecimal characters only (0-9, a-f, A-F)
- **Non-empty:** Cannot be empty

Examples:
- ✅ `bd-abc123` - Valid
- ✅ `bd-ABC123DEF456` - Valid (case-insensitive hex)
- ❌ `abc123` - Invalid (missing prefix)
- ❌ `bd-123-456` - Invalid (hyphen in hex part)
- ❌ `bd-xyz` - Invalid (non-hex characters)

### API Convention

All `BeadId`/`TaskId` creation uses the **`parse()`** method (not `new()`):

```rust
// Create a BeadId/TaskId
let bead_id = BeadId::parse("bd-abc123")?;

// Error handling
match BeadId::parse("invalid") {
    Ok(id) => println!("Valid: {}", id.as_str()),
    Err(IdError::InvalidTaskId(msg)) => eprintln!("Invalid: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Implementation Changes

### 1. Core Domain Layer (`zjj-core`)

#### `domain/identifiers.rs` (Canonical)
- **No changes** - already the single source of truth
- Exports both `TaskId` and `BeadId` (alias)

#### `output/domain_types.rs`
- **Removed:** Custom `BeadId` struct (lines ~148-186)
- **Added:** Re-export from domain layer
  ```rust
  pub use crate::domain::BeadId;
  ```

#### `coordination/domain_types.rs`
- **Removed:** Custom `BeadId` struct (lines ~168-215)
- **Added:** Re-export from domain layer
  ```rust
  pub use crate::domain::identifiers::BeadId;
  ```
- **Updated:** Tests removed (now use canonical tests)

### 2. CLI Layer (`zjj` crate)

#### `cli/handlers/domain.rs`
- **Removed:** Custom `BeadId` struct (lines ~167-245)
- **Added:** Re-export from `zjj_core::domain`
  ```rust
  pub use zjj_core::domain::BeadId;
  ```
- **Updated:** Tests to use `parse()` instead of `new()`

#### `cli/handlers/queue.rs`
- **Changed:** `BeadId::new(s.clone())` → `BeadId::parse(s.as_str())`

#### `cli/handlers/stack.rs`
- **Changed:** `BeadId::new(id)` → `BeadId::parse(id)`

#### `commands/done/newtypes.rs`
- **Removed:** Custom `BeadId` struct (lines ~145-198)
- **Added:** Re-export from `zjj_core::domain`
  ```rust
  pub use zjj_core::domain::BeadId;
  ```

#### `commands/done/mod.rs`
- **Changed:** `BeadId::new(bead_id.to_string())` → `BeadId::parse(bead_id)`

#### `commands/done/bead.rs`
- **Changed:** `BeadId::new(bead.id)` → `BeadId::parse(&bead.id)`

#### `commands/queue.rs`
- **Changed:** All `BeadId::new(bead)` → `BeadId::parse(&bead)`

### 3. Test Updates

#### `output/tests.rs`
- **Changed:** All `BeadId::new("bd-xxx")` → `BeadId::parse("bd-xxx")`

#### `tests/jsonl_schema_validation_test.rs`
- **Changed:** `BeadId::new("bead-001")` → `BeadId::parse("bd-001")`
- **Note:** Also corrected test data to use valid `bd-` prefix format

## Migration Guide for Future Code

### When Creating BeadId/TaskId

```rust
use zjj_core::domain::BeadId;

// ✅ CORRECT - Use parse()
let id = BeadId::parse("bd-abc123")?;

// ❌ WRONG - Do not use new() (method doesn't exist)
let id = BeadId::new("bd-abc123".to_string())?;

// ❌ WRONG - Do not use raw strings in domain logic
fn process_bead(id: &str) { /* ... */ }
```

### When Accepting BeadId/TaskId as Parameters

```rust
// ✅ CORRECT - Accept validated type
fn process_bead(id: &BeadId) -> Result<(), Error> {
    let id_str = id.as_str();
    // ...
}

// ❌ WRONG - Accept raw string (parse at boundaries!)
fn process_bead(id: &str) -> Result<(), Error> {
    // Validation should happen earlier, at the boundary
}
```

### Error Handling

```rust
use zjj_core::domain::{BeadId, IdError};

fn parse_bead_id(input: &str) -> Result<BeadId, MyError> {
    BeadId::parse(input).map_err(|e| match e {
        IdError::InvalidTaskId(msg) => MyError::InvalidInput(msg),
        IdError::Empty => MyError::MissingInput,
        _ => MyError::Unknown,
    })
}
```

## Benefits

1. **Single Source of Truth**
   - One place to update validation logic
   - Consistent behavior across all modules

2. **Type Safety**
   - Cannot confuse BeadId with raw strings
   - Compile-time guarantees of validity

3. **Reduced Duplication**
   - Removed ~200 lines of duplicate code
   - 5 implementations → 1 canonical implementation

4. **Consistent Validation**
   - All BeadId values validated the same way
   - Catches invalid IDs at parse time

5. **Better Error Messages**
   - Centralized error types
   - Consistent error reporting

## Testing

All existing tests pass with the updated implementation:
- Unit tests in `domain/identifiers.rs`
- Integration tests updated to use `parse()` API
- Property tests verify validation rules

## Files Modified

### Core (`zjj-core`)
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs` (canonical - no changes)
- `/home/lewis/src/zjj/crates/zjj-core/src/output/domain_types.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/domain_types.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/output/tests.rs`
- `/home/lewis/src/zjj/crates/zjj-core/tests/jsonl_schema_validation_test.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/beads/types.rs` (fixed const error)

### CLI (`zjj`)
- `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/domain.rs`
- `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/queue.rs`
- `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/stack.rs`
- `/home/lewis/src/zjj/crates/zjj/src/commands/done/newtypes.rs`
- `/home/lewis/src/zjj/crates/zjj/src/commands/done/mod.rs`
- `/home/lewis/src/zjj/crates/zjj/src/commands/done/bead.rs`
- `/home/lewis/src/zjj/crates/zjj/src/commands/queue.rs`

## Verification

To verify the consolidation is working correctly:

```bash
# Build the project
cargo build

# Run tests
cargo test --lib

# Check for any remaining custom BeadId implementations
grep -r "pub struct BeadId" --exclude=target --exclude-dir=target

# Verify all usages use parse()
grep -r "BeadId::new" --exclude=target --exclude-dir=target
```

Expected results:
- ✅ Build succeeds
- ✅ All tests pass
- ✅ No custom `pub struct BeadId` found (except canonical)
- ✅ No `BeadId::new` usages (all converted to `parse()`)

## Conclusion

The consolidation successfully eliminates duplication and establishes a single source of truth for BeadId/TaskId types. This aligns with DDD principles and improves code maintainability, type safety, and consistency across the entire codebase.

**Key Takeaway:** BeadId and TaskId are the **same domain concept** and should be treated as such. The type alias (`pub type BeadId = TaskId`) makes this relationship explicit while maintaining semantic clarity.

---

Generated: 2025-02-23
Author: Claude (Functional Rust Expert)
