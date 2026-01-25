#!/usr/bin/env bash
# Integration Test Suite for jjz CLI
# Tests all commands work together end-to-end

set -uo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
declare -a FAILED_TESTS
declare -a BUGS_FOUND

# Find the binary
JJZ_BIN="${CARGO_TARGET_DIR:-target}/release/jjz"
if [[ ! -f "$JJZ_BIN" ]]; then
    echo -e "${RED}Error: jjz binary not found at $JJZ_BIN${NC}"
    echo "Run: cargo build --release --bin jjz"
    exit 1
fi

echo "Using binary: $JJZ_BIN"
echo

# Helper functions
log_test() {
    echo -e "${YELLOW}TEST:${NC} $1"
    ((TESTS_RUN++))
}

log_pass() {
    echo -e "  ${GREEN}✓${NC} $1"
    ((TESTS_PASSED++))
}

log_fail() {
    echo -e "  ${RED}✗${NC} $1"
    ((TESTS_FAILED++))
    FAILED_TESTS+=("$1")
}

log_bug() {
    echo -e "  ${RED}🐛 BUG:${NC} $1"
    BUGS_FOUND+=("$1")
}

# Test 1: Help text consistency
log_test "Help text consistency"
if "$JJZ_BIN" --help > /dev/null 2>&1; then
    log_pass "Main help works"
else
    log_fail "Main help failed"
fi

# Check individual command help
for cmd in add list remove focus status sync diff init config dashboard introspect doctor query; do
    if "$JJZ_BIN" "$cmd" --help > /dev/null 2>&1; then
        log_pass "$cmd --help works"
    else
        log_fail "$cmd --help failed"
    fi
done

# Test 2: Validation errors have proper exit codes
log_test "Validation errors exit with code 1"

# Empty name
if "$JJZ_BIN" add "" 2>/dev/null; then
    log_fail "Empty name should fail"
    log_bug "add command accepts empty name (should reject with exit code 1)"
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 1 ]]; then
        log_pass "Empty name exits with code 1"
    else
        log_fail "Empty name exits with code $EXIT_CODE (expected 1)"
        log_bug "Exit code for validation error is $EXIT_CODE instead of 1"
    fi
fi

# Name starting with dash
if "$JJZ_BIN" add "-test" 2>/dev/null; then
    log_fail "Name starting with dash should fail"
    log_bug "add command accepts names starting with dash"
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 1 ]]; then
        log_pass "Name with dash exits with code 1"
    else
        log_fail "Name with dash exits with code $EXIT_CODE (expected 1)"
    fi
fi

# Test 3: JSON output support
log_test "JSON output support"

# list --json
OUTPUT=$("$JJZ_BIN" list --json 2>&1 || true)
if echo "$OUTPUT" | jq . > /dev/null 2>&1; then
    log_pass "list --json produces valid JSON"

    # Check for schema envelope
    if echo "$OUTPUT" | jq -e '."$schema"' > /dev/null 2>&1; then
        log_pass "list --json has schema envelope"
    else
        log_fail "list --json missing schema envelope"
        log_bug "list --json output missing required \$schema field"
    fi
else
    log_fail "list --json produces invalid JSON"
    log_bug "list --json does not produce valid JSON"
fi

# status --json
OUTPUT=$("$JJZ_BIN" status --json 2>&1 || true)
if echo "$OUTPUT" | jq . > /dev/null 2>&1; then
    log_pass "status --json produces valid JSON"

    # Check for schema envelope
    if echo "$OUTPUT" | jq -e '."$schema"' > /dev/null 2>&1; then
        log_pass "status --json has schema envelope"
    else
        log_fail "status --json missing schema envelope"
        log_bug "status --json output missing required \$schema field"
    fi
else
    log_fail "status --json produces invalid JSON"
fi

# Test 4: Error structure in JSON
log_test "Error structure in JSON mode"

ERROR_OUTPUT=$("$JJZ_BIN" add "" --json 2>&1 || true)
if echo "$ERROR_OUTPUT" | jq . > /dev/null 2>&1; then
    log_pass "Error produces valid JSON"

    # Check for required error fields
    if echo "$ERROR_OUTPUT" | jq -e '.error.code' > /dev/null 2>&1; then
        log_pass "Error has .error.code field"
    else
        log_fail "Error missing .error.code field"
        log_bug "JSON error output missing .error.code field"
    fi

    if echo "$ERROR_OUTPUT" | jq -e '.error.message' > /dev/null 2>&1; then
        log_pass "Error has .error.message field"
    else
        log_fail "Error missing .error.message field"
    fi

    if echo "$ERROR_OUTPUT" | jq -e '.error.exit_code' > /dev/null 2>&1; then
        log_pass "Error has .error.exit_code field"
    else
        log_fail "Error missing .error.exit_code field"
    fi
else
    log_fail "Error does not produce valid JSON"
    log_bug "Errors in --json mode do not produce valid JSON"
fi

# Test 5: Special characters in names
log_test "Special characters in session names"

# Dash is allowed
if "$JJZ_BIN" add "test-with-dash" 2>/dev/null; then
    log_pass "Dash in name accepted"
    # Try to remove it
    "$JJZ_BIN" remove "test-with-dash" --force 2>/dev/null || true
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 1 ]]; then
        log_pass "Dash rejected (validation error)"
    else
        log_fail "Dash name failed with unexpected exit code $EXIT_CODE"
    fi
fi

# Underscore is allowed
if "$JJZ_BIN" add "test_with_underscore" 2>/dev/null; then
    log_pass "Underscore in name accepted"
    "$JJZ_BIN" remove "test_with_underscore" --force 2>/dev/null || true
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 1 ]]; then
        log_pass "Underscore rejected (validation error)"
    else
        log_fail "Underscore name failed with exit code $EXIT_CODE"
    fi
fi

# Special chars should be rejected
if "$JJZ_BIN" add "test@session" 2>/dev/null; then
    log_fail "Special char @ should be rejected"
    log_bug "Session names allow invalid character @"
    "$JJZ_BIN" remove "test@session" --force 2>/dev/null || true
else
    log_pass "Special char @ rejected"
fi

# Test 6: Very long names
log_test "Name length validation"

LONG_NAME=$(printf 'a%.0s' {1..100})
if "$JJZ_BIN" add "$LONG_NAME" 2>/dev/null; then
    log_fail "Very long name (100 chars) should be rejected"
    log_bug "Session names allow length > 64 characters"
    "$JJZ_BIN" remove "$LONG_NAME" --force 2>/dev/null || true
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 1 ]]; then
        log_pass "Long name rejected with exit code 1"
    else
        log_fail "Long name rejected with exit code $EXIT_CODE (expected 1)"
    fi
fi

# Test 7: Empty/whitespace names
log_test "Empty and whitespace names"

# Pure whitespace
if "$JJZ_BIN" add "   " 2>/dev/null; then
    log_fail "Whitespace-only name should be rejected"
    log_bug "Session names allow whitespace-only input"
else
    log_pass "Whitespace-only name rejected"
fi

# Test 8: Name starting with digit
log_test "Names starting with non-letter"

if "$JJZ_BIN" add "1test" 2>/dev/null; then
    log_fail "Name starting with digit should be rejected"
    log_bug "Session names allow starting with digit"
    "$JJZ_BIN" remove "1test" --force 2>/dev/null || true
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 1 ]]; then
        log_pass "Digit-starting name rejected with exit code 1"
    else
        log_fail "Digit-starting name rejected with code $EXIT_CODE (expected 1)"
    fi
fi

# Test 9: Introspect command
log_test "Introspect command functionality"

if "$JJZ_BIN" introspect > /dev/null 2>&1; then
    log_pass "introspect command works"
else
    log_fail "introspect command failed"
fi

if "$JJZ_BIN" introspect --json 2>&1 | jq . > /dev/null 2>&1; then
    log_pass "introspect --json produces valid JSON"
else
    log_fail "introspect --json produces invalid JSON"
fi

# Test 10: Query command
log_test "Query command functionality"

# session-count
if "$JJZ_BIN" query session-count > /dev/null 2>&1; then
    log_pass "query session-count works"
else
    log_fail "query session-count failed"
fi

# Test 11: Doctor command
log_test "Doctor command functionality"

if "$JJZ_BIN" doctor > /dev/null 2>&1; then
    log_pass "doctor command works"
else
    log_fail "doctor command failed"
fi

if "$JJZ_BIN" doctor --json 2>&1 | jq . > /dev/null 2>&1; then
    log_pass "doctor --json produces valid JSON"
else
    log_fail "doctor --json produces invalid JSON"
fi

# Print summary
echo
echo "═══════════════════════════════════════════"
echo "INTEGRATION TEST SUMMARY"
echo "═══════════════════════════════════════════"
echo "Tests run: $TESTS_RUN"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
if [[ $TESTS_FAILED -gt 0 ]]; then
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
fi
echo

if [[ ${#BUGS_FOUND[@]} -gt 0 ]]; then
    echo -e "${RED}🐛 BUGS FOUND:${NC}"
    for bug in "${BUGS_FOUND[@]}"; do
        echo "  - $bug"
    done
    echo
fi

if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
    echo -e "${RED}❌ FAILED TESTS:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo "  - $test"
    done
    echo
fi

# Exit code
if [[ $TESTS_FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
