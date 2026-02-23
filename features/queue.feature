# Feature: Queue Management
#
# As a multi-agent coordination system
# I want to manage a merge queue for sequential processing
# So that multiple agents can coordinate their work without conflicts
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation

Feature: Queue Management

  Background:
    Given a JJ repository is initialized
    And zjj is initialized

  # ==========================================================================
  # Scenario: List shows entries with status
  # ==========================================================================
  Scenario: List shows entries with status
    Given I add workspace "workspace-alpha" to the queue with priority 5
    And I add workspace "workspace-beta" to the queue with priority 3
    When I list the queue
    Then I should see "workspace-alpha" in the output
    And I should see "workspace-beta" in the output
    And each entry should show its status
    And entries should be ordered by priority

  # ==========================================================================
  # Scenario: Show displays entry details
  # ==========================================================================
  Scenario: Show displays entry details
    Given I add workspace "workspace-test" to the queue with priority 5
    And I attach bead "bd-123" to the entry
    When I show the status of workspace "workspace-test"
    Then I should see "workspace-test" in the output
    And I should see the status
    And I should see the priority
    And I should see "bd-123" in the output

  # ==========================================================================
  # Scenario: Work processes next entry
  # ==========================================================================
  Scenario: Work processes next entry
    Given I add workspace "workspace-next" to the queue with priority 1
    And the workspace is ready to merge
    When I process the next queue entry
    Then the entry should transition to merging
    And the processing lock should be acquired
    And the processing lock should be released after completion

  # ==========================================================================
  # Scenario: Retry failed entry
  # ==========================================================================
  Scenario: Retry failed entry
    Given I add workspace "workspace-retry" to the queue with priority 5
    And the entry is in failed_retryable state
    And the attempt count is less than max attempts
    When I retry the entry
    Then the entry should transition to pending
    And the attempt count should be incremented

  # ==========================================================================
  # Scenario: Cancel entry
  # ==========================================================================
  Scenario: Cancel entry
    Given I add workspace "workspace-cancel" to the queue with priority 5
    And the entry is in pending state
    When I cancel the entry
    Then the entry should transition to cancelled
    And the entry should be in terminal state

  # ==========================================================================
  # Scenario: Cancel merged entry fails
  # ==========================================================================
  Scenario: Cancel merged entry fails
    Given I add workspace "workspace-merged" to the queue with priority 5
    And the entry is in merged state
    When I attempt to cancel the entry
    Then the operation should fail
    And the error should indicate "terminal state"
    And the entry should remain in merged state

  # ==========================================================================
  # Scenario: Retry terminal entry fails
  # ==========================================================================
  Scenario: Retry terminal entry fails
    Given I add workspace "workspace-terminal" to the queue with priority 5
    And the entry is in failed_terminal state
    When I attempt to retry the entry
    Then the operation should fail
    And the error should indicate "not retryable"
    And the entry should remain in failed_terminal state

  # ==========================================================================
  # Scenario: Single worker at a time
  # ==========================================================================
  Scenario: Single worker at a time
    Given I add workspace "workspace-serial" to the queue with priority 5
    And worker "agent-alpha" has acquired the processing lock
    When worker "agent-beta" attempts to acquire the processing lock
    Then the acquisition should fail
    And the queue should indicate it is locked by "agent-alpha"
    And no concurrent merge conflicts should occur

  # ==========================================================================
  # Scenario: Priority ordering preserves FIFO for same priority
  # ==========================================================================
  Scenario: Priority ordering preserves FIFO for same priority
    Given I add workspace "first-p1" to the queue with priority 1
    And I add workspace "second-p1" to the queue with priority 1
    And I add workspace "third-p1" to the queue with priority 1
    When I list the queue
    Then "first-p1" should appear before "second-p1"
    And "second-p1" should appear before "third-p1"

  # ==========================================================================
  # Scenario: Higher priority entries processed first
  # ==========================================================================
  Scenario: Higher priority entries processed first
    Given I add workspace "low-priority" to the queue with priority 9
    And I add workspace "high-priority" to the queue with priority 1
    And I add workspace "medium-priority" to the queue with priority 5
    When I get the next queue entry
    Then it should be "high-priority"
    When I get the next queue entry
    Then it should be "medium-priority"
    When I get the next queue entry
    Then it should be "low-priority"

  # ==========================================================================
  # Scenario: Processing lock expires after timeout
  # ==========================================================================
  Scenario: Processing lock expires after timeout
    Given worker "agent-stale" has acquired the processing lock
    And the lock timeout has expired
    When worker "agent-fresh" attempts to acquire the processing lock
    Then the acquisition should succeed
    And the stale lock should be replaced

  # ==========================================================================
  # Scenario: Retry respects max attempts
  # ==========================================================================
  Scenario: Retry respects max attempts
    Given I add workspace "workspace-max-retry" to the queue with priority 5
    And the entry is in failed_retryable state
    And the attempt count equals max attempts
    When I attempt to retry the entry
    Then the operation should fail
    And the error should indicate "max attempts exceeded"
