//! Session creation workflow orchestration
//!
//! This module orchestrates the complete session creation process:
//! 1. Create database entry with "creating" status
//! 2. Acquire workspace lock
//! 3. Create JJ workspace
//! 4. Generate Zellij layout
//! 5. Run post_create hooks
//! 6. Open Zellij tab
//! 7. Update database status to "active"
//! 8. Process bead integration (if provided)

use std::path::Path;

use anyhow::{Context, Result};
use zjj_core::{
    config::Config,
    hooks::{HookRunner, HookType},
    jj, zellij,
};

use crate::session::{SessionStatus, SessionUpdate};

use super::{bead, security, AddOptions};

/// Create a new session with full workflow orchestration
///
/// # Errors
/// Returns error if any step in the workflow fails
pub async fn create_session(
    db: &crate::database::SessionDb,
    config: &Config,
    repo_root: &Path,
    name: &str,
    workspace_path: &Path,
    options: &AddOptions,
) -> Result<()> {
    let workspace_path_str = workspace_path
        .to_str()
        .context("Workspace path contains invalid UTF-8")?;

    // Step 1: Create database entry with "creating" status
    db.create(name, workspace_path_str)
        .await
        .context("Failed to create session database entry")?;

    // Step 2: Acquire workspace lock for safety
    let _lock = security::acquire_workspace_lock(workspace_path_str)?;

    // Step 3: Create JJ workspace
    jj::workspace_create(name, workspace_path)
        .context("Failed to create JJ workspace")
        .inspect_err(|_e| {
            // Cleanup database entry on failure
            let _ = futures::executor::block_on(db.delete(name));
        })?;

    // Step 4: Generate Zellij layout
    let layout_dir = repo_root.join(&config.workspace_dir).join("layouts");
    let template = parse_template(
        options
            .template
            .as_deref()
            .unwrap_or(&config.default_template),
    );
    let layout_config = zellij::LayoutConfig::new(name.to_string(), workspace_path.to_path_buf());
    let layout = zellij::layout_generate(&layout_config, template, &layout_dir)
        .context("Failed to generate Zellij layout")?;

    // Step 5: Run post_create hooks (unless --no-hooks)
    if !options.no_hooks {
        let hook_runner = HookRunner::new(config.hooks.clone());
        hook_runner.run(HookType::PostCreate, workspace_path).ok(); // Hooks are optional, don't
                                                                    // fail on hook errors
    }

    // Step 6: Open Zellij tab (unless --no-open)
    if !options.no_open {
        let tab_name = format!("jjz:{name}");
        zellij::tab_open(&layout.file_path, &tab_name).context("Failed to open Zellij tab")?;
    }

    // Step 7: Update database status to "active"
    db.update(
        name,
        SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        },
    )
    .await
    .context("Failed to update session status to active")?;

    // Step 8: Process bead integration if --bead flag provided
    if let Some(bead_id) = &options.bead {
        process_bead_integration(db, repo_root, workspace_path, name, bead_id, options).await?;
    }

    Ok(())
}

/// Process bead integration for a session
async fn process_bead_integration(
    db: &crate::database::SessionDb,
    repo_root: &Path,
    workspace_path: &Path,
    session_name: &str,
    bead_id: &str,
    options: &AddOptions,
) -> Result<()> {
    // Validate bead exists
    let bead = bead::validate_bead_exists(repo_root, bead_id)
        .await
        .context("Bead validation failed")?;

    // Build and store bead metadata
    let bead_metadata = bead::build_bead_metadata(&bead);
    db.update(
        session_name,
        SessionUpdate {
            metadata: Some(bead_metadata),
            ..Default::default()
        },
    )
    .await
    .context("Failed to store bead metadata in session")?;

    // Write BEAD_SPEC.md to workspace
    let spec_content = bead::generate_bead_spec(&bead);
    bead::write_bead_spec(workspace_path, &spec_content).context("Failed to write BEAD_SPEC.md")?;

    // Update bead status to in_progress (don't fail if bd update fails)
    if !options.no_hooks {
        bead::update_bead_status(bead_id, "in_progress").ok();
    }

    Ok(())
}

/// Parse template name to `LayoutTemplate` enum
fn parse_template(template_name: &str) -> zellij::LayoutTemplate {
    match template_name {
        "minimal" => zellij::LayoutTemplate::Minimal,
        "full" => zellij::LayoutTemplate::Full,
        "split" => zellij::LayoutTemplate::Split,
        "review" => zellij::LayoutTemplate::Review,
        _ => zellij::LayoutTemplate::Standard, // Default to standard for unknown templates
    }
}
