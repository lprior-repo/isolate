# Rust Contract Specification: Doctor run_fixes Signature Mismatch (zjj-a7lu)

**Generated**: 2026-02-08 06:49:00 UTC
**Bead**: zjj-a7lu
**Title**: [code] Fix compilation errors in doctor.rs - run_fixes function signature mismatch
**Issue Type**: Bug fix (compilation error)

---

## Problem Statement

**Reported Issue**: Pre-existing compilation error in `crates/zjj/src/commands/doctor.rs:95`

**Error**: `run_fixes` called with 4 args but defined with 2 params (E0061)

**Investigation Findings** (2026-02-08 06:49):
Upon inspection of the current codebase:
- Line 95: `run_fixes(&checks, format, dry_run, verbose).await` (4 args)
- Line 1117-1122: `async fn run_fixes(checks: &[DoctorCheck], format: OutputFormat, dry_run: bool, verbose: bool)` (4 params)

**Current Status**: The code appears to already be fixed (signature matches call).

**Possible Explanations**:
1. Issue was already resolved in a previous commit
2. Cargo cache needs clearing
3. The bead description is outdated

**Contract Decision**: **VERIFY AND DOCUMENT**

Since the code appears correct, this contract focuses on:
1. Verifying the code compiles
2. Adding tests to prevent regression
3. Documenting the correct function signature

---

## Module Structure

**File**: `crates/zjj/src/commands/doctor.rs`

**Functions Involved**:
- `run()`: Entry point that calls `run_fixes`
- `run_fixes()`: Function that applies fixes to failed health checks

---

## Public API

**Function Signature** (Current - Correct):

```rust
/// Apply fixes to failed health checks
///
/// # Arguments
/// * `checks` - Slice of health check results
/// * `format` - Output format for fix reporting
/// * `dry_run` - If true, show what would be fixed without applying
/// * `verbose` - If true, show detailed fix output
///
/// # Returns
/// * `Result<()>` - Ok if all fixes attempted, Err on critical failure
async fn run_fixes(
    checks: &[DoctorCheck],
    format: OutputFormat,
    dry_run: bool,
    verbose: bool,
) -> Result<()>
```

**Call Site** (Line 95):

```rust
if fix {
    run_fixes(&checks, format, dry_run, verbose).await
} else {
    show_health_report(&checks, format)
}
```

---

## Type Changes

**No changes needed** - signature is already correct.

---

## CLI Changes

**No changes** - CLI handler passes correct parameters:

```rust
// crates/zjj/src/cli/handlers.rs
let fix = matches.get_flag("fix");
let dry_run = matches.get_flag("dry-run");
let verbose = matches.get_flag("verbose");

doctor::run(format, fix, dry_run, verbose).await
```

---

## Error Types

**No new errors** - this is a signature verification task.

---

## Performance Constraints

**Not applicable** - no logic changes.

---

## Testing Requirements

### Compilation Test (Critical):

```bash
#!/bin/bash
# Verify compilation succeeds

set -euo pipefail

echo "Testing doctor.rs compilation..."

# Clean build to ensure no cached artifacts
cargo clean -p zjj

# Attempt to build zjj crate
if cargo build --bin zjj 2>&1 | tee /tmp/build.log; then
    echo "✓ zjj builds successfully"
else
    echo "✗ Compilation failed"
    echo "Checking for E0061 errors..."
    if grep -q "E0061" /tmp/build.log; then
        echo "Found function signature mismatch (E0061)"
        grep -B 2 -A 2 "E0061" /tmp/build.log
    fi
    exit 1
fi
```

### Unit Tests Required:

```rust
#[cfg(test)]
mod run_fixes_tests {
    use super::*;

    #[tokio::test]
    async fn run_fixes_accepts_four_parameters() {
        // This test verifies the function signature
        // If signature changes, this won't compile

        let checks = vec![
            DoctorCheck {
                name: "test-check".to_string(),
                status: CheckStatus::Failed,
                message: "Test failure".to_string(),
                auto_fixable: true,
                fix_fn: None,
            }
        ];

        // This should compile with 4 parameters
        let result = run_fixes(&checks, OutputFormat::Text, true, false).await;

        // We don't care about result, just that it compiles
        let _ = result;
    }

    #[tokio::test]
    async fn run_calls_run_fixes_with_correct_args() {
        // Integration test: verify run() correctly calls run_fixes
        let format = OutputFormat::Text;
        let dry_run = true;
        let verbose = false;

        // This should not panic or fail to compile
        let result = run(format, true, dry_run, verbose).await;

        // Should succeed (dry_run mode)
        assert!(result.is_ok());
    }
}
```

### Regression Test:

```rust
#[tokio::test]
async fn run_fixes_signature_matches_call_site() {
    // Property: Function signature must match all call sites

    // If this test compiles, the signature is correct
    let checks: Vec<DoctorCheck> = vec![];

    // Call with exact arguments from line 95
    let _ = run_fixes(&checks, OutputFormat::Text, true, false).await;

    // If we get here without compilation error, signature is correct
}
```

---

## Migration Guide

**Not applicable** - no breaking changes.

---

## Implementation Checklist

- [ ] Verify `cargo build --bin zjj` succeeds
- [ ] Verify `moon run :build` succeeds
- [ ] Add test verifying `run_fixes` accepts 4 parameters
- [ ] Add test verifying `run()` calls `run_fixes` correctly
- [ ] Run `moon run :test doctor` (all tests pass)
- [ ] Run `moon run :ci` (full pipeline)
- [ ] Document function signature in code comments (if not present)
- [ ] Close bead if already fixed, or fix if still broken

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let count = checks.len().expect("non-empty");

// ✅ REQUIRED
let count = checks.len();
```

**In run_fixes implementation**:
```rust
// Current code (line 1134) - already correct:
let (fixed, unable_to_fix) = futures::stream::iter(checks)
    .fold(
        (vec![], vec![]),
        |(mut fixed, mut unable), check| async move {
            // ... fix logic
            (fixed, unable)
        },
    )
    .await;
```

---

## Success Criteria

1. Code compiles without E0061 errors
2. `run_fixes` function has 4 parameters matching call site
3. All tests pass
4. `moon run :ci` succeeds

---

## Investigation Notes

**Question**: Why does this bead exist if the code appears correct?

**Possible Answers**:
1. **Already Fixed**: The issue was resolved in commit `3335c68c` (feat: Add --dry-run and --verbose flags to doctor command)
   - That commit likely fixed the signature when adding `dry_run` and `verbose` parameters

2. **Stale Bead**: Bead was created before the fix, not updated after

3. **Cache Issue**: User needs to run `cargo clean`

**Recommendation**: Verify build succeeds, then close bead as "Already Fixed".

---

## Verification Steps

Before closing bead:

```bash
# 1. Clean build
cargo clean -p zjj

# 2. Verify compilation
cargo build --bin zjj

# 3. Run tests
moon run :test doctor

# 4. Full CI
moon run :ci

# 5. If all pass, close bead with note: "Already fixed in commit 3335c68c"
```

---

**Contract Status**: ⚠️ Investigation Required

**Estimated Resolution Time**: 10 minutes (verify compilation and close)

**Risk Level**: Low (appears already fixed)
