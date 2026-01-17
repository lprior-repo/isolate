//! Bead integration for session creation
//!
//! This module handles bead-aware session creation: validating bead IDs,
//! fetching bead metadata, and integrating with the bd CLI.

use std::{fmt::Write, path::Path};

use anyhow::{bail, Context, Result};
use serde_json::json;
use zjj_core::beads::{BeadIssue, IssueStatus, IssueType, Priority};

/// Validate that bead exists in repository
///
/// # Errors
/// Returns error if bead not found or database query fails
pub async fn validate_bead_exists(repo_root: &Path, bead_id: &str) -> Result<BeadIssue> {
    let beads = zjj_core::beads::query_beads(repo_root)
        .await
        .context("Failed to query beads database")?;

    beads
        .iter()
        .find(|b| b.id == bead_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Bead '{bead_id}' not found in .beads/beads.db"))
}

/// Build session metadata from bead issue
pub fn build_bead_metadata(bead: &BeadIssue) -> serde_json::Value {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    json!({
        "bead_id": bead.id,
        "bead_title": bead.title,
        "bead_type": bead.issue_type.as_ref().map(|t| match t {
            IssueType::Bug => "bug",
            IssueType::Feature => "feature",
            IssueType::Task => "task",
            IssueType::Epic => "epic",
            IssueType::Chore => "chore",
            IssueType::MergeRequest => "merge_request",
            IssueType::Event => "event",
        }),
        "bead_priority": bead.priority.as_ref().map(|p| match p {
            Priority::P0 => "P0",
            Priority::P1 => "P1",
            Priority::P2 => "P2",
            Priority::P3 => "P3",
            Priority::P4 => "P4",
        }),
        "bead_status": match bead.status {
            IssueStatus::Open => "open",
            IssueStatus::InProgress => "in_progress",
            IssueStatus::Blocked => "blocked",
            IssueStatus::Closed => "closed",
            IssueStatus::Deferred => "deferred",
        },
        "bead_attached_at": timestamp
    })
}

/// Generate `BEAD_SPEC.md` content from bead issue
pub fn generate_bead_spec(bead: &BeadIssue) -> String {
    let mut spec = String::new();

    let _ = writeln!(spec, "# Bead Spec: {}\n", bead.id);
    let _ = writeln!(spec, "**Title**: {}\n", bead.title);

    let status_str = match bead.status {
        IssueStatus::Open => "open",
        IssueStatus::InProgress => "in_progress",
        IssueStatus::Blocked => "blocked",
        IssueStatus::Closed => "closed",
        IssueStatus::Deferred => "deferred",
    };
    let _ = writeln!(spec, "**Status**: {status_str}");

    if let Some(priority) = &bead.priority {
        let pri_str = match priority {
            Priority::P0 => "P0",
            Priority::P1 => "P1",
            Priority::P2 => "P2",
            Priority::P3 => "P3",
            Priority::P4 => "P4",
        };
        let _ = writeln!(spec, "**Priority**: {pri_str}");
    }

    if let Some(issue_type) = &bead.issue_type {
        let type_str = match issue_type {
            IssueType::Bug => "bug",
            IssueType::Feature => "feature",
            IssueType::Task => "task",
            IssueType::Epic => "epic",
            IssueType::Chore => "chore",
            IssueType::MergeRequest => "merge_request",
            IssueType::Event => "event",
        };
        let _ = writeln!(spec, "**Type**: {type_str}");
    }

    spec.push_str("\n---\n\n");

    if let Some(description) = &bead.description {
        spec.push_str("## Description\n\n");
        spec.push_str(description);
        spec.push_str("\n\n");
    }

    if let Some(labels) = &bead.labels {
        if !labels.is_empty() {
            spec.push_str("## Labels\n\n");
            for label in labels {
                let _ = writeln!(spec, "- {label}");
            }
            spec.push('\n');
        }
    }

    if let Some(depends_on) = &bead.depends_on {
        if !depends_on.is_empty() {
            spec.push_str("## Dependencies\n\n");
            for dep in depends_on {
                let _ = writeln!(spec, "- {dep}");
            }
            spec.push('\n');
        }
    }

    if let Some(blocked_by) = &bead.blocked_by {
        if !blocked_by.is_empty() {
            spec.push_str("## Blocked By\n\n");
            for blocker in blocked_by {
                let _ = writeln!(spec, "- {blocker}");
            }
            spec.push('\n');
        }
    }

    let _ = writeln!(spec, "\n---\n\n*Created*: {}", bead.created_at);
    let _ = writeln!(spec, "*Updated*: {}", bead.updated_at);

    spec
}

/// Write `BEAD_SPEC.md` to workspace
///
/// # Errors
/// Returns error if file write fails
pub fn write_bead_spec(workspace_path: &Path, content: &str) -> Result<()> {
    let spec_path = workspace_path.join("BEAD_SPEC.md");
    std::fs::write(&spec_path, content).context(format!(
        "Failed to write BEAD_SPEC.md to {}",
        spec_path.display()
    ))
}

/// Update bead status via bd CLI
///
/// # Errors
/// Returns error if bd command fails
pub fn update_bead_status(bead_id: &str, status: &str) -> Result<()> {
    let output = std::process::Command::new("bd")
        .arg("update")
        .arg(bead_id)
        .arg("--status")
        .arg(status)
        .output()
        .context("Failed to execute 'bd update'")?;

    if !output.status.success() {
        bail!(
            "Failed to update bead status: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use im::vector;

    use super::*;

    #[test]
    fn test_build_bead_metadata() {
        let bead = BeadIssue {
            id: "zjj-test".to_string(),
            title: "Test Issue".to_string(),
            status: IssueStatus::Open,
            priority: Some(Priority::P0),
            issue_type: Some(IssueType::Feature),
            description: Some("Test description".to_string()),
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let metadata = build_bead_metadata(&bead);

        assert_eq!(metadata["bead_id"], "zjj-test");
        assert_eq!(metadata["bead_title"], "Test Issue");
        assert_eq!(metadata["bead_type"], "feature");
        assert_eq!(metadata["bead_priority"], "P0");
    }

    #[test]
    fn test_generate_bead_spec() {
        let bead = BeadIssue {
            id: "zjj-test".to_string(),
            title: "Test Issue".to_string(),
            status: IssueStatus::Open,
            priority: Some(Priority::P1),
            issue_type: Some(IssueType::Bug),
            description: Some("This is a test bug".to_string()),
            labels: Some(vector!["urgent".to_string(), "backend".to_string()]),
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let spec = generate_bead_spec(&bead);

        assert!(spec.contains("# Bead Spec: zjj-test"));
        assert!(spec.contains("**Title**: Test Issue"));
        assert!(spec.contains("**Priority**: P1"));
        assert!(spec.contains("**Type**: bug"));
        assert!(spec.contains("This is a test bug"));
        assert!(spec.contains("- urgent"));
        assert!(spec.contains("- backend"));
    }
}
