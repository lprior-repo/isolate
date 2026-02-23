# Feature: Doctor Command
#
# As an agent in the ZJJ control plane
# I want to diagnose system health and optionally fix issues
# So that I can maintain a healthy development environment
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation
#
# Invariant: JSON output is always valid
# Invariant: Fix operations are idempotent
# Invariant: Check-only mode is always safe (read-only)

Feature: Doctor Command

  Background:
    Given a JJ repository is initialized
    And zjj is initialized

  # ==========================================================================
  # Scenario: Basic health check runs all diagnostics
  # ==========================================================================
  Scenario: Basic health check runs all diagnostics
    Given the system is in a healthy state
    When I run the doctor command without fix flag
    Then all diagnostic checks should run
    And the output should contain check results
    And the output should be valid JSON
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Doctor detects missing dependencies
  # ==========================================================================
  Scenario: Doctor detects missing dependencies
    Given JJ is not installed
    When I run the doctor command
    Then the "JJ Installation" check should fail
    And the output should contain suggestion "Install JJ"
    And the exit code should be 1

  # ==========================================================================
  # Scenario: Doctor detects missing zellij
  # ==========================================================================
  Scenario: Doctor detects missing zellij
    Given zellij is not installed
    When I run the doctor command
    Then the "Zellij Installation" check should warn
    And the output should contain suggestion "Install Zellij"

  # ==========================================================================
  # Scenario: Doctor detects uninitialized zjj
  # ==========================================================================
  Scenario: Doctor detects uninitialized zjj
    Given zjj is not initialized
    When I run the doctor command
    Then the "zjj Initialized" check should warn
    And the suggestion should include "zjj init"

  # ==========================================================================
  # Scenario: Doctor detects orphaned workspaces
  # ==========================================================================
  Scenario: Doctor detects orphaned workspaces
    Given there are 2 workspaces without session records
    When I run the doctor command
    Then the "Orphaned Workspaces" check should warn
    And the output should show 2 orphaned workspaces
    And the issue should be auto-fixable

  # ==========================================================================
  # Scenario: Doctor detects stale sessions
  # ==========================================================================
  Scenario: Doctor detects stale sessions
    Given there are 3 sessions in "creating" status for over 5 minutes
    When I run the doctor command
    Then the "Stale Sessions" check should warn
    And the output should show 3 stale sessions

  # ==========================================================================
  # Scenario: Fix mode with auto-fixable issues
  # ==========================================================================
  Scenario: Fix mode with auto-fixable issues
    Given there are 2 orphaned workspaces
    And there are 3 stale sessions
    When I run the doctor command with --fix flag
    Then the orphaned workspaces should be removed
    And the stale sessions should be removed
    And the output should show fix results
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Fix idempotency - running twice is safe
  # ==========================================================================
  Scenario: Fix idempotency - running twice is safe
    Given there are 2 orphaned workspaces
    When I run the doctor command with --fix flag
    And I run the doctor command with --fix flag again
    Then the second run should report no issues to fix
    And both runs should complete successfully

  # ==========================================================================
  # Scenario: Dry-run mode shows what would be fixed
  # ==========================================================================
  Scenario: Dry-run mode shows what would be fixed
    Given there are 2 orphaned workspaces
    When I run the doctor command with --fix --dry-run flags
    Then no changes should be made to the system
    And the output should show what would be fixed
    And the output should contain "Dry-run mode"

  # ==========================================================================
  # Scenario: Safety - check mode is read-only
  # ==========================================================================
  Scenario: Safety - check mode is read-only
    Given the system has various issues
    When I run the doctor command without --fix flag
    Then no changes should be made to the system
    And no files should be modified
    And no database records should be deleted
    And the output should only report issues

  # ==========================================================================
  # Scenario: Database integrity check
  # ==========================================================================
  Scenario: Database integrity check
    Given the state database is corrupted
    When I run the doctor command
    Then the "State Database" check should fail
    And the suggestion should include "doctor --fix"

  # ==========================================================================
  # Scenario: Database recovery with fix
  # ==========================================================================
  Scenario: Database recovery with fix
    Given the state database is corrupted
    When I run the doctor command with --fix flag
    Then the corrupted database should be handled
    And the fix result should be reported
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Pending add operations check
  # ==========================================================================
  Scenario: Pending add operations check
    Given there are 5 pending add operations in the journal
    When I run the doctor command
    Then the "Pending Add Operations" check should fail
    And the output should show 5 pending operations
    And the issue should be auto-fixable

  # ==========================================================================
  # Scenario: Pending add operations fix
  # ==========================================================================
  Scenario: Pending add operations fix
    Given there are 5 pending add operations in the journal
    When I run the doctor command with --fix flag
    Then the pending operations should be reconciled
    And the output should report the reconciliation

  # ==========================================================================
  # Scenario: Workspace integrity check
  # ==========================================================================
  Scenario: Workspace integrity check
    Given session "feature-1" has workspace at "/workspaces/feature-1"
    And the workspace directory does not exist
    When I run the doctor command
    Then the "Workspace Integrity" check should fail
    And the output should show the missing workspace

  # ==========================================================================
  # Scenario: Workspace integrity fix with rebind
  # ==========================================================================
  Scenario: Workspace integrity fix with rebind
    Given session "feature-1" has workspace at "/old/path/feature-1"
    And the workspace exists at "/new/path/feature-1"
    When I run the doctor command with --fix flag
    Then the session workspace path should be updated
    And the fix should be reported

  # ==========================================================================
  # Scenario: Non-auto-fixable issues remain after fix
  # ==========================================================================
  Scenario: Non-auto-fixable issues remain after fix
    Given JJ is not installed
    When I run the doctor command with --fix flag
    Then the fix should fail with reason "Requires manual intervention"
    And the exit code should be 1
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Workflow health check - on main with active sessions
  # ==========================================================================
  Scenario: Workflow health check - on main with active sessions
    Given the current directory is the main workspace
    And there are 2 active sessions
    When I run the doctor command
    Then the "Workflow Health" check should warn
    And the suggestion should include "zjj attach"

  # ==========================================================================
  # Scenario: All checks pass for healthy system
  # ==========================================================================
  Scenario: All checks pass for healthy system
    Given the system is in a healthy state
    And all dependencies are installed
    And there are no orphaned workspaces
    And there are no stale sessions
    When I run the doctor command
    Then all checks should pass
    And the exit code should be 0
    And the summary should show "0 error(s)"

  # ==========================================================================
  # Scenario: Verbose output shows fix details
  # ==========================================================================
  Scenario: Verbose output shows fix details
    Given there are 2 orphaned workspaces
    When I run the doctor command with --fix --verbose flags
    Then each fix action should be reported
    And the output should include action status

  # ==========================================================================
  # Scenario: Recent recovery detection
  # ==========================================================================
  Scenario: Recent recovery detection
    Given recovery occurred in the last 5 minutes
    When I run the doctor command
    Then the "State Database" check should warn
    And the output should indicate recovery detected
    And the suggestion should mention "recovery.log"

  # ==========================================================================
  # Scenario: Beads integration check (optional)
  # ==========================================================================
  Scenario: Beads integration check (optional)
    Given beads CLI is not installed
    When I run the doctor command
    Then the "Beads Integration" check should pass
    And the message should include "optional"

  # ==========================================================================
  # Invariant: JSON always valid
  # ==========================================================================
  Scenario: JSON validity invariant - all doctor outputs are valid JSON
    Given the system is in any state
    When I run the doctor command
    Then the output must be valid JSON
    And the output must have a "$schema" field
    And the output must have a "_schema_version" field
    And the output must have a "success" field

  # ==========================================================================
  # Scenario: Exit codes follow conventions
  # ==========================================================================
  Scenario: Exit codes follow conventions
    Given the system has 0 errors and 2 warnings
    When I run the doctor command
    Then the exit code should be 0

  Scenario: Exit code 1 for errors
    Given the system has 1 error and 0 warnings
    When I run the doctor command
    Then the exit code should be 1

  # ==========================================================================
  # Scenario: Summary statistics are accurate
  # ==========================================================================
  Scenario: Summary statistics are accurate
    Given the system has 5 passed checks
    And the system has 2 warnings
    And the system has 1 error
    When I run the doctor command
    Then the summary should show "5 passed"
    And the summary should show "2 warning(s)"
    And the summary should show "1 error(s)"
