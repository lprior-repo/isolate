# Test Performance Guide

This guide covers strategies for writing fast, reliable tests in the ZJJ codebase. Following these patterns ensures the test suite remains responsive and provides quick feedback during development.

## Table of Contents

1. [Performance Targets](#performance-targets)
2. [Slow Test Anti-Patterns](#slow-test-anti-patterns)
3. [Fast Test Patterns](#fast-test-patterns)
4. [Property Test Optimization](#property-test-optimization)
5. [Integration Test Optimization](#integration-test-optimization)
6. [Benchmark Guidelines](#benchmark-guidelines)
7. [Test Classification](#test-classification)

---

## Performance Targets

### Target Times

| Test Type | Target Time | Rationale |
|-----------|-------------|-----------|
| Unit tests | < 1ms each | Instant feedback |
| Property tests | < 100ms per property | Many generated cases |
| Integration tests | < 100ms each | I/O is unavoidable |
| Full suite | < 90 seconds | CI/CD efficiency |

### Performance Comparison

| Approach | Setup Time | Execution Time | Total |
|----------|------------|----------------|-------|
| Subprocess spawn | 50-200ms | 100-500ms | ~500ms |
| In-memory database | <1ms | <5ms | ~5ms |
| Pure functional | <0.1ms | <1ms | ~1ms |

**Key Insight**: In-process tests are 100x faster than subprocess-based tests.

---

## Slow Test Anti-Patterns

### 1. Subprocess Spawning

Subprocess spawning is the #1 cause of slow tests. Each `Command::new()` invocation adds 50-200ms overhead.

**BAD: Spawning CLI for every assertion**

```rust
// DON'T: Each test spawns a subprocess
#[test]
fn test_slow_subprocess() {
    let output = Command::new("./target/debug/zjj")
        .args(["session", "list", "--json"])
        .output()
        .expect("failed to run");
    // 50-200ms overhead just for process spawn
}
```

**GOOD: Use in-process testing or TestHarness sparingly**

```rust
// DO: Unit test the logic directly
#[test]
fn test_session_list_logic() -> Result<()> {
    let sessions = vec![
        SessionOutput::new("a", SessionStatus::Active, WorkspaceState::Working, PathBuf::from("/tmp/a"))?,
        SessionOutput::new("b", SessionStatus::Active, WorkspaceState::Working, PathBuf::from("/tmp/b"))?,
    ];

    // Test formatting logic in-process
    let output = format_sessions_as_json(&sessions)?;
    assert!(output.contains("a"));
    assert!(output.contains("b"));
    // < 1ms execution
    Ok(())
}
```

### 2. File I/O

File system operations add variable latency (1-50ms) depending on disk speed and caching.

**BAD: Real file operations in unit tests**

```rust
// DON'T: Touch the filesystem for unit tests
#[test]
fn test_config_file() {
    let path = PathBuf::from("/tmp/test-config.toml");
    std::fs::write(&path, "key = \"value\"").unwrap();
    let config = Config::load(&path).unwrap();
    // Cleanup issues, slow disk I/O
}
```

**GOOD: In-memory operations**

```rust
// DO: Use in-memory representations
#[test]
fn test_config_parsing() -> Result<()> {
    let config_str = "key = \"value\"";
    let config: Config = toml::from_str(config_str)?;
    assert_eq!(config.key, "value");
    // < 0.1ms, no I/O
    Ok(())
}
```

**ACCEPTABLE: In-memory SQLite for integration tests**

```rust
// OK: In-memory database for integration tests
#[tokio::test]
async fn test_queue_operations() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    queue.add("session-1", None, 5, None).await?;
    // ~5ms, acceptable for integration tests
    Ok(())
}
```

### 3. Network Calls

Network calls add unpredictable latency (10-1000ms) and cause flaky tests.

**BAD: Real network calls**

```rust
// DON'T: Make real HTTP requests in tests
#[tokio::test]
async fn test_api_call() {
    let client = reqwest::Client::new();
    let response = client.get("https://api.example.com/status").send().await;
    // Flaky, slow, requires network
}
```

**GOOD: Mock or trait-based testing**

```rust
// DO: Use trait-based mocking
trait HttpClient {
    async fn get(&self, url: &str) -> Result<String>;
}

struct MockHttpClient {
    response: String,
}

impl HttpClient for MockHttpClient {
    async fn get(&self, _url: &str) -> Result<String> {
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn test_api_logic() -> Result<()> {
    let client = MockHttpClient {
        response: r#"{"status": "ok"}"#.to_string(),
    };

    let result = fetch_status(&client).await?;
    assert_eq!(result.status, "ok");
    // < 1ms, deterministic
    Ok(())
}
```

### 4. Sleep/Timers

`sleep()` calls add fixed latency and make tests non-deterministic.

**BAD: Real sleep in tests**

```rust
// DON'T: Use real sleep
#[tokio::test]
async fn test_timeout() {
    let start = Instant::now();
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(start.elapsed() >= Duration::from_secs(1));
    // 1000ms wasted
}
```

**GOOD: Tokio time mocking or deterministic alternatives**

```rust
// DO: Use tokio::time::pause for async tests
#[tokio::test(start_paused = true)]
async fn test_timeout_with_mocked_time() {
    let start = Instant::now();

    // Advance time manually
    tokio::time::advance(Duration::from_secs(1)).await;

    assert!(start.elapsed() >= Duration::from_secs(1));
    // Instant execution
}

// OR: Design without time dependency
#[test]
fn test_state_machine_timeout_logic() -> Result<()> {
    let state = ProcessingState::new(TimeoutConfig::after_n_operations(5));

    // Process without actual time
    for _ in 0..5 {
        state.process_one()?;
    }

    assert!(state.is_timed_out());
    // < 1ms, pure logic
    Ok(())
}
```

### 5. Repository Initialization

Creating JJ repositories is expensive (100-500ms).

**BAD: Initialize repo per test**

```rust
// DON'T: Each test creates a repo
#[test]
fn test_session_a() {
    let harness = TestHarness::new().expect("harness"); // 100-500ms
    // ...
}

#[test]
fn test_session_b() {
    let harness = TestHarness::new().expect("harness"); // Another 100-500ms
    // ...
}
```

**GOOD: Use TestHarness only when necessary, group related tests**

```rust
// DO: Group tests that need repo setup
mod integration_tests {
    use super::*;

    // Use #[serial] if tests modify shared state
    // Or create harness per test group, not per assertion

    #[tokio::test]
    async fn session_lifecycle_full() -> Result<()> {
        let harness = TestHarness::try_new().ok_or_else(|| anyhow::anyhow!("jj not available"))?;

        // GIVEN: Initialized environment
        harness.zjj(&["init"]).assert_success();

        // WHEN: Create session
        let result = harness.zjj(&["session", "add", "test-1"]);

        // THEN: Session exists
        assert!(result.success);
        harness.assert_workspace_exists("test-1");

        // WHEN: Remove session
        let result = harness.zjj(&["session", "remove", "test-1"]);

        // THEN: Session removed
        assert!(result.success);
        harness.assert_workspace_not_exists("test-1");

        // All assertions in one harness setup
        Ok(())
    }
}
```

---

## Fast Test Patterns

### 1. Pure Functional Testing

Pure functions have no I/O, no side effects, and are instant to test.

```rust
// PureQueue tests run in < 1ms each
#[test]
fn pure_queue_priority_ordering() -> Result<(), PureQueueError> {
    // GIVEN: An empty pure queue
    let queue = PureQueue::new();

    // WHEN: Adding entries with different priorities
    let queue = queue.add("low-priority", 10, None)?;
    let queue = queue.add("high-priority", 1, None)?;
    let queue = queue.add("medium-priority", 5, None)?;

    // THEN: Entries are retrievable in priority order
    let pending: Vec<_> = queue.pending_in_order();
    assert_eq!(pending[0].workspace, "high-priority");
    assert_eq!(pending[1].workspace, "medium-priority");
    assert_eq!(pending[2].workspace, "low-priority");

    Ok(())
}
```

**Key Features**:
- No `mut` - immutable data structures
- No I/O - pure in-memory operations
- Returns `Result` - functional error handling
- < 1ms execution

### 2. In-Memory Database Testing

For tests requiring persistence semantics, use in-memory SQLite.

```rust
#[tokio::test]
async fn in_memory_queue_basic_lifecycle() -> Result<()> {
    // GIVEN: In-memory queue (no file I/O)
    let queue = MergeQueue::open_in_memory().await?;

    // WHEN: Add entry
    let add_response = queue.add("test-session", None, 5, None).await?;

    // THEN: Entry created
    assert_eq!(add_response.entry.workspace, "test-session");
    assert_eq!(add_response.entry.status, QueueStatus::Pending);

    // WHEN: Claim entry
    let claim_result = queue.next_with_lock("test-agent").await?;

    // THEN: Claimed
    let claimed = claim_result.ok_or_else(|| anyhow::anyhow!("No entry claimed"))?;
    assert_eq!(claimed.workspace, "test-session");

    Ok(())
}
```

**Performance**: ~5ms per test vs ~500ms for file-based database.

### 3. Mock Implementations

Use trait-based mocking to avoid external dependencies.

```rust
/// Repository trait for dependency injection
#[async_trait]
pub trait Repository {
    async fn save(&self, entity: &Entity) -> Result<()>;
    async fn load(&self, id: &str) -> Result<Option<Entity>>;
}

/// In-memory mock for testing
pub struct MockRepository {
    data: HashMap<String, Entity>,
}

#[async_trait]
impl Repository for MockRepository {
    async fn save(&mut self, entity: &Entity) -> Result<()> {
        self.data.insert(entity.id.clone(), entity.clone());
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Entity>> {
        Ok(self.data.get(id).cloned())
    }
}

#[tokio::test]
async fn test_entity_lifecycle() -> Result<()> {
    // GIVEN: Mock repository
    let repo = MockRepository::default();

    // WHEN: Save and load
    let entity = Entity::new("test-id", "test-name")?;
    repo.save(&entity).await?;
    let loaded = repo.load("test-id").await?;

    // THEN: Correct data
    assert!(loaded.is_some());
    assert_eq!(loaded.as_ref().map(|e| e.name.as_str()), Some("test-name"));

    Ok(())
}
```

### 4. Test Fixtures

Create reusable test fixtures to minimize per-test setup.

```rust
/// Shared test fixture builder
pub struct TestFixture {
    pub sessions: Vec<SessionOutput>,
    pub queue: PureQueue,
}

impl TestFixture {
    /// Create a fixture with N sessions
    pub fn with_sessions(n: usize) -> Result<Self> {
        let sessions = (0..n)
            .map(|i| {
                SessionOutput::new(
                    format!("session-{}", i),
                    SessionStatus::Active,
                    WorkspaceState::Working,
                    PathBuf::from(format!("/tmp/session-{}", i)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let queue = PureQueue::new();
        let queue = sessions
            .iter()
            .try_fold(queue, |q, s| q.add(&s.name, 5, None))?;

        Ok(Self { sessions, queue })
    }
}

#[test]
fn test_with_fixture() -> Result<()> {
    // GIVEN: Pre-built fixture
    let fixture = TestFixture::with_sessions(10)?;

    // WHEN/THEN: Test against fixture
    assert_eq!(fixture.sessions.len(), 10);
    assert_eq!(fixture.queue.len(), 10);

    Ok(())
}
```

### 5. Deterministic Time Injection

Inject time as a parameter instead of using system time.

```rust
/// Time provider trait
pub trait Clock {
    fn now(&self) -> std::time::Instant;
}

/// System clock (production)
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }
}

/// Mock clock (testing)
pub struct MockClock {
    current: std::cell::RefCell<std::time::Instant>,
}

impl MockClock {
    pub fn new(start: std::time::Instant) -> Self {
        Self {
            current: std::cell::RefCell::new(start),
        }
    }

    pub fn advance(&self, duration: std::time::Duration) {
        *self.current.borrow_mut() += duration;
    }
}

impl Clock for MockClock {
    fn now(&self) -> std::time::Instant {
        *self.current.borrow()
    }
}

#[test]
fn test_with_injected_time() -> Result<()> {
    // GIVEN: Mock clock
    let clock = MockClock::new(std::time::Instant::now());

    let timer = Timer::new(Box::new(clock.clone()));

    // WHEN: Advance mock time
    clock.advance(std::time::Duration::from_secs(10));

    // THEN: Timer reflects elapsed time
    assert!(timer.elapsed() >= std::time::Duration::from_secs(10));

    Ok(())
}
```

---

## Property Test Optimization

### 1. Case Count Tuning

Default proptest runs 256 cases. Tune based on test complexity.

```rust
use proptest::prelude::*;

// Fast tests: More cases for better coverage
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn prop_string_roundtrip_fast(s in ".*") {
        // < 0.1ms per case, can afford more
        prop_assert_eq!(s.clone(), s);
    }
}

// Slow tests: Fewer cases
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_database_roundtrip_slow(data in arbitrary_entity()) {
        // ~5ms per case (in-memory DB), limit cases
        let result = save_and_load(data)?;
        prop_assert_eq!(result, data);
    }
}
```

### 2. Generator Efficiency

Use efficient strategies to avoid test slowdown.

```rust
// INEFFICIENT: Complex regex generation
fn slow_strategy() -> impl Strategy<Value = String> {
    // Complex regex is slow to generate
    "[a-zA-Z][a-zA-Z0-9_-]{0,63}\\.[a-zA-Z]{2,10}"
}

// EFFICIENT: Combinator-based generation
fn fast_strategy() -> impl Strategy<Value = String> {
    // Build string from simpler parts
    ("[a-zA-Z]", "[a-zA-Z0-9_-]{0,63}", "[a-zA-Z]{2,10}")
        .prop_map(|(first, middle, suffix)| format!("{}{}.{}", first, middle, suffix))
}

// EVEN MORE EFFICIENT: Pre-computed choices for simple domains
fn status_strategy() -> impl Strategy<Value = QueueStatus> {
    prop_oneof![
        Just(QueueStatus::Pending),
        Just(QueueStatus::Claimed),
        Just(QueueStatus::Merged),
        Just(QueueStatus::FailedTerminal),
    ]
}
```

### 3. Shrinking Strategies

Configure shrinking for faster failure diagnosis.

```rust
use proptest::test_runner::Config;

fn fast_shrink_config() -> Config {
    Config {
        cases: 256,
        // Limit shrinking iterations for faster failure
        max_shrink_iters: 256,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(fast_shrink_config())]

    #[test]
    fn prop_with_limited_shrinking(input in ".*") {
        // On failure, will try to shrink but stop after 256 iterations
        prop_assert!(input.len() < 100);
    }
}
```

### 4. Deterministic Configuration

Use fixed seeds for reproducibility.

```rust
/// Deterministic proptest configuration for reproducible runs
pub fn deterministic_config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        max_shrink_iters: 1024,
        // Fixed seed ensures reproducibility
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x1234_5678_9ABC_DEF0),
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(deterministic_config())]

    #[test]
    fn prop_deterministic(input: String) {
        // Same seed = same test cases every run
        prop_assert!(true);
    }
}

// Reproduce failures with specific seed:
// PROPTEST_SEED=0x123456789abcdef0 cargo test --test my_properties
```

---

## Integration Test Optimization

### 1. Shared Fixtures

Share expensive setup across multiple tests.

```rust
// Lazy static for shared test resources
use std::sync::OnceLock;

static SHARED_HARNESS: OnceLock<TestHarness> = OnceLock::new();

fn get_shared_harness() -> Option<&'static TestHarness> {
    SHARED_HARNESS.get_or_init(|| TestHarness::try_new())
}

// Or use #[serial] for tests that modify shared state
use serial_test::serial;

#[test]
#[serial]  // Run serially to avoid conflicts
fn test_modifies_shared_state() {
    let harness = get_shared_harness()?;
    // ...
}
```

### 2. Parallel Test Execution

Design tests for parallel execution by default.

```rust
// GOOD: Each test is isolated
#[tokio::test]
async fn test_isolated_a() -> Result<()> {
    let harness = TestHarness::new()?;  // Own temp directory
    let session = format!("session-{}", uuid::Uuid::new_v4());
    // Can run in parallel with test_isolated_b
    Ok(())
}

#[tokio::test]
async fn test_isolated_b() -> Result<()> {
    let harness = TestHarness::new()?;  // Different temp directory
    let session = format!("session-{}", uuid::Uuid::new_v4());
    // Can run in parallel with test_isolated_a
    Ok(())
}

// AVOID: Shared mutable state
static SHARED_COUNTER: AtomicU32 = AtomicU32::new(0);

#[test]
fn test_not_isolated() {
    // Depends on execution order - flaky!
    let prev = SHARED_COUNTER.fetch_add(1, Ordering::SeqCst);
    assert_eq!(prev, 0);  // Fails if test runs twice
}
```

### 3. Test Isolation Strategies

Ensure each test is independent.

```rust
// Strategy 1: Unique identifiers
#[test]
fn test_with_unique_id() -> Result<()> {
    let unique_name = format!("test-{}", uuid::Uuid::new_v4());
    // Guaranteed unique across parallel runs
    Ok(())
}

// Strategy 2: Temp directories per test
#[test]
fn test_with_temp_dir() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("test.db");
    // Isolated file system space
    Ok(())
}

// Strategy 3: In-memory when possible
#[tokio::test]
async fn test_with_in_memory() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    // No file conflicts, faster
    Ok(())
}
```

---

## Benchmark Guidelines

### When to Use Benchmarks vs Tests

| Use Case | Tool | Example |
|----------|------|---------|
| Verify correctness | `#[test]` | "Does this function return the right value?" |
| Verify performance | `criterion` | "Is this function fast enough?" |
| Detect regressions | `criterion` | "Did this change slow things down?" |
| Compare implementations | `criterion` | "Is approach A faster than B?" |

### Criterion Best Practices

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_queue_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue");

    // Benchmark with different sizes
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("add", size), size, |b, &size| {
            b.iter(|| {
                let mut queue = PureQueue::new();
                for i in 0..size {
                    queue = queue.add(&format!("task-{}", i), 5, None).expect("add");
                }
                black_box(queue)
            });
        });
    }

    // Compare implementations
    group.bench_function("pure_vs_in_memory", |b| {
        b.iter(|| {
            // Setup
            let rt = tokio::runtime::Runtime::new().expect("runtime");
            let queue = rt.block_on(async {
                MergeQueue::open_in_memory().await.expect("queue")
            });

            // Measure
            rt.block_on(async {
                for i in 0..100 {
                    queue.add(&format!("task-{}", i), None, 5, None).await.expect("add");
                }
            });
            black_box(queue)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_queue_operations);
criterion_main!(benches);
```

### Benchmark Organization

```
crates/zjj-core/
  benches/
    identifier_parsing.rs    # Bench ID parsing
    queue_operations.rs      # Bench queue add/claim/complete
    state_machine.rs         # Bench state transitions
    serialization.rs         # Bench JSON/TOML serde
```

### Running Benchmarks

```bash
# Run all benchmarks
moon run :bench

# Run specific benchmark
cargo bench --bench queue_operations

# Compare with baseline
cargo bench -- --save-baseline main
cargo bench -- --baseline main

# Generate HTML report
cargo bench -- --save-baseline new
cargo bench -- --baseline main --plotting-backend plotters
```

---

## Test Classification

### Test Categories

| Category | Location | Speed | Dependency |
|----------|----------|-------|------------|
| Unit | `src/**/tests` module | < 1ms | None |
| Property | `tests/*_properties.rs` | < 100ms | proptest |
| In-memory integration | `tests/*.rs` | < 10ms | None |
| Subprocess integration | `tests/*.rs` | < 500ms | jj binary |
| E2E | `tests/e2e_*.rs` | < 2s | jj |

### Running by Category

```bash
# Fast tests only (unit + property + in-memory)
cargo test --lib
cargo test -p zjj-core --test '*_properties'

# Skip slow integration tests
cargo test -- --skip slow

# Only integration tests
cargo test --test '*_feature'

# Full suite
moon run :test
```

### CI Configuration

```yaml
# .github/workflows/test.yml
jobs:
  quick:
    runs-on: ubuntu-latest
    steps:
      - run: moon run :quick    # Unit + property tests (< 30s)

  test:
    runs-on: ubuntu-latest
    steps:
      - run: moon run :test     # Full suite (< 90s)

  bench:
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - run: moon run :bench    # Benchmarks (main only)
```

---

## Summary Checklist

Before adding a test, verify:

- [ ] **No unnecessary subprocess spawning** - Use in-process alternatives
- [ ] **No unnecessary file I/O** - Use in-memory when possible
- [ ] **No network calls** - Use mocks or traits
- [ ] **No real sleep** - Use time injection or mocking
- [ ] **Minimal repo initialization** - Group tests, use TestHarness sparingly
- [ ] **Property tests have appropriate case counts**
- [ ] **Tests are isolated and can run in parallel**
- [ ] **Benchmarks are separate from correctness tests**

Following these guidelines keeps the test suite fast, reliable, and maintainable.
