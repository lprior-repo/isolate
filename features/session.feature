# Feature: Session Management
#
# BDD acceptance tests for session object lifecycle and subcommands.
# Each session has exactly one JJ workspace and exactly one Zellij tab.
#
# State Machine: Created -> Active -> Syncing -> Synced -> Completed
#                            |          |          |
#                            v          v          v
#                          Paused     Failed    Paused/Completed
#
# See: crates/zjj-core/src/session_state.rs for state transition definitions

Feature: Session Management

  As a developer using ZJJ
  I want to manage isolated work sessions
  So that I can work on multiple features in parallel

  Background:
    Given the ZJJ database is initialized
    And I am in a JJ repository

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # CREATE SESSION (add command)
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Create session succeeds
    Given no session named "feature-auth" exists
    When I create a session named "feature-auth" with workspace path "/workspaces/feature-auth"
    Then the session "feature-auth" should exist
    And the session "feature-auth" should have status "creating"
    And the session "feature-auth" should have a JJ workspace at "/workspaces/feature-auth"
    And the session "feature-auth" should have a Zellij tab named "zjj:feature-auth"
    And the session details should be returned as JSON

  Scenario: Create duplicate session fails
    Given a session named "feature-auth" exists
    When I attempt to create a session named "feature-auth"
    Then the operation should fail with error "SESSION_EXISTS"
    And no duplicate session should be created
    And the original session should remain unchanged

  Scenario: Create session with invalid name fails
    Given no session exists
    When I attempt to create a session named "123-invalid"
    Then the operation should fail with error "VALIDATION_ERROR"
    And the error message should indicate "name must start with a letter"

  Scenario: Create session with max sessions reached fails
    Given the maximum number of sessions has been reached
    When I attempt to create a session named "new-feature"
    Then the operation should fail with error "MAX_SESSIONS_EXCEEDED"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # REMOVE SESSION
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Remove session cleans up
    Given a session named "old-feature" exists with status "active"
    When I remove the session "old-feature"
    Then the session "old-feature" should not exist
    And the JJ workspace should be cleaned up
    And the Zellij tab should be closed

  Scenario: Remove non-existent session fails
    Given no session named "nonexistent" exists
    When I attempt to remove the session "nonexistent"
    Then the operation should fail with error "SESSION_NOT_FOUND"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # SYNC SESSION
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Sync session rebases onto main
    Given a session named "feature-sync" exists with status "active"
    And the session has uncommitted changes
    When I sync the session "feature-sync"
    Then the session status should transition to "syncing"
    And the session should rebase onto the main branch
    And the session status should transition to "synced"
    And the last_synced timestamp should be updated

  Scenario: Sync with conflicts reports them
    Given a session named "feature-conflict" exists with status "active"
    And the session has changes that conflict with main
    When I sync the session "feature-conflict"
    Then the session status should transition to "syncing"
    And the operation should fail with error "CONFLICT"
    And the conflicting files should be reported in JSON output
    And the session status should transition to "failed"

  Scenario: Sync clean session succeeds
    Given a session named "feature-clean" exists with status "active"
    And the session has no uncommitted changes
    When I sync the session "feature-clean"
    Then the session status should transition to "syncing"
    And the session status should transition to "synced"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # FOCUS SESSION
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Focus switches to session
    Given a session named "feature-focus" exists with status "active"
    And I am inside Zellij
    When I focus on the session "feature-focus"
    Then the Zellij tab "zjj:feature-focus" should become active
    And the session details should be returned as JSON

  Scenario: Focus from outside Zellij attaches
    Given a session named "feature-attach" exists with status "active"
    And I am not inside Zellij
    When I focus on the session "feature-attach"
    Then Zellij should attach to the session
    And the session details should be returned as JSON

  Scenario: Focus non-existent session fails
    Given no session named "missing" exists
    When I attempt to focus on the session "missing"
    Then the operation should fail with error "SESSION_NOT_FOUND"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # SUBMIT SESSION
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Submit pushes bookmark to remote
    Given a session named "feature-submit" exists with status "synced"
    And the session has a bookmark named "feature-submit"
    When I submit the session "feature-submit"
    Then the bookmark should be pushed to remote
    And the response should include the dedupe key

  Scenario: Submit with dirty workspace fails
    Given a session named "feature-dirty" exists with status "active"
    And the session has uncommitted changes
    When I attempt to submit the session "feature-dirty"
    Then the operation should fail with error "DIRTY_WORKSPACE"
    And the exit code should be 3

  Scenario: Submit with auto-commit succeeds
    Given a session named "feature-autocommit" exists with status "active"
    And the session has uncommitted changes
    When I submit the session "feature-autocommit" with auto-commit
    Then the changes should be committed automatically
    And the bookmark should be pushed to remote

  Scenario: Submit with no bookmark fails
    Given a session named "feature-nobookmark" exists with status "synced"
    And the session has no bookmark
    When I attempt to submit the session "feature-nobookmark"
    Then the operation should fail with error "NO_BOOKMARK"
    And the exit code should be 3

  Scenario: Submit dry-run does not modify state
    Given a session named "feature-dryrun" exists with status "synced"
    And the session has a bookmark named "feature-dryrun"
    When I submit the session "feature-dryrun" with dry-run
    Then no bookmark should be pushed
    And the response should indicate "dry_run: true"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # LIST SESSIONS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: List shows all sessions
    Given sessions "feature-a", "feature-b", and "feature-c" exist
    When I list all sessions
    Then the output should contain 3 sessions
    And each session should show name, status, and workspace path
    And the output should be valid JSON lines

  Scenario: List empty returns empty array
    Given no sessions exist
    When I list all sessions
    Then the output should be an empty array
    And the output should be valid JSON

  Scenario: List with filter shows matching sessions
    Given sessions with statuses "active", "paused", and "completed" exist
    When I list sessions with status filter "active"
    Then only sessions with status "active" should be shown

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # SHOW SESSION DETAILS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Show displays session details
    Given a session named "feature-detail" exists with status "active"
    And the session has branch "feature-detail-branch"
    And the session was last synced at timestamp 1700000000
    When I show the session "feature-detail"
    Then the output should contain the session name "feature-detail"
    And the output should contain the status "active"
    And the output should contain the workspace path
    And the output should contain the Zellij tab name
    And the output should contain the branch "feature-detail-branch"
    And the output should contain the last_synced timestamp
    And the output should be valid JSON

  Scenario: Show non-existent session fails
    Given no session named "missing" exists
    When I attempt to show the session "missing"
    Then the operation should fail with error "SESSION_NOT_FOUND"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # STATE MACHINE TRANSITIONS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Valid state transition Created to Active
    Given a session named "state-test-1" exists with status "creating"
    When the session workspace is successfully created
    Then the session status should transition to "active"
    And the transition should be recorded in history

  Scenario: Valid state transition Active to Paused
    Given a session named "state-test-2" exists with status "active"
    When I pause the session "state-test-2"
    Then the session status should transition to "paused"

  Scenario: Valid state transition Paused to Active
    Given a session named "state-test-3" exists with status "paused"
    When I resume the session "state-test-3"
    Then the session status should transition to "active"

  Scenario: Invalid state transition Created to Paused is prevented
    Given a session named "state-test-4" exists with status "creating"
    When I attempt to pause the session "state-test-4"
    Then the operation should fail with error "INVALID_STATE_TRANSITION"
    And the session status should remain "creating"

  Scenario: Session can be retried after failure
    Given a session named "state-test-5" exists with status "failed"
    When I retry the session "state-test-5"
    Then the session status should transition to "creating"
    And the session can transition to "active"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # INVARIANT: ONE JJ WORKSPACE PER SESSION
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Each session has exactly one JJ workspace
    Given a session named "invariant-test" exists
    When I inspect the session workspace mapping
    Then there should be exactly one JJ workspace for the session
    And the workspace path should match the session workspace_path field

  Scenario: Session cannot have multiple workspaces
    Given a session named "invariant-multi" exists
    Then the session should be associated with at most one workspace
    And any attempt to create a second workspace should fail
