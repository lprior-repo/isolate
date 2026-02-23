# Fast In-Process Integration Tests

This document describes the new fast in-process test modules that avoid subprocess spawning for significantly faster test execution.

## Files Created

### 1. `/home/lewis/src/zjj/crates/zjj-core/tests/fast_in_process_queue.rs`

Tests queue behavior using pure functional and in-memory approaches:

- **PureQueue tests** (8 tests): Zero I/O, instant execution
  - Priority ordering
  - Claim behavior
  - Single-worker invariant
  - Dedupe key enforcement
  - Terminal state dedupe release
  - Consistency invariants
  - FIFO within priority
  - Invalid transition rejection

- **In-memory MergeQueue tests** (10 tests): SQLite in-memory, no file I/O
  - Basic lifecycle
  - Statistics
  - Retry failed entries
  - Concurrent claims
  - Priority listing
  - Terminal failure handling
  - Stale recovery

- **State machine tests** (3 tests): Pure, zero I/O
  - Terminal state identification
  - Valid transitions
  - Invalid transitions

- **Benchmarks**: Performance measurement

### 2. `/home/lewis/src/zjj/crates/zjj-core/tests/fast_domain_validation.rs`

Tests domain validation rules using pure functions:

- **Session name validation** (6 tests)
  - Valid names
  - Empty rejection
  - Too long rejection
  - Numeric start rejection
  - Invalid characters rejection
  - Whitespace trimming

- **Agent ID validation** (4 tests)
  - Valid IDs
  - Empty rejection
  - Too long rejection
  - Invalid characters rejection

- **Bead ID validation** (2 tests)
  - Valid bead IDs
  - Invalid bead ID rejection

- **Composed validation** (3 tests)
  - All must pass
  - Fail on first error
  - Iterator pattern

- **Property tests** (3 tests)
  - Idempotency
  - Consistent rejection
  - Length boundaries

- **Benchmarks**: 10,000 validations in <2ms

## Performance Comparison

| Test Type | Typical Time | Speedup |
|-----------|--------------|---------|
| Subprocess test | 100-500ms | 1x |
| Pure validation test | <1ms | 100-500x |
| In-memory queue test | <10ms | 10-50x |
| PureQueue test | <1ms | 100-500x |

## Design Principles

1. **Zero subprocess spawning**: No `Command::new()` calls
2. **Zero file I/O**: In-memory databases only
3. **Zero network I/O**: No external dependencies
4. **Pure functions**: Deterministic, no side effects
5. **Functional patterns**: Result propagation, combinators
6. **BDD structure**: Given/When/Then documentation

## Usage

Run the fast tests:

```bash
# Run all fast tests
cargo test --package zjj-core --test fast_in_process_queue --test fast_domain_validation

# Run only queue tests
cargo test --package zjj-core --test fast_in_process_queue

# Run only validation tests
cargo test --package zjj-core --test fast_domain_validation

# Run with output
cargo test --package zjj-core --test fast_in_process_queue -- --nocapture
```

## When to Use

### Use Fast In-Process Tests When:
- Testing domain validation rules
- Testing state machine transitions
- Testing queue behavior and invariants
- Running CI pipelines frequently
- Developing locally with fast feedback

### Keep Subprocess Tests When:
- Testing actual CLI behavior
- Testing command-line argument parsing
- Testing end-to-end workflows
- Testing external tool integration (jj, zellij)
- Validating JSON output format

## Migration Guide

To migrate a slow subprocess test to fast in-process:

1. **Identify pure domain logic**: Extract validation, state transitions, invariants
2. **Use PureQueue**: For queue behavior without I/O
3. **Use MergeQueue::open_in_memory()**: For SQLite tests without file I/O
4. **Use validation functions**: For input validation tests
5. **Keep subprocess tests**: Only for actual CLI integration

## Example: Before and After

### Before (slow, ~500ms):
```rust
#[test]
fn test_session_name_validation() {
    let harness = TestHarness::new().unwrap();
    let result = harness.zjj(&["session", "add", "123-invalid"]);
    assert!(!result.success);
}
```

### After (fast, <1ms):
```rust
#[test]
fn test_session_name_validation() {
    let result = validate_session_name("123-invalid");
    assert!(result.is_err());
}
```

## Test Count

- `fast_in_process_queue.rs`: 21 tests
- `fast_domain_validation.rs`: 19 tests
- **Total**: 40 fast in-process tests

## Integration with Existing Tests

These tests complement the existing subprocess-based integration tests in:
- `crates/zjj/tests/atdd_object_commands.rs`
- `crates/zjj/tests/e2e_scenarios.rs`

The fast tests cover domain logic, while subprocess tests cover CLI integration.
