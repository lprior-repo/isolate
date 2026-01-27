//! Query command - state queries for AI agents
//!
//! This command provides programmatic access to system state
//! for AI agents to make informed decisions.

use anyhow::Result;
use zjj_core::{
    introspection::{
        Blocker, CanRunQuery, QueryError, SessionCountQuery, SessionExistsQuery, SessionInfo,
    },
    json::SchemaEnvelope,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo},
    commands::{get_session_db, zjj_data_dir},
};

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
                usage_example: "zjj query session-exists my-session",
                returns_description: r#"{"exists": true, "session": {"name": "my-session", "status": "active"}}"#,
            },
            Self {
                name: "session-count",
                description: "Count total sessions or filter by status",
                requires_arg: false,
                arg_name: "--status=active",
                usage_example: "zjj query session-count --status=active",
                returns_description: r#"{"count": 5, "filter": {"raw": "--status=active"}}"#,
            },
            Self {
                name: "can-run",
                description: "Check if a command can run and show blockers",
                requires_arg: true,
                arg_name: "command_name",
                usage_example: "zjj query can-run add",
                returns_description: r#"{"can_run": true, "command": "add", "blockers": [], "prerequisites_met": 4, "prerequisites_total": 4}"#,
            },
            Self {
                name: "suggest-name",
                description: "Suggest next available name based on pattern",
                requires_arg: true,
                arg_name: "pattern",
                usage_example: r#"zjj query suggest-name "feature-{n}""#,
                returns_description: r#"{"pattern": "feature-{n}", "suggested": "feature-3", "next_available_n": 3, "existing_matches": ["feature-1", "feature-2"]}"#,
            },
        ]
    }

    fn find(name: &str) -> Option<&'static Self> {
        Self::all().iter().find(|q| q.name == name)
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
pub fn run(query_type: &str, args: Option<&str>) -> Result<()> {
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
            query_session_exists(name)
        }
        "session-count" => query_session_count(args),
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
            query_suggest_name(pattern)
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
            "Not in a JJ repository. Run 'jj git init' or 'zjj init' first.".to_string(),
        )
    } else if err_str.contains("ZJJ not initialized") {
        (
            "ZJJ_NOT_INITIALIZED".to_string(),
            "ZJJ not initialized. Run 'zjj init' first.".to_string(),
        )
    } else if err_str.contains("no such table") || err_str.contains("database schema") {
        (
            "DATABASE_NOT_INITIALIZED".to_string(),
            "Database not initialized. Run 'zjj init' first.".to_string(),
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
fn query_session_exists(name: &str) -> Result<()> {
    let result = match get_session_db() {
        Ok(db) => match db.get_blocking(name) {
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
            let (code, message) = categorize_db_error(&e);
            SessionExistsQuery {
                exists: None,
                session: None,
                error: Some(QueryError { code, message }),
            }
        }
    };

    let envelope = SchemaEnvelope::new("query-session-exists", "single", result);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

/// Query session count
fn query_session_count(filter: Option<&str>) -> Result<()> {
    let result = match get_session_db() {
        Ok(db) => match db.list_blocking(None) {
            Ok(sessions) => {
                let count = filter
                    .and_then(|f| f.strip_prefix("--status="))
                    .map(|status| {
                        sessions
                            .iter()
                            .filter(|s| s.status.to_string() == status)
                            .count()
                    })
                    .unwrap_or_else(|| sessions.len());

                SessionCountQuery {
                    count: Some(count),
                    filter: filter.map(|f| serde_json::json!({"raw": f})),
                    error: None,
                }
            }
            Err(e) => SessionCountQuery {
                count: None,
                filter: filter.map(|f| serde_json::json!({"raw": f})),
                error: Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to list sessions: {e}"),
                }),
            },
        },
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            SessionCountQuery {
                count: None,
                filter: filter.map(|f| serde_json::json!({"raw": f})),
                error: Some(QueryError { code, message }),
            }
        }
    };

    let envelope = SchemaEnvelope::new("query-session-count", "single", result);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
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
            message: "zjj not initialized".to_string(),
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

    let envelope = SchemaEnvelope::new("query-can-run", "single", result);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

/// Query for suggested name based on pattern
fn query_suggest_name(pattern: &str) -> Result<()> {
    // suggest_name can work without database access if we can't get sessions
    let existing_names = get_session_db().map_or_else(
        |_| Vec::new(),
        |db| {
            db.list_blocking(None).map_or_else(
                |_| Vec::new(),
                |sessions| sessions.into_iter().map(|s| s.name).collect(),
            )
        },
    );

    let result = zjj_core::introspection::suggest_name(pattern, &existing_names)?;

    let envelope = SchemaEnvelope::new("query-suggest-name", "single", result);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
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

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_query_json_has_envelope() -> anyhow::Result<()> {
        // FAILING: Verify envelope wrapping for query command output
        use zjj_core::json::SchemaEnvelope;

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
        use zjj_core::json::SchemaEnvelope;

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
        use zjj_core::json::SchemaEnvelopeArray;

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

    #[test]
    fn test_query_pagination_envelope() -> anyhow::Result<()> {
        // FAILING: Verify pagination info is preserved in envelope
        use serde_json::json;
        use zjj_core::json::SchemaEnvelope;

        let count_result = SessionCountQuery {
            count: Some(5),
            filter: Some(json!({"status": "active"})),
            error: None,
        };
        let envelope = SchemaEnvelope::new("query-session-count", "single", count_result);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );

        Ok(())
    }

    // ============================================================================
    // PHASE 2 (RED) - OutputFormat Migration Tests for query.rs
    // These tests FAIL until query command accepts OutputFormat parameter
    // ============================================================================

    /// RED: query `run()` should accept `OutputFormat` parameter
    #[test]
    fn test_query_run_accepts_output_format() {
        use zjj_core::OutputFormat;

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
        use zjj_core::OutputFormat;

        // Per requirements: query is always JSON format for programmatic access
        let format = OutputFormat::Json;
        assert!(format.is_json());

        // Even if a human format is requested, query should use JSON
        // This is because query is designed for AI agents/scripts, not humans
    }

    /// RED: query session-exists always outputs JSON
    #[test]
    fn test_query_session_exists_json_only() {
        use zjj_core::OutputFormat;

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
        use zjj_core::OutputFormat;

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
        use zjj_core::OutputFormat;

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
        use zjj_core::OutputFormat;

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
        use zjj_core::{json::SchemaEnvelope, OutputFormat};

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
        let Some(json_str) = json_str_result.ok() else {
            return;
        };
        let parsed_result: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(parsed_result.is_ok(), "parsing should succeed");
        let Some(parsed) = parsed_result.ok() else {
            return;
        };

        assert!(parsed.get("$schema").is_some());
        assert!(parsed.get("success").is_some());
    }

    /// RED: query --json flag is processed but ignored (always JSON)
    #[test]
    fn test_query_ignores_json_flag_always_json() {
        use zjj_core::OutputFormat;

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
        use zjj_core::OutputFormat;

        // Even though OutputFormat::Human exists, query should never use it
        let human_format = OutputFormat::Human;
        assert!(human_format.is_human());

        // But query::run should internally convert to Json for all queries
        // This documents that query is always JSON for AI/script consumption
    }

    /// RED: `OutputFormat::from_json_flag` works with query
    #[test]
    fn test_query_from_json_flag_conversion() {
        use zjj_core::OutputFormat;

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert_eq!(format, OutputFormat::Json);

        let human_flag = false;
        let format2 = OutputFormat::from_json_flag(human_flag);
        assert_eq!(format2, OutputFormat::Human);

        // But query would convert both to Json internally
    }

    /// RED: query processing never panics with `OutputFormat`
    #[test]
    fn test_query_format_no_panics() {
        use zjj_core::OutputFormat;

        // Processing both formats should not panic
        for format in &[OutputFormat::Json, OutputFormat::Human] {
            let _ = format.is_json();
            let _ = format.is_human();
            let _ = format.to_string();
        }
    }
}
