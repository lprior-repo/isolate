use anyhow::{Context, Result};

use crate::{beads::BeadRepository, cli::jj_root};

/// Query bead metadata from repository (SQLite or JSONL)
///
/// Returns JSON metadata with `bead_id`, title, and status information.
/// Returns empty JSON object if bead not found or database doesn't exist.
pub(super) async fn query_bead_metadata(bead_id: &str) -> Result<serde_json::Value> {
    let root = jj_root().await.context("Failed to get JJ root")?;
    let bead_repo = BeadRepository::new(root);

    let bead = bead_repo
        .get_bead(bead_id)
        .await
        .context("Failed to query bead metadata")?;

    match bead {
        Some(b) => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());

            Ok(serde_json::json!({
                "bead_id": b.id,
                "bead_title": b.title,
                "bead_status": b.status.to_string(),
                "cached_at": timestamp
            }))
        }
        None => Ok(serde_json::json!({})),
    }
}
