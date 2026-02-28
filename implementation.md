# Test Results - bd-64rd

## Summary

All CLI tests pass successfully across the full test suite.

## Test Suite Results

### 1. moon run :quick
- **Status**: PASSED (cached)
- **Description**: Quick lint and format checks

### 2. moon run :test
- **Status**: PASSED
- **Tests**: 1698 tests across 24 binaries
- **Skipped**: 1 test
- **Failed**: 0
- **Result**: All tests passed

### 3. moon run :ci
- **Status**: PASSED
- **Total Tests**: 2643 tests
- **Passed**: 2643 (including 1 leaky test)
- **Skipped**: 1
- **Failed**: 0
- **Duration**: ~4 seconds

#### isolate-core:ci breakdown:
- 1306 passed; 0 failed; 1 ignored (main test suite)
- 23 passed; 0 failed (additional test suite)
- 33 passed; 0 failed
- 4 passed; 0 failed
- 49 passed; 0 failed
- 12 passed; 0 failed
- 17 passed; 0 failed
- 19 passed; 0 failed
- 6 passed; 0 failed
- 29 passed; 0 failed
- 10 passed; 0 failed
- 49 passed; 0 failed
- 9 passed; 0 failed
- 14 passed; 0 failed
- 14 passed; 0 failed
- 10 passed; 0 failed
- 7 passed; 0 failed
- 3 passed; 0 failed
- 44 passed; 0 failed
- 4 passed; 0 failed
- 10 passed; 0 failed
- 7 passed; 0 failed
- 1 passed; 0 failed
- 39 passed; 0 failed; 46 ignored

#### isolate:ci breakdown:
- 2643 tests total: 2643 passed (1 leaky), 1 skipped

## Conclusion

**All tests pass successfully.** There are no failures in any test suite.
