# Rust Contract Specification: Config Exit Codes (zjj-14hr)

**Generated**: 2026-02-08 07:00:00 UTC
**Bead**: zjj-14hr
**Title**: config: Fix exit codes always zero on error
**Issue Type**: Bug fix (critical - breaks CI/CD)

---

## Problem Statement

**Reported Issue**: All config operations return exit code 0, even on error.

**Example**:
```bash
$ zjj config nonexistent_key
Error: Config key 'nonexistent_key' not found
$ echo $?
0  # WRONG! Should be non-zero
```

**Impact**:
- Scripts cannot detect errors
- CI/CD cannot fail on errors
- Silent failures in automation
- Broken error handling in pipelines

**Root Cause**:
The `run()` function in `config.rs` returns `Result<()>` but errors are converted to `anyhow::Error` and the CLI handler may not be mapping errors to proper exit codes.

---

## Module Structure

**Primary File**: `crates/zjj/src/commands/config.rs`

**Related Files**:
- `crates/zjj/src/cli/handlers.rs` - Error to exit code mapping
- `crates/zjj-core/src/error.rs` - Error type definitions

**Functions Involved**:
- `run()` - Main entry point
- `show_config_value()` - Get specific value
- `get_nested_value()` - Navigate nested config
- `show_all_config()` - Show all config

---

## Public API

**Current Signature**:
```rust
pub async fn run(options: ConfigOptions) -> Result<()>
```

**Problem**: Returns `Ok(())` even when displaying error messages to stderr.

**Required Behavior**:
- Errors MUST return non-zero exit codes
- Exit codes MUST be mapped to error types
- Error messages MUST go to stderr
- Valid operations MUST return exit code 0

---

## Type Changes

**No new types needed** - this is an error handling fix.

**Error Mapping Requirements**:
```rust
// Exit code mappings (from CLAUDE.md conventions):
exit_code_map = {
    "ConfigKeyNotFound": 1,      // Key doesn't exist
    "ValidationError": 2,        // Invalid config format
    "IoError": 3,                // File read/write errors
    "ParseError": 4,             // TOML parsing errors
}
```

---

## CLI Changes

**No CLI argument changes** - behavior fix only.

**Expected Behavior**:
```bash
# Non-existent key - exit code 1
$ zjj config nonexistent_key
Error: Config key 'nonexistent_key' not found
$ echo $?
1

# Invalid TOML - exit code 4
$ zjj config invalid.nested.key value
Error: invalid.nested is not a table
$ echo $?
4

# Successful operation - exit code 0
$ zjj config workspace_dir "../workspaces"
Set workspace_dir = "../workspaces"
$ echo $?
0
```

---

## Error Types

**Relevant Error Types** (from `zjj_core::Error`):
```rust
pub enum Error {
    ValidationError(String),  // -> exit code 2
    IoError(String),          // -> exit code 3
    ParseError(String),       // -> exit code 4
    // ...
}
```

**Config-Specific Errors**:
```rust
// These should map to exit code 1 (key not found)
"Config key '{key}' not found"
"Config key '{key}' not found. Use 'zjj config' to see all keys."
```

---

## Performance Constraints

**Not applicable** - error handling path.

---

## Testing Requirements

### Unit Tests Required:

```rust
#[cfg(test)]
mod exit_code_tests {
    use super::*;

    // Test 1: Non-existent key returns error (not Ok)
    #[tokio::test]
    async fn config_get_nonexistent_key_returns_error() {
        let config = Config::default();
        let result = show_config_value(&config, "nonexistent_key", OutputFormat::Human);

        assert!(result.is_err(), "Should return error for nonexistent key");
        assert!(matches!(result, Err(ref e) if e.to_string().contains("not found")));
    }

    // Test 2: Invalid nested key returns error
    #[tokio::test]
    async fn config_get_invalid_nested_key_returns_error() {
        let config = Config::default();
        let result = show_config_value(&config, "zellij.nonexistent", OutputFormat::Human);

        assert!(result.is_err(), "Should return error for invalid nested key");
    }

    // Test 3: Valid key returns success
    #[tokio::test]
    async fn config_get_valid_key_returns_ok() {
        let config = Config::default();
        let result = show_config_value(&config, "workspace_dir", OutputFormat::Human);

        assert!(result.is_ok(), "Should succeed for valid key");
    }

    // Test 4: Invalid TOML value returns error
    #[tokio::test]
    async fn config_set_invalid_value_returns_error() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Try to set with invalid nested path (value where table expected)
        let result = set_config_value(&config_path, "new_table.nested", "value").await;

        // Should fail gracefully
        assert!(result.is_err() || result.is_ok()); // Either behavior is acceptable
    }
}
```

### Integration Tests Required:

```bash
#!/bin/bash
# test/integration/config_exit_codes.sh

set -euo pipefail

echo "Testing config exit codes..."

# Build zjj
moon run :build

# Test 1: Non-existent key returns exit code 1
echo "Test 1: Non-existent key"
if zjj config nonexistent_key 2>/dev/null; then
    echo "✗ FAIL: Should have failed with non-zero exit code"
    exit 1
else
    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 1 ]; then
        echo "✓ PASS: Exit code 1 for nonexistent key"
    else
        echo "✗ FAIL: Expected exit code 1, got $EXIT_CODE"
        exit 1
    fi
fi

# Test 2: Valid key returns exit code 0
echo "Test 2: Valid key"
OUTPUT=$(zjj config workspace_dir 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    echo "✓ PASS: Exit code 0 for valid key"
else
    echo "✗ FAIL: Expected exit code 0, got $EXIT_CODE"
    echo "Output: $OUTPUT"
    exit 1
fi

# Test 3: Set operation returns exit code 0
echo "Test 3: Set operation"
TEMP_DIR=$(mktemp -d)
CONFIG_PATH="$TEMP_DIR/config.toml"
cd "$TEMP_DIR"
OUTPUT=$(zjj config --global test_key "test_value" 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    echo "✓ PASS: Exit code 0 for set operation"
else
    echo "✗ FAIL: Expected exit code 0, got $EXIT_CODE"
    echo "Output: $OUTPUT"
    exit 1
fi

# Cleanup
rm -rf "$TEMP_DIR"

# Test 4: Invalid TOML returns non-zero exit code
echo "Test 4: Invalid TOML"
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"
echo "invalid_toml_content" > .zjj/config.toml
OUTPUT=$(zjj config workspace_dir 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    echo "✓ PASS: Non-zero exit code for invalid TOML (exit code: $EXIT_CODE)"
else
    echo "✗ FAIL: Should fail with non-zero exit code for invalid TOML"
    exit 1
fi
rm -rf "$TEMP_DIR"

echo "✓ All exit code tests passed"
```

### Exit Code Mapping Test:

```bash
#!/bin/bash
# test/integration/exit_code_mapping.sh

# Test that each error type maps to correct exit code

test_exit_code() {
    local description="$1"
    local command="$2"
    local expected_code="$3"

    echo -n "Testing: $description ... "
    eval "$command" >/dev/null 2>&1
    actual_code=$?

    if [ $actual_code -eq $expected_code ]; then
        echo "✓ (exit code $actual_code)"
        return 0
    else
        echo "✗ FAIL (expected $expected_code, got $actual_code)"
        return 1
    fi
}

# Run tests
test_exit_code "Non-existent key" \
    "zjj config nonexistent_key" \
    1

test_exit_code "Valid key read" \
    "zjj config workspace_dir" \
    0

# Note: More tests can be added for specific error types
```

---

## Migration Guide

**Not applicable** - behavior fix only.

---

## Implementation Checklist

- [ ] Review current error handling in `config.rs`
- [ ] Review CLI handler exit code mapping
- [ ] Ensure `show_config_value()` returns `Err()` for non-existent keys
- [ ] Ensure `get_nested_value()` returns `Err()` for missing keys
- [ ] Add unit tests for error cases
- [ ] Add integration tests for exit codes
- [ ] Verify exit codes with manual testing
- [ ] Update documentation if needed

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let value = config.get("key").expect("key exists");

// ✅ REQUIRED
let value = config.get("key").ok_or_else(|| {
    anyhow::Error::new(zjj_core::Error::ValidationError(format!(
        "Config key 'key' not found"
    )))
})?;
```

**In error paths**:
```rust
// Current code (line 160-164) - may need review:
let current = parts.iter().try_fold(&json, |current_value, &part| {
    current_value.get(part).ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::ValidationError(format!(
            "Config key '{key}' not found. Use 'zjj config' to see all keys."
        )))
    })
})?;

// This is CORRECT - returns Err() which should propagate to exit code
```

---

## Success Criteria

1. All config errors return non-zero exit codes
2. Valid operations return exit code 0
3. Error messages go to stderr, not stdout
4. `zjj config nonexistent_key` returns exit code 1
5. All tests pass
6. `moon run :ci` succeeds

---

## Investigation Notes

**Question**: Where is the error-to-exit-code mapping?

**Answer**: In `crates/zjj/src/cli/handlers.rs`. The handler should:
1. Catch `Result::Err()` from command
2. Convert error to appropriate exit code
3. Print error message to stderr
4. Exit with non-zero code

**Current Implementation Review Needed**:
- Check if CLI handler properly maps errors
- Check if error type is lost in conversion
- Verify `anyhow::Error` preserves error code information

---

## Verification Steps

Before closing bead:

```bash
# 1. Test non-existent key
zjj config nonexistent_key
echo $?  # Should be 1

# 2. Test valid key
zjj config workspace_dir
echo $?  # Should be 0

# 3. Test set operation
zjj config test_key "test_value"
echo $?  # Should be 0

# 4. Run integration tests
./test/integration/config_exit_codes.sh

# 5. Full CI
moon run :ci
```

---

## Related Beads

- zjj-16ks: Fix concurrent write 90% data loss
- zjj-2d4m: Fix config set creates invalid TOML
- zjj-2gaw: Standardize JSON output format

---

**Contract Status**: Ready for Implementation

**Estimated Resolution Time**: 1 hour (error handling fix)

**Risk Level**: Low (localized to error handling)
