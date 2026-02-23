# Slow Tests Analysis Report

This report identifies patterns in the test suite that cause slowness, organized by severity and category.

## Executive Summary

The test suite contains multiple slow test anti-patterns that significantly impact test execution time. The main culprits are:

1. **Subprocess Spawning** - 200+ tests spawn `Command::new` for jj/zjj binaries
2. **File System I/O** - Extensive use of TempDir, file reads/writes
3. **Sleep/Timers** - Multiple tests use `sleep()` for synchronization
4. **Concurrency Stress Tests** - Tests spawning 50-100+ concurrent tasks
5. **Repository Initialization** - Tests that run `jj git init` repeatedly

---

## Category 1: Subprocess Spawning (CRITICAL)

### Pattern
Tests use `Command::new()` to spawn external processes (jj, zjj, cargo). Process spawning has significant overhead (10-100ms per spawn).

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj/tests/cli_flag_contract_*.rs` (15+ files)
- **Pattern**: Each test spawns a new `zjj` process via `assert_cmd::Command`
- **Line examples**: Multiple tests per file, each calling `Command::new(env!("CARGO_BIN_EXE_zjj"))`
- **Impact**: ~15 files x ~4 tests each = 60+ subprocess spawns
- **Why slow**: Each spawn loads the binary, initializes runtime, parses args

```rust
// Example from cli_flag_contract_ai.rs:22
let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
cmd.arg("ai").arg("--contract");
```

**Suggested Fix**: Combine multiple contract tests into single process invocations using subcommands, or use lib tests for pure contract validation.

#### `/home/lewis/src/zjj/crates/zjj/tests/test_clean_non_interactive.rs`
- **Lines**: 59, 65, 75, 88, 148, 281, 336, 374, 415, 457, 542, 562, 605, 627, 656, 688, 762
- **Count**: 17+ subprocess spawns per test run
- **Impact**: HIGH - Each test creates repo, spawns jj, spawns zjj multiple times

```rust
// Line 59 - Creates jj repo
std::process::Command::new("jj")
    .args(["git", "init", repo_path.to_str().unwrap()])
    .status()?;
```

**Suggested Fix**: Use a shared test harness that caches initialized repos.

#### `/home/lewis/src/zjj/crates/zjj/tests/remove_non_interactive.rs`
- **Lines**: 42, 47, 57, 71, 124, 160, 199, 233, 279, 293, 331, 396, 405, 447, 500, 512, 522, 555, 586, 593, 645, 680, 710, 755, 762, 797, 832
- **Count**: 27+ subprocess spawns
- **Impact**: HIGH

#### `/home/lewis/src/zjj/crates/zjj-core/tests/concurrent_workspace_stress.rs`
- **Lines**: 166, 296, 422, 550 (cleanup spawns)
- **Pattern**: Uses `tokio::process::Command::new("jj")` for workspace cleanup
- **Impact**: MEDIUM - only for cleanup

#### `/home/lewis/src/zjj/crates/zjj/tests/bdd_integration_robustness.rs`
- **Lines**: 43, 82, 116, 160, 193, 235, 297, 334, 389, 443
- **Count**: 10 `Command::new("jj")` spawns for repo initialization
- **Impact**: HIGH - Each BDD scenario creates a fresh repo

---

## Category 2: Sleep/Timer Calls (HIGH)

### Pattern
Tests use `sleep()` for synchronization or simulating delays, which directly adds to test time.

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj/tests/test_red_queen_doctor_done_events.rs`
- **Lines**: 166, 179, 858, 865, 901, 909, 940, 950, 983, 991, 1002
- **Pattern**: Waits up to 8 seconds for event processing
- **Why slow**: Tests wait for real-time events with timeouts

```rust
// Line 865
let streamed = wait_for_line(&follow.stdout_rx, "evt-follow-1", Duration::from_secs(8));
// Line 858
thread::sleep(Duration::from_millis(1200));
```

**Suggested Fix**: Use event channels or condition variables instead of polling with sleep.

#### `/home/lewis/src/zjj/crates/zjj/tests/agent_lifecycle_integration.rs`
- **Lines**: 170 (10ms), 293 (50ms), 526 (2 seconds!)
- **Impact**: HIGH - 2-second sleep in one test

```rust
// Line 526
tokio::time::sleep(Duration::from_secs(2)).await;
```

#### `/home/lewis/src/zjj/crates/zjj/tests/test_behavioral_hostile.rs`
- **Line 83**: `std::thread::sleep(std::time::Duration::from_secs(3));`
- **Impact**: HIGH - 3-second fixed delay

#### `/home/lewis/src/zjj/crates/zjj/tests/test_submit_red_queen.rs`
- **Lines**: 722 (3ms per loop), 817 (2ms per loop)
- **Pattern**: Sleep inside loops
- **Impact**: MEDIUM - Accumulates over iterations

#### `/home/lewis/src/zjj/crates/zjj/tests/test_database_concurrency_race_conditions.rs`
- **Lines**: 63, 109, 450, 456
- **Pattern**: Sleep for 5-10ms in concurrent operations

#### `/home/lewis/src/zjj/crates/zjj-core/tests/concurrent_workspace_stress.rs`
- **Lines**: 123, 254, 374, 494, etc.
- **Pattern**: Exponential backoff sleeps (50ms, 100ms, 200ms...) for lock retries
- **Impact**: HIGH under contention

#### `/home/lewis/src/zjj/crates/zjj-core/tests/queue_stress.rs`
- **Lines**: 62, 69, 132, 256, 328, 394, etc.
- **Pattern**: Multiple small sleeps for contention handling

#### `/home/lewis/src/zjj/crates/zjj-core/tests/test_lock_concurrency_stress.rs`
- **Lines**: 431, 547, 555, 750, 980, 1059, 1151, etc.
- **Pattern**: Sleeps scattered throughout concurrent tests

---

## Category 3: Stress Tests with High Concurrency (HIGH)

### Pattern
Tests spawn large numbers of concurrent tasks (50-100+) which creates resource contention.

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj-core/tests/test_lock_concurrency_stress.rs`
- **Line 262-370**: Spawns 50 concurrent agents
- **Line 381-507**: Spawns 100 concurrent agents doing 5 lock/unlock cycles each
- **Line 517-625**: Spawns 50 agents doing 10 operations each
- **Line 722-843**: 100 agents competing for 5 sessions
- **Line 956-1033**: 8 agents with 5-second deadline
- **Impact**: VERY HIGH - Creates significant CPU and DB contention

#### `/home/lewis/src/zjj/crates/zjj-core/tests/concurrent_workspace_stress.rs`
- **Line 60**: 12 concurrent workspace creations (with retries)
- **Line 190**: 12 staggered workspace creations
- **Line 316**: 12 concurrent with 5 retries each
- **Line 442**: 20 concurrent with 5 retries each
- **Impact**: HIGH - Each workspace creation spawns jj subprocess

#### `/home/lewis/src/zjj/crates/zjj-core/tests/queue_stress.rs`
- **Line 52**: 50 agents competing for 5 entries
- **Line 126**: 20 tasks doing 10 add/remove cycles each
- **Line 160**: 10 tasks adding 10 entries each (100 total)
- **Line 307**: 20 agents with 200ms timeout
- **Impact**: MEDIUM - In-memory queue, but high task count

---

## Category 4: Temporary Directory/Repository Creation (MEDIUM)

### Pattern
Tests create temporary directories and initialize jj/git repositories, which involves filesystem operations and subprocess calls.

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj/tests/common/mod.rs`
- **Lines**: 162-217 `setup_test_repo()` function
- **Pattern**: Creates TempDir, runs `jj git init`, `zjj init`
- **Impact**: Used by most integration tests

```rust
// Lines 169-177
let output = Command::new(jj_binary)
    .args(["git", "init", repo_path.to_str().unwrap()])
    .current_dir(&temp_dir)
    .output()
    .context("Failed to run jj git init")?;
```

#### `/home/lewis/src/zjj/crates/zjj/tests/test_clean_non_interactive.rs`
- **Lines**: 54-70 `setup_test_repo()` - called in every test
- **Count**: 11 tests x repo setup = 11 full repo initializations

#### `/home/lewis/src/zjj/crates/zjj/tests/remove_non_interactive.rs`
- **Lines**: 37-52 `setup_test_repo()` - called in every test
- **Count**: 15+ tests x repo setup

#### `/home/lewis/src/zjj/crates/zjj-core/tests/common/mod.rs`
- **Lines**: 46 - `setup_test_repo()` function

---

## Category 5: Property Tests with High Case Counts (MEDIUM)

### Pattern
Property-based tests using proptest with default or high case counts.

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj-core/tests/doctor_properties.rs`
- **Lines**: 90, 205, 351, 416 - `ProptestConfig::with_cases(100)` and `with_cases(50)`
- **Impact**: 100 test cases per property, multiple properties

#### `/home/lewis/src/zjj/crates/zjj-core/tests/status_properties.rs`
- **Lines**: 116, 257, 387, 489 - `ProptestConfig::with_cases(100)`
- **Impact**: 100 test cases x 4 properties = 400 iterations

#### `/home/lewis/src/zjj/crates/zjj/tests/agent_properties.rs`
- **Line 53**: Uses `deterministic_config()` which may have default case counts
- **Impact**: 6 property tests with multiple strategies

#### `/home/lewis/src/zjj/crates/zjj/tests/task_properties.rs`
- **Line 49**: Similar property test configuration
- **Impact**: 7+ property tests

#### `/home/lewis/src/zjj/crates/zjj/tests/stack_properties.rs`
- **Multiple proptest! blocks** with complex strategies
- **Impact**: High due to recursive tree strategies

---

## Category 6: File System I/O (MEDIUM)

### Pattern
Tests perform multiple file read/write operations which are slower than in-memory operations.

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj/tests/test_submit_red_queen.rs`
- **Lines**: 100, 200, 216, 276, 292, 344, 360, 411, 475, 539, 619, 642, 692, 720, 787, 815
- **Count**: 16+ file writes per test run
- **Pattern**: Creates test files in workspaces

#### `/home/lewis/src/zjj/crates/zjj/tests/test_red_queen_doctor_done_events.rs`
- **Lines**: 193-200, 620, 648, 679, 705, 855, 898, 937, 980, 1027, 1031
- **Count**: 10+ file writes for events.jsonl manipulation

#### `/home/lewis/src/zjj/crates/zjj-core/tests/test_watcher.rs`
- **Lines**: 64, 90, 109, 152, 171, 261-263, 365, 385, 390, 466, 469, 575, 592
- **Count**: 13+ file operations
- **Pattern**: Writes to database files to trigger file watcher

---

## Category 7: Large/Complex Integration Tests (MEDIUM)

### Pattern
Single test files that contain extensive integration scenarios.

### Files Affected

#### `/home/lewis/src/zjj/crates/zjj/tests/e2e_scenarios.rs`
- **Size**: 38KB
- **Pattern**: End-to-end tests that exercise full system
- **Line 758**: 2.1-second sleep

#### `/home/lewis/src/zjj/crates/zjj/tests/atdd_object_commands.rs`
- **Size**: 69KB
- **Pattern**: ATDD tests for all object commands

#### `/home/lewis/src/zjj/crates/zjj/tests/integration_tests.rs`
- **Size**: 22KB
- **Pattern**: General integration tests

---

## Recommended Actions

### Immediate Wins (High Impact, Low Effort)

1. **Reduce sleep durations** in tests:
   - `test_behavioral_hostile.rs:83` - 3s sleep could be 100ms
   - `agent_lifecycle_integration.rs:526` - 2s sleep could be 200ms
   - Replace sleeps with proper synchronization primitives

2. **Reduce proptest case counts**:
   - Change `ProptestConfig::with_cases(100)` to `with_cases(25)` for CI
   - Use env var to control case count for local vs CI

3. **Mark stress tests as ignored by default**:
   ```rust
   #[tokio::test]
   #[ignore] // Run with: cargo test -- --ignored
   async fn stress_concurrent_workspace_creation() { ... }
   ```

### Medium-Term Improvements

4. **Create shared test fixtures**:
   - Initialize repo once per test file, not per test
   - Use `once_cell` or `std::sync::OnceLock` for shared state

5. **Use in-memory databases** where possible:
   - Already done in `queue_stress.rs` with `open_in_memory()`
   - Extend to other tests

6. **Batch contract tests**:
   - Combine multiple CLI contract tests into single process
   - Test contracts as library functions, not subprocess calls

### Long-Term Architecture

7. **Separate test tiers**:
   - Tier 1: Fast unit tests (no I/O, no subprocesses) - run on every commit
   - Tier 2: Integration tests (subprocess, file I/O) - run on PR
   - Tier 3: Stress/E2E tests - run nightly or on release

8. **Use test profiling**:
   ```bash
   cargo test -- --nocapture -Z unstable-options --format json | \
     jq 'select(.type == "test") | .name + ": " + .exec_time'
   ```

---

## Summary Table

| Category | Files Affected | Severity | Est. Time Impact |
|----------|---------------|----------|------------------|
| Subprocess Spawning | 50+ | CRITICAL | 10-100ms each |
| Sleep/Timers | 15+ | HIGH | 10ms-3s each |
| Stress Tests (100+ agents) | 5 | HIGH | 5-30s each |
| TempDir/Repo Init | 30+ | MEDIUM | 50-200ms each |
| Property Tests (100 cases) | 10+ | MEDIUM | Variable |
| File I/O | 20+ | MEDIUM | 1-10ms each |
| Large Integration Tests | 5 | MEDIUM | 5-30s each |

---

## Top 10 Slowest Test Files (Estimated)

1. `/home/lewis/src/zjj/crates/zjj-core/tests/test_lock_concurrency_stress.rs` - 100 agents, multiple tests
2. `/home/lewis/src/zjj/crates/zjj-core/tests/concurrent_workspace_stress.rs` - Subprocess + concurrency
3. `/home/lewis/src/zjj/crates/zjj/tests/test_red_queen_doctor_done_events.rs` - 8-second timeouts
4. `/home/lewis/src/zjj/crates/zjj/tests/test_behavioral_hostile.rs` - 3-second sleep
5. `/home/lewis/src/zjj/crates/zjj/tests/agent_lifecycle_integration.rs` - 2-second sleep
6. `/home/lewis/src/zjj/crates/zjj-core/tests/queue_stress.rs` - 50 concurrent agents
7. `/home/lewis/src/zjj/crates/zjj/tests/e2e_scenarios.rs` - Full E2E scenarios
8. `/home/lewis/src/zjj/crates/zjj/tests/atdd_object_commands.rs` - 69KB of tests
9. `/home/lewis/src/zjj/crates/zjj/tests/test_submit_red_queen.rs` - Multiple file I/O + race tests
10. `/home/lewis/src/zjj/crates/zjj/tests/remove_non_interactive.rs` - 27+ subprocess spawns
