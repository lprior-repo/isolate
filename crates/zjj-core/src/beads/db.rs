#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::types::{BeadIssue, BeadsError, IssueStatus, Priority};

/// Query all issues from the beads database.
///
/// # Errors
///
/// Returns `BeadsError` if the database cannot be opened or queried.
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

    let db_url = format!("sqlite://{path_str}?mode=ro");
    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to connect to beads.db: {e}")))?;

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, title, status, priority, type, description, labels, assignee,
                parent, depends_on, blocked_by, created_at, updated_at, closed_at
         FROM issues ORDER BY priority, created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| BeadsError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| {
            let status_str: String = row
                .try_get(2)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let status = status_str.parse().unwrap_or(IssueStatus::Open);

            let priority_str: Option<String> = row
                .try_get(3)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let priority = priority_str
                .and_then(|p: String| p.strip_prefix('P').and_then(|n| n.parse::<u32>().ok()))
                .and_then(Priority::from_u32);

            let issue_type_str: Option<String> = row
                .try_get(4)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let issue_type = issue_type_str.and_then(|s: String| s.parse().ok());

            let labels_str: Option<String> = row
                .try_get(6)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let labels =
                labels_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

            let depends_on_str: Option<String> = row
                .try_get(9)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let depends_on = depends_on_str
                .map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

            let blocked_by_str: Option<String> = row
                .try_get(10)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let blocked_by = blocked_by_str
                .map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

            let created_at_str: Option<String> = row
                .try_get(11)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let created_at = created_at_str
                .and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let updated_at_str: Option<String> = row
                .try_get(12)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let updated_at = updated_at_str
                .and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let closed_at_str: Option<String> = row
                .try_get(13)
                .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?;
            let closed_at = closed_at_str
                .and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            Ok(BeadIssue {
                id: row
                    .try_get(0)
                    .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?,
                title: row
                    .try_get(1)
                    .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?,
                status,
                priority,
                issue_type,
                description: row
                    .try_get(5)
                    .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?,
                labels,
                assignee: row
                    .try_get(7)
                    .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?,
                parent: row
                    .try_get(8)
                    .map_err(|e: sqlx::Error| BeadsError::QueryFailed(e.to_string()))?,
                depends_on,
                blocked_by,
                created_at,
                updated_at,
                closed_at,
            })
        })
        .collect()
}
