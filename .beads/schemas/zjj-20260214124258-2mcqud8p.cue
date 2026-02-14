
package validation

import "list"

// Validation schema for bead: zjj-20260214124258-2mcqud8p
// Title: cli-consistency: implement phase 4 ai extended 2
//
// This schema validates that implementation is complete.
// Use: cue vet zjj-20260214124258-2mcqud8p.cue implementation.cue

#BeadImplementation: {
  bead_id: "zjj-20260214124258-2mcqud8p"
  title: "cli-consistency: implement phase 4 ai extended 2"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The ZJJ environment is valid",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "AI agents can introspect command capabilities",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Output format is machine-readable JSON",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "context --contract returns valid schema",
      "ai --ai-hints returns usage tips",
    ]

    // Required error path tests
    required_error_tests: [
      "Combining --contract with conflicting flags returns error",
      "Malformed input with --contract returns validation error",
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
//     timestamp: "2026-02-14T12:42:58Z"
//   }
// }