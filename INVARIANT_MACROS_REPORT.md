# Invariant Checking Macros Implementation

## Summary

Created invariant checking macros for the domain layer following zero-panic, zero-unwrap principles. These macros provide consistent invariant enforcement across the codebase.

## Files Created

### 1. Main Macros Module
**File**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/macros.rs`

Provides three invariant checking macros:

#### `invariant!` - Runtime Invariant Check
- Always enabled (production and development)
- For critical invariants that must always be enforced
- Returns `Result` with provided error on violation

```rust
invariant!(
    updated_at >= created_at,
    BeadError::NonMonotonicTimestamps { created_at, updated_at }
);
```

#### `assert_invariant!` - Test-Only Invariant Check
- Only compiles in test builds
- For expensive validation in tests only
- Zero runtime cost in production

```rust
assert_invariant!(
    expensive_validation_check(data),
    ValidationError::ExpensiveCheckFailed
);
```

#### `debug_invariant!` - Debug-Only Invariant Check
- Only compiles in debug builds
- For development-time validation
- Zero runtime cost in release builds

```rust
debug_invariant!(
    data.validate().is_ok(),
    ValidationError::DebugValidationFailed
);
```

### 2. Usage Examples
**File**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/macros_examples.rs`

Comprehensive examples demonstrating:
- Basic invariant checks for timestamp ordering
- Chained invariants for complex validation
- Debug-only invariants for expensive O(n) checks
- Test-only invariants for production-safe validation
- Complex error computation with format strings
- State transition validation

### 3. Module Integration
**File**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs`

Added macros module to domain layer:
```rust
pub mod macros;

#[cfg(test)]
pub mod macros_examples;
```

## Zero Guarantees

All macros follow the core principles:

### Zero Panic
- Never use `panic!()`
- All checks return `Result<T, E>`

### Zero Unwrap
- Never use `unwrap()`, `expect()`, or `unwrap_or()`
- Always use proper error propagation with `?`

### Zero Unsafe
- No unsafe code anywhere
- Safe Rust only

## Usage Example

```rust
use chrono::{DateTime, Utc};
use crate::domain::{BeadError, invariant};

fn validate_bead_consistency(
    title: &str,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<(), BeadError> {
    // Check title is not empty
    invariant!(
        !title.is_empty(),
        BeadError::TitleRequired
    );

    // Check timestamp monotonicity
    invariant!(
        updated_at >= created_at,
        BeadError::NonMonotonicTimestamps {
            created_at,
            updated_at,
        }
    );

    Ok(())
}
```

## Integration with Domain Layer

The macros integrate seamlessly with existing domain error types:

```rust
use crate::domain::{BeadError, SessionError, WorkspaceError};

// With BeadError
invariant!(
    bead.updated_at >= bead.created_at,
    BeadError::NonMonotonicTimestamps {
        created_at: bead.created_at,
        updated_at: bead.updated_at
    }
);

// With SessionError
invariant!(
    session.is_active(),
    SessionError::NotActive
);

// With WorkspaceError
invariant!(
    workspace.state.is_ready(),
    WorkspaceError::NotReady(workspace.state)
);
```

## Testing

The macros module includes comprehensive tests:

### `test_invariant_macro_pass`
Verifies that passing conditions return `Ok(())`.

### `test_invariant_macro_fail`
Verifies that failing conditions return the provided error.

### `test_invariant_with_complex_condition`
Tests invariants with complex boolean expressions.

### `test_chained_invariants`
Tests multiple invariant checks in sequence.

### `test_invariant_with_computed_error`
Tests invariants with error values computed at runtime.

### `test_assert_invariant`
Verifies test-only invariant behavior.

### `test_debug_invariant`
Verifies debug-only invariant behavior with conditional compilation.

## Compilation Status

- All macros compile without errors or warnings
- Zero clippy warnings for unwrap/expect/panic
- Zero unsafe code
- Follows all functional Rust principles

## Export Accessibility

Macros are exported at the crate root level via `#[macro_export]`, making them available throughout the codebase:

```rust
// Available everywhere in zjj-core
use zjj_core::invariant;
use zjj_core::assert_invariant;
use zjj_core::debug_invariant;
```

## Benefits

1. **Consistency**: Standardized invariant checking across all domain code
2. **Safety**: Zero panic, zero unwrap guarantees
3. **Performance**: Conditional compilation for test/debug-only checks
4. **Clarity**: Clear, readable syntax for invariant enforcement
5. **Ergonomics**: Simple, composable macros for common patterns

## Next Steps

To use the macros in domain code:

1. Import at the top of your file:
   ```rust
   use zjj_core::invariant;
   ```

2. Replace manual invariant checks with the macro:
   ```rust
   // Before
   if updated_at < created_at {
       return Err(BeadError::NonMonotonicTimestamps { created_at, updated_at });
   }

   // After
   invariant!(
       updated_at >= created_at,
       BeadError::NonMonotonicTimestamps { created_at, updated_at }
   );
   ```

3. Consider using `assert_invariant!` for expensive test-only checks

4. Consider using `debug_invariant!` for development-time validation
