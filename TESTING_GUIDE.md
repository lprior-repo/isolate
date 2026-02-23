# ZJJ Testing Guide

This guide covers comprehensive testing practices for the ZJJ codebase, including property-based testing, unit testing, integration testing, benchmarks, and CI/CD integration.

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Organization](#test-organization)
3. [Writing Unit Tests](#writing-unit-tests)
4. [Property-Based Tests with Proptest](#property-based-tests-with-proptest)
5. [Integration Tests](#integration-tests)
6. [ATDD/BDD Tests](#atddbdd-tests)
7. [Benchmarks](#benchmarks)
8. [Test Naming Conventions](#test-naming-conventions)
9. [Running Tests](#running-tests)
10. [CI/CD Integration](#cicd-integration)
11. [Coverage Requirements](#coverage-requirements)
12. [Common Patterns](#common-patterns)
13. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## Testing Philosophy

The ZJJ codebase follows a multi-layered testing approach based on functional Rust principles and Domain-Driven Design:

### Test Pyramid

```
        /\
       /  \      E2E Tests (5%)
      /____\
     /      \    Integration Tests (15%)
    /________\
   /          \  Property-Based Tests (30%)
  /____________\
 /              \ Unit Tests (50%)
```

### Core Principles

1. **Zero Unwrap, Zero Panic**: All tests must follow functional Rust patterns
2. **Property-Based Testing**: Use proptest to verify invariants across generated inputs
3. **Test Isolation**: Each test should be independent and runnable in parallel
4. **Reproducibility**: Use deterministic seeds for property tests
5. **Documentation**: Tests should serve as executable documentation

### Test Types

| Type | Purpose | Location | Tool |
|------|---------|----------|------|
| Unit Tests | Test individual functions in isolation | `src/**/*.rs` modules | `rustc` built-in |
| Property Tests | Verify invariants across generated inputs | `tests/*_properties.rs` | `proptest` |
| Integration Tests | Test component interactions | `tests/*_feature.rs` | `tokio::test` |
| ATDD Tests | Acceptance test-driven development | `tests/*_atdd*.rs` | `tokio::test` |
| Benchmarks | Performance regression detection | `benches/*.rs` | `criterion` |

---

## Test Organization

### Directory Structure

```
zjj/
├── crates/
│   ├── zjj-core/
│   │   ├── src/
│   │   │   ├── beads/
│   │   │   │   └── mod.rs          # Unit tests in #[cfg(test)] modules
│   │   │   └── ...
│   │   ├── tests/                  # Integration tests for zjj-core
│   │   │   ├── cli_properties.rs   # Property-based CLI tests
│   │   │   ├── doctor_properties.rs
│   │   │   └── identifier_properties.rs
│   │   └── benches/                # Performance benchmarks
│   │       ├── identifier_parsing.rs
│   │       ├── state_machine.rs
│   │       └── ...
│   └── zjj/
│       └── tests/                  # Integration tests for zjj CLI
│           ├── queue_feature.rs    # BDD-style feature tests
│           ├── session_feature.rs
│           ├── agent_properties.rs # Property-based agent tests
│           └── common/             # Shared test utilities
│               └── mod.rs
└── tests/                          # Workspace-level integration tests
    ├── status_properties.rs
    └── ...
```

### Module-Level Unit Tests

Place unit tests in the same file as the code, using `#[cfg(test)]` modules:

```rust
// crates/zjj-core/src/beads/mod.rs

impl Bead {
    pub fn close(&mut self) -> Result<(), DomainError> {
        // Implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_bead_sets_closed_at() {
        // Test implementation
    }
}
```

---

## Writing Unit Tests

### Basic Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_valid() {
        let name = SessionName::new("valid-session");
        assert!(name.is_ok());
    }

    #[test]
    fn test_session_name_empty_rejected() {
        let name = SessionName::new("");
        assert!(name.is_err());
    }

    #[test]
    fn test_session_name_invalid_chars_rejected() {
        let name = SessionName::new("invalid session!");
        assert!(name.is_err());
    }
}
```

### Async Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_enqueue() {
        let queue = MergeQueue::in_memory();
        let result = queue.enqueue("session-1").await;
        assert!(result.is_ok());
    }
}
```

### Table-Driven Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        input: &'static str,
        expected: bool,
        description: &'static str,
    }

    #[test]
    fn test_session_name_validation() {
        let cases = vec![
            TestCase {
                input: "valid-session",
                expected: true,
                description: "valid session name",
            },
            TestCase {
                input: "",
                expected: false,
                description: "empty string",
            },
            TestCase {
                input: "invalid spaces",
                expected: false,
                description: "contains spaces",
            },
        ];

        for case in cases {
            let result = SessionName::new(case.input).is_ok();
            assert_eq!(
                result, case.expected,
                "{}: input='{}', expected={}",
                case.description, case.input, case.expected
            );
        }
    }
}
```

---

## Property-Based Tests with Proptest

### Why Property-Based Testing?

Property-based tests verify invariants across **hundreds of generated inputs**, catching edge cases that hand-written tests miss. This is critical for:

- Input validation boundaries
- State machine transitions
- Serialization/deserialization
- Collection operations
- Numeric computations

### Basic Property Test

```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_session_name_roundtrip(name in "[a-zA-Z][a-zA-Z0-9_-]{0,63}") {
        // Parse the name
        let session_name = SessionName::new(name.clone()).unwrap();

        // Convert back to string
        let result = session_name.as_str();

        // Should round-trip
        prop_assert_eq!(result, name);
    }
}
```

### Custom Strategies

```rust
/// Generate valid session names
fn session_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,63}"
}

/// Generate session statuses
fn session_status_strategy() -> impl Strategy<Value = SessionStatus> {
    prop_oneof![
        Just(SessionStatus::Creating),
        Just(SessionStatus::Active),
        Just(SessionStatus::Paused),
        Just(SessionStatus::Completed),
        Just(SessionStatus::Failed),
    ]
}

/// Generate absolute paths
fn absolute_path_strategy() -> impl Strategy<Value = PathBuf> {
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| PathBuf::from(format!("/tmp/zjj-test-{}", s)))
}
```

### Invariant Testing

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn prop_session_status_valid_transitions(
        from_status in session_status_strategy(),
        to_status in session_status_strategy(),
    ) {
        let can_transition = from_status.can_transition_to(to_status);

        // Define valid transitions
        let is_valid = match (from_status, to_status) {
            (SessionStatus::Creating, SessionStatus::Active) => true,
            (SessionStatus::Creating, SessionStatus::Failed) => true,
            (SessionStatus::Active, SessionStatus::Paused) => true,
            (SessionStatus::Active, SessionStatus::Completed) => true,
            (SessionStatus::Paused, SessionStatus::Active) => true,
            (SessionStatus::Paused, SessionStatus::Completed) => true,
            (SessionStatus::Completed, _) => false,
            (SessionStatus::Failed, _) => false,
            _ => false,
        };

        prop_assert_eq!(can_transition, is_valid);
    }
}
```

### JSON Serialization Invariants

```rust
proptest! {
    #[test]
    fn prop_session_output_serializes_to_valid_json(
        name in session_name_strategy(),
        status in session_status_strategy(),
    ) {
        let session = SessionOutput::new(name, status)
            .map_err(|_| "Creation failed")?;

        // Serialize
        let json = serde_json::to_string(&session)
            .map_err(|_| "Serialization failed")?;

        // Parse back to verify validity
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "JSON must be valid: {}", json);

        // Verify required fields
        let value = parsed.unwrap();
        prop_assert!(value.get("name").is_some());
        prop_assert!(value.get("status").is_some());
    }
}
```

### Deterministic Configuration

```rust
/// Create deterministic config for reproducible runs
fn deterministic_config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        max_shrink_iters: 1024,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(deterministic_config())]

    #[test]
    fn prop_with_reproducible_config(input in ".*") {
        // Test code here
    }
}
```

### Reproducing Failures

When a property test fails, proptest provides a seed for reproduction:

```bash
# Run with specific seed
PROPTEST_SEED=0x123456789abcdef0 cargo test --test session_properties

# Or paste the failing input directly
PROPTEST="input_strategy=your_input_here" cargo test --test session_properties
```

### Property Test File Template

```rust
//! Property-based tests for [Feature] invariants
//!
//! # Invariants tested:
//! - Invariant 1: Description
//! - Invariant 2: Description
//!
//! Run with: cargo test --test [name]_properties
//! Reproducible: Set PROPTEST_SEED environment variable

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use proptest::prelude::*;

// ═══════════════════════════════════════════════════════════════
// CUSTOM STRATEGIES
// ═══════════════════════════════════════════════════════════════

fn custom_strategy() -> impl Strategy<Value = Type> {
    // Strategy implementation
}

// ═══════════════════════════════════════════════════════════════
// PROPERTY TESTS
// ═══════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_invariant_name(input in strategy()) {
        // Property assertions
        prop_assert!(condition);
    }
}
```

---

## Integration Tests

### Test Harness

Use `TestHarness` for integration tests requiring a full JJ repository:

```rust
use common::TestHarness;

#[tokio::test]
async fn test_session_creation() {
    let Some(harness) = TestHarness::new().ok() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // Initialize zjj
    harness.zjj(&["init"]).expect("init should succeed");

    // Create session
    let result = harness.zjj(&["session", "add", "my-session"]);
    assert!(result.success);
}
```

### Command Execution

```rust
#[tokio::test]
async fn test_queue_list() {
    let ctx = AtddTestContext::try_new().expect("create context");

    // GIVEN
    ctx.init_zjj().expect("init succeeds");
    ctx.get_queue().await.expect("queue initialized");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "list"]);

    // THEN
    assert!(result.success, "queue list should succeed");
    assert!(result.stdout.contains("[]") || result.stdout.contains("entries"));
}
```

### JSON Output Parsing

```rust
use common::parse_json_output;

#[tokio::test]
async fn test_status_output_json() {
    let harness = TestHarness::new().expect("create harness");

    // Run command
    let result = harness.zjj(&["status", "show", "--json"]);
    assert!(result.success);

    // Parse JSON
    let output: serde_json::Value = parse_json_output(&result.stdout);
    assert!(output.is_object());
    assert!(output.get("session").is_some());
}
```

### Parallel Test Execution

Tests must be independent to run in parallel:

```rust
#[tokio::test]
async fn test_isolated_session_a() {
    // Each test gets its own temp directory
    let ctx = TestHarness::new().expect("harness");
    let session_name = format!("session-a-{}", uuid::Uuid::new_v4());

    ctx.zjj(&["session", "add", &session_name])
        .expect("session created");
}

#[tokio::test]
async fn test_isolated_session_b() {
    // Can run in parallel with test_isolated_session_a
    let ctx = TestHarness::new().expect("harness");
    let session_name = format!("session-b-{}", uuid::Uuid::new_v4());

    ctx.zjj(&["session", "add", &session_name])
        .expect("session created");
}
```

---

## ATDD/BDD Tests

### Given-When-Then Structure

```rust
#[tokio::test]
async fn scenario_create_session_succeeds() {
    let Some(ctx) = SessionTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN: Preconditions
    given_steps::zjj_database_is_initialized(&ctx)
        .expect("GIVEN: database initialization should succeed");
    given_steps::in_jj_repository(&ctx)
        .expect("GIVEN: jj repository setup should succeed");
    given_steps::no_session_named_exists(&ctx, "feature-auth")
        .await
        .expect("GIVEN: no session should exist initially");

    // WHEN: Action under test
    when_steps::create_session_with_path(&ctx, "feature-auth", "/workspaces/feature-auth")
        .await
        .expect("WHEN: session creation should succeed");

    // THEN: Expected outcomes
    then_steps::session_should_exist(&ctx, "feature-auth")
        .await
        .expect("THEN: session should exist");
    then_steps::session_should_have_status(&ctx, "feature-auth", "creating")
        .await
        .expect("THEN: session should have creating status");
}
```

### Step Definitions

```rust
pub mod given_steps {
    use super::*;

    pub fn zjj_database_is_initialized(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx.harness.zjj(&["init"]);
        if !result.success {
            anyhow::bail!("ZJJ init failed: {}", result.stderr);
        }
        Ok(())
    }

    pub fn in_jj_repository(ctx: &SessionTestContext) -> Result<()> {
        if !ctx.harness.repo_path.join(".jj").exists() {
            anyhow::bail!("Not a JJ repository");
        }
        Ok(())
    }
}

pub mod when_steps {
    use super::*;

    pub async fn create_session_with_path(
        ctx: &SessionTestContext,
        name: &str,
        path: &str,
    ) -> Result<()> {
        let result = ctx.harness.zjj(&["session", "add", name, "--workspace", path]);
        if !result.success {
            anyhow::bail!("Session creation failed: {}", result.stderr);
        }
        Ok(())
    }
}

pub mod then_steps {
    use super::*;

    pub async fn session_should_exist(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.zjj(&["session", "show", name]);
        if !result.success {
            anyhow::bail!("Session should exist: {}", result.stderr);
        }
        Ok(())
    }
}
```

---

## Benchmarks

### Benchmark Template

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Benchmark [Operation Name] performance
//!
//! This benchmark measures the overhead of [operation description]

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use zjj_core::path::to::module;

fn bench_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("operation_name");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let input = create_test_input(size);
                black_box(operation_to_bench(black_box(input)))
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_operation);
criterion_main!(benches);
```

### Running Benchmarks

```bash
# Run all benchmarks
moon run :bench

# Run specific benchmark
cargo bench --bench identifier_parsing

# Run with filter
cargo bench --bench identifier_parsing -- session_name

# Compare baselines
cargo bench -- --save-baseline main
cargo bench -- --baseline main
```

### Benchmark Categories

1. **Micro-benchmarks**: Single function performance (< 1μs)
2. **Meso-benchmarks**: Multi-step operations (1μs - 1ms)
3. **Macro-benchmarks**: End-to-end workflows (> 1ms)

---

## Test Naming Conventions

### Test Function Names

```rust
// Unit tests: test_<unit>_<scenario>
fn test_session_name_valid() { }
fn test_session_name_empty_rejected() { }
fn test_session_name_too_long_rejected() { }

// Property tests: prop_<invariant>_description
fn prop_session_name_roundtrip() { }
fn prop_session_status_valid_transitions() { }
fn prop_json_serialization_valid() { }

// Integration tests: <feature>_<scenario>
fn test_queue_list_empty_shows_no_entries() { }
fn test_session_creation_succeeds() { }
fn test_session_duplicate_creation_fails() { }

// BDD scenarios: scenario_<feature>_<outcome>
fn scenario_create_session_succeeds() { }
fn scenario_create_duplicate_session_fails() { }
fn scenario_queue_enqueue_succeeds() { }
```

### Test File Names

| Pattern | Example | Purpose |
|---------|---------|---------|
| `*_properties.rs` | `agent_properties.rs` | Property-based tests |
| `*_feature.rs` | `queue_feature.rs` | Feature integration tests |
| `*_atdd*.rs` | `atdd_object_commands.rs` | Acceptance tests |
| `test_*.rs` | `test_session_lifecycle.rs` | Specific scenario tests |

---

## Running Tests

### Quick Tests (Fast Feedback)

```bash
# Run all tests (unit + integration)
moon run :test

# Run only unit tests (in-source)
cargo test --lib

# Run specific test
cargo test test_session_name_valid
```

### Property Tests

```bash
# Run all property tests
cargo test --test *_properties

# Run specific property test
cargo test --test agent_properties prop_agent_id_unique

# Run with specific seed for reproduction
PROPTEST_SEED=0x123456789abcdef0 cargo test --test session_properties
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test '*_feature'

# Run specific feature test
cargo test --test queue_feature

# Run tests with output
cargo test --test queue_feature -- --nocapture
```

### Parallel Test Execution

```bash
# Run tests in parallel (default)
cargo test

# Run tests serially (for debugging)
cargo test -- --test-threads=1

# Run with verbose output
cargo test -- --show-output
```

### Test Selection

```bash
# Run tests matching pattern
cargo test session

# Run tests in package
cargo test -p zjj-core

# Run tests excluding slow ones
cargo test --skip slow
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - Install Rust
      - run: rustup update stable

      - Run quick tests
      - run: moon run :quick

      - Run full test suite
      - run: moon run :test

      - Run benchmarks (on main branch only)
      if: github.ref == 'refs/heads/main'
      - run: moon run :bench

      - Upload coverage
      - uses: codecov/codecov-action@v3
        with:
          files: ./lcov.info
```

### Pre-commit Hooks

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run quick tests before committing
moon run :quick

# Check for unwrap/panic in production code
cargo clippy -- -D unwrap_used -D expect_used -D panic
```

### Test Quality Gates

1. **All tests must pass**: No test failures allowed
2. **No unwrap in src/**: Zero-unwrap policy enforced
3. **Coverage threshold**: Minimum 80% coverage for new code
4. **Benchmarks**: No more than 5% regression allowed

---

## Coverage Requirements

### Minimum Coverage Targets

| Area | Coverage Target | Notes |
|------|----------------|-------|
| Domain Logic | 90% | Critical business logic |
| CLI Handlers | 80% | User-facing code |
| Database Layer | 85% | Persistence operations |
| Output Types | 95% | Serialization contracts |
| Integration | 70% | End-to-end scenarios |

### Generating Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Generate for specific test
cargo tarpaulin --test agent_properties --out Html
```

### Coverage Exclusions

```rust
// Test helpers can be excluded from coverage
#[cfg(test)]
#[expect(dead_code)]
mod test_helpers {
    // Test-only code
}
```

---

## Common Patterns

### Result Propagation in Tests

```rust
#[test]
fn test_operation_returns_result() {
    let result = operation_that_might_fail();
    assert!(result.is_ok(), "Operation should succeed: {:?}", result);
}

#[test]
fn test_operation_error_case() {
    let result = operation_that_might_fail();
    assert!(result.is_err());
    assert!(matches!(result, Err(DomainError::NotFound)));
}
```

### Setup/Teardown Pattern

```rust
#[tokio::test]
async fn test_with_setup_teardown() {
    // SETUP
    let harness = TestHarness::new().expect("harness");
    harness.zjj(&["init"]).expect("init");

    // TEST
    let result = harness.zjj(&["session", "add", "test"]);
    assert!(result.success);

    // TEARDOWN (implicit via TempDrop)
    // TestHarness cleanup is automatic
}
```

### Assertion Helpers

```rust
// In common/mod.rs
pub fn assert_json_has_field(json: &JsonValue, field: &str) {
    assert!(
        json.get(field).is_some(),
        "JSON missing field '{}': {}",
        field,
        json
    );
}

// In tests
use common::assert_json_has_field;

#[test]
fn test_output_structure() {
    let json = parse_json_output(&output);
    assert_json_has_field(&json, "session");
    assert_json_has_field(&json, "status");
}
```

### Mock Repository Pattern

```rust
#[cfg(test)]
struct MockRepository {
    data: Arc<Mutex<HashMap<String, Bead>>>,
}

#[async_trait]
impl Repository for MockRepository {
    async fn save(&self, bead: &Bead) -> Result<()> {
        self.data.lock().await.insert(bead.id().to_string(), bead.clone());
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Bead>> {
        Ok(self.data.lock().await.get(id).cloned())
    }
}
```

---

## Anti-Patterns to Avoid

### 1. Using `unwrap()` in Tests

**BAD:**
```rust
#[test]
fn test_bad() {
    let session = SessionOutput::new("test", status).unwrap();
}
```

**GOOD:**
```rust
#[test]
fn test_good() {
    let result = SessionOutput::new("test", status);
    assert!(result.is_ok(), "Session creation should succeed");
}
```

### 2. Brittle Test Data

**BAD:**
```rust
#[test]
fn test_hardcoded_path() {
    let path = PathBuf::from("/home/user/specific/path");
    // Test depends on external state
}
```

**GOOD:**
```rust
#[test]
fn test_isolated_path() {
    let harness = TestHarness::new().expect("harness");
    let path = harness.repo_path.join("test");
    // Test uses isolated temp directory
}
```

### 3. Testing Implementation Details

**BAD:**
```rust
#[test]
fn test_internal_field() {
    assert_eq!(session.internal_field, "value");
}
```

**GOOD:**
```rust
#[test]
fn test_public_behavior() {
    let output = session.to_json();
    assert!(output.contains("\"status\":\"active\""));
}
```

### 4. Shared Mutable State in Tests

**BAD:**
```rust
lazy_static! {
    static ref SHARED_STATE: Mutex<State> = Mutex::new(State::new());
}

#[test]
fn test_shared_state() {
    // Can't run in parallel
}
```

**GOOD:**
```rust
#[test]
fn test_isolated_state() {
    let state = State::new(); // Each test gets its own
}
```

### 5. Ignoring Test Failures

**BAD:**
```rust
#[test]
#[ignore = "TODO: fix this test"]
fn test_broken() {
    // Fails but is ignored
}
```

**GOOD:**
```rust
#[test]
fn test_with_skip_condition() {
    if !jj_is_available() {
        eprintln!("SKIP: jj not available");
        return;
    }
    // Actual test
}
```

---

## Quick Reference

### Test Attributes

```rust
#[test]                           // Synchronous test
#[tokio::test]                    // Async test
#[should_panic]                   // Expect panic (AVOID)
#[ignore]                         // Skip test (document why)
#[serial_test::serial]            // Run serially (for shared resources)
```

### Common Macros

```rust
assert!(condition)                // Basic assertion
assert_eq!(left, right)           // Equality check
assert_matches!(value, pattern)   // Pattern match
prop_assert!(condition)           // Proptest assertion
format!("{}", args)               // String formatting
vec![1, 2, 3]                     // Vec creation
```

### Test Commands

```bash
moon run :test                    # Run all tests
moon run :quick                   # Quick tests only
cargo test --lib                  # Unit tests only
cargo test --test <name>          # Specific integration test
PROPTEST_SEED=<hex> cargo test   # Reproducible proptest
cargo bench                       # Run benchmarks
```

---

## See Also

- [Functional Rust Principles](CLAUDE.md) - Core coding standards
- [AGENTS.md](AGENTS.md) - Workflow and development guidelines
- [ crates/zjj-core/benches/README.md](crates/zjj-core/benches/README.md) - Benchmark documentation
- [Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/) - Benchmarking guide
- [Proptest Guide](https://altsysrq.github.io/proptest-book/) - Property testing guide
