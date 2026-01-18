//! Query execution and row parsing for database operations

use std::{str::FromStr, time::SystemTime};

use sqlx::{Row, SqlitePool};
use zjj_core::{Error, Result};

use crate::session::{Session, SessionStatus};

/// Get current Unix timestamp
pub(crate) fn get_current_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| Error::Unknown(format!("System time error: {e}")))
}

/// Build a Session struct from components
pub(crate) fn build_session(
    id: i64,
    name: &str,
    status: SessionStatus,
    workspace_path: &str,
    timestamp: u64,
) -> Session {
    Session {
        id: Some(id),
        name: name.to_string(),
        status,
        workspace_path: workspace_path.to_string(),
        zellij_tab: format!("zjj:{name}"),
        branch: None,
        created_at: timestamp,
        updated_at: timestamp,
        last_synced: None,
        metadata: None,
    }
}

/// Parse a database row into a Session
pub(crate) fn parse_session_row(row: &sqlx::sqlite::SqliteRow) -> Result<Session> {
    let id: i64 = row
        .try_get("id")
        .map_err(|e| Error::database_error(format!("Failed to read id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| Error::database_error(format!("Failed to read name: {e}")))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| Error::database_error(format!("Failed to read status: {e}")))?;
    let status = SessionStatus::from_str(&status_str)?;
    let workspace_path: String = row
        .try_get("workspace_path")
        .map_err(|e| Error::database_error(format!("Failed to read workspace_path: {e}")))?;
    let branch: Option<String> = row
        .try_get("branch")
        .map_err(|e| Error::database_error(format!("Failed to read branch: {e}")))?;
    let created_at: u64 = row
        .try_get::<i64, _>("created_at")
        .map_err(|e| Error::database_error(format!("Failed to read created_at: {e}")))?
        .cast_unsigned();
    let updated_at: u64 = row
        .try_get::<i64, _>("updated_at")
        .map_err(|e| Error::database_error(format!("Failed to read updated_at: {e}")))?
        .cast_unsigned();
    let last_synced: Option<i64> = row
        .try_get("last_synced")
        .map_err(|e| Error::database_error(format!("Failed to read last_synced: {e}")))?;
    let metadata_str: Option<String> = row
        .try_get("metadata")
        .map_err(|e| Error::database_error(format!("Failed to read metadata: {e}")))?;

    let metadata = metadata_str
        .map(|s| {
            serde_json::from_str(&s)
                .map_err(|e| Error::parse_error(format!("Invalid metadata JSON: {e}")))
        })
        .transpose()?;

    // Construct zellij_tab before moving name (avoids clone)
    let zellij_tab = format!("zjj:{name}");

    Ok(Session {
        id: Some(id),
        name,
        status,
        workspace_path,
        zellij_tab,
        branch,
        created_at,
        updated_at,
        last_synced: last_synced.map(i64::cast_unsigned),
        metadata,
    })
}

/// Query a session by name
pub(crate) async fn query_session_by_name(
    pool: &SqlitePool,
    name: &str,
) -> Result<Option<Session>> {
    sqlx::query(
        "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
         FROM sessions WHERE name = ?"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::database_error(format!("Failed to query session: {e}")))
    .and_then(|opt_row| opt_row.map_or(Ok(None), |row| parse_session_row(&row).map(Some)))
}

/// Query all sessions with optional status filter
pub(crate) async fn query_sessions(
    pool: &SqlitePool,
    status_filter: Option<SessionStatus>,
) -> Result<Vec<Session>> {
    let rows = match status_filter {
        Some(status) => {
            sqlx::query(
                "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions WHERE status = ? ORDER BY created_at"
            )
            .bind(status.to_string())
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions ORDER BY created_at"
            )
            .fetch_all(pool)
            .await
        }
    }.map_err(|e| Error::database_error(format!("Failed to query sessions: {e}")))?;

    rows.iter().map(parse_session_row).collect()
}
