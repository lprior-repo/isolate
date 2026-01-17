//! Tool metadata, categories, workflows, and prerequisites

use std::collections::HashMap;

use super::types::{CoreConcept, Prerequisite, ToolMetadata, Workflow, WorkflowStep};

pub fn generate_tool_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "jjz".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Manage JJ workspaces with Zellij sessions".to_string(),
        purpose: "A workflow tool that manages isolated development sessions by combining JJ (Jujutsu) workspaces for parallel Git branches, Zellij terminal multiplexer for organized UI layouts, and SQLite database for session state tracking.".to_string(),
        core_concepts: vec![
            CoreConcept {
                name: "Session".to_string(),
                description: "A named development task with its own workspace and Zellij tab".to_string(),
                example: Some("feature-auth, bugfix-123, experiment".to_string()),
            },
            CoreConcept {
                name: "Workspace".to_string(),
                description: "Isolated JJ workspace (similar to Git worktree) with independent working directory".to_string(),
                example: Some("../repo__workspaces/feature-auth".to_string()),
            },
            CoreConcept {
                name: "Layout".to_string(),
                description: "Zellij tab configuration with pane organization".to_string(),
                example: Some("standard: 70% Claude + 30% sidebar (beads + jj log)".to_string()),
            },
        ],
    }
}

pub fn generate_categories() -> HashMap<String, Vec<String>> {
    let mut categories = HashMap::new();
    categories.insert(
        "Session Lifecycle".to_string(),
        vec![
            "add".to_string(),
            "remove".to_string(),
            "list".to_string(),
            "status".to_string(),
            "focus".to_string(),
        ],
    );
    categories.insert(
        "Workspace Sync".to_string(),
        vec!["sync".to_string(), "diff".to_string()],
    );
    categories.insert(
        "System".to_string(),
        vec![
            "init".to_string(),
            "config".to_string(),
            "doctor".to_string(),
        ],
    );
    categories.insert(
        "Introspection".to_string(),
        vec![
            "context".to_string(),
            "introspect".to_string(),
            "dashboard".to_string(),
        ],
    );
    categories.insert(
        "Utilities".to_string(),
        vec![
            "backup".to_string(),
            "restore".to_string(),
            "verify-backup".to_string(),
            "completions".to_string(),
            "query".to_string(),
        ],
    );
    categories
}

pub fn generate_workflows() -> Vec<Workflow> {
    vec![Workflow {
        name: "Standard Development".to_string(),
        description: "Typical workflow for feature development".to_string(),
        steps: vec![
            WorkflowStep {
                order: 1,
                command: "jjz init".to_string(),
                description: "Initialize jjz (once per repository)".to_string(),
                optional: false,
            },
            WorkflowStep {
                order: 2,
                command: "jjz add feature-name".to_string(),
                description: "Create session with workspace and Zellij tab".to_string(),
                optional: false,
            },
            WorkflowStep {
                order: 3,
                command: "[work in session]".to_string(),
                description: "Develop in isolated environment".to_string(),
                optional: false,
            },
            WorkflowStep {
                order: 4,
                command: "jjz sync feature-name".to_string(),
                description: "Rebase on main branch".to_string(),
                optional: true,
            },
            WorkflowStep {
                order: 5,
                command: "jjz remove feature-name".to_string(),
                description: "Cleanup when done".to_string(),
                optional: false,
            },
        ],
    }]
}

pub fn generate_exit_codes() -> HashMap<i32, String> {
    let mut codes = HashMap::new();
    codes.insert(0, "Success".to_string());
    codes.insert(
        1,
        "User error (invalid input, validation failure, bad configuration)".to_string(),
    );
    codes.insert(
        2,
        "System error (IO failure, external command error, hook failure)".to_string(),
    );
    codes.insert(
        3,
        "Not found (session not found, resource missing, JJ not installed)".to_string(),
    );
    codes.insert(
        4,
        "Invalid state (database corruption, unhealthy system)".to_string(),
    );
    codes
}

pub fn generate_prerequisites() -> Vec<Prerequisite> {
    vec![
        Prerequisite {
            name: "jj".to_string(),
            description: "Jujutsu VCS - Git-compatible version control".to_string(),
            install_url: Some("https://github.com/martinvonz/jj".to_string()),
            check_command: Some("jj --version".to_string()),
            required: true,
        },
        Prerequisite {
            name: "zellij".to_string(),
            description: "Terminal multiplexer for organized layouts".to_string(),
            install_url: Some("https://zellij.dev".to_string()),
            check_command: Some("zellij --version".to_string()),
            required: true,
        },
        Prerequisite {
            name: "JJ repository".to_string(),
            description: "Must be inside a JJ repository or use 'jjz init' to create one"
                .to_string(),
            install_url: None,
            check_command: Some("jj status".to_string()),
            required: true,
        },
    ]
}
