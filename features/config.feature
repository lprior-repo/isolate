# Feature: Config Command
#
# As an agent in the Isolate control plane
# I want to view and modify configuration settings
# So that I can customize Isolate behavior for different projects
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation
#
# Invariant: JSON output is always valid
# Invariant: Config operations are type-safe
# Invariant: Invalid keys/values are rejected with clear errors

Feature: Config Command

  Background:
    Given a JJ repository is initialized
    And isolate is initialized

  # ==========================================================================
  # Scenario: List all configuration
  # ==========================================================================
  Scenario: List all configuration
    Given a valid config exists
    When I run "isolate config"
    Then all config values should be displayed
    And the output should include workspace_dir
    And the output should include main_branch
    And the output should be valid TOML
    And the exit code should be 0

  # ==========================================================================
  # Scenario: List all configuration in JSON format
  # ==========================================================================
  Scenario: List all configuration in JSON format
    Given a valid config exists
    When I run "isolate config --json"
    Then the output should be valid JSON
    And the JSON should contain a $schema field
    And the JSON should contain workspace_dir
    And the JSON should contain main_branch
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Get specific config value
  # ==========================================================================
  Scenario: Get specific config value
    Given a valid config exists
    When I run "isolate config workspace_dir"
    Then all config values should be displayed
    And the value should be displayed in "key = value" format
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Get nested config value
  # ==========================================================================
  Scenario: Get nested config value
    Given a valid config exists with nested settings
    When I run "isolate config zellij.use_tabs"
    Then the output should show the nested value
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Get config value in JSON format
  # ==========================================================================
  Scenario: Get config value in JSON format
    Given a valid config exists
    When I run "isolate config workspace_dir --json"
    Then the output should be valid JSON
    And the JSON should contain the key field
    And the JSON should contain the value field
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set config value
  # ==========================================================================
  Scenario: Set config value
    Given a valid config exists
    When I run "isolate config workspace_dir ../custom_workspaces"
    Then the config value should be updated
    And the output should confirm the change
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set nested config value
  # ==========================================================================
  Scenario: Set nested config value
    Given a valid config exists
    When I run "isolate config zellij.use_tabs false"
    Then the nested config value should be updated
    And the output should confirm the change
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set boolean value
  # ==========================================================================
  Scenario: Set boolean value
    Given a valid config exists
    When I run "isolate config zellij.use_tabs true"
    Then the boolean should be stored as a proper boolean
    And reading it back should return "true" not a string
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set integer value
  # ==========================================================================
  Scenario: Set integer value
    Given a valid config exists
    When I run "isolate config max_sessions 10"
    Then the integer should be stored as a proper integer
    And reading it back should return "10" not a string
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set array value
  # ==========================================================================
  Scenario: Set array value
    Given a valid config exists
    When I run "isolate config watch.paths '[".beads/beads.db", "src/"]'"
    Then the array should be stored properly
    And reading it back should show the array
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Invalid key is rejected
  # ==========================================================================
  Scenario: Invalid key is rejected
    Given a valid config exists
    When I run "isolate config invalid..key value"
    Then the operation should fail
    And the error message should explain the key format
    And the exit code should be 1

  # ==========================================================================
  # Scenario: Non-existent key is rejected
  # ==========================================================================
  Scenario: Non-existent key is rejected
    Given a valid config exists
    When I run "isolate config nonexistent.key"
    Then the operation should fail
    And the error message should suggest running "isolate config" to see valid keys
    And the exit code should be 1

  # ==========================================================================
  # Scenario: Value without key is rejected
  # ==========================================================================
  Scenario: Value without key is rejected
    Given a valid config exists
    When I run "isolate config value"
    Then the operation should fail
    And the error should explain that a value requires a key
    And the exit code should be 1

  # ==========================================================================
  # Scenario: Global config scope
  # ==========================================================================
  Scenario: Global config scope
    Given a valid config exists
    When I run "isolate config --global workspace_dir ../global_workspaces"
    Then the value should be set in global config
    And the output should indicate global scope
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Project config scope (default)
  # ==========================================================================
  Scenario: Project config scope (default)
    Given a valid config exists
    When I run "isolate config workspace_dir ../project_workspaces"
    Then the value should be set in project config
    And the output should indicate project scope
    And the exit code should be 0

  # ==========================================================================
  # Scenario: View global config only
  # ==========================================================================
  Scenario: View global config only
    Given a valid config exists
    When I run "isolate config --global"
    Then only global config should be displayed
    And the output should indicate global scope
    And the exit code should be 0

  # ==========================================================================
  # Scenario: View merged config (default)
  # ==========================================================================
  Scenario: View merged config (default)
    Given global config exists
    And project config exists
    And project config overrides global settings
    When I run "isolate config"
    Then merged config should be displayed
    And project values should override global values
    And the output should show config sources
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Config key validation prevents injection
  # ==========================================================================
  Scenario: Config key validation prevents injection
    Given a valid config exists
    When I run "isolate config '../../../etc/passwd' value"
    Then the operation should fail
    And the error should explain invalid key format
    And the exit code should be 1

  # ==========================================================================
  # Scenario: Set creates parent tables automatically
  # ==========================================================================
  Scenario: Set creates parent tables automatically
    Given an empty config file
    When I run "isolate config zellij.panes.main.command nvim"
    Then the nested table structure should be created
    And the TOML should be valid
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Concurrent config writes are serialized
  # ==========================================================================
  Scenario: Concurrent config writes are serialized
    Given a valid config exists
    When multiple processes write to config simultaneously
    Then no data should be lost
    And all writes should succeed
    And the final config should contain all changes

  # ==========================================================================
  # Scenario: Type safety - boolean must be true/false
  # ==========================================================================
  Scenario: Type safety - boolean must be true/false
    Given a valid config exists
    When I run "isolate config zellij.use_tabs yes"
    Then the value should be stored as string "yes"
    And the config should remain valid
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Invalid TOML value is rejected
  # ==========================================================================
  Scenario: Invalid TOML value is rejected
    Given a valid config exists
    When I run "isolate config watch.paths '[invalid'"
    Then the operation should fail
    And the error should explain the value format issue
    And the exit code should be 1
