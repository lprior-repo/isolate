
package validation

import "list"

// Validation schema for bead: zjj-20260212132128-arpxkjco
// Title: tests: Add integration tests for main-moved re-test
//
// This schema validates that implementation is complete.
// Use: cue vet zjj-20260212132128-arpxkjco.cue implementation.cue

#BeadImplementation: {
  bead_id: "zjj-20260212132128-arpxkjco"
  title: "tests: Add integration tests for main-moved re-test"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Worker pipeline is complete",
      "Test can control remote state",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Freshness guard works",
      "Re-test loop completes",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Each test uses controlled git environment",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Fresh merge succeeds",
      "Stale result returns to rebase",
      "Re-tested result merges",
    ]

    // Required error path tests
    required_error_tests: [
      "Continuous main advance prevents progress",
      "Merge conflict is handled",
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
//     timestamp: "2026-02-12T13:21:28Z"
//   }
// }