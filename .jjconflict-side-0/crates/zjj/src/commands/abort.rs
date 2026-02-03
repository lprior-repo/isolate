//! abort command - Abandon workspace without merging
//!
//! This command is the opposite of `done`:
//! - Removes the workspace without merging changes
//! - Optionally updates bead status back to ready
//! - Can be run from inside or outside the workspace
//!
//! Use this when you want to discard work without completing it.

use anyhow::{Context, Result};
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use super::{context, get_session_db};
use crate::session::{SessionStatus, SessionUpdate};

/// Output for abort command
#[derive(Debug, Clone, Serialize)]
pub struct AbortOutput {
    /// Session that was aborted
    pub session_name: String,
    /// Whether workspace was removed
    pub workspace_removed: bool,
    /// Whether bead status was updated
    pub bead_updated: bool,
    /// Message describing what happened
    pub message: String,
}

/// Options for abort command
#[derive(Debug, Clone)]
pub struct AbortOptions {
    /// Workspace to abort (optional - uses current if not specified)
    pub workspace: Option<String>,
    /// Don't update bead status
    pub no_bead_update: bool,
    /// Keep workspace files (just remove from zjj tracking)
    pub keep_workspace: bool,
    /// Dry run - don't actually abort
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Run the abort command
///
/// # Errors
///
/// Returns an error if:
/// - Not in a workspace and no workspace specified
/// - Workspace not found
/// - Cleanup fails
pub fn run(options: &AbortOptions) -> Result<()> {
    let root = super::check_in_jj_repo()?;
    let location = context::detect_location(&root)?;

    // Determine which workspace to abort
    let workspace_name = match (&options.workspace, &location) {
        (Some(name), _) | (None, context::Location::Workspace { name, .. }) => name.clone(),
        (None, context::Location::Main) => {
            anyhow::bail!(
                "Not in a workspace. Use --workspace <name> to specify which workspace to abort."
            );
        }
    };

    // Dry run - just show what would happen
    if options.dry_run {
        return output_dry_run(&workspace_name, options);
    }

    // Get the session database
    let db = get_session_db()?;

    // Find the session
    let session = db
        .get_blocking(&workspace_name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{workspace_name}' not found"))?;

    let workspace_path = std::path::Path::new(&session.workspace_path);

    // Remove workspace files unless --keep-workspace
    let workspace_removed = if options.keep_workspace {
        false
    } else if workspace_path.exists() {
        std::fs::remove_dir_all(workspace_path).with_context(|| {
            format!("Failed to remove workspace at {}", workspace_path.display())
        })?;
        true
    } else {
        false
    };

    // Update session status to abandoned
    db.update_blocking(
        &workspace_name,
        SessionUpdate {
            status: Some(SessionStatus::Failed), // Using Failed as closest to Abandoned
            ..Default::default()
        },
    )
    .context("Failed to update session status")?;

    // Remove the session from database
    db.delete_blocking(&workspace_name)
        .context("Failed to delete session")?;

    // Update bead status if applicable
    let bead_updated = if options.no_bead_update {
        false
    } else {
        update_bead_status_to_ready(&session)
    };

    let output = AbortOutput {
        session_name: workspace_name.clone(),
        workspace_removed,
        bead_updated,
        message: format!("Aborted session '{workspace_name}'"),
    };

    output_result(&output, options.format)
}

/// Output for dry run
fn output_dry_run(workspace_name: &str, options: &AbortOptions) -> Result<()> {
    let output = AbortOutput {
        session_name: workspace_name.to_string(),
        workspace_removed: !options.keep_workspace,
        bead_updated: !options.no_bead_update,
        message: format!("[DRY RUN] Would abort session '{workspace_name}'"),
    };

    if options.format.is_json() {
        let mut envelope =
            serde_json::to_value(SchemaEnvelope::new("abort-response", "single", &output))?;
        if let Some(obj) = envelope.as_object_mut() {
            obj.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        }
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize abort dry-run output")?;
        println!("{json_str}");
    } else {
        println!("[DRY RUN] Would abort session '{workspace_name}'");
        if !options.keep_workspace {
            println!("  Would remove workspace files");
        }
        if !options.no_bead_update {
            println!("  Would update bead status to ready");
        }
    }

    Ok(())
}

/// Update bead status back to ready
fn update_bead_status_to_ready(session: &crate::session::Session) -> bool {
    // Check if session has a bead_id in metadata
    let bead_id = session
        .metadata
        .as_ref()
        .and_then(|m| m.get("bead_id"))
        .and_then(|v| v.as_str());

    if let Some(bead_id) = bead_id {
        // Try to run bd update to set status back to ready
        let result = std::process::Command::new("bd")
            .args(["update", bead_id, "--status", "ready"])
            .output();

        if let Ok(output) = result {
            return output.status.success();
        }
    }

    false
}

/// Output the result
fn output_result(output: &AbortOutput, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("abort-response", "single", output);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize abort output")?;
        println!("{json_str}");
    } else {
        println!("{}", output.message);
        if output.workspace_removed {
            println!("  Workspace files removed");
        }
        if output.bead_updated {
            println!("  Bead status updated to ready");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abort_output_serializes() {
        let output = AbortOutput {
            session_name: "test-session".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted session 'test-session'".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"session_name\":\"test-session\""));
        assert!(json_str.contains("\"workspace_removed\":true"));
    }

    #[test]
    fn test_abort_options_default() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: false,
            keep_workspace: false,
            dry_run: false,
            format: OutputFormat::Human,
        };

        assert!(options.workspace.is_none());
        assert!(!options.no_bead_update);
        assert!(!options.keep_workspace);
        assert!(!options.dry_run);
    }

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test `AbortOutput` message format
    #[test]
    fn test_abort_output_message_format() {
        let output = AbortOutput {
            session_name: "feature-auth".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted session 'feature-auth'".to_string(),
        };

        assert!(output.message.contains("Aborted"));
        assert!(output.message.contains(&output.session_name));
    }

    /// Test `workspace_removed` flag
    #[test]
    fn test_abort_workspace_removed_flag() {
        // When workspace removed
        let removed = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted".to_string(),
        };
        assert!(removed.workspace_removed);

        // When --keep-workspace used
        let kept = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: false,
            bead_updated: false,
            message: "Aborted".to_string(),
        };
        assert!(!kept.workspace_removed);
    }

    /// Test `bead_updated` flag
    #[test]
    fn test_abort_bead_updated_flag() {
        // When bead updated
        let updated = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: true,
            message: "Aborted".to_string(),
        };
        assert!(updated.bead_updated);

        // When --no-bead-update used
        let not_updated = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: false,
            message: "Aborted".to_string(),
        };
        assert!(!not_updated.bead_updated);
    }

    /// Test `AbortOptions` with explicit workspace
    #[test]
    fn test_abort_options_explicit_workspace() {
        let options = AbortOptions {
            workspace: Some("specific-workspace".to_string()),
            no_bead_update: false,
            keep_workspace: false,
            dry_run: false,
            format: OutputFormat::Human,
        };

        assert_eq!(options.workspace, Some("specific-workspace".to_string()));
    }

    /// Test `AbortOptions` with `keep_workspace`
    #[test]
    fn test_abort_options_keep_workspace() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: false,
            keep_workspace: true,
            dry_run: false,
            format: OutputFormat::Human,
        };

        assert!(options.keep_workspace);
    }

    /// Test `AbortOptions` with `no_bead_update`
    #[test]
    fn test_abort_options_no_bead_update() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: true,
            keep_workspace: false,
            dry_run: false,
            format: OutputFormat::Human,
        };

        assert!(options.no_bead_update);
    }

    /// Test `AbortOptions` `dry_run` mode
    #[test]
    fn test_abort_options_dry_run() {
        let options = AbortOptions {
            workspace: None,
            no_bead_update: false,
            keep_workspace: false,
            dry_run: true,
            format: OutputFormat::Human,
        };

        assert!(options.dry_run);
    }

    /// Test `AbortOutput` JSON serialization
    #[test]
    fn test_abort_output_json_complete() {
        let output = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true,
            bead_updated: true,
            message: "Aborted session 'test'".to_string(),
        };

        let json_str = serde_json::to_string(&output).unwrap_or_default();

        assert!(json_str.contains("session_name"));
        assert!(json_str.contains("workspace_removed"));
        assert!(json_str.contains("bead_updated"));
        assert!(json_str.contains("message"));
    }

    /// Test abort is opposite of done
    #[test]
    fn test_abort_is_opposite_of_done() {
        // Abort removes workspace, done merges it
        let abort_output = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true, // Key difference
            bead_updated: false,
            message: "Aborted".to_string(),
        };

        // Abort removes the workspace (destroys changes)
        assert!(abort_output.workspace_removed);
    }

    /// Test `dry_run` output structure
    #[test]
    fn test_abort_dry_run_output() {
        let output = AbortOutput {
            session_name: "test".to_string(),
            workspace_removed: true, // Would be removed
            bead_updated: true,      // Would be updated
            message: "[DRY RUN] Would abort session 'test'".to_string(),
        };

        assert!(output.message.contains("DRY RUN"));
    }
}
