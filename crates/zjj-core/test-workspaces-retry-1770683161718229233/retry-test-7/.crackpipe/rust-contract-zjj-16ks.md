# Rust Contract Specification: Config Concurrent Write Data Loss (zjj-16ks)

**Generated**: 2026-02-08 07:05:00 UTC
**Bead**: zjj-16ks
**Title**: config: Fix concurrent write 90% data loss
**Issue Type**: Bug fix (critical - MASSIVE DATA LOSS)

---

## Problem Statement

**Reported Issue**: Parallel writes to configuration result in 90% data loss.

**Test Case**:
```bash
# Run 20 parallel config writes
for i in {1..20}; do
  zjj config test_key_$i "value_$i" &
done
wait

# Expected: 20 keys in config
# Actual: Only 2 keys - 90% data loss!
```

**Impact**:
- MASSIVE DATA LOSS
- Configuration cannot be set programmatically
- Automated configuration setup fails
- Race condition in file writing

**Root Cause**:
The `set_config_value()` function in `config.rs` uses a read-modify-write pattern without file locking:
1. Read existing config
2. Parse TOML
3. Modify in-memory document
4. Write back to file

**When multiple processes do this simultaneously**:
- Process A reads config (has keys 1-10)
- Process B reads config (has keys 1-10)
- Process A writes (has keys 1-11)
- Process B writes (has keys 1-12) → **overwrites A's change!**

---

## Module Structure

**Primary File**: `crates/zjj/src/commands/config.rs`

**Problematic Function**:
```rust
async fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()> {
    // RACE CONDITION HERE!
    let mut doc = if tokio::fs::try_exists(config_path).await.is_ok_and(|v| v) {
        let content = tokio::fs::read_to_string(config_path).await?;  // READ
        content.parse::<toml_edit::DocumentMut>()?
    } else {
        toml_edit::DocumentMut::new()
    };

    set_nested_value(&mut doc, &parts, value)?;

    tokio::fs::write(config_path, doc.to_string()).await?;  // WRITE - OVERWRITES!
    Ok(())
}
```

---

## Public API

**Current Signature**:
```rust
async fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()>
```

**Required Fix**: Add file locking or use append-only database.

---

## Type Changes

**Option A: File Locking** (Minimal change)
```rust
use fs4::tokio::AsyncFileExt;

async fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()> {
    // Open with exclusive lock
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(config_path)
        .await?;

    file.lock_exclusive().await?;  // CRITICAL: Acquire lock

    // ... rest of logic ...

    file.unlock().await?;  // CRITICAL: Release lock
    Ok(())
}
```

**Option B: SQLite Backend** (Recommended)
- Migrate config to SQLite database
- Use transactions for atomic writes
- Built-in concurrency handling

---

## CLI Changes

**No CLI argument changes** - behavior fix only.

**Expected Behavior After Fix**:
```bash
# Run 20 parallel writes
for i in {1..20}; do
  zjj config test_key_$i "value_$i" &
done
wait

# Verify: All 20 keys present
COUNT=$(zjj config --json | jq 'keys | length')
[ $COUNT -eq 20 ]  # Should pass
```

---

## Error Types

**New Error Required**:
```rust
pub enum ConfigError {
    LockTimeout(Duration),  // Could not acquire file lock
    LockPoisoned,           // Lock was poisoned (crashed while holding)
    // ...
}
```

---

## Performance Constraints

**Lock Timeout**: 5 seconds max
- If lock cannot be acquired in 5s, fail with error
- Prevents indefinite hangs

**Throughput**: Must support 100 writes/second
- Parallel writes should serialize safely
- No data loss under any concurrency pattern

---

## Testing Requirements

See `.crackpipe/martin-fowler-tests-zjj-16ks.md` for detailed test plan.

Key tests:
- Concurrent writes no data loss (20, 50, 100 parallel writers)
- Lock timeout works
- No deadlocks under any load
- Data integrity under mixed read/write

---

## Implementation Checklist

- [ ] Add `fs4` or `tokio-file-lock` dependency
- [ ] Implement file locking in `set_config_value()`
- [ ] Add lock timeout logic
- [ ] Handle lock poisoning
- [ ] Add concurrency tests
- [ ] Add stress tests
- [ ] Verify no data loss under load
- [ ] Update documentation

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let file = tokio::fs::File::create(path).await.unwrap();

// ✅ REQUIRED
let file = tokio::fs::File::create(path).await
    .map_err(|e| anyhow::anyhow!("Failed to create config file: {e}"))?;
```

**With locking**:
```rust
// ✅ CORRECT: Handle lock errors
file.lock_exclusive().await
    .map_err(|e| ConfigError::LockTimeout(e.to_string()))?;
```

---

## Success Criteria

1. 100 concurrent writes result in 100 keys (0% data loss)
2. Lock acquisition times out after 5 seconds
3. No deadlocks under any concurrency pattern
4. All tests pass
5. `moon run :ci` succeeds

---

## Verification Steps

Before closing bead:

```bash
# 1. Run concurrency test
./test/integration/config_concurrency_stress.sh

# 2. Verify no data loss
KEYS=$(zjj config --json | jq 'keys | length')
[ $KEYS -eq 100 ]

# 3. Run unit tests
moon run :test config

# 4. Full CI
moon run :ci
```

---

## Related Beads

- zjj-14hr: Fix exit codes always zero on error
- zjj-2d4m: Fix config set creates invalid TOML
- zjj-37dt: Add config delete command

---

**Contract Status**: Ready for Implementation

**Estimated Resolution Time**: 2 hours (add locking + tests)

**Risk Level**: Medium (file locking can have edge cases)
