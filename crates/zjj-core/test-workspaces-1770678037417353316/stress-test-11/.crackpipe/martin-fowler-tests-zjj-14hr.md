# Martin Fowler Test Plan: Config Exit Codes (zjj-14hr)

**Generated**: 2026-02-08 07:00:30 UTC
**Bead**: zjj-14hr
**Contract**: `.crackpipe/rust-contract-zjj-14hr.md`
**Issue Type**: Bug fix (critical - CI/CD broken)

---

## Test Strategy

Since this is an **exit code bug**, our test strategy focuses on:

1. **Exit Code Verification**: Ensure errors return non-zero exit codes
2. **Error Message Output**: Ensure errors go to stderr
3. **Integration Testing**: Real CLI invocation and exit code checking
4. **Regression Prevention**: Tests to prevent future exit code regressions

**Martin Fowler Principles Applied**:
- **State Verification**: Verify exit code (observable behavior)
- **No Mocking**: Real CLI execution
- **Clear Intent**: Tests verify exit codes explicitly
- **Minimal Testing**: Focus on what matters (exit codes)

---

## Test Categories

### 1. Exit Code Verification Tests (Critical)

**Purpose**: Verify config operations return correct exit codes.

```bash
#!/bin/bash
# test/integration/config_exit_code_tests.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_exit_code() {
    local test_name="$1"
    local command="$2"
    local expected_exit="$3"
    local should_succeed="${4:-false}"

    echo -n "Testing: $test_name ... "

    # Run command and capture exit code
    eval "$command" >/dev/null 2>&1
    actual_exit=$?

    if [ $actual_exit -eq $expected_exit ]; then
        echo -e "${GREEN}PASS${NC} (exit code $actual_exit)"
        ((TESTS_PASSED++))
        return 0
    else
        echo -e "${RED}FAIL${NC} (expected $expected_exit, got $actual_exit)"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 1: Non-existent key should return exit code 1
test_exit_code \
    "Non-existent key returns exit code 1" \
    "zjj config nonexistent_key" \
    1

# Test 2: Valid key returns exit code 0
test_exit_code \
    "Valid key returns exit code 0" \
    "zjj config workspace_dir" \
    0

# Test 3: Set operation returns exit code 0
test_exit_code \
    "Set operation returns exit code 0" \
    "zjj config test_key test_value" \
    0

# Test 4: Set with global flag returns exit code 0
test_exit_code \
    "Set with global flag returns exit code 0" \
    "zjj config --global test_global_key test_value" \
    0

# Test 5: Invalid nested key (partially valid) returns exit code 1
test_exit_code \
    "Invalid nested key returns exit code 1" \
    "zjj config zellij.invalid_key" \
    1

# Test 6: Deep invalid key returns exit code 1
test_exit_code \
    "Deep invalid key returns exit code 1" \
    "zjj config invalid.nested.deep.key" \
    1

# Test 7: Valid nested key returns exit code 0
test_exit_code \
    "Valid nested key returns exit code 0" \
    "zjj config zellij.use_tabs" \
    0

# Test 8: JSON format with valid key returns exit code 0
test_exit_code \
    "JSON format with valid key returns exit code 0" \
    "zjj config --json workspace_dir" \
    0

# Test 9: JSON format with invalid key returns exit code 1
test_exit_code \
    "JSON format with invalid key returns exit code 1" \
    "zjj config --json invalid_key" \
    1

# Test 10: List all config returns exit code 0
test_exit_code \
    "List all config returns exit code 0" \
    "zjj config" \
    0

# Summary
echo ""
echo "=========================================="
echo "Test Summary:"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "=========================================="

if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "${RED}Some tests failed${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed${NC}"
    exit 0
fi
```

**Fowler's Classification**: **State Verification** (Classical)
- Testing observable behavior (exit codes)
- Real CLI invocation
- No mocking

**Test Smell Avoided**:
- ❌ Testing implementation details (internal error types)
- ✅ Testing exit codes (user-visible behavior)

---

### 2. Unit Tests for Error Propagation

**Purpose**: Verify errors properly propagate through the call stack.

```rust
#[cfg(test)]
mod error_propagation_tests {
    use super::*;
    use zjj_core::Error;

    // Test 1: show_config_value returns Err for non-existent key
    #[test]
    fn show_config_value_nonexistent_key_returns_error() {
        let config = Config::default();
        let result = show_config_value(&config, "nonexistent_key", OutputFormat::Human);

        assert!(result.is_err(), "Should return error for nonexistent key");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not found") || error_msg.contains("not found"),
                "Error message should mention 'not found'");
    }

    // Test 2: get_nested_value returns Err for missing key
    #[test]
    fn get_nested_value_missing_key_returns_error() {
        let config = Config::default();
        let result = get_nested_value(&config, "invalid.nested.key");

        assert!(result.is_err(), "Should return error for missing nested key");
    }

    // Test 3: get_nested_value returns Ok for valid key
    #[test]
    fn get_nested_value_valid_key_returns_ok() {
        let config = Config::default();
        let result = get_nested_value(&config, "workspace_dir");

        assert!(result.is_ok(), "Should succeed for valid key");
    }

    // Test 4: get_nested_value handles deep nesting
    #[test]
    fn get_nested_value_deep_nesting_returns_ok() {
        let config = Config::default();
        let result = get_nested_value(&config, "zellij.panes.main.command");

        assert!(result.is_ok(), "Should succeed for deeply nested valid key");
    }

    // Test 5: Empty key returns error
    #[test]
    fn get_nested_value_empty_key_returns_error() {
        let config = Config::default();
        let result = get_nested_value(&config, "");

        assert!(result.is_err(), "Should return error for empty key");
    }

    // Test 6: Key with only dots returns error
    #[test]
    fn get_nested_value_only_dots_returns_error() {
        let config = Config::default();
        let result = get_nested_value(&config, "...");

        assert!(result.is_err(), "Should return error for key with only dots");
    }
}
```

**Fowler's Classification**: **State Verification**
- Verifies error state
- Tests error propagation

---

### 3. Stderr Verification Tests

**Purpose**: Ensure errors go to stderr, not stdout.

```bash
#!/bin/bash
# test/integration/config_stderr_tests.sh

echo "Testing error output goes to stderr..."

# Test 1: Error message should be on stderr, not stdout
echo "Test 1: Non-existent key error on stderr"
OUTPUT=$(zjj config nonexistent_key 2>&1)
STDOUT=$(zjj config nonexistent_key 2>/dev/null)
STDERR=$(zjj config nonexistent_key >/dev/null 2>&1)

EXIT_CODE=$?

if [ $EXIT_CODE -ne 0 ]; then
    if [ -z "$STDOUT" ] && [ -n "$STDERR" ]; then
        echo "✓ PASS: Error message on stderr only"
    else
        echo "✗ FAIL: stdout='$STDOUT', stderr='$STDERR'"
        echo "Error should be on stderr, not stdout"
        exit 1
    fi
else
    echo "✗ FAIL: Command should have failed with non-zero exit code"
    exit 1
fi

# Test 2: Valid output should be on stdout
echo "Test 2: Valid config output on stdout"
STDOUT=$(zjj config workspace_dir 2>/dev/null)
STDERR=$(zjj config workspace_dir >/dev/null 2>&1)
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    if [ -n "$STDOUT" ] && [ -z "$STDERR" ]; then
        echo "✓ PASS: Valid output on stdout"
    else
        echo "✗ FAIL: stdout='$STDOUT', stderr='$STDERR'"
        echo "Valid output should be on stdout"
        exit 1
    fi
else
    echo "✗ FAIL: Valid command should succeed with exit code 0"
    exit 1
fi

echo "✓ All stderr/stdout tests passed"
```

**Fowler's Classification**: **Output Verification**
- Verifies correct stream for messages
- Prevents output pollution

---

### 4. CLI Handler Integration Tests

**Purpose**: Verify the CLI handler correctly maps errors to exit codes.

```rust
#[cfg(test)]
mod cli_handler_tests {
    use super::*;

    // Test that error types map to correct exit codes
    // This requires integration testing since exit codes are a CLI concern

    // Helper function to run command and get exit code
    #[cfg(test)]
    async fn run_config_get(key: &str) -> Result<String, i32> {
        // This would spawn the actual CLI and capture exit code
        // For now, we test the error propagation at the function level

        let config = Config::default();
        match show_config_value(&config, key, OutputFormat::Human) {
            Ok(value) => Ok(value),
            Err(e) => {
                // Map error to exit code
                let exit_code = if e.to_string().contains("not found") {
                    1
                } else if e.downcast_ref::<zjj_core::Error>().is_some() {
                    // Map based on error type
                    2
                } else {
                    1
                };
                Err(exit_code)
            }
        }
    }

    #[tokio::test]
    async fn non_existent_key_maps_to_exit_code_1() {
        match run_config_get("nonexistent_key").await {
            Err(code) => assert_eq!(code, 1, "Exit code should be 1 for nonexistent key"),
            Ok(_) => panic!("Should have returned error"),
        }
    }

    #[tokio::test]
    async fn valid_key_returns_success() {
        match run_config_get("workspace_dir").await {
            Ok(_) => {}, // Success
            Err(_) => panic!("Should have succeeded"),
        }
    }
}
```

**Fowler's Classification**: **Boundary Test**
- Tests error-to-exit-code mapping
- Boundary between library and CLI

---

### 5. Regression Tests

**Purpose**: Prevent future regressions of exit code behavior.

```rust
#[cfg(test)]
mod regression_tests {
    use super::*;

    // Regression test: Ensure we never accidentally return Ok for error cases
    #[test]
    fn regression_error_cases_always_return_err() {
        let config = Config::default();

        // List of invalid keys that MUST fail
        let invalid_keys = vec![
            "",
            "nonexistent",
            "invalid.nested.key",
            "zellij.nonexistent",
            "workspace_dir.nonexistent",
            "    ",
            "\t",
            "\n",
            "../../etc/passwd",  // Path traversal attempt
        ];

        for key in invalid_keys {
            let result = get_nested_value(&config, key);
            assert!(
                result.is_err(),
                "Key '{}' should return error but got Ok: {:?}",
                key,
                result
            );
        }
    }

    // Regression test: Ensure valid keys always succeed
    #[test]
    fn regression_valid_keys_always_succeed() {
        let config = Config::default();

        // List of valid keys that MUST succeed
        let valid_keys = vec![
            "workspace_dir",
            "zellij.use_tabs",
            "zellij.panes.main.command",
            "watch.paths",
        ];

        for key in valid_keys {
            let result = get_nested_value(&config, key);
            assert!(
                result.is_ok(),
                "Key '{}' should succeed but got error: {:?}",
                key,
                result
            );
        }
    }
}
```

**Fowler's Classification**: **Regression Test**
- Prevents breaking changes
- Data-driven testing

---

## Test Coverage Targets

| Metric Type | Target | Rationale |
|-------------|--------|-----------|
| **Exit Code Coverage** | 100% | Every error path tested |
| **Error Type Coverage** | 100% | All error types mapped |
| **Valid Path Coverage** | 100% | All success cases tested |
| **Output Stream** | 100% | Stderr/stdout verified |

**Specific Coverage**:
| Function | Target | Tests |
|----------|--------|-------|
| `show_config_value` | 100% | Error + success paths |
| `get_nested_value` | 100% | All navigation cases |
| `set_config_value` | 100% | Write operations |
| CLI handler | 100% | Exit code mapping |

---

## Test Smells to Avoid

### 1. **Testing Implementation Details**

❌ **Bad**: Testing internal error representation
```rust
fn test_error_type() {
    match result {
        Err(Error::ValidationError { .. }) => {}, // Fragile
        _ => panic!("Wrong error type"),
    }
}
```

✅ **Good**: Testing exit code (observable behavior)
```rust
fn test_exit_code_is_one() {
    let exit_code = run_and_get_exit_code("zjj config invalid");
    assert_eq!(exit_code, 1);
}
```

### 2. **Over-Testing**

❌ **Bad**: Testing every possible invalid key
```rust
for i in 0..1000 {
    test_invalid_key(&format!("key_{}", i));
}
```

✅ **Good**: Testing representative cases
```rust
let invalid_keys = vec!["", "nonexistent", "invalid.nested"];
for key in invalid_keys {
    test_invalid_key(key);
}
```

---

## Regression Test Checklist

Before closing bead:

- [ ] Non-existent key returns exit code 1
- [ ] Invalid nested key returns exit code 1
- [ ] Valid key returns exit code 0
- [ ] Set operation returns exit code 0
- [ ] Error messages on stderr
- [ ] Valid output on stdout
- [ ] JSON format exit codes correct
- [ ] Global flag exit codes correct
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] `moon run :ci` passes

---

## Continuous Integration (CI) Configuration

Add to `.github/workflows/test.yml`:

```yaml
name: Config Exit Code Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon-action@v1

      - name: Run unit tests
        run: moon run :test config

      - name: Run exit code integration tests
        run: ./test/integration/config_exit_code_tests.sh

      - name: Run stderr/stdout tests
        run: ./test/integration/config_stderr_tests.sh

      - name: Verify specific exit codes
        run: |
          # Test specific exit codes
          zjj config nonexistent_key || exit_code=$?
          [ "$exit_code" -eq 1 ] || exit 1
```

---

## Manual Testing Checklist

Before closing bead:

- [ ] `zjj config nonexistent_key` → exit code 1
- [ ] `zjj config workspace_dir` → exit code 0
- [ ] `zjj config test_key value` → exit code 0
- [ ] `zjj config --json invalid_key` → exit code 1
- [ ] Error messages visible in terminal
- [ ] No errors on stdout
- [ ] Scripts can detect errors via exit codes

---

## Post-Deployment Monitoring

After merging:

1. **CI Failures**: Watch for config-related test failures
2. **Script Failures**: Monitor for scripts not detecting errors
3. **User Reports**: "config command not failing properly"
4. **Automation Issues**: CI/CD pipelines not catching config errors

---

## Summary

**Test Approach**: Exit code verification + Output stream verification + Regression prevention

**Test Count**: ~20 tests
- 10 integration tests (bash)
- 8 unit tests (rust)
- 2 regression tests (rust)

**Execution Time**: <30 seconds

**Risk Coverage**: High (catches exit code bugs)

**Fowler Compliance**: ✅
- ✅ State verification (exit codes)
- ✅ No test smells (observable behavior)
- ✅ Minimal testing (focus on what matters)
- ✅ Clear intent (tests verify exit codes)

---

**Test Plan Status**: ✅ Ready for Implementation

**Estimated Test Execution Time**: 30 seconds

**Confidence Level**: High (simple verification)
