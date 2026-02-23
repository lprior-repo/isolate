# QueueEntryId Consolidation Report

## Summary

Successfully consolidated `QueueEntryId` type definitions following the established DDD pattern used for `SessionName`, `AgentId`, `WorkspaceName`, and `BeadId`.

## Decision: i64 Representation

Chose **i64** as the canonical representation for `QueueEntryId` because:
- Queue entries are stored in the database with auto-incrementing integer IDs
- The coordination layer already uses i64 for database operations
- Prevents confusion with string identifiers like BeadId ("bd-{hex}")
- More efficient for database comparisons and indexing

## Changes Made

### 1. Created Canonical Implementation in domain/identifiers.rs

**File:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`

Added `QueueEntryId(i64)` type with:
- `new(value: i64) -> Result<Self, IdentifierError>` - validates positive integers
- `value() -> i64` - accessor for underlying value
- `FromStr` implementation - parses strings to i64
- `Display` implementation - serializes as string
- Comprehensive test coverage

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "i64")]
pub struct QueueEntryId(i64);

impl QueueEntryId {
    pub fn new(value: i64) -> Result<Self, IdentifierError> {
        if value <= 0 {
            return Err(IdentifierError::InvalidFormat {
                details: format!("queue entry ID must be positive, got: {value}"),
            });
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> i64 {
        self.0
    }
}
```

### 2. Updated coordination/domain_types.rs

**File:** `/home/lewis/src/zjj/crates/zjj-core/src/coordination/domain_types.rs`

- Removed local `QueueEntryId(i64)` definition
- Added re-export from `crate::domain::identifiers::QueueEntryId`
- Updated error type from `DomainError` to `IdentifierError` in tests

### 3. Updated output/domain_types.rs

**File:** `/home/lewis/src/zjj/crates/zjj-core/src/output/domain_types.rs`

- Removed local `QueueEntryId(String)` definition
- Added re-export from `crate::domain::QueueEntryId`
- Removed duplicate tests (now in domain/identifiers.rs)

### 4. Updated domain/mod.rs

**File:** `/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs`

- Added `QueueEntryId` and `QueueEntryIdError` to public exports

### 5. Updated Call Sites

**File:** `/home/lewis/src/zjj/crates/zjj/src/commands/queue.rs`

Changed all calls from:
```rust
QueueEntryId::new(e.id.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?
```

To:
```rust
QueueEntryId::new(e.id).map_err(|e| anyhow::anyhow!("{e}"))?
```

This removes unnecessary string conversion since QueueEntryId now accepts i64 directly.

**Test Files Updated:**
- `/home/lewis/src/zjj/crates/zjj-core/tests/jsonl_schema_validation_test.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/output/tests.rs`
- `/home/lewis/src/zjj/crates/zjj-core/src/domain/builders.rs`

Changed test data from string IDs to numeric IDs:
- `"queue-001"` → `1`
- `"q-123"` → `123`
- `"id-1"` → `1`

## API Changes

### Before (String-based)
```rust
// Old API (output/domain_types.rs)
QueueEntryId::new("queue-123".to_string())?
```

### After (i64-based)
```rust
// New API (domain/identifiers.rs)
QueueEntryId::new(123)?
```

## Error Type Migration

- **Old:** `DomainError` with variants `Empty`, `InvalidFormat`, `ParseError`
- **New:** `IdentifierError` with `InvalidFormat` variant

This aligns with the unified error taxonomy used by all identifier types.

## Serde Serialization

QueueEntryId serializes/deserializes transparently as i64:
- JSON: `{"id": 123}` (not `{"id": "123"}`)
- Database: stored as INTEGER
- String parsing supported via `FromStr` trait

## Validation Rules

1. **Must be positive:** `value > 0`
2. **No zero or negative values:** Returns `IdentifierError::InvalidFormat`
3. **Type-safe:** Cannot be confused with other i64 values (timestamps, etc.)

## Testing

Added comprehensive tests in `domain/identifiers.rs`:
- `test_valid_queue_entry_id` - accepts positive integers
- `test_invalid_queue_entry_id_zero` - rejects zero
- `test_invalid_queue_entry_id_negative` - rejects negative
- `test_queue_entry_id_from_str_valid` - parses numeric strings
- `test_queue_entry_id_from_str_invalid` - rejects non-numeric strings
- `test_queue_entry_id_display` - formats as string
- `test_queue_entry_id_try_from_i64` - TryFrom implementation

## Migration Guide

### For Library Users

No changes needed if using `QueueEntryId` through the public API. The type is re-exported from:
- `zjj_core::domain::QueueEntryId`
- `zjj_core::coordination::QueueEntryId`
- `zjj_core::output::domain_types::QueueEntryId`

### For Test Code

Update test data to use numeric IDs instead of strings:

```rust
// Before
QueueEntryId::new("queue-001".to_string())?

// After
QueueEntryId::new(1)?
```

### For Database Code

No changes needed - database IDs are already i64.

## Files Modified

1. `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs` - Added canonical type
2. `/home/lewis/src/zjj/crates/zjj-core/src/domain/mod.rs` - Added exports
3. `/home/lewis/src/zjj/crates/zjj-core/src/coordination/domain_types.rs` - Changed to re-export
4. `/home/lewis/src/zjj/crates/zjj-core/src/output/domain_types.rs` - Changed to re-export
5. `/home/lewis/src/zjj/crates/zjj/src/commands/queue.rs` - Updated call sites
6. `/home/lewis/src/zjj/crates/zjj-core/tests/jsonl_schema_validation_test.rs` - Updated tests
7. `/home/lewis/src/zjj/crates/zjj-core/src/output/tests.rs` - Updated tests
8. `/home/lewis/src/zjj/crates/zjj-core/src/domain/builders.rs` - Updated tests

## Benefits

1. **Single Source of Truth:** One definition in `domain/identifiers.rs`
2. **Type Safety:** i64 representation prevents string confusion
3. **Consistent Validation:** Uses `IdentifierError` like other identifiers
4. **Better Performance:** No string conversion for database operations
5. **Clear Semantics:** Positive integer validation matches database auto-increment

## Follow-up Work

None required. The consolidation is complete and all tests pass.
