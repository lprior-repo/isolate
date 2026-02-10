# Martin Fowler Test Plan: Config Concurrent Write Data Loss (zjj-16ks)

**Generated**: 2026-02-08 07:05:30 UTC
**Bead**: zjj-16ks
**Contract**: `.crackpipe/rust-contract-zjj-16ks.md`
**Issue Type**: Bug fix (critical - 90% data loss)

---

## Test Strategy

Since this is a **race condition bug**, our test strategy focuses on:

1. **Concurrency Testing**: Multiple simultaneous writes
2. **Data Integrity Verification**: All writes must persist
3. **Stress Testing**: High-load scenarios
4. **Lock Testing**: Timeout and contention handling

**Martin Fowler Principles Applied**:
- **State Verification**: Verify all keys present after concurrent writes
- **No Mocking**: Real file operations
- **Thread Safety**: Tests must catch race conditions
- **Minimal Testing**: Focus on what matters (data loss)

---

## Test Categories

### 1. Concurrency Stress Tests (Critical)

**Purpose**: Verify no data loss under concurrent writes.

```bash
#!/bin/bash
# test/integration/config_concurrency_stress.sh

set -euo pipefail

echo "=== Config Concurrency Stress Test ==="

TEMP_DIR=$(mktemp -d)
CONFIG="$TEMP_DIR/config.toml"

# Set up environment
export HOME="$TEMP_DIR"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "Test 1: 20 parallel writes (reproduces reported bug)"
for i in {1..20}; do
  zjj config test_key_$i "value_$i" >/dev/null 2>&1 &
done
wait

# Count keys
KEY_COUNT=$(zjj config --json 2>/dev/null | jq 'keys | length' || echo "0")

if [ $KEY_COUNT -eq 20 ]; then
    echo "✓ PASS: All 20 keys preserved (0% data loss)"
else
    echo "✗ FAIL: Expected 20 keys, got $KEY_COUNT"
    echo "Data loss: $((100 - KEY_COUNT * 100 / 20))%"
    exit 1
fi

echo ""
echo "Test 2: 100 parallel writes (stress test)"
rm -f "$CONFIG"

for i in {1..100}; do
  zjj config stress_key_$i "value_$i" >/dev/null 2>&1 &
done
wait

KEY_COUNT=$(zjj config --json 2>/dev/null | jq 'keys | length' || echo "0")

if [ $KEY_COUNT -eq 100 ]; then
    echo "✓ PASS: All 100 keys preserved (0% data loss)"
else
    echo "✗ FAIL: Expected 100 keys, got $KEY_COUNT"
    DATA_LOSS=$((100 - KEY_COUNT))
    echo "Data loss: $DATA_LOSS%"
    exit 1
fi

echo ""
echo "=== All concurrency tests passed ==="
```

---

### 2. Unit Tests for File Locking

```rust
#[cfg(test)]
mod file_locking_tests {
    use super::*;

    #[tokio::test]
    async fn concurrent_writes_respect_file_lock() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut tasks = Vec::new();
        for i in 0..10 {
            let path = config_path.clone();
            let task = tokio::spawn(async move {
                let key = format!("key_{}", i);
                set_config_value(&path, &key, &format!("value_{}", i)).await
            });
            tasks.push(task);
        }

        for task in tasks {
            task.await.unwrap().unwrap();
        }

        // Verify all keys present
        let content = tokio::fs::read_to_string(&config_path).await.unwrap();
        for i in 0..10 {
            assert!(content.contains(&format!("key_{}", i)));
        }
    }
}
```

---

### 3. Performance Tests

```rust
#[tokio::test]
async fn sequential_writes_performance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let start = Instant::now();
    for i in 0..100 {
        let key = format!("perf_key_{}", i);
        set_config_value(&config_path, &key, &format!("value_{}", i)).await
            .expect("Write should succeed");
    }
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs() < 10, "100 writes should take < 10s");
}
```

---

## Test Coverage Targets

| Metric Type | Target |
|-------------|--------|
| Concurrency Coverage | 100% |
| Data Integrity | 100% |
| Lock Coverage | 100% |

---

## Summary

**Test Approach**: Concurrency stress tests + Data integrity verification

**Test Count**: ~15 tests
- 3 stress tests (bash)
- 8 unit tests (rust)
- 4 regression tests (rust)

**Execution Time**: ~60 seconds

---

**Test Plan Status**: ✅ Ready for Implementation
