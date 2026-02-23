# ZJJ Performance Benchmarks

This directory contains comprehensive performance benchmarks for critical operations in the zjj codebase.

## Overview

The benchmarks ensure performance doesn't regress as the codebase evolves and validate that functional programming patterns and DDD abstractions maintain zero-cost abstractions.

## Benchmark Suites

### 1. Identifier Parsing (`identifier_parsing.rs`)

Measures the overhead of validating identifiers at boundaries.

**What it tests:**
- `SessionName`, `AgentId`, `TaskId`, `WorkspaceName` parsing overhead
- Valid vs invalid input performance
- Bulk parsing operations
- `as_ref()` and `Display` method overhead

**Why it matters:**
Identifiers use the "parse-at-boundaries" pattern - validation happens once during construction. This benchmark ensures that validation overhead is acceptable and doesn't become a bottleneck.

### 2. State Machine Transitions (`state_machine.rs`)

Validates that state transitions are zero-cost abstractions.

**What it tests:**
- `IssueState::transition_to()` performance
- `Issue::transition_to()` method overhead
- `Bead` state transitions (start, block, defer, close)
- State query methods (`is_active()`, `is_blocked()`, etc.)
- Common workflow patterns

**Why it matters:**
State machines should compile down to simple enum swaps with minimal overhead. This benchmark validates that the functional core maintains predictable performance.

### 3. Event Serialization (`event_serialization.rs`)

Measures JSON serialization/deserialization for domain events.

**What it tests:**
- Serialization/deserialization of all event types
- String vs bytes serialization
- Single event vs bulk operations
- Event creation overhead
- Metadata extraction performance

**Why it matters:**
Events are used for event sourcing and audit logging. Serialization performance is critical for high-throughput scenarios and event replay.

### 4. Repository Operations (`repository_operations.rs`)

Compares mock vs real repository implementations.

**What it tests:**
- CRUD operation performance (save, load, delete)
- Query methods (exists, list_all, list_sorted)
- Lock contention overhead
- Static vs dynamic dispatch
- Clone overhead for aggregate roots

**Why it matters:**
The repository pattern abstracts persistence behind traits. This benchmark identifies bottlenecks and ensures the abstraction doesn't add significant overhead.

### 5. Aggregate Operations (`aggregate_operations.rs`)

Measures performance of aggregate root operations.

**What it tests:**
- Construction overhead (new vs builder)
- Field update performance with validation
- Query method performance
- Clone/copy operations
- Bulk operations (create, update, filter)
- Validation overhead

**Why it matters:**
Aggregates are the core domain objects. Their operations must be efficient, especially validation which happens on every field update.

## Running Benchmarks

### Run all benchmarks:

```bash
# Using moon (recommended)
moon run :bench

# Or using cargo directly
cargo bench --workspace
```

### Run specific benchmark suite:

```bash
cargo bench --bench identifier_parsing
cargo bench --bench state_machine
cargo bench --bench event_serialization
cargo bench --bench repository_operations
cargo bench --bench aggregate_operations
```

### Run with specific filter:

```bash
# Only run benchmarks matching "session_name"
cargo bench --bench identifier_parsing -- session_name

# Only run validation benchmarks
cargo bench --bench aggregate_operations -- validation
```

### Generate plots (requires gnuplot):

```bash
cargo bench -- --save-baseline main
cargo bench -- --baseline main --plotting-backend gnuplot
```

## Interpreting Results

### Key Metrics

- **Time**: Lower is better. Measured in nanoseconds (ns) per iteration.
- **Throughput**: Higher is better. Measured in bytes/second for serialization.
- **Iterations**: Number of times the benchmark ran (more = more accurate).

### What to Look For

1. **Baseline Performance**: Establish a baseline for each operation.
2. **Regressions**: Watch for significant increases in time after changes.
3. **Outliers**: Investigate unexpected spikes in certain operations.
4. **Trends**: Compare across runs to identify performance trends.

### Example Output

```
session_name_parse/valid
                        time:   [245.12 ns 247.34 ns 249.89 ns]
                        change: [-2.341% -1.023% +0.123%] (p = 0.023 < 0.05)
                        Performance has improved.
```

## Benchmark Characteristics

### Expected Performance Ranges

- **Identifier Parsing**: 100-500ns for valid identifiers
- **State Transitions**: 50-200ns for simple transitions
- **Event Serialization**: 500ns-5μs depending on event size
- **Repository Operations**: 1-10μs for in-memory operations
- **Field Updates**: 100-500ns with validation

### Zero-Cost Abstraction Validation

The following patterns should have near-zero overhead:

- `IssueState` transitions (enum variants)
- Query methods (`is_active()`, `is_blocked()`)
- `Copy` types vs `Clone` types
- Static dispatch vs dynamic dispatch

## Performance Guidelines

### When Adding New Benchmarks

1. **Be Specific**: Benchmark one operation at a time
2. **Use Realistic Data**: Avoid pathological cases
3. **Compare**: Always have a baseline to compare against
4. **Document**: Explain what and why you're measuring

### When Optimizing

1. **Profile First**: Use `perf` or `flamegraph` to find bottlenecks
2. **Measure**: Use benchmarks to validate improvements
3. **Document**: Add comments explaining the optimization
4. **Trade-offs**: Consider readability vs performance

## Continuous Integration

Benchmarks run in CI to detect performance regressions:

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: moon run :bench

- name: Store benchmark results
  uses: benchmark-action/github-action-benchmark@v1
```

## See Also

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/index.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Functional Rust Principles](/CLAUDE.md)
