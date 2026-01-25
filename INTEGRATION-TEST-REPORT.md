# Integration Test Report

**Date**: 2026-01-25
**Binary**: target/release/jjz
**Test Script**: integration-test.sh

## Executive Summary

Ran comprehensive integration tests covering:
- Help text consistency across all commands
- Exit code validation for error cases
- JSON output format compliance
- Input validation (special characters, length limits, format rules)
- Command functionality (introspect, query, doctor)

**Results**: 27/32 tests passed (84% pass rate)

## Test Results Summary

### ✅ Passing Test Categories

1. **Help Text Consistency** (14/14 tests)
   - All commands provide working --help output
   - Main help menu accessible
   - Subcommand help accessible for: add, list, remove, focus, status, sync, diff, init, config, dashboard, introspect, doctor, query

2. **Exit Code Validation** (6/6 tests)
   - Empty names exit with code 1 ✓
   - Names starting with dash exit with code 1 ✓
   - Names starting with digit exit with code 1 ✓
   - Very long names (>64 chars) exit with code 1 ✓
   - Whitespace-only names rejected ✓
   - Special characters (@) properly rejected ✓

3. **Command Functionality** (5/5 tests)
   - `introspect` command works ✓
   - `introspect --json` produces valid JSON ✓
   - `query session-count` works ✓
   - `doctor` command works ✓
   - `doctor --json` produces valid JSON ✓

4. **Input Validation** (2/2 tests)
   - Long names properly rejected
   - Non-letter starting characters properly rejected

### ❌ Failing Tests

#### Test Group: JSON Output Support (2/4 passed)

**BUG #1: Missing JSON Schema Envelope in `list --json`**
- **Severity**: Medium
- **Status**: Failing
- **Description**: `jjz list --json` outputs raw JSON array instead of schema-wrapped envelope
- **Expected**:
  ```json
  {
    "$schema": "https://jjz.dev/schemas/list-response.json",
    "schema_type": "list_response",
    "version": "1.0",
    "data": {
      "sessions": [...]
    }
  }
  ```
- **Actual**:
  ```json
  [
    {
      "name": "test-session",
      ...
    }
  ]
  ```
- **Impact**: JSON consumers can't validate schema or detect API version changes
- **Recommendation**: Wrap list output in schema envelope per zjj JSON standards

**BUG #2: Missing JSON Schema Envelope in `status --json`**
- **Severity**: Medium
- **Status**: Failing
- **Description**: `jjz status --json` outputs raw JSON instead of schema-wrapped envelope
- **Expected**: Schema envelope with `$schema`, `schema_type`, `version`, `data` fields
- **Actual**: Raw JSON without envelope
- **Impact**: Same as BUG #1 - no schema validation or versioning
- **Recommendation**: Wrap status output in schema envelope

#### Test Group: Error Structure in JSON Mode (0/3 passed)

**BUG #3: Errors Ignore --json Flag**
- **Severity**: High
- **Status**: Failing
- **Description**: When --json flag is provided, validation errors still output plain text to stderr instead of JSON
- **Example**:
  ```bash
  $ jjz add "" --json
  Error: Invalid session name: Validation error: Session name cannot be empty
  # Expected JSON output with .error.code, .error.message, .error.exit_code fields
  ```
- **Expected**:
  ```json
  {
    "$schema": "https://jjz.dev/schemas/error-response.json",
    "schema_type": "error_response",
    "version": "1.0",
    "error": {
      "code": "VALIDATION_ERROR",
      "message": "Session name cannot be empty",
      "exit_code": 1,
      "suggestion": "Provide a non-empty session name starting with a letter"
    }
  }
  ```
- **Impact**: Scripts using --json can't reliably parse error responses
- **Recommendation**: Implement JSON error output that respects --json flag across all commands

#### Test Group: Special Characters in Session Names (1/3 passed)

**BUG #4: Session Creation Succeeds But Zellij Integration Panics**
- **Severity**: Critical
- **Status**: Failing
- **Description**: Valid session names (with dash/underscore) create database entries but fail when launching Zellij, resulting in exit code 101 (panic)
- **Test Cases**:
  ```bash
  $ jjz add test-with-dash  # Exit code 101
  $ jjz add test_with_underscore  # Exit code 101
  ```
- **Evidence**: Sessions appear in database (jjz list shows them) but creation exits with code 101
- **Root Cause**: Likely panic during Zellij tab creation or workspace initialization
- **Impact**: CLI appears to accept valid names but crashes, leaving orphaned database entries
- **Recommendation**:
  1. Fix panic in Zellij integration (add proper error handling)
  2. Add transaction rollback if Zellij tab creation fails
  3. Add integration test that mocks Zellij to test without real Zellij environment

## Detailed Test Output

```
Using binary: target/release/jjz

TEST: Help text consistency
  ✓ Main help works
  ✓ add --help works
  ✓ list --help works
  ✓ remove --help works
  ✓ focus --help works
  ✓ status --help works
  ✓ sync --help works
  ✓ diff --help works
  ✓ init --help works
  ✓ config --help works
  ✓ dashboard --help works
  ✓ introspect --help works
  ✓ doctor --help works
  ✓ query --help works

TEST: Validation errors exit with code 1
  ✓ Empty name exits with code 1
  ✓ Name with dash exits with code 1

TEST: JSON output support
  ✓ list --json produces valid JSON
  ✗ list --json missing schema envelope
  ✓ status --json produces valid JSON
  ✗ status --json missing schema envelope

TEST: Error structure in JSON mode
  ✗ Error does not produce valid JSON

TEST: Special characters in session names
  ✗ Dash name failed with unexpected exit code 101
  ✗ Underscore name failed with exit code 101
  ✓ Special char @ rejected

TEST: Name length validation
  ✓ Long name rejected with exit code 1

TEST: Empty and whitespace names
  ✓ Whitespace-only name rejected

TEST: Names starting with non-letter
  ✓ Digit-starting name rejected with exit code 1

TEST: Introspect command functionality
  ✓ introspect command works
  ✓ introspect --json produces valid JSON

TEST: Query command functionality
  ✓ query session-count works

TEST: Doctor command functionality
  ✓ doctor command works
  ✓ doctor --json produces valid JSON
```

## Bugs Summary

| ID | Severity | Component | Description | Exit Code Issue |
|----|----------|-----------|-------------|-----------------|
| #1 | Medium | list command | Missing JSON schema envelope | No |
| #2 | Medium | status command | Missing JSON schema envelope | No |
| #3 | High | Error handling | --json flag ignored for errors | No |
| #4 | Critical | Session creation | Panic during Zellij integration | Yes (101 instead of graceful error) |

## Recommendations

### Immediate (P0)

1. **Fix BUG #4**: Add error handling to Zellij integration to prevent panics
   - Wrap Zellij commands in Result types
   - Add transaction rollback for failed session creation
   - Test without requiring actual Zellij running

2. **Fix BUG #3**: Implement proper JSON error responses
   - Create ErrorResponse type with schema envelope
   - Respect --json flag in error formatting
   - Add tests for JSON error output

### Short-term (P1)

3. **Fix BUG #1 & #2**: Add JSON schema envelopes to all commands
   - Wrap list/status outputs in schema envelope
   - Implement consistent schema versioning
   - Document JSON schema format

### Long-term (P2)

4. **Expand Integration Tests**:
   - Add state consistency tests (concurrent operations)
   - Add workflow tests (create → list → remove cycle)
   - Add error recovery tests
   - Add mock Zellij environment for testing

5. **Schema Validation**:
   - Generate JSON schemas for all command outputs
   - Add schema validation in integration tests
   - Document breaking vs non-breaking changes

## Test Environment

- **OS**: Linux 6.18.3-arch1-1
- **Shell**: bash
- **Required tools**: jq (for JSON validation)
- **JJ repo**: Yes (tests run in zjj repository itself)
- **Zellij**: Not required for most tests (causes BUG #4 when present)

## Next Steps

1. Create beads for all 4 bugs
2. Fix BUG #4 first (critical - causes panics)
3. Fix BUG #3 (high - breaks JSON API contract)
4. Fix BUG #1 & #2 (medium - API consistency)
5. Re-run integration tests after fixes
6. Add regression tests for fixed bugs

## Notes

- Old database schema issue found and resolved (missing `status` column in sessions table from previous version)
- Tests successfully validate exit codes for all error scenarios
- Help text is consistent and comprehensive across all commands
- Core functionality (introspect, doctor, query) works correctly
- Main issue is Zellij integration and JSON formatting, not validation logic
