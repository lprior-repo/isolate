//! Tool metadata, categories, workflows, and prerequisites

use im::{hashmap, vector, HashMap, Vector};

use super::types::{CoreConcept, Prerequisite, ToolMetadata, Workflow, WorkflowStep};

pub fn generate_tool_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "zjj".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Manage JJ workspaces with Zellij sessions".to_string(),
        purpose: "A workflow tool that manages isolated development sessions by combining JJ (Jujutsu) workspaces for parallel Git branches, Zellij terminal multiplexer for organized UI layouts, and SQLite database for session state tracking.".to_string(),
        core_concepts: vector![
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

pub fn generate_categories() -> HashMap<String, Vector<String>> {
    hashmap! {
        "Session Lifecycle".into() => vector![
            "add".into(),
            "remove".into(),
            "list".into(),
            "status".into(),
            "focus".into(),
        ],
        "Workspace Sync".into() => vector!["sync".into(), "diff".into()],
        "System".into() => vector![
            "init".into(),
            "config".into(),
            "doctor".into(),
        ],
        "Introspection".into() => vector![
            "context".into(),
            "introspect".into(),
            "dashboard".into(),
        ],
        "Utilities".into() => vector![
            "backup".into(),
            "restore".into(),
            "verify-backup".into(),
            "completions".into(),
            "query".into(),
        ],
    }
}

pub fn generate_workflows() -> Vector<Workflow> {
    vector![Workflow {
        name: "Standard Development".to_string(),
        description: "Typical workflow for feature development".to_string(),
        steps: vector![
            WorkflowStep {
                order: 1,
                command: "zjj init".to_string(),
                description: "Initialize zjj (once per repository)".to_string(),
                optional: false,
            },
            WorkflowStep {
                order: 2,
                command: "zjj add feature-name".to_string(),
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
                command: "zjj sync feature-name".to_string(),
                description: "Rebase on main branch".to_string(),
                optional: true,
            },
            WorkflowStep {
                order: 5,
                command: "zjj remove feature-name".to_string(),
                description: "Cleanup when done".to_string(),
                optional: false,
            },
        ],
    }]
}

pub fn generate_exit_codes() -> HashMap<i32, String> {
    hashmap! {
        0 => "Success".into(),
        1 => "User error (invalid input, validation failure, bad configuration)".into(),
        2 => "System error (IO failure, external command error, hook failure)".into(),
        3 => "Not found (session not found, resource missing, JJ not installed)".into(),
        4 => "Invalid state (database corruption, unhealthy system)".into(),
    }
}

pub fn generate_prerequisites() -> Vector<Prerequisite> {
    vector![
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
            description: "Must be inside a JJ repository or use 'zjj init' to create one"
                .to_string(),
            install_url: None,
            check_command: Some("jj status".to_string()),
            required: true,
        },
    ]
}
