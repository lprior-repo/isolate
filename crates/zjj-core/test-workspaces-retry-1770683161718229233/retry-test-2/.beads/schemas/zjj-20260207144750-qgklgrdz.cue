
package validation

import "list"

// Validation schema for bead: zjj-20260207144750-qgklgrdz
// Title: concurrency: Implement LockManager and lock/unlock commands
//
// This schema validates that implementation is complete.
// Use: cue vet zjj-20260207144750-qgklgrdz.cue implementation.cue

#BeadImplementation: {
  bead_id: "zjj-20260207144750-qgklgrdz"
  title: "concurrency: Implement LockManager and lock/unlock commands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Agent must be registered",
      "Session must exist",
      "Agent authenticated",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Lock acquired/released in database",
      "Lock expiration set",
      "Unique lock_id returned",
      "Only lock holder can unlock",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Only one agent can hold lock at a time",
      "Locks expire after reasonable timeout",
      "Lock holder can always unlock",
      "Non-holders cannot unlock",
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
      "test_lock_acquires_successfully_on_unlocked_session",
      "test_lock_returns_lock_id_and_expires_at",
      "test_unlock_releases_lock_for_holder",
      "test_expired_lock_can_be_reacquired_by_different_agent",
    ]

    // Required error path tests
    required_error_tests: [
      "test_lock_fails_if_session_already_locked",
      "test_lock_fails_if_session_does_not_exist",
      "test_lock_fails_if_agent_not_registered",
      "test_unlock_fails_if_lock_not_held_by_agent",
      "test_unlock_fails_if_session_not_locked",
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
//     timestamp: "2026-02-07T14:47:50Z"
//   }
// }