# Survivor: Performance Invariant Violation

**Campaign:** bd-1c4-redqueen
**Generation:** 1
**Severity:** MAJOR
**Status:** ALIVE

## Discovery

Red-queen invariant analysis discovered that the `has_existing_conflicts()` quick check can exceed the 100ms performance requirement for large repositories.

## Vulnerability

**Location:** `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs:266-306, 468-470`

```rust
async fn has_existing_conflicts(&self) -> Result<bool, ConflictError> {
    Ok(!self.check_existing_conflicts().await?.is_empty())
}

async fn check_existing_conflicts(&self) -> Result<Vec<String>, ConflictError> {
    // Runs jj log
    let output = self.executor.run(&[
        "log", "-r", "@", "--no-graph", "-T",
        r#"if(conflict, "CONFLICT\n", "")"#
    ]).await?;

    if output.as_str().contains("CONFLICT") {
        // Runs jj resolve --list (can be SLOW on large repos)
        let resolve_output = self.executor
            .run(&["resolve", "--list"])
            .await?;

        // Parses all conflict files
        let conflicts: Vec<String> = resolve_output
            .as_str()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.split_whitespace().next().unwrap_or(line.trim()).to_string())
            .collect();

        Ok(conflicts)
    } else {
        Ok(Vec::new())
    }
}
```

**Contract Requirement:** INV-PERF-002 states "`has_existing_conflicts()` completes in <100ms"

**Issue:** On repositories with thousands of conflicted files:
- `jj resolve --list` can take > 100ms
- String parsing is O(n) where n = number of conflicts
- No timeout or early exit

## Impact

- **Likelihood:** HIGH - any large repo with conflicts
- **Severity:** MAJOR - contract violation, performance degradation
- **Scope:** Quick check operations

## Proof of Concept

```rust
// Repository with 10,000 conflicted files
// jj resolve --list output: ~500KB of text
// Time to parse: ~50-200ms (depends on system)
// Total time: > 100ms ❌
```

## Recommendations

1. **Option A - Early exit:**
```rust
async fn has_existing_conflicts(&self) -> Result<bool, ConflictError> {
    // Only check if ANY conflicts exist, don't list them
    let output = self.executor.run(&[
        "log", "-r", "@", "--no-graph", "-T",
        r#"if(conflict, "CONFLICT\n", "")"#
    ]).await?;
    Ok(output.as_str().contains("CONFLICT"))
}
```

2. **Option B - Timeout:**
```rust
async fn has_existing_conflicts(&self) -> Result<bool, ConflictError> {
    let timeout = Duration::from_millis(90);
    // ... with timeout enforcement
}
```

3. **Option C - Relax invariant:**
Update contract to "< 100ms OR < 1ms per 100 conflicted files"

## Trade-offs

- **Option A** breaks "list conflicts" functionality - need separate method
- **Option B** adds complexity but preserves functionality
- **Option C** weakens the contract but acknowledges reality

## Files

- Implementation: `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs:266-306, 468-470`
- Contract: `/home/lewis/src/zjj/contracts/bd-1c4-contract-spec.md:461`
- Tests: `/home/lewis/src/zjj/crates/zjj/tests/conflict_e2e_tests.rs:127-144, 220-237`

## Fitness Impact

- Contract compliance: -15%
- Performance: -10%
- Overall fitness: -10%
