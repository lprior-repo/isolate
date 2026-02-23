# CLI Property Tests Report

**Bead:** zjj-20260222164827-7s4seabv

This report documents the property-based tests for CLI structure invariants using proptest.

## Summary

- **Total Tests:** 31
- **Status:** ALL PASSING
- **Test File:** `/home/lewis/src/zjj/crates/zjj-core/tests/cli_properties.rs`

## Invariants Tested

The tests verify these core invariants:

1. **Noun-verb pattern:** All commands follow `<object> <action>` structure
2. **JSON validity:** All JSON output is valid and parseable
3. **Argument consistency:** Command argument parsing is consistent
4. **Exit codes:** Follow conventions (0=success, non-zero=error)
5. **Object completeness:** All 8 objects have required subcommands

---

## Test Catalog

### PROPERTY 1: Noun-Verb Pattern (5 tests)

| Test | Invariant |
|------|-----------|
| `prop_object_names_are_valid_tokens` | Object names are lowercase ASCII without special characters |
| `prop_all_commands_follow_noun_verb_pattern` | Every valid command is `<object> <action>` |
| `prop_all_objects_exist` | CLI has exactly 8 top-level objects |
| `prop_objects_have_required_actions` | All objects have at least one action with query capability |

### PROPERTY 2: JSON Validity (5 tests)

| Test | Invariant |
|------|-----------|
| `prop_session_output_valid_json` | SessionOutput serializes to valid JSON with required fields |
| `prop_summary_valid_json` | Summary serializes to valid JSON with required fields |
| `prop_issue_valid_json` | Issue serializes to valid JSON with required fields |
| `prop_output_line_variants_valid_json` | All OutputLine variants produce valid JSON |
| `prop_json_field_names_snake_case` | All JSON field names follow snake_case convention |

### PROPERTY 3: Argument Consistency (4 tests)

| Test | Invariant |
|------|-----------|
| `prop_session_name_validation_consistent` | Session name validation is consistent across commands |
| `prop_workspace_path_validation` | Workspace paths are validated consistently |
| `prop_json_flag_consistent` | All commands accept --json flag consistently |
| `prop_argument_requirements_consistent` | Required vs optional arguments follow patterns |

### PROPERTY 4: Exit Code Conventions (4 tests)

| Test | Invariant |
|------|-----------|
| `prop_exit_code_0_means_success` | Exit code 0 is only returned on successful completion |
| `prop_non_zero_exit_codes_indicate_errors` | Non-zero exit codes indicate error categories |
| `prop_exit_codes_in_valid_range` | Exit codes are 0-4 (small integer range) |
| `prop_error_severity_exit_code_mapping` | Higher severity issues map to higher exit codes |

### PROPERTY 5: Output Type Completeness (3 tests)

| Test | Invariant |
|------|-----------|
| `prop_output_line_kinds_unique` | Each OutputLine variant has a unique type identifier |
| `prop_timestamps_valid` | Timestamped output includes valid timestamps |
| `prop_summary_type_serialization` | SummaryType enum values serialize to lowercase |

### Additional Invariant Tests (4 tests)

| Test | Invariant |
|------|-----------|
| `prop_session_statuses_finite` | Exactly 5 session status values exist |
| `prop_workspace_states_finite` | Exactly 6 workspace state values exist |
| `prop_issue_kinds_finite` | Exactly 7 issue kind values exist |
| `prop_issue_severities_finite` | Exactly 4 issue severity values exist |

### Unit Tests (6 tests)

| Test | Purpose |
|------|---------|
| `test_harness_works` | Confirms test harness is functional |
| `test_all_objects_defined` | Validates 8 objects are defined |
| `test_objects_have_actions` | Each object has at least one action |
| `test_empty_session_name_rejected` | Empty session names are rejected |
| `test_empty_summary_message_rejected` | Empty summary messages are rejected |
| `test_exit_code_conventions` | Exit code constants are correct |
| `test_session_output_json_serialization` | SessionOutput JSON round-trips correctly |

---

## CLI Structure Constants

### Valid Objects (8)

```
task, session, queue, stack, agent, status, config, doctor
```

### Object Actions

| Object | Actions |
|--------|---------|
| task | list, show, claim, yield, start, done |
| session | list, add, remove, focus, pause, resume, clone, rename, attach, spawn, sync, init |
| queue | list, enqueue, dequeue, status, process |
| stack | status, list, create, push, pop |
| agent | list, register, unregister, heartbeat, status, broadcast |
| status | show, whereami, whoami, context |
| config | list, get, set, schema |
| doctor | check, fix, integrity, clean |

### Exit Codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | EXIT_SUCCESS | Successful completion |
| 1 | EXIT_USAGE_ERROR | Invalid arguments |
| 2 | EXIT_CONFIG_ERROR | Configuration error |
| 3 | EXIT_STATE_ERROR | Invalid state transition |
| 4 | EXIT_EXTERNAL_ERROR | External tool error (jj, zellij, etc.) |

---

## Test Execution

```bash
# Run all property tests
cargo test --package zjj-core --test cli_properties

# Reproducible runs with seed
PROPTEST_SEED=0x12345678 cargo test --package zjj-core --test cli_properties
```

### Results

```
running 31 tests
...............................
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## Zero-Unwrap Verification

The test file uses `#![allow(clippy::unwrap_used, ...)]` for test ergonomics, which is appropriate for test code. Production code in `src/` must use strict zero-unwrap/panic patterns.

Production code clippy verification:

```bash
cargo clippy -p zjj-core -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
```

---

## Notes

- Doctor uses "check" as its query action (not "list") since it's diagnostic in nature
- Session name validation: non-empty, max 64 chars, alphanumeric/dash/underscore, starts with letter
- JSON field names use snake_case consistently
- Timestamps are Unix milliseconds, validated to be in reasonable range (2020-3000)
