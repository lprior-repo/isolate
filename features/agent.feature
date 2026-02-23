# Feature: Agent Management
#
# BDD acceptance tests for agent object lifecycle and subcommands.
# Agents are autonomous workers that can register, send heartbeats,
# and coordinate via locks.
#
# State Machine: Unregistered -> Registered -> (Active | Stale) -> Unregistered
#
# Key Invariants:
# - Unique agent IDs (no two agents can have the same ID)
# - Agents must be registered before sending heartbeats
# - Heartbeats track liveness (stale after timeout)
#
# See: crates/zjj/src/commands/agents/mod.rs for implementation

Feature: Agent Management

  As an autonomous agent using ZJJ
  I want to manage my agent identity
  So that I can coordinate work with other agents

  Background:
    Given the ZJJ database is initialized
    And I am in a JJ repository

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # REGISTER AGENT
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Register creates agent
    Given no agent with ID "agent-test-001" exists
    When I register an agent with ID "agent-test-001"
    Then the agent "agent-test-001" should exist
    And the agent "agent-test-001" should be registered
    And the environment variable "ZJJ_AGENT_ID" should be set to "agent-test-001"
    And the agent details should be returned as JSON

  Scenario: Register with auto-generated ID
    Given no agent is registered
    When I register an agent without specifying an ID
    Then an agent should be created with an auto-generated ID
    And the agent ID should match pattern "agent-XXXXXXXX-XXXX"
    And the environment variable "ZJJ_AGENT_ID" should be set

  Scenario: Duplicate ID fails
    Given an agent with ID "agent-duplicate" exists
    When I attempt to register an agent with ID "agent-duplicate"
    Then the operation should succeed with update semantics
    And the agent "agent-duplicate" should have an updated last_seen timestamp

  Scenario: Register with invalid ID fails
    Given no agent is registered
    When I attempt to register an agent with ID ""
    Then the operation should fail with error "VALIDATION_ERROR"
    And the error message should indicate "Agent ID cannot be empty"

  Scenario: Register with whitespace ID fails
    Given no agent is registered
    When I attempt to register an agent with ID "   "
    Then the operation should fail with error "VALIDATION_ERROR"
    And the error message should indicate "Agent ID cannot be empty or whitespace-only"

  Scenario: Register with reserved keyword fails
    Given no agent is registered
    When I attempt to register an agent with ID "null"
    Then the operation should fail with error "VALIDATION_ERROR"
    And the error message should indicate "reserved keyword"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # HEARTBEAT
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Heartbeat updates timestamp
    Given an agent with ID "agent-heartbeat" exists
    And the environment variable "ZJJ_AGENT_ID" is set to "agent-heartbeat"
    When I send a heartbeat for the current agent
    Then the agent "agent-heartbeat" should have an updated last_seen timestamp
    And the actions_count should be incremented
    And the heartbeat timestamp should be returned

  Scenario: Heartbeat with command updates current_command
    Given an agent with ID "agent-cmd" exists
    And the environment variable "ZJJ_AGENT_ID" is set to "agent-cmd"
    When I send a heartbeat with command "zjj add feature-x"
    Then the agent "agent-cmd" should have current_command set to "zjj add feature-x"
    And the last_seen timestamp should be updated

  Scenario: Unknown agent heartbeat fails
    Given no agent is registered in the environment
    When I attempt to send a heartbeat
    Then the operation should fail with error "NO_AGENT_REGISTERED"
    And the error message should indicate "No agent registered"

  Scenario: Heartbeat for unregistered agent fails
    Given the environment variable "ZJJ_AGENT_ID" is set to "agent-ghost"
    And no agent with ID "agent-ghost" exists in the database
    When I attempt to send a heartbeat
    Then the operation should fail with error "AGENT_NOT_FOUND"
    And the error message should indicate "not found"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # WHOAMI
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Whoami returns identity
    Given an agent with ID "agent-whoami" exists
    And the environment variable "ZJJ_AGENT_ID" is set to "agent-whoami"
    When I query whoami
    Then the output should contain "agent-whoami"
    And the registered field should be true
    And the agent_id field should be "agent-whoami"

  Scenario: Whoami returns unregistered when no agent
    Given no agent is registered in the environment
    When I query whoami
    Then the output should contain "unregistered"
    And the registered field should be false
    And the agent_id field should be null

  Scenario: Whoami includes session context
    Given an agent with ID "agent-session" exists
    And the environment variable "ZJJ_AGENT_ID" is set to "agent-session"
    And the environment variable "ZJJ_SESSION" is set to "feature-test"
    When I query whoami
    Then the output should contain "agent-session"
    And the current_session field should be "feature-test"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # LIST AGENTS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: List shows all agents
    Given agents "agent-alpha", "agent-beta", and "agent-gamma" exist
    When I list all agents
    Then the output should contain 3 agents
    And each agent should show agent_id, registered_at, and last_seen
    And the output should be valid JSON

  Scenario: List empty returns empty array
    Given no agents exist
    When I list all agents
    Then the output should show 0 agents
    And the total_active count should be 0
    And the output should be valid JSON

  Scenario: List with --all shows stale agents
    Given agent "agent-active" exists and is active
    And agent "agent-stale" exists and is stale
    When I list all agents with --all flag
    Then the output should contain 2 agents
    And the total_stale count should be at least 1

  Scenario: List filters by session
    Given agent "agent-a" is working on session "feature-auth"
    And agent "agent-b" is working on session "feature-db"
    When I list agents filtered by session "feature-auth"
    Then the output should contain only "agent-a"
    And "agent-b" should not appear in the output

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # INVARIANT: UNIQUE AGENT IDS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Each agent has a unique ID
    Given agents "agent-1", "agent-2", and "agent-3" exist
    When I inspect the agent IDs
    Then all agent IDs should be unique
    And no two agents should share the same ID

  Scenario: Agent ID cannot be reused after unregister
    Given an agent with ID "agent-reuse" exists
    When I unregister the agent "agent-reuse"
    And I register an agent with ID "agent-reuse"
    Then the agent "agent-reuse" should exist
    And the agent should have a new registered_at timestamp

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # STATUS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Status shows agent details
    Given an agent with ID "agent-status" exists
    And the environment variable "ZJJ_AGENT_ID" is set to "agent-status"
    When I query the agent status
    Then the output should show agent_id "agent-status"
    And the output should show registered_at timestamp
    And the output should show last_seen timestamp
    And the output should show actions_count

  Scenario: Status shows not registered
    Given no agent is registered in the environment
    When I query the agent status
    Then the registered field should be false
    And the message should indicate "not registered"

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # UNREGISTER
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Unregister removes agent
    Given an agent with ID "agent-unregister" exists
    And the environment variable "ZJJ_AGENT_ID" is set to "agent-unregister"
    When I unregister the current agent
    Then the agent "agent-unregister" should not exist
    And the environment variable "ZJJ_AGENT_ID" should be cleared

  Scenario: Unregister non-existent agent fails
    Given no agent with ID "agent-ghost" exists
    When I attempt to unregister agent "agent-ghost"
    Then the operation should fail with error "AGENT_NOT_FOUND"
    And the error message should indicate "not found"
