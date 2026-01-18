//! Specification builders for individual commands
//!
//! This module contains builder functions for constructing command specifications.
//! Each builder function creates a complete CommandIntrospection for its command.

use zjj_core::introspection::{
    ArgumentSpec, CommandExample, CommandIntrospection, ErrorCondition, FlagSpec, Prerequisites,
};

/// Build introspection spec for the "add" command
pub fn add() -> CommandIntrospection {
    CommandIntrospection {
        command: "add".to_string(),
        description: "Create new parallel development session".to_string(),
        aliases: vec!["a".to_string(), "new".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: Some("^[a-zA-Z0-9_-]+$".to_string()),
            examples: vec![
                "feature-auth".to_string(),
                "bugfix-123".to_string(),
                "experiment".to_string(),
            ],
        }],
        flags: vec![
            FlagSpec {
                long: "no-hooks".to_string(),
                short: None,
                description: "Skip post_create hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "template".to_string(),
                short: Some("t".to_string()),
                description: "Layout template name".to_string(),
                flag_type: "string".to_string(),
                default: Some(serde_json::json!("standard")),
                possible_values: vec![
                    "minimal".to_string(),
                    "standard".to_string(),
                    "full".to_string(),
                ],
            },
            FlagSpec {
                long: "no-open".to_string(),
                short: None,
                description: "Create workspace but don't open Zellij tab".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz add feature-auth".to_string(),
                description: "Create session with default template".to_string(),
            },
            CommandExample {
                command: "jjz add bugfix-123 --no-hooks".to_string(),
                description: "Create without running hooks".to_string(),
            },
            CommandExample {
                command: "jjz add experiment -t minimal".to_string(),
                description: "Create with minimal layout".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: true,
            custom: vec!["Session name must be unique".to_string()],
        },
        side_effects: vec![
            "Creates JJ workspace".to_string(),
            "Generates Zellij layout file".to_string(),
            "Opens Zellij tab".to_string(),
            "Executes post_create hooks".to_string(),
            "Records session in state.db".to_string(),
        ],
        error_conditions: vec![
            ErrorCondition {
                code: "SESSION_ALREADY_EXISTS".to_string(),
                description: "Session with this name exists".to_string(),
                resolution: "Use different name or remove existing session".to_string(),
            },
            ErrorCondition {
                code: "INVALID_SESSION_NAME".to_string(),
                description: "Session name contains invalid characters".to_string(),
                resolution: "Use only alphanumeric, hyphens, underscores".to_string(),
            },
            ErrorCondition {
                code: "ZELLIJ_NOT_RUNNING".to_string(),
                description: "Zellij is not running".to_string(),
                resolution: "Start Zellij first: zellij".to_string(),
            },
        ],
    }
}

/// Build introspection spec for the "remove" command
pub fn remove() -> CommandIntrospection {
    CommandIntrospection {
        command: "remove".to_string(),
        description: "Remove a session and its workspace".to_string(),
        aliases: vec!["rm".to_string(), "delete".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to remove".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![
            FlagSpec {
                long: "force".to_string(),
                short: Some("f".to_string()),
                description: "Skip confirmation prompt and hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "merge".to_string(),
                short: Some("m".to_string()),
                description: "Squash-merge to main before removal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "keep-branch".to_string(),
                short: Some("k".to_string()),
                description: "Preserve branch after removal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz remove my-session".to_string(),
                description: "Remove session with confirmation".to_string(),
            },
            CommandExample {
                command: "jjz remove my-session -f".to_string(),
                description: "Remove without confirmation".to_string(),
            },
            CommandExample {
                command: "jjz remove my-session -m".to_string(),
                description: "Merge changes before removing".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![
            "Closes Zellij tab".to_string(),
            "Removes JJ workspace".to_string(),
            "Deletes layout file".to_string(),
            "Removes session from state.db".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session does not exist".to_string(),
            resolution: "Check session name with 'jjz list'".to_string(),
        }],
    }
}

/// Build introspection spec for the "list" command
pub fn list() -> CommandIntrospection {
    CommandIntrospection {
        command: "list".to_string(),
        description: "List all sessions".to_string(),
        aliases: vec!["ls".to_string()],
        arguments: vec![],
        flags: vec![
            FlagSpec {
                long: "all".to_string(),
                short: None,
                description: "Include completed and failed sessions".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz list".to_string(),
                description: "List active sessions".to_string(),
            },
            CommandExample {
                command: "jjz list --all".to_string(),
                description: "List all sessions including completed".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Build introspection spec for the "init" command
pub fn init() -> CommandIntrospection {
    CommandIntrospection {
        command: "init".to_string(),
        description: "Initialize jjz in a JJ repository".to_string(),
        aliases: vec![],
        arguments: vec![],
        flags: vec![],
        examples: vec![CommandExample {
            command: "jjz init".to_string(),
            description: "Initialize jjz in current directory".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![
            "Creates .zjj directory".to_string(),
            "Creates config.toml".to_string(),
            "Creates sessions.db".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "ALREADY_INITIALIZED".to_string(),
            description: "JJZ already initialized".to_string(),
            resolution: "Remove .zjj directory to reinitialize".to_string(),
        }],
    }
}

/// Build introspection spec for the "focus" command
pub fn focus() -> CommandIntrospection {
    CommandIntrospection {
        command: "focus".to_string(),
        description: "Switch to a session's Zellij tab".to_string(),
        aliases: vec!["switch".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to focus".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "jjz focus my-session".to_string(),
            description: "Switch to my-session tab".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            zellij_running: true,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec!["Switches Zellij tab".to_string()],
        error_conditions: vec![ErrorCondition {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session does not exist".to_string(),
            resolution: "Check session name with 'jjz list'".to_string(),
        }],
    }
}

/// Build introspection spec for the "status" command
pub fn status() -> CommandIntrospection {
    CommandIntrospection {
        command: "status".to_string(),
        description: "Show detailed session status".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "watch".to_string(),
                short: None,
                description: "Continuously update status".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz status".to_string(),
                description: "Show status of all sessions".to_string(),
            },
            CommandExample {
                command: "jjz status my-session".to_string(),
                description: "Show status of specific session".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Build introspection spec for the "sync" command
pub fn sync() -> CommandIntrospection {
    CommandIntrospection {
        command: "sync".to_string(),
        description: "Sync session workspace with main (rebase)".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (syncs current if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "jjz sync my-session".to_string(),
            description: "Sync session with main branch".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![
            "Rebases workspace onto main".to_string(),
            "Updates last_synced timestamp".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "CONFLICTS".to_string(),
            description: "Rebase resulted in conflicts".to_string(),
            resolution: "Resolve conflicts manually".to_string(),
        }],
    }
}

/// Build introspection spec for the "diff" command
pub fn diff() -> CommandIntrospection {
    CommandIntrospection {
        command: "diff".to_string(),
        description: "Show diff between session and main".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "stat".to_string(),
            short: None,
            description: "Show diffstat only".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
        }],
        examples: vec![
            CommandExample {
                command: "jjz diff my-session".to_string(),
                description: "Show full diff".to_string(),
            },
            CommandExample {
                command: "jjz diff my-session --stat".to_string(),
                description: "Show diffstat summary".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Build introspection spec for the "introspect" command
pub fn introspect() -> CommandIntrospection {
    CommandIntrospection {
        command: "introspect".to_string(),
        description: "Discover jjz capabilities".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "command".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Command to introspect (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["add".to_string(), "remove".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
        }],
        examples: vec![
            CommandExample {
                command: "jjz introspect".to_string(),
                description: "Show all capabilities".to_string(),
            },
            CommandExample {
                command: "jjz introspect add --json".to_string(),
                description: "Get add command schema as JSON".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Build introspection spec for the "doctor" command
pub fn doctor() -> CommandIntrospection {
    CommandIntrospection {
        command: "doctor".to_string(),
        description: "Run system health checks".to_string(),
        aliases: vec!["check".to_string()],
        arguments: vec![],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "fix".to_string(),
                short: None,
                description: "Auto-fix issues where possible".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz doctor".to_string(),
                description: "Check system health".to_string(),
            },
            CommandExample {
                command: "jjz doctor --fix".to_string(),
                description: "Auto-fix issues".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec!["May fix issues with --fix flag".to_string()],
        error_conditions: vec![],
    }
}

/// Build introspection spec for the "query" command
pub fn query() -> CommandIntrospection {
    CommandIntrospection {
        command: "query".to_string(),
        description: "Query system state".to_string(),
        aliases: vec![],
        arguments: vec![
            ArgumentSpec {
                name: "query_type".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Type of query".to_string(),
                validation: None,
                examples: vec![
                    "session-exists".to_string(),
                    "session-count".to_string(),
                    "can-run".to_string(),
                    "suggest-name".to_string(),
                ],
            },
            ArgumentSpec {
                name: "args".to_string(),
                arg_type: "string".to_string(),
                required: false,
                description: "Query-specific arguments".to_string(),
                validation: None,
                examples: vec!["my-session".to_string(), "feature-{n}".to_string()],
            },
        ],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(true)),
            possible_values: vec![],
        }],
        examples: vec![
            CommandExample {
                command: "jjz query session-exists my-session".to_string(),
                description: "Check if session exists".to_string(),
            },
            CommandExample {
                command: "jjz query can-run add".to_string(),
                description: "Check if add command can run".to_string(),
            },
            CommandExample {
                command: "jjz query suggest-name feature-{n}".to_string(),
                description: "Suggest next available name".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}
