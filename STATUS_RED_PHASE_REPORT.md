# Status-Red Phase Report: Property Tests with proptest

**Status-Red (bd-udmj)**: Property tests - MUST FAIL initially

## Summary

✅ **RED PHASE CONFIRMED**: Tests are failing as expected.

### Test Results
- **9 passed**: Basic functionality (JSON serialization, basic field validation)
- **5 failed**: Advanced invariants (field completeness, state consistency, validation rules)

## Failed Tests (RED Phase - Expected Failures)

### 1. Field Completeness Issues
- **`prop_session_output_branch_optional`**
  - **Failure**: Branch field missing when it should be present
  - **Error**: `'branch' field should be present: {"name":"A","status":"creating","state":"created","workspace_path":"/tmp/zjj-test--","created_at":1771816861248,"updated_at":1771816861248}`
  - **Root Cause**: Implementation not including branch field in JSON output

### 2. State Consistency Issues
- **`prop_session_status_valid_transitions`**
  - **Failure**: Invalid status transitions are being allowed
  - **Error**: `Invalid transition Completed -> Creating should be rejected`
  - **Root Cause**: No validation of status transition rules

- **`prop_terminal_states_have_no_transitions`**
  - **Failure**: Terminal states can transition to other states
  - **Error**: `Terminal state Completed cannot transition to Creating`
  - **Root Cause**: Terminal state validation not implemented

### 3. Validation Rule Issues
- **`prop_workspace_path_must_be_absolute`**
  - **Failure**: Relative paths are being accepted
  - **Error**: `Relative path should be rejected`
  - **Minimal Input**: `name = "A", status = Creating, state = Created, path = ""`
  - **Root Cause**: Path validation not implemented

### 4. Unit Test Issues
- **`test_terminal_state_cannot_transition`**
  - **Failure**: Terminal states can be created
  - **Error**: `Terminal state (Completed) should not be creatable for new transitions`
  - **Root Cause**: Terminal state restrictions not implemented

## Passed Tests (Basic Functionality)

These tests pass because they test basic serialization which likely exists:
- `prop_session_output_serializes_to_valid_json`
- `prop_summary_serializes_to_valid_json`
- `prop_output_line_session_serializes`
- `prop_session_output_has_required_fields`
- `prop_session_status_serialization_lowercase`
- `prop_session_name_validation`
- `prop_timestamps_present_and_valid`

## Reproducibility

The failing test cases have been saved in:
- **File**: `/home/lewis/src/zjj/crates/zjj-core/tests/status_properties.proptest-regressions`
- **Saved Cases**: 4 specific failing inputs that reproduce the failures

Example seed-based reproduction:
```bash
# Using saved regression case
cargo test --test status_properties --package zjj-core -- --exact
```

## Test Configuration

- **Test Runner**: proptest with 100 cases per property
- **Parallel Execution**: Enabled (4 threads)
- **Regression Tracking**: Automatic via proptest-regressions
- **Seed Support**: Environment variable `PROPTEST_SEED`

## Next Steps (Status-Green)

1. Fix field completeness (branch field)
2. Implement state transition validation
3. Enforce terminal state restrictions
4. Add path validation (absolute paths only)
5. Update unit tests to reflect new validation

## Implementation Notes

The RED phase is successful because:
- Tests define clear invariants that must hold
- All expected failures are documented with specific root causes
- Failing cases are reproducible via saved seeds
- Basic functionality (JSON serialization) works
- Advanced validation rules are missing (as expected)

The property tests successfully identify gaps in the implementation that need to be addressed in the GREEN phase.