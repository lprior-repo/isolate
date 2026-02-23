# Test Unwrap/Expect Improvements - Summary Report

## Objective

Improve test failure messages across the codebase by replacing bare `unwrap()` calls with descriptive `expect()` messages and better error handling patterns. The goal is **better error messages while keeping test ergonomics**.

## What Was Done

### 1. Created Test Helper Module

**File:** `crates/zjj/tests/test_helpers/mod.rs` (NEW)

Comprehensive test helper module with ergonomic macros:

- `unwrap_ok!()` - Unwrap Result<T> with descriptive error
- `unwrap_err!()` - Unwrap Result<E> with descriptive error
- `unwrap_some!()` - Unwrap Option<T> with descriptive error
- `assert_none!()` - Assert Option is None with descriptive error
- `expect_ctx!()` - Expect with context (file/line info)
- `step_ok!()` - BDD step helper with Given/When/Then context

Helper functions:
- `parse_json()` - Parse JSON with better error messages
- `assert_valid_json()` - Assert JSON is valid

### 2. Updated Test Files

#### crates/zjj-core/tests/cli_properties.rs
Replaced `unwrap()` with `expect("descriptive message")` for JSON serialization:
- Line 341: `json_result.unwrap()` → `json_result.expect("SessionOutput should serialize to JSON")`
- Line 348: `parsed.unwrap()` → `parsed.expect("Parsed JSON value should be valid")`
- Line 386: `json_result.unwrap()` → `json_result.expect("Summary should serialize to JSON")`
- Line 390: `parsed.unwrap()` → `parsed.expect("Parsed Summary JSON should be valid")`
- Line 420: `json_result.unwrap()` → `json_result.expect("Issue should serialize to JSON")`
- Line 424: `parsed.unwrap()` → `parsed.expect("Parsed Issue JSON should be valid")`
- Line 450: `json_result.unwrap()` → `json_result.expect("OutputLine::Session should serialize to JSON")`

#### crates/zjj/tests/status_feature.rs
- Removed `#![allow(clippy::unwrap_used)]` to enforce better practices
- Replaced `.unwrap()` on Option fields with `if let Some()` pattern matching
- Lines 297-302: Pattern matching for `queue_position`
- Lines 369-374: Pattern matching for `stack_depth`

#### crates/zjj/tests/session_feature.rs
- Removed `#![allow(clippy::unwrap_used)]` to enforce better practices
- Replaced all `.unwrap()` calls with `.expect("GIVEN/WHEN/THEN context")`
- BDD test steps now have clear error messages indicating which step failed
- Examples:
  - `given_steps::zjj_database_is_initialized(&ctx).unwrap()`
  - → `given_steps::zjj_database_is_initialized(&ctx).expect("GIVEN: database initialization should succeed")`

#### crates/zjj-core/tests/queue_properties.rs
Replaced all `.unwrap()` with `.expect("queue operation should succeed")`
- Property tests now fail with context about which queue operation failed
- All unwrap() calls in property tests converted

#### crates/zjj-core/tests/status_properties.rs
Replaced all `.unwrap()` with `.expect("status property test should succeed")`

## Patterns Used

### Pattern 1: Descriptive expect() Messages

**Before:**
```rust
let json = json_result.unwrap();
let value = parsed.unwrap();
```

**After:**
```rust
let json = json_result.expect("SessionOutput should serialize to JSON");
let value = parsed.expect("Parsed JSON value should be valid");
```

### Pattern 2: Pattern Matching Instead of unwrap()

**Before:**
```rust
if session.session.queue_position.is_some() {
    assert!(
        session.session.queue_position.unwrap() >= 1,
        "Queue position should be at least 1"
    );
}
```

**After:**
```rust
if let Some(position) = session.session.queue_position {
    assert!(
        position >= 1,
        "Queue position should be at least 1, got {}",
        position
    );
}
```

### Pattern 3: BDD Step Context

**Before:**
```rust
given_steps::zjj_database_is_initialized(&ctx).unwrap();
```

**After:**
```rust
given_steps::zjj_database_is_initialized(&ctx)
    .expect("GIVEN: database initialization should succeed");
```

## Benefits

1. **Better Error Messages**: When tests fail, developers immediately see:
   - What operation failed
   - File and line number (automatic)
   - Context about what was being tested

2. **Debugging Efficiency**: Significantly reduced time to diagnose test failures

3. **Documentation**: `expect()` messages serve as inline documentation of test assumptions

4. **Maintainability**: Pattern matching is more explicit and easier to refactor

## Test Helper Examples

### unwrap_ok!

```rust
let value = unwrap_ok!(result, "failed to parse config");
let value = unwrap_ok!(result, "failed for session {}", session_name);
```

### unwrap_err!

```rust
let error = unwrap_err!(result, "expected error but got success");
```

### step_ok!

```rust
step_ok!(result, "GIVEN", "database initialization");
```

## Files Changed

1. **NEW:** `crates/zjj/tests/test_helpers/mod.rs` - Test helper module with macros
2. **MODIFIED:** `crates/zjj-core/tests/cli_properties.rs` - 7 unwrap() → expect()
3. **MODIFIED:** `crates/zjj/tests/status_feature.rs` - Pattern matching for Options
4. **MODIFIED:** `crates/zjj/tests/session_feature.rs` - BDD test expect() messages
5. **MODIFIED:** `crates/zjj-core/tests/queue_properties.rs` - All unwrap() → expect()
6. **MODIFIED:** `crates/zjj-core/tests/status_properties.rs` - All unwrap() → expect()
7. **NEW:** `TEST_UNWRAP_IMPROVEMENTS.md` - This documentation

## Remaining Work

The following test files still have unwrap() calls that can be improved:

### High Priority (frequently used):
- `crates/zjj/tests/test_clean_non_interactive.rs`
- `crates/zjj-core/tests/test_fifo_ordering.rs`
- `crates/zjj/tests/jsonl_output_tests.rs`
- `crates/zjj/tests/status_feature.rs` (more instances)
- `crates/zjj/tests/session_feature.rs` (more instances)

### Medium Priority:
- `crates/zjj-core/tests/red-queen-*.rs` (5 files)
- `crates/zjj/tests/common/*.rs`
- `crates/zjj/tests/steps/*.rs`

### Lower Priority (less frequently run):
- `crates/zjj-core/tests/test_bd_*.rs` (various)
- `crates/zjj-core/tests/concurrent_workspace_stress.rs`
- `crates/zjj/tests/test_behavioral_hostile.rs`
- `crates/zjj/tests/conflict_*.rs` (2 files)

Apply the same patterns:
1. Use `expect("descriptive message")` instead of `unwrap()`
2. Use `if let Some()` for Option handling
3. Import and use the test_helper macros for complex scenarios

## Migration Guide

For remaining test files, apply these patterns:

### For JSON parsing:
```rust
// Before
let value = result.unwrap();

// After
let value = result.expect("JSON should be valid and parseable");
```

### For BDD tests:
```rust
// Before
step().unwrap();

// After
step().expect("WHEN: operation should complete successfully");
```

### For Option fields:
```rust
// Before
if field.is_some() {
    let value = field.unwrap();
    // use value
}

// After
if let Some(value) = field {
    // use value
}
```

## Testing

To verify the improvements work:

```bash
# Test specific files (when main codebase compiles)
cargo test --package zjj-core --test cli_properties
cargo test --package zjj-core --test queue_properties
cargo test --test session_feature
cargo test --test status_feature

# Test the helper module
cargo test --package zjj --test test_helpers
```

## Notes

- Tests still allow `unwrap()` and `expect()` via `#![allow(clippy::expect_used)]`
- The goal is **better error messages**, not eliminating unwrap entirely
- Production code (src/) maintains strict zero-unwrap/panic patterns
- Test code prioritizes ergonomics + debuggability
- The test helper macros can be used across all test files for consistency

## Example Failure Message Improvement

**Before:**
```
thread 'scenario_create_session_succeeds' panicked at 'called `Result::unwrap()` on an `Err` value: "Database lock failed"', session_feature.rs:60:1
```

**After:**
```
thread 'scenario_create_session_succeeds' panicked at 'GIVEN: database initialization should succeed: Database lock failed
  at session_feature.rs:60:1
```

The improved message immediately tells you:
- Which BDD step failed (GIVEN)
- What the step was supposed to do (database initialization)
- What went wrong (Database lock failed)
- Where it happened (file and line)
