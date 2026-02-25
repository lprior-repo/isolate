# Feature: Status Query
#
# As an agent in the ZJJ control plane
# I want to query my current session status and context
# So that I can understand my work environment and make informed decisions
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation
#
# Invariant: JSON output is always valid

Feature: Status Query

  Background:
    Given a JJ repository is initialized
    And zjj is initialized

  # ==========================================================================
  # Scenario: Status shows current session
  # ==========================================================================
  Scenario: Status shows current session
    Given I have created a session named "feature-status"
    And the session has status "active"
    When I query the status
    Then the output should contain the session name "feature-status"
    And the output should contain the status "active"
    And the output should contain the workspace path
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Missing session handled gracefully
  # ==========================================================================
  Scenario: Missing session handled gracefully
    Given no session exists
    When I query the status
    Then the output should indicate no active session
    And the exit code should be 0
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: JSON output is valid
  # ==========================================================================
  Scenario: JSON output is valid
    Given I have created a session named "json-test"
    And the session has status "active"
    When I query the status with JSON output
    Then the output should be valid JSONL
    And each line should be a valid JSON object
    And the output should contain a "session" type line
    And the output should contain a "summary" type line

  # ==========================================================================
  # Scenario: Status with detailed information
  # ==========================================================================
  Scenario: Status with detailed information
    Given I have created a session named "detailed-status"
    And the session has 3 modified files
    And the session has 5 open beads
    When I query the status with details for "detailed-status"
    Then the output should show file change statistics
    And the output should show bead statistics
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Status for non-existent session fails gracefully
  # ==========================================================================
  Scenario: Status for non-existent session fails gracefully
    Given no session named "nonexistent" exists
    When I attempt to query the status for "nonexistent"
    Then the operation should fail with error "NOT_FOUND"
    And the exit code should be 2
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Status output is read-only
  # ==========================================================================
  Scenario: Status output is read-only
    Given I have created a session named "readonly-test"
    And the session has status "active"
    When I query the status for "readonly-test"
    Then the session status should remain unchanged
    And no files should be modified
    And no state transitions should occur

  # ==========================================================================
  # Scenario: Multiple sessions status
  # ==========================================================================
  Scenario: Multiple sessions status
    Given I have created sessions "session-a", "session-b", and "session-c"
    And "session-a" has status "active"
    And "session-b" has status "paused"
    And "session-c" has status "syncing"
    When I query the status for all sessions
    Then the output should contain 3 session entries
    And each session should show its status
    And the summary should show the count of active sessions
    And the output should be valid JSONL

  # ==========================================================================
  # Invariant: JSON always valid
  # ==========================================================================
  Scenario: JSON validity invariant - all status outputs are valid JSON
    Given I have created a session named "invariant-test"
    When I query the status
    Then the output must be valid JSON
    And the output must have a "$schema" field
    And the output must have a "_schema_version" field
    And the output must have a "success" field
