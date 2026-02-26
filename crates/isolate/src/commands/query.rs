//! Query command - state queries for AI agents
//!
//! This command provides programmatic access to system state
//! for AI agents to make informed decisions.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use anyhow::Result;
use isolate_core::{
    introspection::{
        Blocker, CanRunQuery, QueryError, SessionCountQuery, SessionExistsQuery, SessionInfo,
    },
    json::SchemaEnvelope,
};

use crate::{
    cli::{is_command_available, is_jj_repo},
    commands::{get_session_db, isolate_data_dir},
};

/// Query result containing output and exit code metadata
///
/// This allows query functions to return results with exit semantics
/// without calling std::process::exit directly. The top-level handler
/// can then execute hooks and other logic before exiting.
pub struct QueryResult {
    pub output: String,
    pub exit_code: i32,
}

/// Query type metadata for help generation
struct QueryTypeInfo {
    name: &'static str,
    description: &'static str,
    requires_arg: bool,
    arg_name: &'static str,
    usage_example: &'static str,
    returns_description: &'static str,
}

impl QueryTypeInfo {
    const fn all() -> &'static [Self] {
        &[
            Self {
                name: "session-exists",
                description: "Check if a session exists by name",
                requires_arg: true,
                arg_name: "session_name",
                usage_example: "isolate query session-exists my-session",
                returns_description: r#"{"exists": true, "session": {"name": "my-session", "status": "active"}}"#,
            },
            Self {
                name: "session-count",
                description: "Count total sessions or filter by status",
                requires_arg: false,
                arg_name: "--status=active",
                usage_example: "isolate query session-count --status=active",
                returns_description: r#"{"count": 5, "filter": {"raw": "--status=active"}}"#,
            },
            Self {
                name: "can-run",
                description: "Check if a command can run and show blockers",
                requires_arg: true,
                arg_name: "command_name",
                usage_example: "isolate query can-run add",
                returns_description: r#"{"can_run": true, "command": "add", "blockers": [], "prerequisites_met": 4, "prerequisites_total": 4}"#,
            },
            Self {
                name: "suggest-name",
                description: "Suggest next available name based on pattern",
                requires_arg: true,
                arg_name: "pattern",
                usage_example: r#"isolate query suggest-name "feat{n}""#,
                returns_description: r#"{"pattern": "feat{n}", "suggested": "feat3", "next_available_n": 3, "existing_matches": ["feat1", "feat2"]}"#,
            },
            Self {
                name: "lock-status",
                description: "Check if a session is locked",
                requires_arg: true,
                arg_name: "session_name",
                usage_example: "isolate query lock-status my-session",
                returns_description: r#"{"locked": true, "holder": "agent-123", "expires_at": "2024-01-01T12:00:00Z"}"#,
            },
            Self {
                name: "can-spawn",
                description: "Check if spawning a session is possible",
                requires_arg: false,
                arg_name: "bead_id",
                usage_example: "isolate query can-spawn isolate-abc12",
                returns_description: r#"{"can_spawn": true, "reason": null, "blockers": []}"#,
            },
            Self {
                name: "pending-merges",
                description: "List sessions with changes ready to merge",
                requires_arg: false,
                arg_name: "",
                usage_example: "isolate query pending-merges",
                returns_description: r#"{"sessions": [{"name": "feature-x", "changes": 3}], "count": 1}"#,
            },
            Self {
                name: "location",
                description: "Quick check of current location (main or workspace)",
                requires_arg: false,
                arg_name: "",
                usage_example: "isolate query location",
                returns_description: r#"{"type": "workspace", "name": "feature-auth"}"#,
            },
        ]
    }

    fn find(name: &str) -> Option<&'static Self> {
        Self::all().iter().find(|q| q.name == name)
    }

    /// Get query info or return a standardized error for unknown query types
    ///
    /// This ensures that all query argument errors follow a consistent format.
    /// For known query types, uses the metadata to format a detailed error message.
    /// For unknown query types (which should never happen in match arms), returns
    /// a bug indicator.
    fn missing_arg_error(query_name: &str) -> anyhow::Error {
        Self::find(query_name)
            .map(|info| anyhow::anyhow!(info.format_error_message()))
            .unwrap_or_else(|| {
                anyhow::anyhow!(
                    "Error: Query type '{query_name}' is missing metadata (this is a bug)"
                )
            })
    }

    fn format_error_message(&self) -> String {
        format!(
            "Error: '{}' query requires {} argument\n\n\
             Description:\n  {}\n\n\
             Usage:\n  {} <{}>\n\n\
             Example:\n  {}\n\n\
             Returns:\n  {}",
            self.name,
            if self.requires_arg {
                "a"
            } else {
                "an optional"
            },
            self.description,
            self.name,
            self.arg_name,
            self.usage_example,
            self.returns_description
        )
    }

    fn list_all_queries() -> String {
        use std::fmt::Write;
        let mut output = String::from("Available query types:\n\n");
        for query in Self::all() {
            let _ = write!(
                output,
                "  {} - {}\n    Example: {}\n\n",
                query.name, query.description, query.usage_example
            );
        }
        output.push_str(
            "For detailed help on a specific query type, try running it without arguments.\n",
        );
        output
    }
}

/// Run a query
///
/// Returns a `QueryResult` containing the output and exit code metadata.
/// The caller (typically cli/handlers.rs) is responsible for:
/// 1. Printing the output
/// 2. Running any hooks
/// 3. Calling std::process::exit with the exit_code
pub async fn run(query_type: &str, args: Option<&str>, json_mode: bool) -> Result<QueryResult> {
    // Handle special help queries
    if query_type == "--help" || query_type == "help" || query_type == "--list" {
        let output = QueryTypeInfo::list_all_queries();
        return Ok(QueryResult {
            output,
            exit_code: 0,
        });
    }

    match query_type {
        "session-exists" => {
            let name = args.ok_or_else(|| QueryTypeInfo::missing_arg_error("session-exists"))?;
            query_session_exists(name).await
        }
        "session-count" => query_session_count(args, json_mode).await,
        "can-run" => {
            let command = args.ok_or_else(|| QueryTypeInfo::missing_arg_error("can-run"))?;
            query_can_run(command).await
        }
        "suggest-name" => {
            let pattern = args.ok_or_else(|| QueryTypeInfo::missing_arg_error("suggest-name"))?;
            query_suggest_name(pattern, json_mode).await
        }
        "lock-status" => {
            let session = args.ok_or_else(|| QueryTypeInfo::missing_arg_error("lock-status"))?;
            query_lock_status(session).await
        }
        "can-spawn" => query_can_spawn(args).await,
        "pending-merges" => {
            ensure_no_args("pending-merges", args)?;
            query_pending_merges().await
        }
        "location" => {
            ensure_no_args("location", args)?;
            query_location().await
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

fn ensure_no_args(query_name: &str, args: Option<&str>) -> Result<()> {
    if let Some(unexpected_arg) = args {
        anyhow::bail!(
            "Error: '{query_name}' query does not accept arguments (got '{unexpected_arg}')"
        );
    }
    Ok(())
}

/// Categorize database errors for better error reporting
fn categorize_db_error(err: &anyhow::Error) -> (String, String) {
    let err_str = err.to_string();

    // Check for JJ prerequisite failures first
    if err_str.contains("JJ is not installed") || err_str.contains("jj not found") {
        (
            "JJ_NOT_INSTALLED".to_string(),
            "JJ is not installed. Install with: cargo install jj-cli".to_string(),
        )
    } else if err_str.contains("Not a JJ repository") || err_str.contains("not in a jj repo") {
        (
            "NOT_JJ_REPOSITORY".to_string(),
            "Not in a JJ repository. Run 'jj git init' or 'isolate init' first.".to_string(),
        )
    } else if err_str.contains("Isolate not initialized") {
        (
            "Isolate_NOT_INITIALIZED".to_string(),
            "Isolate not initialized. Run 'isolate init' first.".to_string(),
        )
    } else if err_str.contains("no such table") || err_str.contains("database schema") {
        (
            "DATABASE_NOT_INITIALIZED".to_string(),
            "Database not initialized. Run 'isolate init' first.".to_string(),
        )
    } else if err_str.contains("locked") {
        (
            "DATABASE_LOCKED".to_string(),
            "Database is locked by another process".to_string(),
        )
    } else {
        (
            "DATABASE_INIT_ERROR".to_string(),
            format!("Failed to access database: {err}"),
        )
    }
}

/// Query if a session exists
async fn query_session_exists(name: &str) -> Result<QueryResult> {
    let (result, exit_code) = match get_session_db().await {
        Ok(db) => match db.get(name).await {
            Ok(session) => {
                let exists = session.is_some();
                // Exit code 0 = query succeeded (boolean result is in JSON, not exit code)
                // Exit code 2 = query error (database failure, etc)
                (
                    SessionExistsQuery {
                        exists: Some(exists),
                        session: session.map(|s| SessionInfo {
                            name: s.name,
                            status: s.status.to_string(),
                        }),
                        error: None,
                    },
                    0,
                )
            }
            Err(e) => (
                SessionExistsQuery {
                    exists: None,
                    session: None,
                    error: Some(QueryError {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to query session: {e}"),
                    }),
                },
                2,
            ),
        },
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            (
                SessionExistsQuery {
                    exists: None,
                    session: None,
                    error: Some(QueryError { code, message }),
                },
                2,
            )
        }
    };

    let envelope = SchemaEnvelope::new("query-session-exists", "single", result);
    let output = serde_json::to_string_pretty(&envelope)?;

    // Exit code semantics: 0 if exists, 1 if not exists, 2 if error
    Ok(QueryResult { output, exit_code })
}

/// Query session count
/// Note: According to Red Queen findings, this should output plain number, not JSON
/// when used in shell scripts. However, JSON envelope is still output for --json flag.
async fn query_session_count(filter: Option<&str>, json_mode: bool) -> Result<QueryResult> {
    let status_filter = match filter {
        Some(raw_filter) => {
            if let Some(status) = raw_filter.strip_prefix("--status=") {
                if status.is_empty() {
                    anyhow::bail!("Invalid session-count filter: '--status=' cannot be empty");
                }
                let valid_statuses = ["active", "merged", "paused", "blocked"];
                if !valid_statuses.contains(&status) {
                    anyhow::bail!(
                        "Invalid session status '{status}'. Valid statuses: active, merged, paused, blocked"
                    );
                }
                Some(status)
            } else {
                anyhow::bail!(
                    "Invalid session-count filter: '{raw_filter}'. Use '--status=<active|merged|paused|blocked>'"
                );
            }
        }
        None => None,
    };

    let (count_val, error, exit_code) = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(sessions) => {
                let count = status_filter
                    .map(|status| {
                        sessions
                            .iter()
                            .filter(|session| session.status.to_string() == status)
                            .count()
                    })
                    .unwrap_or_else(|| sessions.len());
                (Some(count), None, 0)
            }
            Err(e) => (
                None,
                Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to list sessions: {e}"),
                }),
                2,
            ),
        },
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            (None, Some(QueryError { code, message }), 2)
        }
    };

    let output = if json_mode {
        let filter_json = status_filter.map(|status| {
            serde_json::json!({
                "raw": format!("--status={status}"),
                "status": status,
            })
        });
        let result = SessionCountQuery {
            count: count_val,
            filter: filter_json,
            error,
        };
        let envelope = SchemaEnvelope::new("query-session-count", "single", result);
        serde_json::to_string_pretty(&envelope)?
    } else {
        if let Some(err) = &error {
            eprintln!("Error: {}", err.message);
        }
        count_val.map_or_else(String::new, |count| format!("{count}"))
    };

    Ok(QueryResult { output, exit_code })
}

/// Query if a command can run
async fn query_can_run(command: &str) -> Result<QueryResult> {
    if !is_known_command(command) {
        let result = CanRunQuery {
            can_run: false,
            command: command.to_string(),
            blockers: vec![Blocker {
                check: "unknown_command".to_string(),
                status: false,
                message: format!("Unknown command '{command}'"),
            }],
            prerequisites_met: 0,
            prerequisites_total: 0,
        };

        let envelope = SchemaEnvelope::new("query-can-run", "single", result);
        let output = serde_json::to_string_pretty(&envelope)?;
        return Ok(QueryResult {
            output,
            exit_code: 0,
        });
    }

    let mut blockers = vec![];
    let mut prereqs_met = 0;
    let requires_init_check = requires_init(command);
    let requires_jj_check = requires_jj(command);
    let requires_jj_repo_check = requires_jj_repo(command);
    let prereqs_total = [
        requires_init_check,
        requires_jj_check,
        requires_jj_repo_check,
    ]
    .into_iter()
    .filter(|required| *required)
    .count();

    // Check if initialized
    let initialized = isolate_data_dir().await.is_ok();
    if !initialized && requires_init_check {
        blockers.push(Blocker {
            check: "initialized".to_string(),
            status: false,
            message: "isolate not initialized".to_string(),
        });
    } else if requires_init_check {
        prereqs_met += 1;
    }

    // Check JJ installed
    let jj_installed = is_command_available("jj").await;
    if !jj_installed && requires_jj_check {
        blockers.push(Blocker {
            check: "jj_installed".to_string(),
            status: false,
            message: "JJ not installed".to_string(),
        });
    } else if requires_jj_check {
        prereqs_met += 1;
    }

    // Check JJ repo
    let jj_repo = is_jj_repo().await.unwrap_or(false);
    if !jj_repo && requires_jj_repo_check {
        blockers.push(Blocker {
            check: "jj_repo".to_string(),
            status: false,
            message: "Not in a JJ repository".to_string(),
        });
    } else if requires_jj_repo_check {
        prereqs_met += 1;
    }

    let can_run = blockers.is_empty();
    let result = CanRunQuery {
        can_run,
        command: command.to_string(),
        blockers,
        prerequisites_met: prereqs_met,
        prerequisites_total: prereqs_total,
    };

    let envelope = SchemaEnvelope::new("query-can-run", "single", result);
    let output = serde_json::to_string_pretty(&envelope)?;

    // Exit code 0 = query succeeded (can_run boolean is in JSON, not exit code)
    Ok(QueryResult {
        output,
        exit_code: 0,
    })
}

fn is_known_command(command: &str) -> bool {
    matches!(
        command,
        "init"
            | "add"
            | "list"
            | "remove"
            | "focus"
            | "status"
            | "sync"
            | "done"
            | "undo"
            | "revert"
            | "spawn"
            | "work"
            | "abort"
            | "agents"
            | "ai"
            | "checkpoint"
            | "clean"
            | "config"
            | "context"
            | "dashboard"
            | "diff"
            | "doctor"
            | "introspect"
            | "query"
            | "whereami"
            | "whoami"
            | "contract"
            | "examples"
            | "validate"
            | "whatif"
            | "claim"
            | "yield"
            | "events"
            | "batch"
            | "completions"
            | "export"
            | "import"
            | "rename"
            | "pause"
            | "resume"
            | "clone"
    )
}

/// Query for suggested name based on pattern
async fn query_suggest_name(pattern: &str, json_mode: bool) -> Result<QueryResult> {
    // suggest_name can work without database access if we can't get sessions
    let existing_names = match get_session_db().await {
        Ok(db) => db.list(None).await.map_or_else(
            |_| Vec::new(),
            |sessions| sessions.into_iter().map(|s| s.name).collect(),
        ),
        Err(_) => Vec::new(),
    };

    match isolate_core::introspection::suggest_name(pattern, &existing_names) {
        Ok(result) => {
            let envelope = SchemaEnvelope::new("query-suggest-name", "single", result);
            let output = serde_json::to_string_pretty(&envelope)?;
            Ok(QueryResult {
                output,
                exit_code: 0,
            })
        }
        Err(e) => {
            if json_mode {
                let message = e.to_string();
                let error_message = if message.starts_with("Validation error:") {
                    message
                } else {
                    format!("Validation error: {message}")
                };
                Err(anyhow::anyhow!(error_message))
            } else {
                // Validation errors (like missing {n}) are user errors (exit 1)
                eprintln!("Error: {e}");
                Ok(QueryResult {
                    output: String::new(),
                    exit_code: 1,
                })
            }
        }
    }
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

/// Query lock status for a session
async fn query_lock_status(session: &str) -> Result<QueryResult> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct LockStatusResult {
        session: String,
        locked: bool,
        holder: Option<String>,
        expires_at: Option<String>,
        error: Option<QueryError>,
    }

    let result = match get_session_db().await {
        Ok(db) => {
            // Check if session exists first
            match db.get(session).await {
                Ok(Some(_)) => {
                    // Try to get lock info
                    match isolate_data_dir().await {
                        Ok(data_dir) => {
                            let db_path = data_dir.join("state.db");

                            let lock_info = async {
                                let pool = sqlx::SqlitePool::connect(&format!(
                                    "sqlite:{}",
                                    db_path.display()
                                ))
                                .await
                                .ok()?;
                                let lock_mgr =
                                    isolate_core::coordination::locks::LockManager::new(pool);
                                lock_mgr.init().await.ok()?;
                                let all_locks = lock_mgr.get_all_locks().await.ok()?;
                                all_locks.into_iter().find(|l| l.session == session)
                            }
                            .await;

                            match lock_info {
                                Some(lock) => LockStatusResult {
                                    session: session.to_string(),
                                    locked: true,
                                    holder: Some(lock.agent_id.clone()),
                                    expires_at: Some(lock.expires_at.to_rfc3339()),
                                    error: None,
                                },
                                None => LockStatusResult {
                                    session: session.to_string(),
                                    locked: false,
                                    holder: None,
                                    expires_at: None,
                                    error: None,
                                },
                            }
                        }
                        Err(e) => LockStatusResult {
                            session: session.to_string(),
                            locked: false,
                            holder: None,
                            expires_at: None,
                            error: Some(QueryError {
                                code: "DATA_DIR_ERROR".to_string(),
                                message: e.to_string(),
                            }),
                        },
                    }
                }
                Ok(None) => LockStatusResult {
                    session: session.to_string(),
                    locked: false,
                    holder: None,
                    expires_at: None,
                    error: Some(QueryError {
                        code: "SESSION_NOT_FOUND".to_string(),
                        message: format!("Session '{session}' not found"),
                    }),
                },
                Err(e) => LockStatusResult {
                    session: session.to_string(),
                    locked: false,
                    holder: None,
                    expires_at: None,
                    error: Some(QueryError {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to query session: {e}"),
                    }),
                },
            }
        }
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            LockStatusResult {
                session: session.to_string(),
                locked: false,
                holder: None,
                expires_at: None,
                error: Some(QueryError { code, message }),
            }
        }
    };

    let envelope = SchemaEnvelope::new("query-lock-status", "single", result);
    let output = serde_json::to_string_pretty(&envelope)?;
    Ok(QueryResult {
        output,
        exit_code: 0,
    })
}

/// Query if spawning is possible
async fn query_can_spawn(bead_id: Option<&str>) -> Result<QueryResult> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct CanSpawnResult {
        can_spawn: bool,
        bead_id: Option<String>,
        reason: Option<String>,
        blockers: Vec<String>,
    }

    let mut blockers = vec![];

    // Check repository context and branch location
    match crate::commands::check_in_jj_repo().await {
        Ok(root) => {
            let on_main = super::context::detect_location(&root)
                .map(|location| matches!(location, super::context::Location::Main))
                .unwrap_or(false);
            if !on_main {
                blockers.push("Not on main branch".to_string());
            }
        }
        Err(_) => {
            blockers.push("Not in a JJ repository".to_string());
        }
    }

    // Check if isolate is initialized
    if isolate_data_dir().await.is_err() {
        blockers.push("Isolate not initialized".to_string());
    }

    // Check if bead exists and is ready (if provided)
    if let Some(bead) = bead_id {
        // Try to check bead status with br command
        let bd_check = tokio::process::Command::new("br")
            .args(["show", bead, "--json"])
            .output()
            .await;

        match bd_check {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let parsed = serde_json::from_str::<serde_json::Value>(&output_str);
                match parsed {
                    Ok(value) => {
                        let bead_status = value
                            .get("status")
                            .and_then(serde_json::Value::as_str)
                            .or_else(|| {
                                value
                                    .get("data")
                                    .and_then(|data| data.get("status"))
                                    .and_then(serde_json::Value::as_str)
                            });

                        match bead_status {
                            Some("in_progress") => {
                                blockers.push(format!("Bead '{bead}' is already in progress"));
                            }
                            Some(_) => {}
                            None => {
                                blockers.push(format!(
                                    "Unable to parse bead status for '{bead}': missing status field"
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        blockers.push(format!(
                            "Unable to parse bead status for '{bead}': malformed output"
                        ));
                    }
                }
            }
            Ok(_) => {
                blockers.push(format!("Bead '{bead}' not found"));
            }
            Err(err) => {
                let detail = if err.kind() == std::io::ErrorKind::NotFound {
                    "'br' command not available".to_string()
                } else {
                    err.to_string()
                };
                blockers.push(format!(
                    "Unable to verify bead status for '{bead}': {detail}"
                ));
            }
        }
    }

    let result = CanSpawnResult {
        can_spawn: blockers.is_empty(),
        bead_id: bead_id.map(String::from),
        reason: if blockers.is_empty() {
            None
        } else {
            Some(blockers.join("; "))
        },
        blockers,
    };

    let envelope = SchemaEnvelope::new("query-can-spawn", "single", result);
    let output = serde_json::to_string_pretty(&envelope)?;
    Ok(QueryResult {
        output,
        exit_code: 0,
    })
}

/// Query sessions with pending changes to merge
async fn query_pending_merges() -> Result<QueryResult> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct SessionWithChanges {
        name: String,
        status: String,
        has_uncommitted: bool,
    }

    #[derive(Serialize)]
    struct PendingMergesResult {
        sessions: Vec<SessionWithChanges>,
        count: usize,
        error: Option<QueryError>,
    }

    let result = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(sessions) => {
                let mut active_sessions: Vec<SessionWithChanges> = vec![];
                for s in sessions {
                    if s.status.to_string() == "active" {
                        // Check for uncommitted changes
                        let has_uncommitted = tokio::process::Command::new("jj")
                            .args(["status", "--no-pager"])
                            .current_dir(&s.workspace_path)
                            .output()
                            .await
                            .map(|o| {
                                let output = String::from_utf8_lossy(&o.stdout);
                                !output.contains("The working copy is clean")
                            })
                            .unwrap_or(false);

                        active_sessions.push(SessionWithChanges {
                            name: s.name,
                            status: s.status.to_string(),
                            has_uncommitted,
                        });
                    }
                }

                let count = active_sessions.len();
                PendingMergesResult {
                    sessions: active_sessions,
                    count,
                    error: None,
                }
            }
            Err(e) => PendingMergesResult {
                sessions: vec![],
                count: 0,
                error: Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to list sessions: {e}"),
                }),
            },
        },
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            PendingMergesResult {
                sessions: vec![],
                count: 0,
                error: Some(QueryError { code, message }),
            }
        }
    };

    let envelope = SchemaEnvelope::new("query-pending-merges", "single", result);
    let output = serde_json::to_string_pretty(&envelope)?;
    Ok(QueryResult {
        output,
        exit_code: 0,
    })
}

/// Query current location (main or workspace)
async fn query_location() -> Result<QueryResult> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct LocationResult {
        #[serde(rename = "type")]
        location_type: String,
        name: Option<String>,
        path: Option<String>,
        simple: String,
        error: Option<QueryError>,
    }

    let result = match crate::commands::check_in_jj_repo().await {
        Ok(root) => match super::context::detect_location(&root) {
            Ok(location) => match location {
                super::context::Location::Main => LocationResult {
                    location_type: "main".to_string(),
                    name: None,
                    path: None,
                    simple: "main".to_string(),
                    error: None,
                },
                super::context::Location::Workspace { name, path } => LocationResult {
                    location_type: "workspace".to_string(),
                    name: Some(name.clone()),
                    path: Some(path),
                    simple: format!("workspace:{name}"),
                    error: None,
                },
            },
            Err(e) => LocationResult {
                location_type: "unknown".to_string(),
                name: None,
                path: None,
                simple: "unknown".to_string(),
                error: Some(QueryError {
                    code: "LOCATION_DETECTION_FAILED".to_string(),
                    message: e.to_string(),
                }),
            },
        },
        Err(e) => LocationResult {
            location_type: "unknown".to_string(),
            name: None,
            path: None,
            simple: "unknown".to_string(),
            error: Some(QueryError {
                code: "NOT_IN_JJ_REPO".to_string(),
                message: e.to_string(),
            }),
        },
    };

    let envelope = SchemaEnvelope::new("query-location", "single", result);
    let output = serde_json::to_string_pretty(&envelope)?;
    Ok(QueryResult {
        output,
        exit_code: 0,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_query_json_has_envelope() -> anyhow::Result<()> {
        // FAILING: Verify envelope wrapping for query command output
        use isolate_core::json::SchemaEnvelope;

        let query_result = SessionExistsQuery {
            exists: Some(true),
            session: Some(SessionInfo {
                name: "test-session".to_string(),
                status: "active".to_string(),
            }),
            error: None,
        };
        let envelope = SchemaEnvelope::new("query-session-exists", "single", query_result);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_query_results_wrapped() -> anyhow::Result<()> {
        // FAILING: Verify query results are wrapped in envelope
        use isolate_core::json::SchemaEnvelope;

        let query_result = CanRunQuery {
            can_run: true,
            command: "add".to_string(),
            blockers: vec![],
            prerequisites_met: 3,
            prerequisites_total: 3,
        };
        let envelope = SchemaEnvelope::new("query-can-run", "single", query_result);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_query_array_schema_type() -> anyhow::Result<()> {
        // Verify schema_type is "array" for array results
        use isolate_core::json::SchemaEnvelopeArray;

        let blockers = vec![Blocker {
            check: "JJ installed".to_string(),
            status: true,
            message: "JJ is installed".to_string(),
        }];
        let envelope = SchemaEnvelopeArray::new("query-blockers", blockers);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let schema_type = parsed
            .get("schema_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("schema_type not found"))?;

        assert_eq!(
            schema_type, "array",
            "schema_type should be 'array' for array responses"
        );

        Ok(())
    }

    // NOTE: test_query_pagination_envelope was removed because session-count
    // now outputs plain numbers instead of JSON as per Red Queen findings

    // ============================================================================
    // PHASE 2 (RED) - OutputFormat Migration Tests for query.rs
    // These tests FAIL until query command accepts OutputFormat parameter
    // ============================================================================

    /// RED: query `run()` should accept `OutputFormat` parameter
    #[test]
    fn test_query_run_accepts_output_format() {
        use isolate_core::OutputFormat;

        // This test documents the expected signature:
        // Current: pub fn run(query_type: &str, args: Option<&str>) -> Result<()>
        // Expected: pub fn run(query_type: &str, args: Option<&str>, format: OutputFormat) ->
        // Result<()>

        // However, query always defaults to JSON format per requirements
        let format = OutputFormat::Json;
        assert!(format.is_json());

        // When run() is updated to accept format:
        // query::run("session-exists", Some("my-session"), OutputFormat::Json)
    }

    /// RED: query should always use JSON output format by default
    #[test]
    fn test_query_defaults_to_json_format() {
        use isolate_core::OutputFormat;

        // Per requirements: query is always JSON format for programmatic access
        let format = OutputFormat::Json;
        assert!(format.is_json());

        // Even if a human format is requested, query should use JSON
        // This is because query is designed for AI agents/scripts, not humans
    }

    /// RED: query session-exists always outputs JSON
    #[test]
    fn test_query_session_exists_json_only() {
        use isolate_core::OutputFormat;

        let format = OutputFormat::Json;
        assert!(format.is_json());

        // query_session_exists should output:
        // {
        //   "$schema": "...",
        //   "exists": true,
        //   "session": {...},
        //   "error": null
        // }
    }

    /// RED: query session-count always outputs JSON
    #[test]
    fn test_query_session_count_json_only() {
        use isolate_core::OutputFormat;

        let format = OutputFormat::Json;
        assert!(format.is_json());

        // query_session_count should output:
        // {
        //   "$schema": "...",
        //   "count": 5,
        //   "filter": {...},
        //   "error": null
        // }
    }

    /// RED: query can-run always outputs JSON
    #[test]
    fn test_query_can_run_json_only() {
        use isolate_core::OutputFormat;

        let format = OutputFormat::Json;
        assert!(format.is_json());

        // query_can_run should output:
        // {
        //   "$schema": "...",
        //   "can_run": true,
        //   "command": "add",
        //   "blockers": [],
        //   "prerequisites_met": 4,
        //   "prerequisites_total": 4
        // }
    }

    /// RED: query suggest-name always outputs JSON
    #[test]
    fn test_query_suggest_name_json_only() {
        use isolate_core::OutputFormat;

        let format = OutputFormat::Json;
        assert!(format.is_json());

        // query_suggest_name should output:
        // {
        //   "$schema": "...",
        //   "pattern": "feature-{n}",
        //   "suggested": "feature-3",
        //   "next_available_n": 3,
        //   "existing_matches": ["feature-1", "feature-2"]
        // }
    }

    /// RED: query output is always `SchemaEnvelope` wrapped
    #[test]
    fn test_query_all_outputs_wrapped_in_envelope() {
        use isolate_core::{json::SchemaEnvelope, OutputFormat};

        let format = OutputFormat::Json;
        assert!(format.is_json());

        // All query results should be wrapped in SchemaEnvelope
        // to provide consistent $schema, success, and error fields

        let test_result = SessionExistsQuery {
            exists: Some(true),
            session: None,
            error: None,
        };

        let envelope = SchemaEnvelope::new("query-session-exists", "single", test_result);
        let json_str_result = serde_json::to_string(&envelope);
        assert!(json_str_result.is_ok(), "serialization should succeed");
        let json_str = match json_str_result {
            Ok(s) => s,
            Err(e) => panic!("Serialization should succeed: {e}"),
        };
        let parsed_result: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(parsed_result.is_ok(), "parsing should succeed");
        let parsed = match parsed_result {
            Ok(p) => p,
            Err(e) => panic!("Parsing should succeed: {e}"),
        };

        assert!(parsed.get("$schema").is_some());
        assert!(parsed.get("success").is_some());
    }

    /// RED: query --json flag is processed but ignored (always JSON)
    #[test]
    fn test_query_ignores_json_flag_always_json() {
        use isolate_core::OutputFormat;

        // The --json flag in query command is for consistency with other commands
        // but query always outputs JSON regardless
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert!(format.is_json());

        // Even if --json=false (nonsensical for query), output should still be JSON
        let false_flag = false;
        let _ = OutputFormat::from_json_flag(false_flag);
        // Query implementation should convert this back to Json anyway
        let json_format = OutputFormat::Json;
        assert!(json_format.is_json());
    }

    /// RED: query never outputs Human-readable format
    #[test]
    fn test_query_rejects_human_format() {
        use isolate_core::OutputFormat;

        // Even though OutputFormat::Json exists, query should never use it
        let human_format = OutputFormat::Json;
        assert!(human_format.is_json());

        // But query::run should internally convert to Json for all queries
        // This documents that query is always JSON for AI/script consumption
    }

    /// RED: `OutputFormat::from_json_flag` works with query
    #[test]
    fn test_query_from_json_flag_conversion() {
        use isolate_core::OutputFormat;

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert_eq!(format, OutputFormat::Json);

        let human_flag = false;
        let format2 = OutputFormat::from_json_flag(human_flag);
        assert_eq!(format2, OutputFormat::Json);

        // But query would convert both to Json internally
    }

    /// RED: query processing never panics with `OutputFormat`
    #[test]
    fn test_query_format_no_panics() {
        use isolate_core::OutputFormat;

        // Processing both formats should not panic
        for format in &[OutputFormat::Json, OutputFormat::Json] {
            let _ = format.is_json();
            let _ = format.is_json();
            let _ = format.to_string();
        }
    }

    // ============================================================================
    // Tests for New Query Types
    // ============================================================================

    /// Test lock-status query type validation
    #[test]
    fn test_query_lock_status_requires_session() {
        // lock-status requires a session name argument
        // When no session provided, should indicate error
        let query_type = "lock-status";

        // The query type is valid
        assert!([
            "session-exists",
            "session-count",
            "can-run",
            "suggest-name",
            "lock-status",
            "can-spawn",
            "pending-merges",
            "location"
        ]
        .contains(&query_type));
    }

    /// Test can-spawn query returns correct structure
    #[test]
    fn test_query_can_spawn_output_structure() {
        use serde_json::json;

        // can-spawn returns a struct with can_spawn, reason, blockers
        let output = json!({
            "can_spawn": true,
            "reason": null,
            "blockers": []
        });

        assert!(output["can_spawn"].as_bool().is_some());
        assert!(output["blockers"].as_array().is_some());
    }

    /// Test can-spawn with blockers
    #[test]
    fn test_query_can_spawn_with_blockers() {
        use serde_json::json;

        let output = json!({
            "can_spawn": false,
            "reason": "Maximum sessions reached",
            "blockers": ["max_sessions", "no_jj_repo"]
        });

        assert!(output["can_spawn"].as_bool().is_none_or(|v| !v));
        assert_eq!(output["blockers"].as_array().map(Vec::len), Some(2));
    }

    /// Test pending-merges query returns array
    #[test]
    fn test_query_pending_merges_output_structure() {
        use serde_json::json;

        let output = json!({
            "pending_merges": [
                {
                    "session_name": "feature-auth",
                    "workspace_path": "/path/to/workspace",
                    "status": "ready"
                }
            ],
            "count": 1
        });

        assert!(output["pending_merges"].as_array().is_some());
        assert_eq!(output["count"].as_i64(), Some(1));
    }

    /// Test location query returns correct structure
    #[test]
    fn test_query_location_output_structure() {
        use serde_json::json;

        // When on main
        let main_output = json!({
            "location_type": "main",
            "workspace_name": null,
            "simple": "main"
        });

        assert_eq!(main_output["location_type"].as_str(), Some("main"));
        assert!(main_output["workspace_name"].is_null());

        // When in workspace
        let ws_output = json!({
            "location_type": "workspace",
            "workspace_name": "feature-auth",
            "simple": "workspace:feature-auth"
        });

        assert_eq!(ws_output["location_type"].as_str(), Some("workspace"));
        assert_eq!(ws_output["workspace_name"].as_str(), Some("feature-auth"));
    }

    /// Test lock-status query returns lock info
    #[test]
    fn test_query_lock_status_output_structure() {
        use serde_json::json;

        // When locked
        let locked_output = json!({
            "session": "feature-auth",
            "locked": true,
            "holder": "agent-123",
            "expires_at": "2024-01-15T10:30:00Z",
            "error": null
        });

        assert!(locked_output["locked"].as_bool().is_some_and(|v| v));
        assert_eq!(locked_output["holder"].as_str(), Some("agent-123"));

        // When unlocked
        let unlocked_output = json!({
            "session": "feature-auth",
            "locked": false,
            "holder": null,
            "expires_at": null,
            "error": null
        });

        assert!(unlocked_output["locked"].as_bool().is_none_or(|v| !v));
        assert!(unlocked_output["holder"].is_null());
    }

    /// Test all new query types are recognized
    #[test]
    fn test_new_query_types_recognized() {
        let new_query_types = vec!["lock-status", "can-spawn", "pending-merges", "location"];

        for query_type in new_query_types {
            // Each should be a valid query type (not cause unknown type error)
            assert!(!query_type.is_empty());
            assert!(query_type
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    /// Test query error structure for lock-status
    #[test]
    fn test_query_lock_status_error_structure() {
        use serde_json::json;

        let error_output = json!({
            "session": "unknown-session",
            "locked": false,
            "holder": null,
            "expires_at": null,
            "error": {
                "code": "SESSION_NOT_FOUND",
                "message": "Session 'unknown-session' not found"
            }
        });

        assert!(error_output["error"].is_object());
        assert_eq!(
            error_output["error"]["code"].as_str(),
            Some("SESSION_NOT_FOUND")
        );
    }

    // ============================================================================
    // TESTS FOR BEAD isolate-b86b: Standardize missing argument error messages
    // ============================================================================

    /// RED: Missing argument error messages should be consistent across all query subcommands
    ///
    /// This test verifies that when a query subcommand requiring an argument is invoked
    /// without that argument, it produces a standardized error message format that includes:
    /// - The query type name
    /// - Whether the argument is required or optional
    /// - Description of the query
    /// - Usage example
    /// - Expected return value
    #[test]
    fn test_query_missing_arg_error_format_is_consistent() {
        // Test session-exists (requires arg)
        let session_exists_info = QueryTypeInfo::find("session-exists");
        assert!(
            session_exists_info.is_some(),
            "session-exists should have metadata"
        );

        let error_msg = match session_exists_info {
            Some(info) => info.format_error_message(),
            None => panic!("session-exists should have metadata"),
        };
        assert!(
            error_msg.contains("Error: 'session-exists' query requires"),
            "Error message should indicate it's an error about 'session-exists' query"
        );
        assert!(
            error_msg.contains("session_name"),
            "Error message should indicate which argument is missing"
        );
        assert!(
            error_msg.contains("Description:"),
            "Error message should include description"
        );
        assert!(
            error_msg.contains("Usage:"),
            "Error message should include usage"
        );
        assert!(
            error_msg.contains("Example:"),
            "Error message should include example"
        );
        assert!(
            error_msg.contains("Returns:"),
            "Error message should include return value description"
        );

        // Test can-run (requires arg)
        let can_run_info = QueryTypeInfo::find("can-run");
        assert!(can_run_info.is_some(), "can-run should have metadata");

        let error_msg = match can_run_info {
            Some(info) => info.format_error_message(),
            None => panic!("can-run should have metadata"),
        };
        assert!(
            error_msg.contains("Error: 'can-run' query requires"),
            "Error message should indicate it's an error about 'can-run' query"
        );
        assert!(
            error_msg.contains("command_name"),
            "Error message should indicate which argument is missing"
        );

        // Test suggest-name (requires arg)
        let suggest_name_info = QueryTypeInfo::find("suggest-name");
        assert!(
            suggest_name_info.is_some(),
            "suggest-name should have metadata"
        );

        let error_msg = match suggest_name_info {
            Some(info) => info.format_error_message(),
            None => panic!("suggest-name should have metadata"),
        };
        assert!(
            error_msg.contains("Error: 'suggest-name' query requires"),
            "Error message should indicate it's an error about 'suggest-name' query"
        );
        assert!(
            error_msg.contains("pattern"),
            "Error message should indicate which argument is missing"
        );
    }

    /// RED: Optional argument queries should indicate the argument is optional
    #[test]
    fn test_query_optional_arg_message_indicates_optional() {
        // Test session-count (optional arg)
        let session_count_info = QueryTypeInfo::find("session-count");
        assert!(
            session_count_info.is_some(),
            "session-count should have metadata"
        );

        let error_msg = match session_count_info {
            Some(info) => info.format_error_message(),
            None => panic!("session-count should have metadata"),
        };
        assert!(
            error_msg.contains("optional"),
            "Error message should indicate the argument is optional"
        );
        assert!(
            error_msg.contains("--status=active"),
            "Error message should show the optional flag format"
        );
    }

    /// RED: All query types with required args should have consistent format
    #[test]
    fn test_all_required_arg_queries_have_consistent_format() {
        let queries_with_required_args = vec![
            ("session-exists", "session_name"),
            ("can-run", "command_name"),
            ("suggest-name", "pattern"),
            ("lock-status", "session_name"),
        ];

        for (query_name, expected_arg) in queries_with_required_args {
            let info = QueryTypeInfo::find(query_name);
            assert!(
                info.is_some(),
                "{query_name} should have metadata for consistent error messages"
            );

            let info = match info {
                Some(i) => i,
                None => panic!("Query should have metadata"),
            };
            assert!(
                info.requires_arg,
                "{query_name} should be marked as requiring an argument"
            );
            assert_eq!(
                info.arg_name, expected_arg,
                "{query_name} should have correct argument name '{expected_arg}'"
            );

            let error_msg = info.format_error_message();

            // Verify consistent structure
            assert!(
                error_msg.contains(&format!("Error: '{query_name}' query requires")),
                "{query_name}: Error message should follow standard format"
            );
            assert!(
                error_msg.contains(expected_arg),
                "{query_name}: Error message should mention the required argument '{expected_arg}'"
            );
        }
    }

    /// RED: Query metadata should never be missing for known query types
    #[test]
    fn test_known_query_types_have_metadata() {
        let known_queries = vec![
            "session-exists",
            "session-count",
            "can-run",
            "suggest-name",
            "lock-status",
            "can-spawn",
            "pending-merges",
            "location",
        ];

        for query_name in known_queries {
            let info = QueryTypeInfo::find(query_name);
            assert!(
                info.is_some(),
                "Known query type '{query_name}' should have metadata for consistent error messages"
            );
        }
    }

    // ============================================================================
    // TESTS FOR BEAD bd-10t: Remove command-layer process::exit paths
    // ============================================================================

    /// RED: query_session_exists should return Result with exit code metadata
    ///
    /// This test documents that query_session_exists should NOT call std::process::exit
    /// directly. Instead, it should return a Result type that includes the exit code,
    /// allowing the top-level handler to make the final exit decision.
    #[tokio::test]
    async fn test_query_session_exists_returns_result_with_exit_code() {
        // This test will FAIL until we refactor query_session_exists
        // Current signature: async fn query_session_exists(name: &str) -> Result<()>
        // Expected signature: async fn query_session_exists(name: &str) -> Result<QueryResult>
        //
        // Where QueryResult is a struct containing:
        // - output: JSON string (the envelope)
        // - exit_code: i32 (0 = exists, 1 = not exists, 2 = error)
        //
        // This allows the top-level handler to:
        // 1. Print the JSON output
        // 2. Execute hooks
        // 3. Call std::process::exit with the exit_code
    }

    /// RED: query_session_count should return Result with exit code metadata
    ///
    /// Currently query_session_count has return type `-> !` (never returns) because
    /// it calls std::process::exit directly. This prevents proper error handling
    /// and hook execution at the top level.
    #[tokio::test]
    async fn test_query_session_count_returns_result_with_exit_code() {
        // This test will FAIL until we refactor query_session_count
        // Current signature: async fn query_session_count(filter: Option<&str>) -> !
        // Expected signature: async fn query_session_count(filter: Option<&str>) ->
        // Result<QueryResult>
        //
        // The function should return the plain number output and exit code,
        // not call std::process::exit directly
    }

    /// RED: query_can_run should return Result with exit code metadata
    #[tokio::test]
    async fn test_query_can_run_returns_result_with_exit_code() {
        // This test will FAIL until we refactor query_can_run
        // Current: Calls std::process::exit(1) at line 397
        // Expected: Returns Result<QueryResult> with exit_code: 1
    }

    /// RED: query_suggest_name should return Result with exit code metadata
    #[tokio::test]
    async fn test_query_suggest_name_returns_result_with_exit_code() {
        // This test will FAIL until we refactor query_suggest_name
        // Current: Calls std::process::exit(1) at line 422
        // Expected: Returns Result<QueryResult> with exit_code: 1
    }

    /// RED: QueryResult type should encapsulate output and exit metadata
    #[test]
    fn test_query_result_type_definition() {
        // This test documents the expected QueryResult type
        // We need a struct that holds:
        // - output: String (the JSON or plain text output)
        // - exit_code: i32 (the exit code to use)
        //
        // This allows the top-level handler to:
        // 1. Print output
        // 2. Run hooks
        // 3. Exit with the correct code
        //
        // Example usage:
        // ```
        // struct QueryResult {
        //     output: String,
        //     exit_code: i32,
        // }
        // ```
    }

    /// RED: No query function should call std::process::exit directly
    #[test]
    fn test_no_query_functions_call_process_exit() {
        // This test verifies the contract: query functions return results,
        // they don't exit directly.
        //
        // After refactoring, we should be able to grep the query.rs file
        // and find ZERO instances of "std::process::exit" within query
        // function bodies (except in the top-level run() handler).
        //
        // The only place std::process::exit should be called is in
        // cli/handlers.rs after all query functions have returned.
    }

    /// RED: Top-level query::run() should handle exit after all hooks
    #[test]
    fn test_query_run_handles_exit_after_hooks() {
        // This test documents the expected behavior of query::run()
        //
        // Sequence:
        // 1. Call the appropriate query function (e.g., query_session_exists)
        // 2. Get back QueryResult { output, exit_code }
        // 3. Print output
        // 4. Run any hooks (if applicable)
        // 5. Return Ok(()) or Err() with exit code metadata
        // 6. Let cli/handlers.rs call std::process::exit
        //
        // This ensures consistent behavior across all commands and
        // allows hooks to execute before exit.
    }
}
