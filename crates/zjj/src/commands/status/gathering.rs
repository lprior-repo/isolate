//! Status data gathering functions

use std::path::Path;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, Row};

use super::types::{BeadStats, DiffStats, FileChanges, SessionStatusInfo};
use crate::session::Session;

/// Gather detailed status for a session
pub async fn gather_session_status(session: &Session) -> Result<SessionStatusInfo> {
    let workspace_path = Path::new(&session.workspace_path);

    // Get file changes
    let changes = get_file_changes(workspace_path);

    // Get diff stats
    let diff_stats = get_diff_stats(workspace_path);

    // Get beads stats
    let beads = get_beads_stats().await?;

    Ok(SessionStatusInfo {
        name: session.name.clone(),
        status: session.status.to_string(),
        workspace_path: session.workspace_path.clone(),
        branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
        changes,
        diff_stats,
        beads,
        session: session.clone(),
    })
}

/// Get file changes from JJ status
fn get_file_changes(workspace_path: &Path) -> FileChanges {
    if !workspace_path.exists() {
        return FileChanges::default();
    }

    match zjj_core::jj::workspace_status(workspace_path) {
        Ok(status) => FileChanges {
            modified: status.modified.len(),
            added: status.added.len(),
            deleted: status.deleted.len(),
            renamed: status.renamed.len(),
            unknown: status.unknown.len(),
        },
        Err(_) => FileChanges::default(),
    }
}

/// Get diff statistics from JJ diff
fn get_diff_stats(workspace_path: &Path) -> DiffStats {
    if !workspace_path.exists() {
        return DiffStats::default();
    }

    zjj_core::jj::workspace_diff(workspace_path)
        .map(|summary| DiffStats {
            insertions: summary.insertions,
            deletions: summary.deletions,
        })
        .unwrap_or_default()
}

/// Get beads statistics from the repository's beads database
async fn get_beads_stats() -> Result<BeadStats> {
    // Find repository root
    let repo_root = zjj_core::jj::check_in_jj_repo().ok();

    let Some(root) = repo_root else {
        return Ok(BeadStats::default());
    };

    let beads_db_path = root.join(".beads").join("beads.db");

    if !beads_db_path.exists() {
        return Ok(BeadStats::default());
    }

    // Optimization: Use single GROUP BY query instead of 4 separate COUNTs
    let db_url = format!("sqlite:{}", beads_db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open beads database: {e}"))?;

    let rows = sqlx::query("SELECT status, COUNT(*) as count FROM issues GROUP BY status")
        .fetch_all(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute query: {e}"))?;

    // Functional fold: accumulate stats from database rows
    let stats = rows.into_iter().try_fold(
        BeadStats::default(),
        |mut stats, row| -> Result<BeadStats> {
            let bead_status: String = row
                .try_get("status")
                .map_err(|e| anyhow::anyhow!("Failed to read status: {e}"))?;
            let count: i64 = row
                .try_get("count")
                .map_err(|e| anyhow::anyhow!("Failed to read count: {e}"))?;
            // Safe cast: counts cannot be negative, but we handle it defensively
            let count_usize = usize::try_from(count.max(0)).unwrap_or(0);

            match bead_status.as_str() {
                "open" => stats.open = count_usize,
                "in_progress" => stats.in_progress = count_usize,
                "blocked" => stats.blocked = count_usize,
                "closed" => stats.closed = count_usize,
                _ => {} // Ignore unknown statuses
            }

            Ok(stats)
        },
    )?;

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_changes_missing_workspace() {
        let result = get_file_changes(Path::new("/nonexistent/path"));
        assert_eq!(result.modified, 0);
        assert_eq!(result.added, 0);
        assert_eq!(result.deleted, 0);
        assert_eq!(result.renamed, 0);
    }

    #[test]
    fn test_get_diff_stats_missing_workspace() {
        let result = get_diff_stats(Path::new("/nonexistent/path"));
        assert_eq!(result.insertions, 0);
        assert_eq!(result.deletions, 0);
    }
}
