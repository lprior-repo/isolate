use std::path::Path;

use anyhow::{Context, Result};

/// Query bead metadata from .beads/beads.db
///
/// Returns JSON metadata with `bead_id`, title, and status information.
/// Returns empty JSON object if bead not found or database doesn't exist.
pub(super) fn query_bead_metadata(bead_id: &str) -> Result<serde_json::Value> {
    let beads_db = Path::new(".beads/beads.db");
    if !beads_db.exists() {
        return Ok(serde_json::json!({}));
    }

    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;

    rt.block_on(async {
        let connection_string = format!("sqlite:{}", beads_db.display());
        let pool = sqlx::SqlitePool::connect(&connection_string)
            .await
            .context("Failed to connect to beads database")?;

        let result: Option<(String,)> = sqlx::query_as("SELECT title FROM issues WHERE id = ?1")
            .bind(bead_id)
            .fetch_optional(&pool)
            .await
            .context("Failed to query bead from database")?;

        let title = result.map(|r| r.0).unwrap_or_else(|| "Unknown".to_string());

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        Ok(serde_json::json!({
            "bead_id": bead_id,
            "bead_title": title,
            "bead_status": "in_progress",
            "cached_at": timestamp
        }))
    })
}
