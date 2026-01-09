//! Query command - state queries for AI agents
//!
//! This command provides programmatic access to system state
//! for AI agents to make informed decisions.

use anyhow::Result;
use zjj_core::introspection::{
    Blocker, CanRunQuery, SessionCountQuery, SessionExistsQuery, SessionInfo,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo},
    commands::{get_session_db, zjj_data_dir},
};

/// Run a query
pub fn run(query_type: &str, args: Option<&str>) -> Result<()> {
    match query_type {
        "session-exists" => {
            let name = args.ok_or_else(|| anyhow::anyhow!("Session name required"))?;
            query_session_exists(name)
        }
        "session-count" => query_session_count(args),
        "can-run" => {
            let command = args.ok_or_else(|| anyhow::anyhow!("Command name required"))?;
            query_can_run(command)
        }
        "suggest-name" => {
            let pattern = args.ok_or_else(|| anyhow::anyhow!("Pattern required"))?;
            query_suggest_name(pattern)
        }
        _ => Err(anyhow::anyhow!("Unknown query type: {query_type}")),
    }
}

/// Query if a session exists
fn query_session_exists(name: &str) -> Result<()> {
    let db = get_session_db()?;
    let session = db.get(name)?;

    let result = SessionExistsQuery {
        exists: session.is_some(),
        session: session.map(|s| SessionInfo {
            name: s.name,
            status: s.status.to_string(),
        }),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Query session count
fn query_session_count(filter: Option<&str>) -> Result<()> {
    let db = get_session_db()?;
    let sessions = db.list(None)?;

    let count = if let Some(status_filter) = filter {
        // Parse filter like "--status=active"
        if let Some(status) = status_filter.strip_prefix("--status=") {
            sessions
                .iter()
                .filter(|s| s.status.to_string() == status)
                .count()
        } else {
            sessions.len()
        }
    } else {
        sessions.len()
    };

    let result = SessionCountQuery {
        count,
        filter: filter.map(|f| serde_json::json!({"raw": f})),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Query if a command can run
fn query_can_run(command: &str) -> Result<()> {
    let mut blockers = vec![];
    let mut prereqs_met = 0;
    let prereqs_total = 4; // Adjust based on command

    // Check if initialized
    let initialized = zjj_data_dir().is_ok();
    if !initialized && requires_init(command) {
        blockers.push(Blocker {
            check: "initialized".to_string(),
            status: false,
            message: "jjz not initialized".to_string(),
        });
    } else if requires_init(command) {
        prereqs_met += 1;
    }

    // Check JJ installed
    let jj_installed = is_command_available("jj");
    if !jj_installed && requires_jj(command) {
        blockers.push(Blocker {
            check: "jj_installed".to_string(),
            status: false,
            message: "JJ not installed".to_string(),
        });
    } else if requires_jj(command) {
        prereqs_met += 1;
    }

    // Check JJ repo
    let jj_repo = is_jj_repo().unwrap_or(false);
    if !jj_repo && requires_jj_repo(command) {
        blockers.push(Blocker {
            check: "jj_repo".to_string(),
            status: false,
            message: "Not in a JJ repository".to_string(),
        });
    } else if requires_jj_repo(command) {
        prereqs_met += 1;
    }

    // Check Zellij running
    let zellij_running = is_inside_zellij();
    if !zellij_running && requires_zellij(command) {
        blockers.push(Blocker {
            check: "zellij_running".to_string(),
            status: false,
            message: "Zellij is not running".to_string(),
        });
    } else if requires_zellij(command) {
        prereqs_met += 1;
    }

    let result = CanRunQuery {
        can_run: blockers.is_empty(),
        command: command.to_string(),
        blockers,
        prerequisites_met: prereqs_met,
        prerequisites_total: prereqs_total,
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Query for suggested name based on pattern
fn query_suggest_name(pattern: &str) -> Result<()> {
    let db = get_session_db()?;
    let sessions = db.list(None)?;
    let existing_names: Vec<String> = sessions.into_iter().map(|s| s.name).collect();

    let result = zjj_core::introspection::suggest_name(pattern, &existing_names)?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Check if command requires initialization
fn requires_init(command: &str) -> bool {
    matches!(
        command,
        "add" | "remove" | "list" | "focus" | "status" | "sync" | "diff"
    )
}

/// Check if command requires JJ to be installed
fn requires_jj(command: &str) -> bool {
    matches!(
        command,
        "init" | "add" | "remove" | "status" | "sync" | "diff"
    )
}

/// Check if command requires being in a JJ repo
fn requires_jj_repo(command: &str) -> bool {
    matches!(command, "add" | "remove" | "status" | "sync" | "diff")
}

/// Check if command requires Zellij to be running
fn requires_zellij(command: &str) -> bool {
    matches!(command, "add" | "focus")
}
