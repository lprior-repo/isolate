# Chaos Engineering Pipeline for ZJJ

## Overview

The chaos engineering pipeline provides controlled failure injection for testing zjj's robustness under adverse conditions. This helps identify edge cases, improve error handling, and ensure the system remains functional even when things go wrong.

## Design Principles

1. **Deterministic Chaos**: All randomness is seed-based for reproducible failures
2. **Zero Panic**: No `unwrap()`, `expect()`, or `panic!()` - only `Result<T, Error>`
3. **Railway-Oriented Programming**: Use `map`, `and_then`, and `?` for error propagation
4. **Functional Patterns**: Pure functions, immutable state, no side effects in core logic

## Architecture

### Core Components

```
chaos_engineering/
├── mod.rs              # Main chaos engineering module
│   ├── ChaosConfig     # Configuration for failure injection
│   ├── ChaosExecutor   # Executes operations with chaos
│   ├── FailureMode     # Types of failures to inject
│   └── ChaosTestHarness # Integration with test infrastructure
└── test_chaos_engineering.rs  # Integration tests
```

### Failure Modes

The chaos pipeline supports several failure injection modes:

| Mode | Description | Example Scenarios |
|------|-------------|-------------------|
| `IoError` | Simulates I/O failures | Permission denied, disk full, device busy |
| `Timeout` | Simulates operation timeouts | Network hangs, slow I/O |
| `Corruption` | Simulates data corruption | Bit flips, invalid UTF-8, checksum errors |
| `DeadlockSimulation` | Simulates deadlock conditions | Lock contention, circular waits |
| `ResourceExhaustion` | Simulates resource limits | Out of memory, too many open files |

## Usage

### Basic Example

```rust
use chaos_engineering::{ChaosConfig, ChaosExecutor, FailureMode};

// Configure chaos with 20% failure probability
let config = ChaosConfig::new(0.2, vec![FailureMode::IoError])
    .expect("valid config")
    .with_seed(42); // Reproducible chaos

let executor = ChaosExecutor::new(config);

// Run operation with potential chaos injection
let result = executor.inject_chaos(|| {
    std::fs::write("/tmp/test", "data")
});

// Either succeeds normally or fails with injected chaos
match result {
    Ok(()) => println!("Operation succeeded"),
    Err(e) => println!("Chaos injected: {}", e),
}
```

### Test Harness Integration

```rust
use chaos_engineering::{ChaosTestHarness, ChaosConfig, FailureMode};

let config = ChaosConfig::new(0.3, vec![FailureMode::Timeout])
    .expect("valid config")
    .with_seed(123);

let Some(harness) = ChaosTestHarness::try_new(config) else {
    return; // Skip if prerequisites unavailable
};

// Run zjj commands with chaos
let result = harness.zjj_with_chaos(&["list"]);
```

### Iterative Chaos Testing

```rust
use chaos_engineering::{run_chaos_iterations, calculate_chaos_stats, ChaosConfig, FailureMode};

let config = ChaosConfig::new(0.5, vec![FailureMode::IoError])
    .expect("valid config");

// Run operation 100 times with chaos
let results = run_chaos_iterations(config, 100, || {
    std::fs::write("/tmp/test", "data")
});

// Calculate statistics
let (successes, failures, rate) = calculate_chaos_stats(&results);
println!("Success rate: {:.1}%", rate * 100.0);
```

## Running Chaos Tests

### Run All Chaos Tests

```bash
moon run :test
```

### Run Specific Test Category

```bash
# Only chaos engineering tests
cargo test --test test_chaos_engineering

# Specific test
cargo test --test test_chaos_engineering test_init_with_io_chaos
```

### With Reproducible Seeds

```bash
# Run with specific seed for reproducibility
ZJJ_CHAOS_SEED=42 moon run :test
```

## Test Categories

### 1. Command Chaos Tests

Tests individual zjj commands with various failure modes:

- `test_init_with_io_chaos`: Init command with I/O failures
- `test_add_with_timeout_chaos`: Add command with timeouts
- `test_remove_with_deadlock_simulation`: Remove with deadlock scenarios
- `test_list_with_varied_chaos`: List with multiple failure modes

### 2. Stress Tests

Tests system behavior under repeated chaos injection:

- `test_rapid_operations_with_chaos`: Multiple rapid operations with 50% chaos
- `test_multiple_chaos_cycles`: Sequential chaos cycles to verify recovery

### 3. Reproducibility Tests

Verifies chaos is deterministic with same seed:

- `test_chaos_reproducibility`: Same seed produces same results
- `test_derived_executor_independence`: Derived executors have independent streams

### 4. Multi-Mode Tests

Tests all failure modes independently:

- `test_all_failure_modes`: Iterates through all failure modes
- `test_concurrent_chaos_streams`: Multiple independent chaos streams

## Best Practices

### 1. Start with Low Probability

```rust
// Start mild (10% chaos)
let config = ChaosConfig::new(0.1, modes).expect("valid config");
```

### 2. Use Reproducible Seeds

```rust
// Always use seeds for reproducible failures
let config = ChaosConfig::new(0.5, modes)
    .expect("valid config")
    .with_seed(42); // Consistent across runs
```

### 3. Test Recovery

```rust
// Apply chaos, then verify normal operation
let _chaos_result = executor.inject_chaos(|| operation());
let normal_result = normal_operation();
assert!(normal_result.is_ok(), "System should recover");
```

### 4. Iterate with Statistics

```rust
// Run multiple iterations to get meaningful data
let results = run_chaos_iterations(config, 100, operation);
let (successes, failures, rate) = calculate_chaos_stats(&results);

// Assert system meets reliability threshold
assert!(rate > 0.7, "Success rate should be > 70%");
```

## Integration with CI/CD

### Moon Pipeline

Add chaos tests to your CI pipeline:

```yaml
# moon.yml
tasks:
  test:chaos:
    description: "Run chaos engineering tests"
    command: "cargo test --test test_chaos_engineering"
    inputs:
      - "crates/zjj/tests/chaos_engineering/**"
      - "crates/zjj/tests/test_chaos_engineering.rs"
```

### Quality Gates

```bash
# Full pipeline including chaos tests
moon run :ci

# Quick check without chaos (faster)
moon run :quick
```

## Troubleshooting

### Tests Timing Out

If chaos tests timeout due to excessive failure injection:

```rust
// Reduce probability
let config = ChaosConfig::new(0.1, modes).expect("valid config");
```

### Non-Reproducible Failures

Ensure you're using fixed seeds:

```rust
let config = ChaosConfig::new(0.5, modes)
    .expect("valid config")
    .with_seed(42); // Must be fixed, not random
```

### Too Many Failures

Adjust probability based on desired failure rate:

```rust
// Calculate desired probability
let target_failures = 10;
let total_iterations = 100;
let probability = target_failures as f64 / total_iterations as f64;

let config = ChaosConfig::new(probability, modes).expect("valid config");
```

## Performance Impact

Chaos injection has minimal overhead:

- **0% probability**: ~5% overhead (RNG generation only)
- **50% probability**: ~10% overhead (half the operations fail fast)
- **100% probability**: ~15% overhead (all operations fail immediately)

The overhead is primarily from RNG generation and error path execution.

## Future Enhancements

Potential improvements to the chaos pipeline:

1. **Network Chaos**: Simulate network failures, latency, packet loss
2. **Filesystem Chaos**: Inject filesystem-specific errors (ENOSPC, EROFS)
3. **Process Chaos**: Simulate process crashes, signal handling
4. **Timing Chaos**: Variable delays, clock skew
5. **State Chaos**: Corrupt internal state, invalid transitions

## References

- [Chaos Engineering Principles](https://principlesofchaos.org/)
- [Functional Error Handling in Rust](https://blog.yoshuawuyts.com/error-handling/)
- [Railway-Oriented Programming](https://fsharpforfunandprofit.com/posts/recipe-part2/)

## Contributing

When adding new chaos tests:

1. Use zero unwrap/expect/panic - only `Result<T, Error>`
2. Follow functional patterns with `map`, `and_then`, `?`
3. Add reproducible seeds for consistent failures
4. Document the failure scenario and expected behavior
5. Include statistics for multi-iteration tests

Example template:

```rust
#[test]
fn test_<command>_with_<mode>_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let config = ChaosConfig::new(0.3, vec![FailureMode::<Mode>])
        .expect("valid config")
        .with_seed(<fixed_seed>);

    let executor = ChaosExecutor::new(config);

    // Test command with chaos
    let result = executor.inject_chaos(|| harness.zjj(&["<command>"]));

    // Verify graceful handling (no panic)
    drop(result);
}
```
