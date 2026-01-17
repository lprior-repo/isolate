//! Bead status update integration for session removal
//!
//! This module handles automatic bead status updates when sessions are removed.
//! Based on the removal context (merge vs abandon), it determines the appropriate
//! bead status transition and executes bd CLI commands.

use std::path::Path;

use anyhow::{Context, Result};
use zjj_core::config::Config;

use crate::session::Session;

/// Determine target bead status based on removal context
///
/// Returns the appropriate status string for `bd update --status`:
/// - "closed" if session merged successfully
/// - "deferred" if session removed without merge
const fn determine_bead_status(merged: bool) -> &'static str {
    if merged {
        "closed"
    } else {
        "deferred"
    }
}

/// Extract bead ID from session metadata
///
/// Returns None if session has no bead metadata or `bead_id` field
fn extract_bead_id(session: &Session) -> Option<String> {
    session
        .metadata
        .as_ref()
        .and_then(|m| m.get("bead_id"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Update bead status via bd CLI
///
/// This is a best-effort operation: if bd update fails, we log but don't fail
/// the session removal. This ensures bead integration is optional and doesn't
/// break core functionality.
///
/// # Errors
/// Returns error if bd command execution fails
fn update_bead_status_via_bd(bead_id: &str, status: &str) -> Result<()> {
    let output = std::process::Command::new("bd")
        .arg("update")
        .arg(bead_id)
        .arg("--status")
        .arg(status)
        .output()
        .context("Failed to execute 'bd update'")?;

    if !output.status.success() {
        anyhow::bail!(
            "bd update failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Process bead status update on session removal
///
/// This is the main entry point for bead integration during removal.
/// It extracts bead metadata, determines the target status, and updates
/// the bead via bd CLI.
///
/// Returns Ok(true) if bead was updated, Ok(false) if no bead or update skipped.
///
/// # Errors
/// Returns error if bd command fails (caller should handle gracefully)
pub fn process_bead_removal(
    session: &Session,
    merged: bool,
    config: &Config,
    _workspace_path: &Path,
) -> Result<bool> {
    // Check if auto-close is enabled
    if !config.session.bead_auto_close {
        return Ok(false);
    }

    // Extract bead ID from session metadata
    let Some(bead_id) = extract_bead_id(session) else {
        return Ok(false); // No bead attached, nothing to do
    };

    // Determine target status
    let target_status = determine_bead_status(merged);

    // Update bead status via bd CLI (best-effort)
    update_bead_status_via_bd(&bead_id, target_status).context(format!(
        "Failed to update bead {bead_id} to {target_status}"
    ))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_determine_bead_status_merged() {
        assert_eq!(determine_bead_status(true), "closed");
    }

    #[test]
    fn test_determine_bead_status_not_merged() {
        assert_eq!(determine_bead_status(false), "deferred");
    }

    #[test]
    fn test_extract_bead_id_present() {
        let session = Session {
            name: "test".to_string(),
            metadata: Some(json!({"bead_id": "zjj-test"})),
            ..Default::default()
        };

        let bead_id = extract_bead_id(&session);
        assert_eq!(bead_id, Some("zjj-test".to_string()));
    }

    #[test]
    fn test_extract_bead_id_missing() {
        let session = Session {
            name: "test".to_string(),
            metadata: Some(json!({"other_field": "value"})),
            ..Default::default()
        };

        let bead_id = extract_bead_id(&session);
        assert_eq!(bead_id, None);
    }

    #[test]
    fn test_extract_bead_id_no_metadata() {
        let session = Session {
            name: "test".to_string(),
            metadata: None,
            ..Default::default()
        };

        let bead_id = extract_bead_id(&session);
        assert_eq!(bead_id, None);
    }
}
