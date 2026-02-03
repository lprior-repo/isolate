#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::types::{BeadIssue, BeadsError, IssueStatus, Priority};

/// Query all issues from the beads database.
///
/// # Errors
///
/// Returns `BeadsError` if the database cannot be opened or queried.
pub fn query_beads(workspace_path: &Path) -> std::result::Result<Vec<BeadIssue>, BeadsError> {
    let beads_db = workspace_path.join(".beads/beads.db");

    if !beads_db.exists() {
        eprintln!(
            "Warning: Beads database not found at {}. It will be created when needed.",
            beads_db.display()
        );
        return Ok(Vec::new());
    }

    let conn = Connection::open(&beads_db)
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to open beads.db: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, status, priority, type, description, labels, assignee,
                    parent, depends_on, blocked_by, created_at, updated_at, closed_at
             FROM issues ORDER BY priority, created_at DESC",
        )
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to prepare query: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            let status_str: String = row.get(2)?;
            let status = status_str.parse().unwrap_or(IssueStatus::Open);

            let priority_str: Option<String> = row.get(3)?;
            let priority = priority_str
                .and_then(|p| p.strip_prefix('P').and_then(|n| n.parse().ok()))
                .and_then(Priority::from_u32);

            let issue_type_str: Option<String> = row.get(4)?;
            let issue_type = issue_type_str.and_then(|s| s.parse().ok());

            let labels_str: Option<String> = row.get(6)?;
            let labels = labels_str.map(|s| s.split(',').map(String::from).collect());

            let depends_on_str: Option<String> = row.get(9)?;
            let depends_on = depends_on_str.map(|s| s.split(',').map(String::from).collect());

            let blocked_by_str: Option<String> = row.get(10)?;
            let blocked_by = blocked_by_str.map(|s| s.split(',').map(String::from).collect());

            let created_at_str: Option<String> = row.get(11)?;
            let created_at = created_at_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let updated_at_str: Option<String> = row.get(12)?;
            let updated_at = updated_at_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let closed_at_str: Option<String> = row.get(13)?;
            let closed_at = closed_at_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            Ok(BeadIssue {
                id: row.get(0)?,
                title: row.get(1)?,
                status,
                priority,
                issue_type,
                description: row.get(5)?,
                labels,
                assignee: row.get(7)?,
                parent: row.get(8)?,
                depends_on,
                blocked_by,
                created_at,
                updated_at,
                closed_at,
            })
        })
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.collect::<std::result::Result<Vec<BeadIssue>, _>>()
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to collect results: {e}")))
}
