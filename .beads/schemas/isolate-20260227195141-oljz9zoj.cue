
package validation

import "list"

// Validation schema for bead: isolate-20260227195141-oljz9zoj
// Title: session-remove: Locked sessions bypass contract and are removed
//
// This schema validates that implementation is complete.
// Use: cue vet isolate-20260227195141-oljz9zoj.cue implementation.cue

#BeadImplementation: {
  bead_id: "isolate-20260227195141-oljz9zoj"
  title: "session-remove: Locked sessions bypass contract and are removed"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Session exists in database",
      "Session is not locked by another agent",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Session removed from database",
      "Session removed from workspace",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Lock state unchanged during failed removal operation",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Remove unlocked session succeeds",
      "Remove unlocked session cleans workspace",
    ]

    // Required error path tests
    required_error_tests: [
      "Remove locked session fails with SessionLocked error",
      "Remove locked session preserves session state",
      "Remove non-existent session fails with NotFound error",
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
//     timestamp: "2026-02-27T19:51:41Z"
//   }
// }