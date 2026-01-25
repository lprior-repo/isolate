# Integration Testing Guide

## Quick Start

```bash
# Build the binary
cargo build --release --bin jjz

# Run integration tests
./integration-test.sh

# View results
cat INTEGRATION-TEST-SUMMARY.md
```

## Files

### Test Suite
- **integration-test.sh** - Executable test suite with 32 assertions
  - Tests help text, exit codes, JSON output, validation
  - Color-coded output
  - Automatic bug detection

### Reports
- **INTEGRATION-TEST-SUMMARY.md** - Executive summary (start here!)
- **INTEGRATION-TEST-REPORT.md** - Detailed test results with examples
- **BUGS-FOUND.md** - Complete bug reports with fix guidance

## Test Results (2026-01-25)

**Pass Rate**: 27/32 (84%)

### Found Bugs

1. **Critical**: Session creation panics during Zellij integration (exit code 101)
2. **High**: Errors ignore --json flag (breaks automation)
3. **Medium**: list --json missing schema envelope
4. **Medium**: status --json missing schema envelope

## Running Tests

### Prerequisites
```bash
# Required tools
- cargo (to build)
- jq (for JSON validation)
- bash

# Optional
- Zellij (will reveal BUG #4 if running)
```

### Run All Tests
```bash
./integration-test.sh
```

### Run Specific Test Categories
```bash
# Help text only
./integration-test.sh 2>&1 | grep -A 20 "TEST: Help"

# JSON output only
./integration-test.sh 2>&1 | grep -A 20 "TEST: JSON"

# Exit codes only
./integration-test.sh 2>&1 | grep -A 20 "TEST: Validation errors"
```

### Debug Failed Tests
```bash
# Run specific command manually
./target/release/jjz list --json | jq .

# Check exit code
./target/release/jjz add "" ; echo "Exit code: $?"

# See error output
./target/release/jjz add "" --json 2>&1
```

## After Fixing Bugs

### Re-run Tests
```bash
# Rebuild
cargo build --release --bin jjz

# Test again
./integration-test.sh

# Should show 32/32 passing
```

### Regression Testing
Add the integration test to CI/CD:

```yaml
# .github/workflows/ci.yml
- name: Integration Tests
  run: |
    cargo build --release --bin jjz
    ./integration-test.sh
```

## Test Coverage

### ✅ Currently Tested
- Help text for all commands
- Exit codes for validation errors
- JSON output validity
- Input validation (length, format, special chars)
- Command functionality (introspect, doctor, query)

### 🔜 Future Tests
- Workflow tests (create → list → remove cycle)
- Concurrent operation tests
- State consistency after errors
- Database migration tests
- Mock Zellij environment

## Interpreting Results

### Green (✓)
Test passed - functionality works as expected

### Red (✗)
Test failed - see BUGS-FOUND.md for details

### Red Bug (🐛)
Automated bug detection found an issue

## Notes

### Database Reset
If tests fail due to database issues:
```bash
rm /home/lewis/src/zjj/.jjz/sessions.db
./integration-test.sh
```

### Zellij Environment
Tests run without Zellij by default to avoid environment issues.
This reveals BUG #4 - CLI should handle missing Zellij gracefully.

### Exit Codes
- 0 = Success
- 1 = Validation error (user input invalid)
- 2 = Runtime error (JJ not installed, not in repo, etc.)
- 3 = Not found error (session doesn't exist)
- 101 = Panic (critical bug!)

## Contributing

### Adding New Tests

Edit `integration-test.sh`:

```bash
# Add new test group
log_test "New test category"

# Run command and check result
if command_to_test; then
    log_pass "Test passed"
else
    log_fail "Test failed"
    log_bug "Description of bug if found"
fi
```

### Test Naming Convention
- Test names describe what is being tested
- Failure messages explain what went wrong
- Bug messages explain impact and expected behavior

## Resources

- [JSON Schema Spec](https://json-schema.org/)
- [Exit Code Standards](https://tldp.org/LDP/abs/html/exitcodes.html)
- [Zellij Actions](https://zellij.dev/documentation/integration.html)

## Questions?

See:
- INTEGRATION-TEST-SUMMARY.md - High-level overview
- INTEGRATION-TEST-REPORT.md - Detailed results
- BUGS-FOUND.md - Bug fix instructions
