//! Command documentation generation
//!
//! NOTE: This file is large (~1200 lines) as it contains
//! documentation for all 19 CLI commands.

use std::collections::HashMap;

use super::types::{
    ArgumentDoc, CommandDocumentation, Example, OptionDoc, StateChange, ValidationRule,
    WorkflowPosition,
};

#[allow(clippy::too_many_lines)]
pub fn generate_command_docs() -> HashMap<String, CommandDocumentation> {
    let mut commands = HashMap::new();

    // === INIT COMMAND ===
    commands.insert("init".to_string(), CommandDocumentation {
        name: "init".to_string(),
        aliases: vec![],
        category: "System".to_string(),
        description: "Initialize jjz in a JJ repository (or create one)".to_string(),
        long_description: "Sets up ZJJ infrastructure by: (1) Checking for JJ repository (creates one if needed), (2) Creating .zjj/ directory for state and layouts, (3) Initializing SQLite database, (4) Creating default configuration, (5) Setting up workspace directory structure, (6) Running health checks on dependencies".to_string(),
        usage: "jjz init [OPTIONS]".to_string(),
        arguments: vec![],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON for machine parsing".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "repair".to_string(),
                short: None,
                long: "repair".to_string(),
                description: "Attempt to repair corrupted database (preserves sessions)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec!["force".to_string()],
                requires: vec![],
            },
            OptionDoc {
                name: "force".to_string(),
                short: Some("f".to_string()),
                long: "force".to_string(),
                description: "Force reinitialize - destroys ALL session data (creates backup first)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec!["repair".to_string()],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz init".to_string(),
                description: "First-time setup".to_string(),
                use_case: "Initialize jjz in a new repository".to_string(),
            },
            Example {
                command: "jjz init --repair".to_string(),
                description: "Fix database corruption".to_string(),
                use_case: "Repair corrupted session database".to_string(),
            },
            Example {
                command: "jjz init --force".to_string(),
                description: "Complete reset (creates backup)".to_string(),
                use_case: "Start fresh with clean state".to_string(),
            },
        ],
        prerequisites: vec![
            "jj installed (https://github.com/martinvonz/jj)".to_string(),
            "zellij installed (https://zellij.dev)".to_string(),
            "Write permissions in current directory".to_string(),
        ],
        workflow_position: WorkflowPosition {
            typical_order: 1,
            comes_after: vec![],
            comes_before: vec!["add".to_string()],
            can_run_parallel_with: vec!["doctor".to_string(), "config".to_string()],
        },
        related_commands: vec!["doctor".to_string(), "config".to_string(), "backup".to_string(), "restore".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1, 2],
        ai_guidance: "Run this ONCE per repository. Check exit code: 0 = success, 1 = user error, 2 = system error. Run 'jjz doctor' after init to verify setup.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Creates .zjj directory".to_string(),
                how: "mkdir .zjj".to_string(),
                reversible: true,
                reverse_command: Some("rm -rf .zjj".to_string()),
            },
            StateChange {
                what: "Creates SQLite database".to_string(),
                how: "Initialize .zjj/state.db".to_string(),
                reversible: true,
                reverse_command: Some("jjz init --force".to_string()),
            },
            StateChange {
                what: "Creates config file".to_string(),
                how: "Write .zjj/config.toml".to_string(),
                reversible: true,
                reverse_command: Some("rm .zjj/config.toml".to_string()),
            },
        ],
    });

    // === ADD COMMAND ===
    commands.insert("add".to_string(), CommandDocumentation {
        name: "add".to_string(),
        aliases: vec![],
        category: "Session Lifecycle".to_string(),
        description: "Create a new session with JJ workspace + Zellij tab".to_string(),
        long_description: "Creates an isolated development environment by: (1) Creating a JJ workspace, (2) Generating Zellij layout, (3) Opening Zellij tab, (4) Storing session metadata in database, (5) Running post_create hooks".to_string(),
        usage: "jjz add [OPTIONS] <name>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "name".to_string(),
                required: true,
                description: "Name for the new session".to_string(),
                validation_rules: vec![
                    ValidationRule {
                        rule_type: "starts_with_letter".to_string(),
                        description: "Must start with a letter (a-z, A-Z)".to_string(),
                        example_valid: "feature-auth".to_string(),
                        example_invalid: "123-feature".to_string(),
                    },
                    ValidationRule {
                        rule_type: "max_length".to_string(),
                        description: "Maximum 64 characters".to_string(),
                        example_valid: "my-feature".to_string(),
                        example_invalid: "a".repeat(65),
                    },
                    ValidationRule {
                        rule_type: "allowed_chars".to_string(),
                        description: "Letters, numbers, hyphens, underscores only".to_string(),
                        example_valid: "feature_auth-v2".to_string(),
                        example_invalid: "feature@auth".to_string(),
                    },
                ],
                examples: vec![
                    "feature-auth".to_string(),
                    "bugfix-123".to_string(),
                    "experiment_new_api".to_string(),
                ],
            },
        ],
        options: vec![
            OptionDoc {
                name: "template".to_string(),
                short: Some("t".to_string()),
                long: "template".to_string(),
                description: "Zellij layout template".to_string(),
                value_type: Some("minimal|standard|full|split|review".to_string()),
                default: Some("standard".to_string()),
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "no-open".to_string(),
                short: None,
                long: "no-open".to_string(),
                description: "Create workspace without opening Zellij tab".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "no-hooks".to_string(),
                short: None,
                long: "no-hooks".to_string(),
                description: "Skip executing post_create hooks".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "dry-run".to_string(),
                short: None,
                long: "dry-run".to_string(),
                description: "Preview what would happen without executing".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON for machine parsing".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz add feature-auth".to_string(),
                description: "Create session with standard layout".to_string(),
                use_case: "Start new feature development".to_string(),
            },
            Example {
                command: "jjz add bugfix-123 --no-open".to_string(),
                description: "Create background session without opening tab".to_string(),
                use_case: "Prepare workspace for later use".to_string(),
            },
            Example {
                command: "jjz add experiment -t minimal".to_string(),
                description: "Create session with minimal layout (single pane)".to_string(),
                use_case: "Quick experiment or test".to_string(),
            },
        ],
        prerequisites: vec![
            "Must be in a JJ repository (run 'jjz init' first)".to_string(),
            "Must be inside Zellij session (unless using --no-open)".to_string(),
            "Session name must not already exist".to_string(),
        ],
        workflow_position: WorkflowPosition {
            typical_order: 2,
            comes_after: vec!["init".to_string()],
            comes_before: vec!["sync".to_string(), "remove".to_string()],
            can_run_parallel_with: vec!["list".to_string(), "status".to_string()],
        },
        related_commands: vec![
            "list".to_string(),
            "status".to_string(),
            "remove".to_string(),
            "sync".to_string(),
            "focus".to_string(),
        ],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1, 2, 3],
        ai_guidance: "Use --dry-run first to preview. Use --json for structured output. Check 'jjz context --json' for environment state before adding.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Creates database entry".to_string(),
                how: "INSERT INTO sessions".to_string(),
                reversible: true,
                reverse_command: Some("jjz remove <name>".to_string()),
            },
            StateChange {
                what: "Creates JJ workspace".to_string(),
                how: "jj workspace add".to_string(),
                reversible: true,
                reverse_command: Some("jjz remove <name>".to_string()),
            },
            StateChange {
                what: "Creates Zellij layout file".to_string(),
                how: "Write to .zjj/layouts/<name>.kdl".to_string(),
                reversible: true,
                reverse_command: Some("jjz remove <name>".to_string()),
            },
        ],
    });

    // === LIST COMMAND ===
    commands.insert("list".to_string(), CommandDocumentation {
        name: "list".to_string(),
        aliases: vec![],
        category: "Session Lifecycle".to_string(),
        description: "List all sessions".to_string(),
        long_description: "Shows all development sessions with their status, workspace path, Zellij tab name, and timestamps. By default shows only active and creating sessions. Use --all for historical data including completed and failed sessions.".to_string(),
        usage: "jjz list [OPTIONS]".to_string(),
        arguments: vec![],
        options: vec![
            OptionDoc {
                name: "all".to_string(),
                short: None,
                long: "all".to_string(),
                description: "Include completed and failed sessions (historical data)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON array of session objects".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "silent".to_string(),
                short: None,
                long: "silent".to_string(),
                description: "Minimal output for pipes (auto-detected when stdout is not a TTY)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz list".to_string(),
                description: "List active sessions".to_string(),
                use_case: "See what sessions are currently running".to_string(),
            },
            Example {
                command: "jjz list --all".to_string(),
                description: "List everything including completed/failed".to_string(),
                use_case: "View session history".to_string(),
            },
            Example {
                command: "jjz list --json | jq '.[] | select(.status == \"active\")'".to_string(),
                description: "Filter active sessions with jq".to_string(),
                use_case: "Programmatic filtering of sessions".to_string(),
            },
        ],
        prerequisites: vec!["jjz initialized in repository".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec!["add".to_string(), "status".to_string(), "focus".to_string()],
        },
        related_commands: vec!["add".to_string(), "status".to_string(), "focus".to_string(), "dashboard".to_string()],
        output_formats: vec!["table".to_string(), "silent".to_string(), "json".to_string()],
        exit_codes: vec![0, 2],
        ai_guidance: "Always use --json for programmatic access. Parse the JSON array to find sessions by name or status. Use this before 'add' to check if a session name exists.".to_string(),
        state_changes: vec![],
    });

    // === REMOVE COMMAND ===
    commands.insert("remove".to_string(), CommandDocumentation {
        name: "remove".to_string(),
        aliases: vec![],
        category: "Session Lifecycle".to_string(),
        description: "Remove a session and its workspace".to_string(),
        long_description: "Removes a session by: (1) Running pre_remove hooks, (2) Closing Zellij tab, (3) Removing JJ workspace, (4) Deleting database entry, (5) Cleaning up layout files. Optionally can squash-merge to main before removal.".to_string(),
        usage: "jjz remove [OPTIONS] <name>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "name".to_string(),
                required: true,
                description: "Name of the session to remove".to_string(),
                validation_rules: vec![],
                examples: vec!["feature-auth".to_string(), "bugfix-123".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "force".to_string(),
                short: Some("f".to_string()),
                long: "force".to_string(),
                description: "Skip confirmation prompt and hooks".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "merge".to_string(),
                short: Some("m".to_string()),
                long: "merge".to_string(),
                description: "Squash-merge to main before removal".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "keep-branch".to_string(),
                short: Some("k".to_string()),
                long: "keep-branch".to_string(),
                description: "Preserve branch after removal".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "dry-run".to_string(),
                short: None,
                long: "dry-run".to_string(),
                description: "Show what would be done without executing".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz remove feature-auth".to_string(),
                description: "Remove session with confirmation".to_string(),
                use_case: "Clean up completed session".to_string(),
            },
            Example {
                command: "jjz remove bugfix-123 --merge".to_string(),
                description: "Merge to main then remove".to_string(),
                use_case: "Finish work and merge".to_string(),
            },
            Example {
                command: "jjz remove test --force".to_string(),
                description: "Force remove without confirmation".to_string(),
                use_case: "Quick cleanup of test session".to_string(),
            },
        ],
        prerequisites: vec!["Session must exist".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 5,
            comes_after: vec!["add".to_string(), "sync".to_string()],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["add".to_string(), "list".to_string(), "sync".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1, 2, 3],
        ai_guidance: "Use --dry-run to preview. Use --merge to integrate work before cleanup. Check 'jjz list' before removing.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Removes database entry".to_string(),
                how: "DELETE FROM sessions WHERE name = ?".to_string(),
                reversible: false,
                reverse_command: None,
            },
            StateChange {
                what: "Removes JJ workspace".to_string(),
                how: "jj workspace forget".to_string(),
                reversible: false,
                reverse_command: None,
            },
            StateChange {
                what: "Closes Zellij tab".to_string(),
                how: "zellij action close-tab".to_string(),
                reversible: false,
                reverse_command: None,
            },
        ],
    });

    // === FOCUS COMMAND ===
    commands.insert("focus".to_string(), CommandDocumentation {
        name: "focus".to_string(),
        aliases: vec![],
        category: "Session Lifecycle".to_string(),
        description: "Switch to a session's Zellij tab".to_string(),
        long_description: "Switches focus to the Zellij tab associated with the specified session. Uses zellij action go-to-tab-name to switch tabs.".to_string(),
        usage: "jjz focus <name>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "name".to_string(),
                required: true,
                description: "Name of the session to focus".to_string(),
                validation_rules: vec![],
                examples: vec!["feature-auth".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz focus feature-auth".to_string(),
                description: "Switch to feature-auth session".to_string(),
                use_case: "Return to a session you were working on".to_string(),
            },
        ],
        prerequisites: vec!["Session must exist".to_string(), "Must be inside Zellij".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec!["add".to_string()],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["list".to_string(), "add".to_string(), "dashboard".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 2, 3],
        ai_guidance: "Use 'jjz list' to find available sessions. Cannot focus if not inside Zellij.".to_string(),
        state_changes: vec![],
    });

    // === STATUS COMMAND ===
    commands.insert("status".to_string(), CommandDocumentation {
        name: "status".to_string(),
        aliases: vec![],
        category: "Session Lifecycle".to_string(),
        description: "Show detailed session status".to_string(),
        long_description: "Shows detailed information about one or all sessions including workspace path, status, Zellij tab, timestamps, and workspace health. Can watch status continuously.".to_string(),
        usage: "jjz status [OPTIONS] [name]".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "name".to_string(),
                required: false,
                description: "Session name to show status for (shows all if omitted)".to_string(),
                validation_rules: vec![],
                examples: vec!["feature-auth".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "watch".to_string(),
                short: None,
                long: "watch".to_string(),
                description: "Continuously update status (1s refresh)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz status".to_string(),
                description: "Show status of all sessions".to_string(),
                use_case: "Overview of all sessions".to_string(),
            },
            Example {
                command: "jjz status feature-auth".to_string(),
                description: "Show detailed status of one session".to_string(),
                use_case: "Check specific session details".to_string(),
            },
            Example {
                command: "jjz status --watch".to_string(),
                description: "Continuously monitor status".to_string(),
                use_case: "Watch session status in real-time".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec!["list".to_string(), "add".to_string()],
        },
        related_commands: vec!["list".to_string(), "dashboard".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 2, 3],
        ai_guidance: "Use --json for structured output. --watch is useful for monitoring long operations.".to_string(),
        state_changes: vec![],
    });

    // === SYNC COMMAND ===
    commands.insert("sync".to_string(), CommandDocumentation {
        name: "sync".to_string(),
        aliases: vec![],
        category: "Workspace Sync".to_string(),
        description: "Sync a session's workspace with main (rebase)".to_string(),
        long_description: "Rebases the session's workspace onto the latest main branch using 'jj rebase -d main'. Keeps session's changes up-to-date with main branch.".to_string(),
        usage: "jjz sync [OPTIONS] [name]".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "name".to_string(),
                required: false,
                description: "Session name to sync (syncs current workspace if omitted)".to_string(),
                validation_rules: vec![],
                examples: vec!["feature-auth".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "dry-run".to_string(),
                short: None,
                long: "dry-run".to_string(),
                description: "Show what would be done without executing".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz sync feature-auth".to_string(),
                description: "Sync feature-auth with main".to_string(),
                use_case: "Update session with latest main changes".to_string(),
            },
            Example {
                command: "jjz sync".to_string(),
                description: "Sync current workspace".to_string(),
                use_case: "Quick sync from within workspace".to_string(),
            },
            Example {
                command: "jjz sync feature-auth --dry-run".to_string(),
                description: "Preview sync operation".to_string(),
                use_case: "Check what will change before syncing".to_string(),
            },
        ],
        prerequisites: vec!["Session must exist".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 4,
            comes_after: vec!["add".to_string()],
            comes_before: vec!["remove".to_string()],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["diff".to_string(), "add".to_string(), "remove".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1, 2, 3],
        ai_guidance: "Use --dry-run to preview. Check 'jjz diff' before and after sync to see changes.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Rebases workspace onto main".to_string(),
                how: "jj rebase -d main".to_string(),
                reversible: true,
                reverse_command: Some("jj undo".to_string()),
            },
        ],
    });

    // === DIFF COMMAND ===
    commands.insert("diff".to_string(), CommandDocumentation {
        name: "diff".to_string(),
        aliases: vec![],
        category: "Workspace Sync".to_string(),
        description: "Show diff between session and main branch".to_string(),
        long_description: "Displays the differences between a session's workspace and the main branch. Can show full diff or just statistics.".to_string(),
        usage: "jjz diff [OPTIONS] <name>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "name".to_string(),
                required: true,
                description: "Session name to show diff for".to_string(),
                validation_rules: vec![],
                examples: vec!["feature-auth".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "stat".to_string(),
                short: None,
                long: "stat".to_string(),
                description: "Show diffstat only (summary of changes)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz diff feature-auth".to_string(),
                description: "Show full diff".to_string(),
                use_case: "Review all changes in session".to_string(),
            },
            Example {
                command: "jjz diff feature-auth --stat".to_string(),
                description: "Show summary of changes".to_string(),
                use_case: "Quick overview of changed files".to_string(),
            },
        ],
        prerequisites: vec!["Session must exist".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec!["add".to_string()],
            comes_before: vec![],
            can_run_parallel_with: vec!["list".to_string(), "status".to_string()],
        },
        related_commands: vec!["sync".to_string(), "status".to_string()],
        output_formats: vec!["diff".to_string(), "stat".to_string(), "json".to_string()],
        exit_codes: vec![0, 2, 3],
        ai_guidance: "Use --stat for quick overview. Parse JSON output for programmatic analysis.".to_string(),
        state_changes: vec![],
    });

    // === CONFIG COMMAND ===
    commands.insert("config".to_string(), CommandDocumentation {
        name: "config".to_string(),
        aliases: vec!["cfg".to_string()],
        category: "System".to_string(),
        description: "View or modify configuration".to_string(),
        long_description: "Manage jjz configuration. Can view entire config, get specific values, or set values. Supports both project-local (.zjj/config.toml) and global (~/.config/jjz/config.toml) configuration.".to_string(),
        usage: "jjz config [OPTIONS] [key] [value]".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "key".to_string(),
                required: false,
                description: "Config key to view/set (dot notation: 'zellij.use_tabs')".to_string(),
                validation_rules: vec![],
                examples: vec!["workspace_dir".to_string(), "default_template".to_string()],
            },
            ArgumentDoc {
                name: "value".to_string(),
                required: false,
                description: "Value to set (omit to view)".to_string(),
                validation_rules: vec![],
                examples: vec!["/custom/path".to_string(), "minimal".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "global".to_string(),
                short: Some("g".to_string()),
                long: "global".to_string(),
                description: "Operate on global config instead of project".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "validate".to_string(),
                short: None,
                long: "validate".to_string(),
                description: "Validate configuration without modifying".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz config".to_string(),
                description: "View all configuration".to_string(),
                use_case: "See current settings".to_string(),
            },
            Example {
                command: "jjz config workspace_dir".to_string(),
                description: "Get specific value".to_string(),
                use_case: "Check where workspaces are stored".to_string(),
            },
            Example {
                command: "jjz config workspace_dir /custom/path".to_string(),
                description: "Set configuration value".to_string(),
                use_case: "Change workspace directory".to_string(),
            },
        ],
        prerequisites: vec!["jjz initialized".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec!["init".to_string()],
            comes_before: vec![],
            can_run_parallel_with: vec!["list".to_string(), "status".to_string()],
        },
        related_commands: vec!["init".to_string(), "doctor".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1, 2],
        ai_guidance: "Use --json for structured output. Use --validate before modifying critical settings.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Modifies configuration file".to_string(),
                how: "Update .zjj/config.toml or ~/.config/jjz/config.toml".to_string(),
                reversible: true,
                reverse_command: Some("jjz config <key> <old_value>".to_string()),
            },
        ],
    });

    // === DASHBOARD COMMAND ===
    commands.insert("dashboard".to_string(), CommandDocumentation {
        name: "dashboard".to_string(),
        aliases: vec!["dash".to_string()],
        category: "Introspection".to_string(),
        description: "Launch interactive TUI dashboard with kanban view".to_string(),
        long_description: "Opens an interactive terminal UI dashboard showing all sessions in a kanban-style view. Allows quick navigation, session management, and status overview.".to_string(),
        usage: "jjz dashboard".to_string(),
        arguments: vec![],
        options: vec![],
        examples: vec![
            Example {
                command: "jjz dashboard".to_string(),
                description: "Open interactive dashboard".to_string(),
                use_case: "Visual overview of all sessions".to_string(),
            },
        ],
        prerequisites: vec!["jjz initialized".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["list".to_string(), "status".to_string(), "focus".to_string()],
        output_formats: vec!["tui".to_string()],
        exit_codes: vec![0, 2],
        ai_guidance: "Not suitable for automation - use 'list --json' instead for programmatic access.".to_string(),
        state_changes: vec![],
    });

    // === CONTEXT COMMAND ===
    commands.insert("context".to_string(), CommandDocumentation {
        name: "context".to_string(),
        aliases: vec!["ctx".to_string()],
        category: "Introspection".to_string(),
        description: "Show full environment context for AI agents".to_string(),
        long_description: "Provides comprehensive environment context including: JJ repository state, Zellij session info, active sessions, configuration, system health, and available commands. Designed for AI agents to understand the current state.".to_string(),
        usage: "jjz context [OPTIONS]".to_string(),
        arguments: vec![],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz context".to_string(),
                description: "Show context in human-readable format".to_string(),
                use_case: "Quick environment overview".to_string(),
            },
            Example {
                command: "jjz context --json".to_string(),
                description: "JSON output for AI agents".to_string(),
                use_case: "Programmatic environment analysis".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec!["list".to_string(), "status".to_string()],
        },
        related_commands: vec!["introspect".to_string(), "doctor".to_string(), "status".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 2],
        ai_guidance: "ALWAYS use --json. This is the primary command for understanding environment state before operations.".to_string(),
        state_changes: vec![],
    });

    // === INTROSPECT COMMAND ===
    commands.insert("introspect".to_string(), CommandDocumentation {
        name: "introspect".to_string(),
        aliases: vec![],
        category: "Introspection".to_string(),
        description: "Discover jjz capabilities and command details".to_string(),
        long_description: "Provides detailed information about jjz commands, their arguments, options, and usage. Can show all commands or specific command details. Designed for command discovery and learning.".to_string(),
        usage: "jjz introspect [OPTIONS] [command]".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "command".to_string(),
                required: false,
                description: "Command to introspect (shows all if omitted)".to_string(),
                validation_rules: vec![],
                examples: vec!["add".to_string(), "sync".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz introspect".to_string(),
                description: "Show all available commands".to_string(),
                use_case: "Discover what jjz can do".to_string(),
            },
            Example {
                command: "jjz introspect add".to_string(),
                description: "Show detailed info about add command".to_string(),
                use_case: "Learn how to use add command".to_string(),
            },
            Example {
                command: "jjz introspect --json".to_string(),
                description: "Get all command metadata as JSON".to_string(),
                use_case: "Build command completion or documentation".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["context".to_string(), "help".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0],
        ai_guidance: "Use --json to discover all available commands and their schemas. Useful for building AI tools or integrations.".to_string(),
        state_changes: vec![],
    });

    // === DOCTOR COMMAND ===
    commands.insert("doctor".to_string(), CommandDocumentation {
        name: "doctor".to_string(),
        aliases: vec!["check".to_string()],
        category: "System".to_string(),
        description: "Run system health checks".to_string(),
        long_description: "Verifies system health by checking: (1) JJ installation, (2) Zellij installation, (3) Database integrity, (4) Workspace directories, (5) Configuration validity, (6) Beads integration. Can auto-fix issues with --fix flag.".to_string(),
        usage: "jjz doctor [OPTIONS]".to_string(),
        arguments: vec![],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "fix".to_string(),
                short: None,
                long: "fix".to_string(),
                description: "Auto-fix issues where possible".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz doctor".to_string(),
                description: "Check system health".to_string(),
                use_case: "Diagnose issues".to_string(),
            },
            Example {
                command: "jjz doctor --fix".to_string(),
                description: "Auto-fix issues".to_string(),
                use_case: "Repair common problems".to_string(),
            },
            Example {
                command: "jjz doctor --json".to_string(),
                description: "JSON output for monitoring".to_string(),
                use_case: "Automated health checks".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec!["init".to_string()],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["init".to_string(), "config".to_string(), "context".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 2, 4],
        ai_guidance: "Run after init to verify setup. Use --json for structured health status. Exit code 4 indicates unhealthy system.".to_string(),
        state_changes: vec![
            StateChange {
                what: "May repair database".to_string(),
                how: "Rebuild tables, fix corruption".to_string(),
                reversible: false,
                reverse_command: None,
            },
        ],
    });

    // === QUERY COMMAND ===
    commands.insert("query".to_string(), CommandDocumentation {
        name: "query".to_string(),
        aliases: vec![],
        category: "Utilities".to_string(),
        description: "Query system state programmatically".to_string(),
        long_description: "Provides programmatic queries for system state. Query types: session-exists (check if session exists), session-count (count sessions), can-run (check if command can run), suggest-name (suggest available session name).".to_string(),
        usage: "jjz query <query_type> [args]".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "query_type".to_string(),
                required: true,
                description: "Type of query (session-exists, session-count, can-run, suggest-name)".to_string(),
                validation_rules: vec![],
                examples: vec!["session-exists".to_string(), "session-count".to_string()],
            },
            ArgumentDoc {
                name: "args".to_string(),
                required: false,
                description: "Query-specific arguments".to_string(),
                validation_rules: vec![],
                examples: vec!["feature-auth".to_string()],
            },
        ],
        options: vec![],
        examples: vec![
            Example {
                command: "jjz query session-exists feature-auth".to_string(),
                description: "Check if session exists".to_string(),
                use_case: "Validate before operations".to_string(),
            },
            Example {
                command: "jjz query session-count".to_string(),
                description: "Count active sessions".to_string(),
                use_case: "Monitoring script".to_string(),
            },
            Example {
                command: "jjz query suggest-name feature".to_string(),
                description: "Get available name suggestion".to_string(),
                use_case: "Generate unique session name".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["list".to_string(), "context".to_string()],
        output_formats: vec!["text".to_string()],
        exit_codes: vec![0, 1],
        ai_guidance: "Use for validation before operations. Outputs simple text for easy parsing in scripts.".to_string(),
        state_changes: vec![],
    });

    // === COMPLETIONS COMMAND ===
    commands.insert("completions".to_string(), CommandDocumentation {
        name: "completions".to_string(),
        aliases: vec![],
        category: "Utilities".to_string(),
        description: "Generate shell completion scripts".to_string(),
        long_description: "Generates shell completion scripts for bash, zsh, or fish. Output can be saved to appropriate completion directory for the shell.".to_string(),
        usage: "jjz completions <shell>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "shell".to_string(),
                required: true,
                description: "Shell type (bash, zsh, fish)".to_string(),
                validation_rules: vec![],
                examples: vec!["bash".to_string(), "zsh".to_string(), "fish".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "instructions".to_string(),
                short: Some("i".to_string()),
                long: "instructions".to_string(),
                description: "Print installation instructions".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz completions bash > ~/.local/share/bash-completion/completions/jjz".to_string(),
                description: "Generate bash completions".to_string(),
                use_case: "Enable tab completion in bash".to_string(),
            },
            Example {
                command: "jjz completions zsh --instructions".to_string(),
                description: "Generate with installation instructions".to_string(),
                use_case: "Learn how to install completions".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec![],
        output_formats: vec!["script".to_string()],
        exit_codes: vec![0, 1],
        ai_guidance: "Generate once per shell. Not needed for programmatic use.".to_string(),
        state_changes: vec![],
    });

    // === BACKUP COMMAND ===
    commands.insert("backup".to_string(), CommandDocumentation {
        name: "backup".to_string(),
        aliases: vec![],
        category: "Utilities".to_string(),
        description: "Create a backup of the session database".to_string(),
        long_description: "Creates a JSON backup of the session database including all session metadata, configuration, and state. Backups are timestamped and stored in .zjj/backups/ by default.".to_string(),
        usage: "jjz backup [OPTIONS] [path]".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "path".to_string(),
                required: false,
                description: "Backup file path (default: .zjj/backups/jjz-backup-<timestamp>.json)".to_string(),
                validation_rules: vec![],
                examples: vec!["~/backups/jjz-sessions.json".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz backup".to_string(),
                description: "Create backup with auto-generated filename".to_string(),
                use_case: "Regular backup before risky operations".to_string(),
            },
            Example {
                command: "jjz backup ~/backups/jjz-sessions.json".to_string(),
                description: "Create backup at specific path".to_string(),
                use_case: "Custom backup location".to_string(),
            },
        ],
        prerequisites: vec!["jjz initialized".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec!["restore".to_string()],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["restore".to_string(), "verify-backup".to_string(), "init".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 2],
        ai_guidance: "Create backups before destructive operations (init --force, restore). Use --json to get backup file path.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Creates backup file".to_string(),
                how: "Write JSON to .zjj/backups/".to_string(),
                reversible: true,
                reverse_command: Some("rm <backup-file>".to_string()),
            },
        ],
    });

    // === RESTORE COMMAND ===
    commands.insert("restore".to_string(), CommandDocumentation {
        name: "restore".to_string(),
        aliases: vec![],
        category: "Utilities".to_string(),
        description: "Restore session database from a backup file".to_string(),
        long_description: "Restores session database from a JSON backup file. WARNING: This REPLACES ALL existing session data. Creates automatic backup before restore. Use --force to skip confirmation.".to_string(),
        usage: "jjz restore [OPTIONS] <path>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "path".to_string(),
                required: true,
                description: "Path to backup file to restore from".to_string(),
                validation_rules: vec![],
                examples: vec!["~/backups/jjz-sessions.json".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "force".to_string(),
                short: Some("f".to_string()),
                long: "force".to_string(),
                description: "Skip confirmation prompt (DANGEROUS)".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz restore ~/backups/jjz-sessions.json".to_string(),
                description: "Restore from backup (with confirmation)".to_string(),
                use_case: "Recover from data loss".to_string(),
            },
            Example {
                command: "jjz restore backup.json --force".to_string(),
                description: "Restore without confirmation (DANGEROUS)".to_string(),
                use_case: "Automated restore in scripts".to_string(),
            },
        ],
        prerequisites: vec!["Valid backup file".to_string()],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec!["backup".to_string()],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["backup".to_string(), "verify-backup".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1, 2],
        ai_guidance: "DANGEROUS operation - always verify backup with verify-backup first. Creates automatic backup before restore.".to_string(),
        state_changes: vec![
            StateChange {
                what: "Replaces entire database".to_string(),
                how: "DROP ALL tables, recreate from backup".to_string(),
                reversible: true,
                reverse_command: Some("jjz restore <auto-backup-file>".to_string()),
            },
        ],
    });

    // === VERIFY-BACKUP COMMAND ===
    commands.insert("verify-backup".to_string(), CommandDocumentation {
        name: "verify-backup".to_string(),
        aliases: vec![],
        category: "Utilities".to_string(),
        description: "Verify integrity of a backup file".to_string(),
        long_description: "Validates that a backup file is well-formed JSON, contains required fields, and can be successfully restored. Does not modify any data.".to_string(),
        usage: "jjz verify-backup [OPTIONS] <path>".to_string(),
        arguments: vec![
            ArgumentDoc {
                name: "path".to_string(),
                required: true,
                description: "Path to backup file to verify".to_string(),
                validation_rules: vec![],
                examples: vec!["backup.json".to_string()],
            },
        ],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz verify-backup backup.json".to_string(),
                description: "Verify backup file".to_string(),
                use_case: "Check backup before restore".to_string(),
            },
            Example {
                command: "jjz verify-backup backup.json --json".to_string(),
                description: "JSON output for scripting".to_string(),
                use_case: "Automated backup validation".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec!["backup".to_string()],
            comes_before: vec!["restore".to_string()],
            can_run_parallel_with: vec![],
        },
        related_commands: vec!["backup".to_string(), "restore".to_string()],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0, 1],
        ai_guidance: "ALWAYS verify backups before restore. Exit code 1 means invalid backup.".to_string(),
        state_changes: vec![],
    });

    // === VERSION COMMAND ===
    commands.insert("version".to_string(), CommandDocumentation {
        name: "version".to_string(),
        aliases: vec![],
        category: "System".to_string(),
        description: "Show detailed version information".to_string(),
        long_description: "Displays version information including jjz version, Rust version, build date, commit hash, and feature flags. Useful for bug reports and compatibility checking.".to_string(),
        usage: "jjz version [OPTIONS]".to_string(),
        arguments: vec![],
        options: vec![
            OptionDoc {
                name: "json".to_string(),
                short: None,
                long: "json".to_string(),
                description: "Output as JSON".to_string(),
                value_type: None,
                default: None,
                conflicts_with: vec![],
                requires: vec![],
            },
        ],
        examples: vec![
            Example {
                command: "jjz version".to_string(),
                description: "Show version".to_string(),
                use_case: "Check installed version".to_string(),
            },
            Example {
                command: "jjz version --json".to_string(),
                description: "JSON output for AI agents".to_string(),
                use_case: "Programmatic version check".to_string(),
            },
        ],
        prerequisites: vec![],
        workflow_position: WorkflowPosition {
            typical_order: 0,
            comes_after: vec![],
            comes_before: vec![],
            can_run_parallel_with: vec![],
        },
        related_commands: vec![],
        output_formats: vec!["human".to_string(), "json".to_string()],
        exit_codes: vec![0],
        ai_guidance: "Include version in bug reports. Use --json for programmatic access.".to_string(),
        state_changes: vec![],
    });

    commands
}
