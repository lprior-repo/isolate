
package validation

import "list"

// Validation schema for bead: isolate-20260228085443-pl50mpxw
// Title: handlers: Fix config routing to subcommands
//
// This schema validates that implementation is complete.
// Use: cue vet isolate-20260228085443-pl50mpxw.cue implementation.cue

#BeadImplementation: {
  bead_id: "isolate-20260228085443-pl50mpxw"
  title: "handlers: Fix config routing to subcommands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "handle_config receives Args struct with subcommand() method",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "config list returns all config",
      "config get returns key value",
      "config set updates key",
      "config schema returns JSON schema",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Legacy 'isolate config' still works with deprecation warning",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "isolate config list returns all settings",
      "isolate config get key returns value",
      "isolate config set key value updates",
      "isolate config schema returns schema",
    ]

    // Required error path tests
    required_error_tests: [
      "isolate config invalid returns error",
      "isolate config get missing_key returns error",
    ]
  }

  // Code completion
  code_complete: {
    implementation_exists: string  // Path to implementation file
    tests_exist: string  // Path to test file
    ci_passing: bool & true
    no_unwrap_calls: bool & true  // Rust/functional constraint
    no_panics: bool & true  // Rust constraint
  }

  // Completion criteria
  completion: {
    all_sections_complete: bool & true
    documentation_updated: bool
    beads_closed: bool
    timestamp: string  // ISO8601 completion timestamp
  }
}

// Example implementation proof - create this file to validate completion:
//
// implementation.cue:
// package validation
//
// implementation: #BeadImplementation & {
//   contracts_verified: {
//     preconditions_checked: true
//     postconditions_verified: true
//     invariants_maintained: true
//     precondition_checks: [/* documented checks */]
//     postcondition_checks: [/* documented verifications */]
//     invariant_checks: [/* documented invariants */]
//   }
//   tests_passing: {
//     all_tests_pass: true
//     happy_path_tests: ["test_version_flag_works", "test_version_format", "test_exit_code_zero"]
//     error_path_tests: ["test_invalid_flag_errors", "test_no_flags_normal_behavior"]
//   }
//   code_complete: {
//     implementation_exists: "src/main.rs"
//     tests_exist: "tests/cli_test.rs"
//     ci_passing: true
//     no_unwrap_calls: true
//     no_panics: true
//   }
//   completion: {
//     all_sections_complete: true
//     documentation_updated: true
//     beads_closed: false
//     timestamp: "2026-02-28T08:54:43Z"
//   }
// }