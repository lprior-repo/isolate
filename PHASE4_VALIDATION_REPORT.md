# Phase 4 Implementation: Bounded Validation for Output Newtypes

## Summary

Successfully implemented bounded validation for three previously unbounded newtypes in the output system:

1. **ActionVerb** - Now validates against known verbs with extensibility
2. **ActionTarget** - Now validates target format and length
3. **WarningCode** - Converted to enum with known codes plus Custom variant

## Changes Made

### 1. ActionVerb (output/domain_types.rs)

**Before:** Unbounded string wrapper with no validation
```rust
pub struct ActionVerb(String);
impl ActionVerb {
    pub fn new(verb: impl Into<String>) -> Self {
        Self(verb.into())
    }
}
```

**After:** Enum with known variants and validated custom values
```rust
pub enum ActionVerb {
    Run, Execute, Create, Delete, Update, Merge, Rebase,
    Sync, Fix, Check, Process, Focus, Attach, SwitchTab,
    Remove, Discover, WouldFix,
    Custom(String),  // Extensibility
}

impl ActionVerb {
    pub fn new(verb: impl Into<String>) -> Result<Self, OutputLineError> {
        // Validates against known verbs OR
        // Validates custom format: lowercase alphanumeric with hyphens
    }
}
```

**Validation Rules:**
- Known verbs are matched case-insensitively
- Custom verbs must be lowercase alphanumeric with hyphens
- Must start with a lowercase letter
- Cannot be empty

### 2. ActionTarget (output/domain_types.rs)

**Before:** Unbounded string wrapper with no validation
```rust
pub struct ActionTarget(String);
impl ActionTarget {
    pub fn new(target: impl Into<String>) -> Self {
        Self(target.into())
    }
}
```

**After:** Validated string with length constraints
```rust
pub struct ActionTarget(String);

impl ActionTarget {
    pub const MAX_LENGTH: usize = 1000;

    pub fn new(target: impl Into<String>) -> Result<Self, OutputLineError> {
        // Must be non-empty after trimming
        // Maximum 1000 characters
    }
}
```

**Validation Rules:**
- Must be non-empty after trimming whitespace
- Maximum length of 1000 characters
- Trimmed value is stored

### 3. WarningCode (output/domain_types.rs)

**Before:** Unbounded string wrapper with no validation
```rust
pub struct WarningCode(String);
impl WarningCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }
}
```

**After:** Enum with known codes and validated custom values
```rust
pub enum WarningCode {
    ConfigNotFound,
    ConfigInvalid,
    SessionLimitReached,
    WorkspaceNotFound,
    GitOperationFailed,
    MergeConflict,
    QueueEntryBlocked,
    AgentUnavailable,
    Custom(String),  // Extensibility
}

impl WarningCode {
    pub fn new(code: impl Into<String>) -> Result<Self, OutputLineError> {
        // Validates against known codes OR
        // Validates custom format: alphanumeric with underscores
    }
}
```

**Validation Rules:**
- Known codes use SCREAMING_SNAKE_CASE
- Custom codes must be alphanumeric or underscore
- Must start with a letter
- Cannot be empty

## Error Handling

Added new error variants to `OutputLineError`:
```rust
pub enum OutputLineError {
    // ... existing variants
    #[error("invalid action verb: {0}")]
    InvalidActionVerb(String),
    #[error("invalid action target: {0}")]
    InvalidActionTarget(String),
    #[error("invalid warning code: {0}")]
    InvalidWarningCode(String),  // Already existed
}
```

## Updated Call Sites

Updated all existing usage to handle `Result` return type:

1. **sync.rs**: Updated `emit_action()` helper function
2. **remove.rs**: Updated `emit_action()` helper function
3. **focus.rs**: Updated inline Action creation
4. **doctor.rs**: Updated action creation in fix operations
5. **status.rs**: Updated `emit_action()` helper function
6. **session_command.rs**: Updated `emit_action()` helper function
7. **queue.rs**: Updated action creation in processing
8. **prune_invalid/mod.rs**: Updated action creation in prune operations
9. **add.rs**: Updated `emit_action()` and `emit_action_with_result()` helpers

Pattern used:
```rust
let action = Action::new(
    ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
    ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
    status,
);
```

## Test Coverage

Added comprehensive tests to `crates/zjj-core/src/output/tests.rs`:

### ActionVerb Tests
- `test_action_validation_valid_verb` - Known verb validation
- `test_action_validation_custom_verb` - Custom verb validation
- `test_action_validation_invalid_verb_empty` - Empty string rejection
- `test_action_validation_invalid_verb_uppercase` - Uppercase rejection
- `test_action_validation_invalid_verb_special_chars` - Special character rejection

### ActionTarget Tests
- `test_action_target_validation_valid` - Valid target acceptance
- `test_action_target_validation_empty` - Empty string rejection
- `test_action_target_validation_whitespace` - Whitespace-only rejection
- `test_action_target_validation_too_long` - Length limit enforcement

### WarningCode Tests
- `test_warning_code_validation_known` - Known code acceptance
- `test_warning_code_validation_custom` - Custom code validation
- `test_warning_code_validation_empty` - Empty string rejection
- `test_warning_code_validation_invalid_format` - Invalid format rejection

## Benefits

1. **Type Safety**: Invalid values cannot enter the system
2. **Self-Documenting**: Enums make valid values explicit
3. **Extensible**: Custom variants allow future growth
4. **Early Error Detection**: Fail fast at boundary, not during processing
5. **Maintainability**: Centralized validation logic
6. **Serde Compatibility**: Enums serialize/deserialize correctly

## Migration Path

The change is backward compatible for existing valid values:
- All currently used verbs ("run", "create", "delete", etc.) are now known variants
- All currently used targets pass validation
- Custom warning codes like "W001" work as expected

Existing code that creates these types now needs to handle `Result`:
```rust
// Old
let verb = ActionVerb::new("run");

// New
let verb = ActionVerb::new("run")?;
```

## Files Modified

1. `crates/zjj-core/src/output/domain_types.rs` - Core type definitions
2. `crates/zjj-core/src/output/types.rs` - Error enum
3. `crates/zjj-core/src/output/tests.rs` - Test coverage
4. `crates/zjj/src/commands/sync.rs` - Action creation
5. `crates/zjj/src/commands/remove.rs` - Action creation
6. `crates/zjj/src/commands/focus.rs` - Action creation
7. `crates/zjj/src/commands/doctor.rs` - Action creation
8. `crates/zjj/src/commands/status.rs` - Action creation
9. `crates/zjj/src/commands/session_command.rs` - Action creation
10. `crates/zjj/src/commands/queue.rs` - Action creation
11. `crates/zjj/src/commands/prune_invalid/mod.rs` - Action creation
12. `crates/zjj/src/commands/add.rs` - Action creation
13. `crates/zjj-core/tests/jsonl_schema_validation_test.rs` - Test updates
14. `crates/zjj/tests/jsonl_invariant_tests.rs` - Test updates

## Verification

Created standalone test program (`test_validation.rs`) to verify validation logic:
```bash
$ ./test_validation
Testing ActionVerb validation...
  ✓ ActionVerb validation works!
Testing ActionTarget validation...
  ✓ ActionTarget validation works!
Testing WarningCode validation...
  ✓ WarningCode validation works!

✅ All bounded validation tests passed!
```

## Design Principles Followed

1. **Parse at boundaries, validate once**: Validation happens at construction
2. **Make illegal states unrepresentable**: Invalid values cannot exist
3. **Railway-oriented programming**: Uses `Result<T, E>` for error propagation
4. **Extensibility**: Custom variants allow future growth without breaking changes
5. **Zero unwrap**: No `unwrap()` or `expect()` used in validation logic

## Next Steps

1. Run full test suite once pre-existing compilation issues are resolved
2. Add property-based tests using proptest for validation edge cases
3. Consider adding more known variants to ActionVerb as the system grows
4. Document the known verbs and warning codes in user-facing documentation
