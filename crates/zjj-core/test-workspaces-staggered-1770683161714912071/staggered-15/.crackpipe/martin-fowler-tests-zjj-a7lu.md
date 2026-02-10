# Martin Fowler Test Plan: Doctor run_fixes Signature (zjj-a7lu)

**Generated**: 2026-02-08 06:49:30 UTC
**Bead**: zjj-a7lu
**Contract**: `.crackpipe/rust-contract-zjj-a7lu.md`
**Issue Type**: Bug fix (compilation error - likely already fixed)

---

## Test Strategy

Since this is a **compilation error verification**, our test strategy focuses on:

1. **Build Verification**: Ensure code compiles
2. **Signature Tests**: Verify function signature matches call sites
3. **Integration Tests**: Verify doctor command works end-to-end

**Martin Fowler Principles Applied**:
- **Minimal Testing**: Focus on what matters (compilation)
- **Clear Intent**: Tests verify signature compatibility
- **No Mocking**: Real compilation check

---

## Test Categories

### 1. Compilation Verification (Critical)

**Purpose**: Verify code compiles without E0061 errors.

```bash
#!/bin/bash
# test/integration/compilation/doctor_build_test.sh

set -euo pipefail

echo "Testing doctor.rs compilation..."

# Clean build to ensure no cached artifacts
echo "Step 1: Clean build cache"
cargo clean -p zjj

# Attempt to build zjj crate
echo "Step 2: Build zjj binary"
if cargo build --bin zjj 2>&1 | tee /tmp/build.log; then
    echo "✓ zjj builds successfully"
else
    echo "✗ Compilation failed"
    exit 1
fi

# Check for E0061 errors specifically
echo "Step 3: Check for function signature errors"
if grep -q "E0061" /tmp/build.log; then
    echo "✗ Found function signature mismatch (E0061)"
    grep -B 2 -A 2 "E0061" /tmp/build.log
    exit 1
else
    echo "✓ No E0061 errors found"
fi

# Verify using Moon build system
echo "Step 4: Verify with Moon"
if moon run :build; then
    echo "✓ Moon build succeeds"
else
    echo "✗ Moon build failed"
    exit 1
fi

echo "✓ All compilation checks passed"
```

**Fowler's Classification**: **Compilation Test**
- Verifies code compiles
- No runtime behavior tested
- Fast feedback (compile-time)

**Test Smell Avoided**:
- ❌ Testing implementation details
- ✅ Testing compilation success

---

### 2. Signature Compatibility Tests

**Purpose**: Verify `run_fixes` signature matches call sites.

```rust
#[cfg(test)]
mod signature_tests {
    use super::*;
    use crate::commands::doctor::{run, run_fixes, DoctorCheck, CheckStatus};
    use crate::core::output_format::OutputFormat;

    #[tokio::test]
    async fn run_fixes_accepts_four_parameters() {
        // This test verifies the function signature at compile time
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

        // This call must match the function signature
        // Parameters: checks: &[DoctorCheck], format: OutputFormat, dry_run: bool, verbose: bool
        let result = run_fixes(&checks, OutputFormat::Text, true, false).await;

        // We don't assert on result, just that it compiles
        let _ = result;
    }

    #[tokio::test]
    async fn run_calls_run_fixes_with_four_args() {
        // Integration test: verify run() correctly calls run_fixes
        let format = OutputFormat::Text;
        let dry_run = true;
        let verbose = false;
        let fix = true;

        // This is the exact call pattern from line 95 of doctor.rs
        // If signature mismatches, this won't compile
        let result = run(format, fix, dry_run, verbose).await;

        // Should succeed (dry_run mode)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_fixes_all_parameter_combinations() {
        let checks: Vec<DoctorCheck> = vec![];

        // Test all combinations of bool parameters
        let combinations = [
            (true, true),   // dry_run=true, verbose=true
            (true, false),  // dry_run=true, verbose=false
            (false, true),  // dry_run=false, verbose=true
            (false, false), // dry_run=false, verbose=false
        ];

        for (dry_run, verbose) in combinations {
            // If this compiles for all combinations, signature is correct
            let _ = run_fixes(&checks, OutputFormat::Text, dry_run, verbose).await;
        }
    }
}
```

**Fowler's Classification**: **Compile-Time Test**
- Signature verified at compile time
- If wrong, test won't compile (not fail at runtime)
- Zero runtime overhead

**Test Smell Avoided**:
- ❌ Testing parameter names (implementation detail)
- ✅ Testing parameter types (interface contract)

---

### 3. Integration Tests

**Purpose**: Verify doctor command works end-to-end.

```bash
#!/bin/bash
# test/integration/doctor_command_test.sh

set -euo pipefail

echo "Testing doctor command integration..."

# Build zjj
echo "Step 1: Build zjj"
moon run :build

# Test: doctor command runs without crashing
echo "Step 2: Run doctor command"
if zjj doctor; then
    echo "✓ doctor command succeeds"
else
    echo "✗ doctor command failed"
    exit 1
fi

# Test: doctor --fix (dry-run mode)
echo "Step 3: Run doctor with --fix --dry-run"
if zjj doctor --fix --dry-run; then
    echo "✓ doctor --fix --dry-run succeeds"
else
    echo "✗ doctor --fix --dry-run failed"
    exit 1
fi

# Test: doctor --verbose
echo "Step 4: Run doctor with --verbose"
if zjj doctor --verbose; then
    echo "✓ doctor --verbose succeeds"
else
    echo "✗ doctor --verbose failed"
    exit 1
fi

# Test: doctor --fix --dry-run --verbose (all flags)
echo "Step 5: Run doctor with all flags"
if zjj doctor --fix --dry-run --verbose; then
    echo "✓ doctor with all flags succeeds"
else
    echo "✗ doctor with all flags failed"
    exit 1
fi

# Test: doctor --json (output format)
echo "Step 6: Run doctor with JSON output"
OUTPUT=$(zjj doctor --json)
if echo "$OUTPUT" | jq . >/dev/null 2>&1; then
    echo "✓ doctor --json outputs valid JSON"
else
    echo "✗ doctor --json output is invalid"
    exit 1
fi

echo "✓ All integration tests passed"
```

**Fowler's Classification**: **State Verification** (Classical)
- Testing end-to-end behavior
- Real CLI invocation
- No mocking

---

### 4. Regression Tests

**Purpose**: Prevent future signature mismatches.

```rust
#[tokio::test]
async fn regression_test_run_fixes_signature_stable() {
    // This test prevents accidental signature changes
    // If someone adds/removes parameters, this won't compile

    use crate::commands::doctor::{run_fixes, DoctorCheck, CheckStatus};
    use crate::core::output_format::OutputFormat;

    let checks: Vec<DoctorCheck> = vec![];

    // Expected signature:
    // async fn run_fixes(checks: &[DoctorCheck], format: OutputFormat, dry_run: bool, verbose: bool) -> Result<()>

    // If this compiles, signature is as expected
    let _ = run_fixes(&checks, OutputFormat::Text, false, false).await;
}
```

**Fowler's Classification**: **Regression Test**
- Prevents breaking changes
- Compile-time verification

---

## Test Doubles Strategy

**No test doubles needed** - all tests are real:
- Real compilation
- Real function calls
- Real CLI execution

---

## Test Coverage Targets

| Metric Type | Target | Rationale |
|-------------|--------|-----------|
| **Compilation** | 100% | Must compile without errors |
| **Signature** | 100% | All parameter combinations tested |
| | | |
| **Specific Coverage** | | |
| `run_fixes` call sites | 100% | All calls tested |
| `run()` function | 100% | Integration test covers |
| CLI handler | 100% | End-to-end test covers |

---

## Test Smells to Avoid

### 1. **Testing Implementation Details**

❌ **Bad**: Testing parameter names
```rust
function_param_names!(run_fixes, ["checks", "format", "dry_run", "verbose"]);
```

✅ **Good**: Testing parameter types
```rust
run_fixes(&checks, OutputFormat::Text, true, false).await;
```

### 2. **Over-Testing**

❌ **Bad**: Testing every possible combination
```rust
for format in [Text, Json] {
    for dry_run in [true, false] {
        for verbose in [true, false] {
            // 6 combinations, excessive
        }
    }
}
```

✅ **Good**: Testing representative combinations
```rust
let combinations = [(true, true), (true, false), (false, true), (false, false)];
```

---

## Regression Test Checklist

Before closing bead:

- [ ] Code compiles without errors
- [ ] No E0061 errors in build output
- [ ] `run_fixes` signature tests compile
- [ ] Integration tests pass
- [ ] `moon run :test doctor` passes
- [ ] `moon run :ci` passes
- [ ] Manual verification: `zjj doctor --help` works
- [ ] Manual verification: `zjj doctor` runs successfully

---

## Continuous Integration (CI) Configuration

Add to `.github/workflows/test.yml`:

```yaml
name: Doctor Compilation Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon-action@v1

      - name: Verify compilation
        run: |
          cargo clean -p zjj
          cargo build --bin zjj

      - name: Check for E0061 errors
        run: |
          ! cargo build --bin zjj 2>&1 | grep -q "E0061"

      - name: Run doctor tests
        run: moon run :test doctor

      - name: Integration test
        run: ./test/integration/doctor_command_test.sh
```

---

## Manual Testing Checklist

Before closing bead:

- [ ] Run `cargo clean -p zjj && cargo build --bin zjj`
- [ ] Run `zjj doctor` - should work
- [ ] Run `zjj doctor --fix --dry-run` - should work
- [ ] Run `zjj doctor --verbose` - should work
- [ ] Run `zjj doctor --json` - should output valid JSON
- [ ] Check build log for E0061 errors

---

## Post-Deployment Monitoring

After merging:

1. **Build failures**: Watch for E0061 errors in CI
2. **User reports**: "doctor command broken"
3. **Future changes**: If someone modifies `run_fixes`, this test catches signature mismatches

---

## Investigation Notes

**If tests pass and code compiles**:
- Bead was likely already fixed
- Check git history for commit `3335c68c`
- Close bead with note: "Already fixed"

**If tests fail**:
- Investigate why signature mismatches
- Fix signature or call site
- Re-run tests

---

## Summary

**Test Approach**: Compilation verification + Signature tests + Integration tests

**Test Count**: ~10 tests
- 1 compilation test (bash)
- 3 signature compatibility tests (compile-time)
- 6 integration tests (bash)

**Execution Time**: <30 seconds (mostly compilation time)

**Risk Coverage**: High (catches signature mismatches)

**Fowler Compliance**: ✅
- ✅ Minimal testing (focus on what matters)
- ✅ Clear intent (tests verify compilation)
- ✅ No test smells (no over-testing)

---

**Test Plan Status**: ✅ Ready for Verification

**Estimated Test Execution Time**: 5 minutes (mostly compilation)

**Confidence Level**: High (simple verification)
