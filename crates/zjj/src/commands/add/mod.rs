//! Add command - creates a new session with JJ workspace and Zellij tab
//!
//! This module handles the full session creation workflow:
//! 1. Validate preconditions (dependencies, session name, workspace availability)
//! 2. Create JJ workspace
//! 3. Generate Zellij layout
//! 4. Create database entry
//! 5. Run post-create hooks
//! 6. Open Zellij tab (unless --no-open)

pub mod bead;
pub mod dry_run;
mod error_messages;
pub mod presentation;
pub mod security;
pub mod validation;
pub mod validators;
pub mod workflow;

use anyhow::{Context, Result};

use crate::commands::get_session_db;

/// Options for the add command
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct AddOptions {
    pub name: String,
    pub no_hooks: bool,
    pub template: Option<String>,
    pub no_open: bool,
    pub json: bool,
    pub dry_run: bool,
    pub bead: Option<String>,
}

/// Run add command with default options (called by dashboard)
///
/// # Errors
/// Returns error if the add command fails
pub async fn run(name: &str) -> Result<()> {
    run_with_options(&AddOptions {
        name: name.to_string(),
        no_hooks: false,
        template: None,
        no_open: false,
        json: false,
        dry_run: false,
        bead: None,
    })
    .await
}

/// Run add command with custom options (called by dispatch)
///
/// # Errors
/// Returns error if the add command fails
pub async fn run_with_options(options: &AddOptions) -> Result<()> {
    match run_add_impl(options).await {
        Ok(()) => Ok(()),
        Err(e) if options.json => {
            presentation::output_json_error(&options.name, &e);
        }
        Err(e) => Err(e),
    }
}

/// Internal implementation of add command
async fn run_add_impl(options: &AddOptions) -> Result<()> {
    // Get database connection
    let db = get_session_db().await?;

    // Load configuration
    let config = zjj_core::config::load_config()?;

    // Get repository root
    let repo_root = zjj_core::jj::check_in_jj_repo()?;

    // Build workspace path
    let workspace_path = repo_root.join(&config.workspace_dir).join(&options.name);
    let workspace_path_str = workspace_path
        .to_str()
        .context("Workspace path contains invalid UTF-8")?;

    // Run all validations
    validation::validate_all(&options.name, &db, &repo_root, options.no_open).await?;

    // Security validations
    security::validate_workspace_path(workspace_path_str, &repo_root, &config.workspace_dir)?;
    security::validate_no_symlinks(workspace_path_str, &repo_root)?;
    security::validate_workspace_dir(workspace_path_str)?;
    security::check_workspace_writable(workspace_path_str)?;

    // DRY-RUN: Show what would be done without executing
    if options.dry_run {
        let params = dry_run::DryRunParams {
            session_name: &options.name,
            workspace_path: workspace_path_str,
            root: &repo_root,
            config: &config,
            template: options.template.as_deref(),
            no_open: options.no_open,
            no_hooks: options.no_hooks,
            bead: options.bead.as_deref(),
        };
        return dry_run::execute(&params, options.json);
    }

    // Execute session creation workflow
    workflow::create_session(
        &db,
        &config,
        &repo_root,
        &options.name,
        &workspace_path,
        options,
    )
    .await?;

    // Output success
    presentation::output_success(&options.name, workspace_path_str, options.json)?;

    Ok(())
}
