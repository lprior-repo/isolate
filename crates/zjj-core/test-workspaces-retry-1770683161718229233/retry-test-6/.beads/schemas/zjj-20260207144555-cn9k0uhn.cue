
package validation

import "list"

// Validation schema for bead: zjj-20260207144555-cn9k0uhn
// Title: template: Add KDL parsing validation
//
// This schema validates that implementation is complete.
// Use: cue vet zjj-20260207144555-cn9k0uhn.cue implementation.cue

#BeadImplementation: {
  bead_id: "zjj-20260207144555-cn9k0uhn"
  title: "template: Add KDL parsing validation"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Template file exists and is readable",
      "KDL parser library is available",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Valid KDL files are accepted and stored",
      "Invalid KDL files are rejected with specific error messages",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "All stored templates have valid KDL syntax",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Valid KDL template file is accepted and stored",
      "Complex KDL with nested structures validates successfully",
      "KDL with comments and whitespace is accepted",
    ]

    // Required error path tests
    required_error_tests: [
      "KDL with unmatched brackets is rejected with line/column error",
      "KDL with invalid node names shows specific parse error",
      "KDL with missing required fields shows validation error",
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
//     timestamp: "2026-02-07T14:45:55Z"
//   }
// }