#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::types::{BeadIssue, BeadsError, IssueStatus, Priority};

/// Parse a datetime string from RFC3339 format.
///
/// # Errors
///
/// Returns `BeadsError::QueryFailed` if the string is missing or invalid.
pub(crate) fn parse_datetime(datetime_str: Option<String>) -> Result<DateTime<Utc>, BeadsError> {
    datetime_str
        .ok_or_else(|| BeadsError::QueryFailed("Missing required datetime field".to_string()))
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid datetime format '{s}': {e}")))
        })
}

/// Parse a status string into `IssueStatus`.
///
/// # Errors
///
/// Returns `BeadsError::QueryFailed` if the status string is invalid.
pub(crate) fn parse_status(status_str: &str) -> Result<IssueStatus, BeadsError> {
    status_str.parse().map_err(|_| {
        BeadsError::QueryFailed(format!("Invalid status value '{status_str}'. Expected one of: open, in_progress, done, cancelled"))
    })
}

/// Enable `WAL` mode on the `SQLite` connection for better crash recovery.
///
/// # Errors
///
/// Returns `BeadsError` if the `PRAGMA` statement fails.
pub(crate) async fn enable_wal_mode(pool: &SqlitePool) -> std::result::Result<(), BeadsError> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to enable WAL mode: {e}")))?;
    Ok(())
}

/// Query all issues from the beads database.
///
/// Parse a single row from the beads database into a `BeadIssue`
///
/// # Errors
///
/// Returns `BeadsError` if any required field is missing or malformed
pub(crate) fn parse_bead_row(
    row: &sqlx::sqlite::SqliteRow,
) -> std::result::Result<BeadIssue, BeadsError> {
    let status_str: String = row
        .try_get("status")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'status' error: {e}")))?;
    let status = parse_status(&status_str)?;

    let priority_str: Option<String> = row.try_get("priority").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'priority' error: {e}"))
    })?;
    let priority = priority_str
        .and_then(|p: String| p.strip_prefix('P').and_then(|n| n.parse::<u32>().ok()))
        .and_then(Priority::from_u32);

    let issue_type_str: Option<String> = row
        .try_get("type")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'type' error: {e}")))?;
    let issue_type = issue_type_str.and_then(|s: String| s.parse().ok());

    let labels_str: Option<String> = row
        .try_get("labels")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'labels' error: {e}")))?;
    let labels =
        labels_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    let depends_on_str: Option<String> = row.try_get("depends_on").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'depends_on' error: {e}"))
    })?;
    let depends_on =
        depends_on_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    let blocked_by_str: Option<String> = row.try_get("blocked_by").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'blocked_by' error: {e}"))
    })?;
    let blocked_by =
        blocked_by_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    // Required datetime fields - fail if missing or invalid
    let created_at_str: Option<String> = row.try_get("created_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'created_at' error: {e}"))
    })?;
    let created_at = parse_datetime(created_at_str)?;

    let updated_at_str: Option<String> = row.try_get("updated_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'updated_at' error: {e}"))
    })?;
    let updated_at = parse_datetime(updated_at_str)?;

    // Optional datetime field
    let closed_at_str: Option<String> = row.try_get("closed_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'closed_at' error: {e}"))
    })?;
    let closed_at = closed_at_str
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid closed_at datetime: {e}")))
        })
        .transpose()?;

    Ok(BeadIssue {
        id: row
            .try_get("id")
            .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'id' error: {e}")))?,
        title: row.try_get("title").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'title' error: {e}"))
        })?,
        status,
        priority,
        issue_type,
        description: row.try_get("description").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'description' error: {e}"))
        })?,
        labels,
        assignee: row.try_get("assignee").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'assignee' error: {e}"))
        })?,
        parent: row.try_get("parent").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'parent' error: {e}"))
        })?,
        depends_on,
        blocked_by,
        created_at,
        updated_at,
        closed_at,
    })
}

/// # Errors
///
/// Returns `BeadsError` if:
/// - The database cannot be opened or queried
/// - Any required field is missing or malformed
/// - Status or datetime values are invalid
pub async fn query_beads(workspace_path: &Path) -> std::result::Result<Vec<BeadIssue>, BeadsError> {
    let beads_db = workspace_path.join(".beads/beads.db");

    if !beads_db.exists() {
        tracing::warn!(
            "Beads database not found at {}. It will be created when needed.",
            beads_db.display()
        );
        return Ok(Vec::new());
    }

    let path_str = beads_db.to_str().ok_or_else(|| {
        BeadsError::DatabaseError("Beads database path contains invalid UTF-8".to_string())
    })?;

    let db_url = format!("sqlite://{path_str}?mode=rw");
    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to connect to beads.db: {e}")))?;

    // Enable WAL mode for better crash recovery
    enable_wal_mode(&pool).await?;

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, title, status, priority, type, description, labels, assignee,
                parent, depends_on, blocked_by, created_at, updated_at, closed_at
         FROM issues ORDER BY priority, created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| BeadsError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.iter()
        .map(parse_bead_row)
        .collect::<std::result::Result<Vec<_>, BeadsError>>()
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to parse bead issues: {e}")))
}
