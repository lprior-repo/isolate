# Feature: Stack Management
#
# BDD acceptance tests for stack object operations.
# Stacks represent parent-child relationships between sessions/workspaces.
#
# Key Invariants:
# - Acyclic graph: No workspace can be its own ancestor
# - Parent always exists for non-root workspaces
# - Depth = parent's depth + 1
#
# See: crates/zjj-core/src/coordination/stack_depth.rs for core logic

Feature: Stack Management

  As a developer using ZJJ
  I want to manage stacked sessions for hierarchical work
  So that I can build features on top of other features

  Background:
    Given the ZJJ database is initialized
    And I am in a JJ repository

  # ==========================================================================
  # Scenario: List shows tree structure
  # ==========================================================================
  Scenario: List shows tree structure
    Given a root session named "main-feature" exists
    And a child session named "sub-feature-a" exists with parent "main-feature"
    And a child session named "sub-feature-b" exists with parent "main-feature"
    And a grandchild session named "nested-feature" exists with parent "sub-feature-a"
    When I list the stack
    Then the output should show a tree structure
    And "main-feature" should appear as a root
    And "sub-feature-a" should appear as a child of "main-feature"
    And "sub-feature-b" should appear as a child of "main-feature"
    And "nested-feature" should appear as a child of "sub-feature-a"
    And the output should be valid JSON lines

  # ==========================================================================
  # Scenario: Show displays stack context
  # ==========================================================================
  Scenario: Show displays stack context
    Given a root session named "root-session" exists
    And a child session named "child-session" exists with parent "root-session"
    And a grandchild session named "grandchild-session" exists with parent "child-session"
    When I show the stack status for "grandchild-session"
    Then the output should contain the workspace name "grandchild-session"
    And the output should show depth 2
    And the output should show parent "child-session"
    And the output should show root "root-session"
    And the output should be valid JSON

  # ==========================================================================
  # Scenario: Restack updates children
  # ==========================================================================
  Scenario: Restack updates children
    Given a root session named "base-feature" exists
    And a child session named "dependent-feature" exists with parent "base-feature"
    And the parent session "base-feature" has been rebased onto main
    When I restack the stack rooted at "base-feature"
    Then all children should be marked for restacking
    And the dependent sessions should rebase onto their updated parents
    And the stack structure should be preserved

  # ==========================================================================
  # Scenario: Cycle detection prevents creation
  # ==========================================================================
  Scenario: Cycle detection prevents creation
    Given a root session named "ancestor" exists
    And a child session named "descendant" exists with parent "ancestor"
    When I attempt to set the parent of "ancestor" to "descendant"
    Then the operation should fail with error "CYCLE_DETECTED"
    And the error should indicate the cycle path
    And the parent relationship should remain unchanged
    And the stack should remain acyclic

  # ==========================================================================
  # Scenario: Restack root with no children is no-op
  # ==========================================================================
  Scenario: Restack root with no children is no-op
    Given a root session named "lonely-root" exists
    And the session has no children
    When I restack the stack rooted at "lonely-root"
    Then the operation should succeed
    And no changes should be made to the session
    And the output should indicate "no children to restack"

  # ==========================================================================
  # Scenario: Create stacked session with parent
  # ==========================================================================
  Scenario: Create stacked session with parent
    Given a root session named "parent-session" exists
    When I create a stacked session named "child-session" with parent "parent-session"
    Then the session "child-session" should exist
    And the session "child-session" should have parent "parent-session"
    And the session "child-session" should have depth 1
    And the session "child-session" should have root "parent-session"

  # ==========================================================================
  # Scenario: Create stacked session with non-existent parent fails
  # ==========================================================================
  Scenario: Create stacked session with non-existent parent fails
    Given no session named "nonexistent-parent" exists
    When I attempt to create a stacked session named "orphan-session" with parent "nonexistent-parent"
    Then the operation should fail with error "PARENT_NOT_FOUND"
    And no session should be created

  # ==========================================================================
  # Scenario: Depth calculation is consistent
  # ==========================================================================
  Scenario: Depth calculation is consistent
    Given a root session named "depth-0" exists
    And a child session named "depth-1" exists with parent "depth-0"
    And a child session named "depth-2" exists with parent "depth-1"
    And a child session named "depth-3" exists with parent "depth-2"
    When I show the stack status for "depth-3"
    Then the output should show depth 3
    And the root should be "depth-0"
    And the parent chain should be ["depth-2", "depth-1", "depth-0"]

  # ==========================================================================
  # Scenario: Self-parent cycle detection
  # ==========================================================================
  Scenario: Self-parent cycle detection
    Given a session named "self-loop-test" exists
    When I attempt to set the parent of "self-loop-test" to "self-loop-test"
    Then the operation should fail with error "CYCLE_DETECTED"
    And the cycle path should contain "self-loop-test"
    And the session should remain unchanged

  # ==========================================================================
  # INVARIANT: Acyclic graph
  # ==========================================================================
  Scenario: Acyclic invariant is enforced
    Given sessions "a", "b", and "c" exist in a chain a <- b <- c
    When I attempt to set the parent of "a" to "c"
    Then the operation should fail with error "CYCLE_DETECTED"
    And the graph should remain acyclic
    And the existing parent relationships should be preserved

  # ==========================================================================
  # Scenario: List empty stack returns empty
  # ==========================================================================
  Scenario: List empty stack returns empty
    Given no sessions exist
    When I list the stack
    Then the output should indicate no workspaces
    And the output should be valid JSON
