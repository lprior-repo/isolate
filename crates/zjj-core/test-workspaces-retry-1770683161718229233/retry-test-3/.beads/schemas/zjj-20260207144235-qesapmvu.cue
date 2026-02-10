
package validation

import "list"

// Validation schema for bead: zjj-20260207144235-qesapmvu
// Title: security: Add session name validation to clone command
//
// This schema validates that implementation is complete.
// Use: cue vet zjj-20260207144235-qesapmvu.cue implementation.cue

#BeadImplementation: {
  bead_id: "zjj-20260207144235-qesapmvu"
  title: "security: Add session name validation to clone command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "clone command invoked",
      "target workspace exists",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Valid session names create workspace",
      "Invalid names are rejected with error",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Validation regex: ^[a-zA-Z][a-zA-Z0-9_-]*$",
      "Max length: 64 characters",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(5)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Clone with valid name succeeds: 'test-workspace'",
      "Clone with hyphens succeeds: 'my-workspace'",
      "Clone with underscores succeeds: 'my_workspace'",
      "Clone with alphanumeric name succeeds: 'workspace123'",
    ]

    // Required error path tests
    required_error_tests: [
      "Clone with slashes rejected: 'test/with/slashes'",
      "Clone with dots rejected: 'test.workspace'",
      "Clone with empty string rejected: ''",
      "Clone starting with number rejected: '1workspace'",
      "Clone with special chars rejected: 'test@workspace'",
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
//     timestamp: "2026-02-07T14:42:35Z"
//   }
// }