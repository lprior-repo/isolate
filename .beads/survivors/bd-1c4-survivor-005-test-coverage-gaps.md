# Survivor: Test Coverage Gaps

**Campaign:** bd-1c4-redqueen
**Generation:** 1
**Severity:** MAJOR
**Status:** ALIVE

## Discovery

Red-queen test analysis discovered significant gaps in the E2E test coverage that could hide bugs.

## Vulnerabilities

### 1. Silent Test Skipping

**Location:** `/home/lewis/src/zjj/crates/zjj/tests/conflict_e2e_tests.rs:44-46, 58-59`

```rust
let Some(harness) = common::TestHarness::try_new() else {
    return;  // SILENTLY skips test
};
```

**Issue:** If JJ is unavailable, ALL tests silently return without any indication that they were skipped. No summary of how many tests ran vs skipped.

**Impact:**
- CI/CD could pass even if 0 tests actually ran
- No visibility into test execution rate
- Impossible to detect if test environment is broken

### 2. No JSON Type Verification

**Location:** `/home/lewis/src/zjj/crates/zjj/tests/conflict_e2e_tests.rs:86-93`

```rust
assert!(json.get("overlapping_files").is_some());
```

**Issue:** Tests check if fields exist but not their types. What if:
- `overlapping_files` is a string instead of array?
- `detection_time_ms` is a string instead of number?
- `has_existing_conflicts` is a string instead of boolean?

**Impact:** JSON consumers would crash at runtime due to type mismatches.

### 3. No Adversarial Input Testing

**Missing Tests:**
- Unicode file names (partially addressed in ADV-004)
- Very long file paths (> 255 chars)
- Special characters in paths (`\n`, `\t`, `\0`, control codes)
- Paths with " -> " in the name (SURVIVOR-002)
- Empty vs null JSON fields
- Malformed JSON from JJ commands

**Impact:** Edge case bugs remain undetected.

### 4. No Concurrency Testing

**Missing:**
- Concurrent conflict detection requests
- Race conditions in workspace state
- Lock timeout behavior
- Multiple agents accessing same workspace

**Impact:** INV-CONC-* invariants untested.

### 5. Performance Testing Inadequate

**Location:** `/home/lewis/src/zjj/crates/zjj/tests/conflict_e2e_tests.rs:117-122`

```rust
let diff = (detection_ms as i64 - external_ms as i64).abs();
assert!(diff < 100);  // 100ms tolerance!
```

**Issue:** 100ms tolerance is too loose to catch timing bugs. Also doesn't test:
- Repositories with 10,000 files (INV-PERF-001)
- Systems under load
- Cold cache vs warm cache

## Recommendations

1. **Add test execution tracking:**
```rust
#[test]
fn verify_tests_ran() {
    assert!(jj_is_available(), "JJ is not available - cannot run tests");
}
```

2. **Add JSON type checking:**
```rust
fn verify_json_schema(json: &JsonValue) {
    assert!(json["overlapping_files"].is_array());
    assert!(json["detection_time_ms"].is_u64());
    assert!(json["has_existing_conflicts"].is_boolean());
    // ... etc
}
```

3. **Add adversarial test suite:**
- Property-based testing with `proptest`
- Fuzz testing with arbitrary inputs
- Boundary value testing

4. **Add concurrency tests:**
```rust
#[tokio::test]
async fn concurrent_detection() {
    let detector = create_detector();
    let results = futures::join!(
        detector.detect_conflicts(),
        detector.detect_conflicts(),
        detector.detect_conflicts()
    );
    // All should return consistent results
}
```

5. **Enhance performance testing:**
```rust
#[tokio::test]
async fn performance_with_10k_files() {
    let repo = create_repo_with_n_files(10_000);
    let start = Instant::now();
    let result = detector.detect_conflicts().await.unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(5));
}
```

## Files

- Tests: `/home/lewis/src/zjj/crates/zjj/tests/conflict_e2e_tests.rs`
- Adversarial: `/home/lewis/src/zjj/crates/zjj/tests/conflict_adversarial_tests.rs`

## Fitness Impact

- Test coverage: -20%
- Confidence in correctness: -15%
- Overall fitness: -12%
