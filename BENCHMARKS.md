# Performance Benchmarks and Scalability Tests

## Overview

This document describes the performance benchmark suite for zjj, including how to run benchmarks, interpret results, and expected performance characteristics.

## Benchmark Suites

### 1. Session Operations (`session_operations.rs`)

Benchmarks for database operations:

**Operations Tested:**
- `session_create` - Session creation performance
- `session_get` - Session retrieval by name
- `session_update` - Session field updates
- `session_delete` - Session deletion
- `session_list` - List all sessions (parametrized by count: 10, 50, 100, 500, 1000)
- `session_list_filtered` - List sessions with status filter (100 sessions, 50% match)
- `session_concurrent_reads` - 10 concurrent read operations
- `session_concurrent_writes` - 10 concurrent write operations
- `session_backup` - Database backup (parametrized by count: 10, 50, 100)
- `session_restore` - Database restore (50 sessions)

**Expected Performance:**
- Session CRUD operations: < 1ms per operation
- List 100 sessions: < 5ms
- List 1000 sessions: < 20ms
- Concurrent reads (10 threads): < 5ms total
- Concurrent writes (10 threads): < 10ms total
- Backup 100 sessions: < 10ms
- Restore 50 sessions: < 15ms

**Scalability Characteristics:**
- Linear O(n) scaling for list operations
- Constant O(1) for individual CRUD operations
- Thread-safe with Arc<Mutex<Connection>> - scales with thread count

### 2. Config Operations (`config_operations.rs`)

Benchmarks for configuration loading and parsing:

**Operations Tested:**
- `config_load_defaults` - Create default config
- `config_parse_toml` - Parse TOML config file
- `config_load_full` - Full config loading with merging
- `config_serialize_toml` - Serialize config to TOML
- `config_serialize_json` - Serialize config to JSON
- `config_clone` - Clone config structure
- `config_merge` - Merge default and loaded configs (depth: 1, 2, 3)
- `config_parse_size` - Parse configs of varying sizes (small, medium, large)

**Expected Performance:**
- Default config creation: < 100µs
- Parse TOML config: < 500µs
- Full config load + merge: < 1ms
- Serialize to TOML: < 200µs
- Serialize to JSON: < 150µs
- Clone config: < 50µs

**Scalability Characteristics:**
- Config size has minimal impact on parse time (well-optimized TOML parser)
- Merging is O(n) where n = number of config fields

### 3. Validation (`validation.rs`)

Benchmarks for input validation and sanitization:

**Operations Tested:**
- `validate_session_name_valid` - Validate valid names (8 cases)
- `validate_session_name_invalid` - Validate invalid names (10 cases)
- `validate_name_length` - Varying lengths (1, 10, 32, 64, 100 chars)
- `validate_special_chars` - Special character detection (10 cases)
- `validate_unicode` - Unicode detection (10 cases)
- `validate_path_traversal` - Path traversal detection (9 cases)
- `validate_control_chars` - Control character detection (5 cases)
- `validate_sql_injection` - SQL injection detection (4 cases)
- `validate_shell_metacharacters` - Shell metachar detection (7 cases)
- `validate_high_load` - High-volume validation (100, 1000, 10000 names)
- `validate_mixed` - Mixed valid/invalid names (8 cases)
- `validate_early_rejection` - Early rejection optimization (6 cases)

**Expected Performance:**
- Single validation: < 10µs
- Batch 100 validations: < 500µs
- Batch 1000 validations: < 5ms
- Batch 10000 validations: < 50ms

**Scalability Characteristics:**
- O(n) where n = name length
- Early rejection optimization: O(1) for invalid first character
- Unicode check is O(n) but very fast (native Rust)
- Scales linearly with input size

## Running Benchmarks

### Prerequisites

```bash
# Install Rust (1.80+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install criterion (included in dev-dependencies)
cd crates/zjj
```

### Run All Benchmarks

```bash
# Run all benchmarks
moon run :bench

# Or using cargo directly
cd crates/zjj
cargo bench
```

### Run Specific Benchmark Suites

```bash
# Session operations only
cargo bench --bench session_operations

# Config operations only
cargo bench --bench config_operations

# Validation only
cargo bench --bench validation
```

### Run Specific Benchmarks

```bash
# Run only session_create benchmark
cargo bench --bench session_operations session_create

# Run only high-load validation
cargo bench --bench validation validate_high_load
```

### Generate HTML Reports

Criterion automatically generates HTML reports in:

```
target/criterion/
├── session_create/
│   ├── report/
│   │   └── index.html
│   └── base/
├── session_list/
│   └── ...
└── ...
```

Open `target/criterion/report/index.html` in a browser to view interactive charts.

### Compare Against Baseline

```bash
# Save current results as baseline
cargo bench -- --save-baseline my-baseline

# Make code changes...

# Compare against baseline
cargo bench -- --baseline my-baseline
```

## Interpreting Results

### Example Output

```
session_create          time:   [245.32 µs 248.91 µs 252.84 µs]
                        change: [-2.3421% +0.1234% +2.7891%] (p = 0.89 > 0.05)
                        No change in performance detected.

session_list/10         time:   [1.2341 ms 1.2456 ms 1.2598 ms]
session_list/50         time:   [2.3456 ms 2.3891 ms 2.4234 ms]
session_list/100        time:   [4.5678 ms 4.6234 ms 4.6891 ms]
```

**Reading the Output:**
- **time**: [lower_bound estimate upper_bound] - 95% confidence interval
- **change**: Performance change from previous run
- **p-value**: Statistical significance (p < 0.05 = significant change)

### Performance Regression Detection

A performance regression is detected when:
1. p-value < 0.05 (statistically significant)
2. change > +5% (slower than baseline)

Example:
```
validation_high_load/10000  time:   [52.891 ms 53.234 ms 53.678 ms]
                            change: [+8.1234% +9.4567% +10.891%] (p = 0.00 < 0.05)
                            Performance has regressed.
```

## Continuous Integration

Benchmarks run automatically on:
- **Pull Requests**: Compare PR performance against base branch
- **Main Branch**: Track performance over time
- **Manual Trigger**: Via GitHub Actions workflow_dispatch

### Benchmark CI Workflow

Located at `.github/workflows/benchmarks.yml`:

```yaml
name: Benchmarks

on:
  push:
    branches: [main, master]
  pull_request:
  workflow_dispatch:
```

**Artifacts:**
- Benchmark results (TXT format)
- Criterion HTML reports
- Performance comparison (PR vs base)

### Viewing CI Results

1. Go to Actions tab in GitHub
2. Click on "Benchmarks" workflow
3. Select a run
4. Download "benchmark-results" artifact
5. Extract and open `index.html`

## Performance Characteristics

### Database Operations (SQLite)

| Operation | Time | Scalability |
|-----------|------|-------------|
| CREATE | < 1ms | O(1) |
| READ (by name) | < 1ms | O(1) |
| UPDATE | < 1ms | O(1) |
| DELETE | < 1ms | O(1) |
| LIST (10) | < 2ms | O(n) |
| LIST (100) | < 5ms | O(n) |
| LIST (1000) | < 20ms | O(n) |
| BACKUP (100) | < 10ms | O(n) |
| RESTORE (100) | < 15ms | O(n) |

**Thread Safety:**
- Uses `Arc<Mutex<Connection>>` for thread-safe access
- Read operations: Concurrent but sequential (mutex)
- Write operations: Fully serialized (SQLite limitation)
- Future optimization: Read-write lock or connection pool

### Config Operations

| Operation | Time | Scalability |
|-----------|------|-------------|
| Defaults | < 100µs | O(1) |
| Parse TOML | < 500µs | O(config_size) |
| Load + Merge | < 1ms | O(fields) |
| Serialize | < 200µs | O(fields) |
| Clone | < 50µs | O(fields) |

### Validation

| Operation | Time | Scalability |
|-----------|------|-------------|
| Single name | < 10µs | O(name_length) |
| Batch 100 | < 500µs | O(batch_size × name_length) |
| Batch 1000 | < 5ms | O(batch_size × name_length) |
| Batch 10000 | < 50ms | O(batch_size × name_length) |

**Optimization Techniques:**
- Early rejection (first character check)
- ASCII-only validation (no expensive unicode operations)
- Compiled regex for complex patterns
- Zero-allocation validation where possible

## Optimization Guidelines

### When to Optimize

Only optimize if benchmarks show:
1. **Regression**: > 10% slowdown from baseline
2. **User-Facing Latency**: > 100ms for interactive operations
3. **Scalability Issues**: Non-linear growth beyond expected O(n)

### Optimization Priorities

1. **Hot Path**: Operations run on every command (validation, config load)
2. **User-Facing**: Operations users wait for (session create, list)
3. **Background**: Operations run async (backup, sync)

### Profiling

For detailed profiling:

```bash
# CPU profiling with flamegraph
cargo install flamegraph
cargo flamegraph --bench session_operations

# Memory profiling with valgrind
cargo bench --bench session_operations --no-run
valgrind --tool=massif target/release/deps/session_operations-*

# Detailed timing with criterion
cargo bench --bench session_operations -- --profile-time=60
```

## Benchmarking Best Practices

1. **Consistent Environment**:
   - Close unnecessary applications
   - Run on consistent hardware
   - Disable CPU frequency scaling: `sudo cpupower frequency-set --governor performance`

2. **Statistical Significance**:
   - Run benchmarks multiple times
   - Check p-values (< 0.05 for significance)
   - Look for consistent trends, not single runs

3. **Realistic Workloads**:
   - Use production-like data sizes
   - Test edge cases (empty, very large)
   - Include concurrent operations

4. **Baseline Comparisons**:
   - Save baselines before major changes
   - Compare against previous versions
   - Track performance over time

## Future Enhancements

### Planned Benchmarks

- [ ] End-to-end command benchmarks (`zjj add`, `zjj list`, etc.)
- [ ] Zellij integration benchmarks
- [ ] JJ workspace operation benchmarks
- [ ] Large-scale stress tests (10k+ sessions)
- [ ] Memory usage profiling
- [ ] Startup time benchmarks

### Optimization Opportunities

- [ ] Connection pooling for concurrent DB access
- [ ] Config caching (avoid reparsing)
- [ ] Lazy loading for large session lists
- [ ] Incremental backup/restore
- [ ] Index optimization for filtered queries

## Troubleshooting

### Benchmark Fails to Compile

```bash
# Clean and rebuild
cargo clean
cargo bench --bench session_operations --no-run
```

### Inconsistent Results

```bash
# Increase sample size
cargo bench --bench session_operations -- --sample-size 1000

# Increase measurement time
cargo bench --bench session_operations -- --measurement-time 60
```

### File Lock Errors

```bash
# Wait for other cargo processes to finish
# Or kill conflicting processes
pkill -9 cargo
```

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [SQLite Performance Tuning](https://www.sqlite.org/optoverview.html)

## License

MIT License - See LICENSE file for details.

---

**Last Updated**: 2026-01-11
**Benchmark Version**: 1.0.0
**Rust Version**: 1.80+
