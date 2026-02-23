# SessionName Migration - Execution Plan

## Critical Discovery

There are **TWO different `SessionName` implementations** with incompatible validation:

### Domain Module (`domain/identifiers.rs`)
- **MAX_LENGTH = 63** ⚠️
- Constructor: `parse(s)`
- Error type: `IdError`
- Used by: Output types (Stack, QueueEntry, Train, etc.)

### Types Module (`types.rs`)
- **MAX_LENGTH = 64** ⚠️
- Constructor: `new(s)`
- Error type: `Error::ValidationError`
- Used by: Tests, some handlers

### The Incompatibility

```rust
// Domain version (MAX_LENGTH = 63)
let name = domain::SessionName::parse("a".repeat(64)).unwrap_err(); // FAILS

// Types version (MAX_LENGTH = 64)
let name = types::SessionName::new("a".repeat(64)).unwrap(); // SUCCEEDS
```

This means:
1. A session name valid in one module is invalid in the other
2. Type confusion bugs are possible
3. JSON deserialization may fail depending on which module reads it

## Migration Strategy

### Phase 1: Unify MAX_LENGTH (IMMEDIATE)

**Decision**: Standardize on MAX_LENGTH = 63 (RFC 1035 style)

**Rationale**:
- 63 is the standard DNS label length limit
- More conservative (safer boundary)
- Domain module is already the canonical source for output types
- Easier to relax constraints later than tighten them

**Action**: Update domain module to use 63 (already done), document decision

### Phase 2: Re-export from types module (PRIMARY FIX)

Replace duplicate implementation in `types.rs` with re-export + compatibility shim:

```rust
// types.rs

// Re-export from domain module (single source of truth)
pub use crate::domain::SessionName;

// Backward compatibility: provide new() as alias for parse()
impl SessionName {
    /// Create a SessionName (backward compatibility alias)
    ///
    /// # Note
    ///
    /// This is a compatibility alias for `SessionName::parse()`.
    /// New code should use `parse()` for consistency with domain types.
    #[inline]
    pub fn new(name: impl Into<String>) -> Result<Self> {
        Self::parse(name).map_err(|e| Error::ValidationError {
            message: e.to_string(),
            field: Some("name".to_string()),
            value: None,
            constraints: vec![],
        })
    }
}
```

**Benefits**:
- Single type (no more type confusion)
- Backward compatible (existing `new()` calls work)
- Clear path to migrate to `parse()`
- Consistent validation everywhere

### Phase 3: Update Tests

Fix test expectations to use MAX_LENGTH = 63:

```rust
// types_tests.rs

// OLD (incorrect):
// let long_name = "a".repeat(65); // MAX_LENGTH is 64

// NEW (correct):
let long_name = "a".repeat(64); // MAX_LENGTH is 63
assert!(result.is_err());

#[test]
fn given_max_length_constant_then_is_63() {
    assert_eq!(SessionName::MAX_LENGTH, 63);
}
```

### Phase 4: Update Domain Types Tests

Fix tests in `output/domain_types.rs` that use `new()` instead of `parse()`:

```rust
// OLD:
let id = BeadId::new("bead-abc").expect("valid id");

// NEW:
let id = BeadId::parse("bead-abc").expect("valid id");
```

### Phase 5: Audit and Update Handlers

Search for all uses of `types::SessionName` and ensure they work with re-export:

```bash
grep -r "use.*types::SessionName" crates/
grep -r "SessionName::new" crates/
```

## Implementation Steps

1. **Replace duplicate in types.rs** (10 minutes)
   - Replace lines 24-195 with re-export + shim
   - Verify compiles

2. **Fix domain_types tests** (5 minutes)
   - Change `BeadId::new()` to `BeadId::parse()`
   - Verify compiles

3. **Fix types_tests expectations** (5 minutes)
   - Update MAX_LENGTH constant test to 63
   - Update length tests to use 64 (should fail)
   - Verify all tests pass

4. **Run full test suite** (5 minutes)
   ```bash
   cargo test --lib
   cargo test --all-targets
   ```

5. **Verify no regressions** (10 minutes)
   - Check handlers compile
   - Check output types serialize correctly
   - Verify JSON round-trips work

## Risk Mitigation

### Risk: Breaking existing session names with 64 characters

**Mitigation**:
- Audit existing data for 64-character names
- If found, document migration path
- Consider grace period with warning

### Risk: Tests fail due to MAX_LENGTH change

**Mitigation**:
- Update test expectations explicitly
- Add comment explaining why 63 is correct
- Document in migration guide

### Risk: Handler code expects `new()` not `parse()`

**Mitigation**:
- Compatibility shim provides `new()` method
- Gradual migration path via deprecation warnings
- Clear documentation

## Success Criteria

1. ✅ Only ONE `SessionName` type exists (defined in domain, re-exported elsewhere)
2. ✅ All tests pass with MAX_LENGTH = 63
3. ✅ No type confusion between domain and types versions
4. ✅ Backward compatibility maintained via `new()` shim
5. ✅ Clear documentation of canonical source
6. ✅ JSON serialization works consistently

## Files to Modify

1. `/home/lewis/src/zjj/crates/zjj-core/src/types.rs` - Replace with re-export
2. `/home/lewis/src/zjj/crates/zjj-core/src/types_tests.rs` - Update MAX_LENGTH test
3. `/home/lewis/src/zjj/crates/zjj-core/src/output/domain_types.rs` - Fix BeadId::new() -> parse()
4. Documentation files to explain the change

## Estimated Time

- Total: 30-40 minutes
- Risk: Low (backward compatible)
- Impact: High (eliminates type confusion)
