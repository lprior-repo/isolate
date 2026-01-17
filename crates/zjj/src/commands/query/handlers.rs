//! Query handlers - execution logic for different query types
//!
//! Each query type has a dedicated handler function that:
//! 1. Processes input arguments
//! 2. Constructs result structures
//! 3. Outputs JSON results
//!
//! All handlers use error propagation with Result type, no unwraps or panics.

use anyhow::Result;
use zjj_core::introspection::{
    Blocker, CanRunQuery, QueryError, SessionCountQuery, SessionExistsQuery, SessionInfo,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo},
    commands::{get_session_db, zjj_data_dir},
};

use super::{filtering, formatting, types::QueryTypeInfo};

/// Run a query
///
/// Main entry point that handles special help queries and dispatches to specific handlers.
pub async fn run(query_type: &str, args: Option<&str>) -> Result<()> {
    // Handle special help queries
    if query_type == "--help" || query_type == "help" || query_type == "--list" {
        println!("{}", QueryTypeInfo::list_all_queries());
        return Ok(());
    }

    match query_type {
        "session-exists" => {
            let name = args.ok_or_else(|| {
                QueryTypeInfo::find("session-exists")
                    .map(|info| anyhow::anyhow!(info.format_error_message()))
                    .unwrap_or_else(|| anyhow::anyhow!("Query type metadata not found"))
            })?;
            query_session_exists(name).await
        }
        "session-count" => query_session_count(args).await,
        "can-run" => {
            let command = args.ok_or_else(|| {
                QueryTypeInfo::find("can-run")
                    .map(|info| anyhow::anyhow!(info.format_error_message()))
                    .unwrap_or_else(|| anyhow::anyhow!("Query type metadata not found"))
            })?;
            query_can_run(command)
        }
        "suggest-name" => {
            let pattern = args.ok_or_else(|| {
                QueryTypeInfo::find("suggest-name")
                    .map(|info| anyhow::anyhow!(info.format_error_message()))
                    .unwrap_or_else(|| anyhow::anyhow!("Query type metadata not found"))
            })?;
            query_suggest_name(pattern).await
        }
        _ => {
            let error_msg = format!(
                "Error: Unknown query type '{}'\n\n{}",
                query_type,
                QueryTypeInfo::list_all_queries()
            );
            Err(anyhow::anyhow!(error_msg))
        }
    }
}

/// Query if a session exists
///
/// Checks the database for a session with the given name and returns
/// session details if found, or error information if the query fails.
async fn query_session_exists(name: &str) -> Result<()> {
    let result = match get_session_db().await {
        Ok(db) => match db.get(name).await {
            Ok(session) => SessionExistsQuery {
                exists: Some(session.is_some()),
                session: session.map(|s| SessionInfo {
                    name: s.name,
                    status: s.status.to_string(),
                }),
                error: None,
            },
            Err(e) => SessionExistsQuery {
                exists: None,
                session: None,
                error: Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to query session: {e}"),
                }),
            },
        },
        Err(e) => {
            let (code, message) = filtering::categorize_db_error(&e);
            SessionExistsQuery {
                exists: None,
                session: None,
                error: Some(QueryError { code, message }),
            }
        }
    };

    formatting::output_json(&result)
}

/// Query session count with optional filtering
///
/// Counts sessions, optionally filtering by status. Filters are specified
/// as `--status=<value>` format.
async fn query_session_count(filter: Option<&str>) -> Result<()> {
    let result = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(sessions) => {
                let count = filtering::extract_status_filter(filter)
                    .map(|status| {
                        sessions
                            .iter()
                            .filter(|s| s.status.to_string() == status)
                            .count()
                    })
                    .unwrap_or_else(|| sessions.len());

                SessionCountQuery {
                    count: Some(count),
                    filter: formatting::create_filter_json(filter),
                    error: None,
                }
            }
            Err(e) => SessionCountQuery {
                count: None,
                filter: formatting::create_filter_json(filter),
                error: Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to list sessions: {e}"),
                }),
            },
        },
        Err(e) => {
            let (code, message) = filtering::categorize_db_error(&e);
            SessionCountQuery {
                count: None,
                filter: formatting::create_filter_json(filter),
                error: Some(QueryError { code, message }),
            }
        }
    };

    formatting::output_json(&result)
}

/// Query if a command can run and show blockers
///
/// Checks all prerequisites for a command and returns which ones are met
/// or blocking execution.
fn query_can_run(command: &str) -> Result<()> {
    // Functional approach: define all prerequisite checks as data
    let checks = [
        (
            "initialized",
            zjj_data_dir().is_ok(),
            "jjz not initialized",
            requires_init(command),
        ),
        (
            "jj_installed",
            is_command_available("jj"),
            "JJ not installed",
            requires_jj(command),
        ),
        (
            "jj_repo",
            is_jj_repo().unwrap_or(false),
            "Not in a JJ repository",
            requires_jj_repo(command),
        ),
        (
            "zellij_running",
            is_inside_zellij(),
            "Zellij is not running",
            requires_zellij(command),
        ),
    ];

    // Functional fold: accumulate blockers and count met prerequisites
    let (blockers, prereqs_met) = checks.iter().fold(
        (Vec::new(), 0_usize),
        |(mut blockers, prereqs_met), &(check_name, status, message, required)| {
            if !status && required {
                blockers.push(Blocker {
                    check: check_name.to_string(),
                    status: false,
                    message: message.to_string(),
                });
                (blockers, prereqs_met)
            } else if required {
                (blockers, prereqs_met.saturating_add(1))
            } else {
                (blockers, prereqs_met)
            }
        },
    );

    let prereqs_total = checks
        .iter()
        .filter(|(_, _, _, required)| *required)
        .count();

    let result = CanRunQuery {
        can_run: blockers.is_empty(),
        command: command.to_string(),
        blockers,
        prerequisites_met: prereqs_met,
        prerequisites_total: prereqs_total,
    };

    formatting::output_json(&result)
}

/// Query for suggested name based on pattern
///
/// Suggests a next available name matching the given pattern.
/// If database access fails, gracefully falls back to empty list.
async fn query_suggest_name(pattern: &str) -> Result<()> {
    // suggest_name can work without database access if we can't get sessions
    let existing_names = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(sessions) => sessions.into_iter().map(|s| s.name).collect(),
            Err(_) => Vec::new(), // Fallback to empty list
        },
        Err(_) => Vec::new(), // Fallback to empty list if prerequisites not met
    };

    let result = zjj_core::introspection::suggest_name(pattern, &existing_names)?;

    formatting::output_json(&result)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_init() {
        assert!(requires_init("add"));
        assert!(requires_init("list"));
        assert!(!requires_init("init"));
        assert!(!requires_init("unknown"));
    }

    #[test]
    fn test_requires_jj() {
        assert!(requires_jj("init"));
        assert!(requires_jj("add"));
        assert!(!requires_jj("list"));
    }

    #[test]
    fn test_requires_jj_repo() {
        assert!(requires_jj_repo("add"));
        assert!(requires_jj_repo("remove"));
        assert!(!requires_jj_repo("init"));
        assert!(!requires_jj_repo("list"));
    }

    #[test]
    fn test_requires_zellij() {
        assert!(requires_zellij("add"));
        assert!(requires_zellij("focus"));
        assert!(!requires_zellij("list"));
        assert!(!requires_zellij("remove"));
    }
}
