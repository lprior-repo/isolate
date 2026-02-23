# SessionName Migration Analysis and Completion Plan

## Current State

The codebase has **TWO different implementations** of `SessionName` with slightly different validation rules:

### 1. Domain Layer (Canonical Implementation)
**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`

```rust
pub const MAX_LENGTH: usize = 63;  // Different!
pub fn parse(s: impl Into<String>) -> Result<Self, IdError>  // Different method name!
```

- **Validation Rules**:
  - Must start with a letter
  - Can contain letters, numbers, hyphens, underscores
  - Max 63 characters (not 64!)
  - Uses `parse()` for construction
  - Uses `IdError` for validation errors

### 2. Types Module (Legacy Implementation)
**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/types.rs`

```rust
pub const MAX_LENGTH: usize = 64;  // Different!
pub fn new(name: impl Into<String>) -> Result<Self>  // Different method name!
```

- **Validation Rules**:
  - Must start with a letter
  - Can contain letters, numbers, hyphens, underscores
  - Max 64 characters (not 63!)
  - Uses `new()` for construction
  - Uses `Error::ValidationError` for errors
  - Has `HasContract` trait implementation

### 3. Output Module (Correctly Re-exporting)
**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/output/domain_types.rs`

```rust
// Line 20: Correctly re-exports from domain
pub use crate::domain::SessionName;
```

### 4. Output Types (Using Domain Version)
**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/output/types.rs`

```rust
// Line 33: Uses domain_types which re-exports domain::SessionName
use super::domain_types::{..., SessionName};
```

## The Problem

1. **Two implementations with different rules**:
   - Domain: MAX_LENGTH = 63
   - Types: MAX_LENGTH = 64
   - This creates a potential for confusion and bugs

2. **Different constructor methods**:
   - Domain: `SessionName::parse()`
   - Types: `SessionName::new()`

3. **Different error types**:
   - Domain: Returns `IdError`
   - Types: Returns `Error::ValidationError`

4. **Tests using the wrong version**:
   - `types_tests.rs` uses `types::SessionName` which has MAX_LENGTH = 64
   - These tests are testing the wrong implementation

## Migration Status

### Already Complete
- Output types are correctly using `domain::SessionName` via re-export
- Stack, QueueEntry, Train all use the domain version

### Needs Migration
- Types module should re-export instead of defining its own
- Tests in `types_tests.rs` need to use domain version or be updated
- Any code using `types::SessionName::new()` needs to use `domain::SessionName::parse()`

## Recommended Solution

### Option 1: Re-export from types module (RECOMMENDED)

Replace the duplicate implementation in `types.rs` with a re-export:

```rust
// ═══════════════════════════════════════════════════════════════════════════
// SESSION NAME VALUE OBJECT
// ═══════════════════════════════════════════════════════════════════════════

// Re-export from domain module (single source of truth)
//
// The domain::SessionName is the canonical implementation.
// This re-export provides backward compatibility for code using `types::SessionName`.
pub use crate::domain::SessionName;

// Backward compatibility: provide a `new()` method that delegates to `parse()
//
// Note: This is a temporary compatibility shim. Code should migrate to using `parse()` directly.
impl SessionName {
    /// Create a new SessionName (backward compatibility shim)
    ///
    /// # Deprecated
    ///
    /// Use `SessionName::parse()` instead for consistency with other domain types.
    /// This method exists for backward compatibility during migration.
    #[deprecated(since = "0.1.0", note = "Use `SessionName::parse()` instead")]
    pub fn new(name: impl Into<String>) -> crate::Result<Self> {
        let name = name.into();
        Self::parse(name).map_err(|e| crate::Error::ValidationError {
            message: e.to_string(),
            field: Some("name".to_string()),
            value: None,
            constraints: vec![],
        })
    }
}
```

### Benefits of This Approach

1. **Single source of truth**: Domain module owns the implementation
2. **Backward compatibility**: Existing code using `types::SessionName` continues to work
3. **Clear migration path**: Deprecation warning guides users to `parse()`
4. **Consistent validation**: All code uses the same MAX_LENGTH = 63
5. **Type compatibility**: All `SessionName` values are the same type

### Migration Steps

1. Replace duplicate implementation in `types.rs` with re-export
2. Add backward compatibility shim for `new()` method
3. Update tests to use domain version or accept the compatibility shim
4. Run full test suite to verify no regressions
5. Add deprecation warnings to guide future migrations

## Testing Strategy

After migration, verify:

1. All tests pass with the unified implementation
2. JSON deserialization works for both old and new formats
3. SessionName validation is consistent (MAX_LENGTH = 63)
4. No type confusion between different SessionName types

## Files to Modify

1. `/home/lewis/src/zjj/crates/zjj-core/src/types.rs` - Replace with re-export
2. `/home/lewis/src/zjj/crates/zjj-core/src/types_tests.rs` - Verify tests work
3. Any other files using `types::SessionName` - Verify compatibility

## Backward Compatibility

- The re-export ensures existing imports continue to work
- The `new()` shim provides compatibility for existing code
- Deprecation warnings guide gradual migration to `parse()`
