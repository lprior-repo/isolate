//! Remove a session and its workspace
//!
//! This module provides functionality to cleanly remove a session including:
//! - Closing Zellij tab (if inside Zellij)
//! - Running pre-remove hooks
//! - Optionally merging to main (with --merge flag)
//! - Removing workspace directory
//! - Forgetting JJ workspace
//! - Deleting database entry

mod bead;
mod confirmation;
mod dry_run;
mod hooks;
mod merge;
mod operations;
mod validation;

use std::process;

use anyhow::Result;

use crate::{
    commands::get_session_db,
    json_output::{RemoveOperation, RemoveOutput},
};

/// Options for the remove command
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct RemoveOptions {
    /// Skip confirmation prompt and hooks
    pub force: bool,
    /// Squash-merge to main before removal
    pub merge: bool,
    /// Preserve branch after removal
    #[allow(dead_code)]
    pub keep_branch: bool,
    /// Output as JSON
    pub json: bool,
    /// Show what would be done without executing
    pub dry_run: bool,
}

/// Run the remove command
#[allow(dead_code)]
pub async fn run(name: &str) -> Result<()> {
    run_with_options(name, &RemoveOptions::default()).await
}

/// Run the remove command with options
pub async fn run_with_options(name: &str, options: &RemoveOptions) -> Result<()> {
    // Execute and handle errors with JSON output if requested
    match run_remove_impl(name, options).await {
        Ok(()) => Ok(()),
        Err(e) if options.json => {
            output_json_error(name, &e);
        }
        Err(e) => Err(e),
    }
}

/// Internal implementation of remove command
async fn run_remove_impl(name: &str, options: &RemoveOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Validate and get session
    let session = validation::validate_and_get_session(&db, name).await?;

    // DRY-RUN: Show what would be done without executing
    if options.dry_run {
        let plan = dry_run::build_plan(name, &session, options)?;
        return dry_run::output(&plan, options.json);
    }

    // Confirm removal unless --force
    if !options.force && !confirmation::confirm_removal(name)? {
        return handle_cancellation(name, options.json);
    }

    // Load config for hooks and merge operations
    let config = zjj_core::config::load_config()?;

    // Run pre_remove hooks unless --force
    if !options.force {
        hooks::run_pre_remove_hooks(name, &session.workspace_path, &config);
    }

    // If --merge: squash-merge to main
    if options.merge {
        merge::merge_to_main(name, &session.workspace_path, &config)?;
    }

    // Update bead status if session has attached bead (best-effort)
    let workspace_path = std::path::Path::new(&session.workspace_path);
    if let Err(e) = bead::process_bead_removal(&session, options.merge, &config, workspace_path) {
        eprintln!("Warning: Failed to update bead status: {e}");
    }

    // Execute removal operations
    let operations = execute_removal_operations(&db, name, &session).await?;

    // Output success
    output_success(name, operations, options.json)?;

    Ok(())
}

/// Execute all removal operations in the correct order
///
/// Operation order matters for atomicity (zjj-gmk):
/// 1. Close Zellij tab (optional, can fail gracefully)
/// 2. Remove workspace directory (fail fast if filesystem issues)
/// 3. Forget JJ workspace (only after directory is gone)
/// 4. Delete database entry (final step after all resources cleaned)
///
/// This function uses Railway-Oriented Programming: each fallible step returns
/// `Result`, and the entire pipeline short-circuits on first error using `?`.
async fn execute_removal_operations(
    db: &crate::database::SessionDb,
    name: &str,
    session: &crate::session::Session,
) -> Result<Vec<RemoveOperation>> {
    // Step 1: Close Zellij tab if inside Zellij (optional)
    let zellij_op = operations::close_zellij_tab_if_present(&session.zellij_tab);

    // Steps 2-4: Execute required operations, collecting into Result<Vec<_>>
    // Each operation returns Result<RemoveOperation>, so we chain with `?`
    let required_ops = [
        operations::remove_workspace_directory(session)?, // Step 2
        operations::forget_jj_workspace(name)?,           // Step 3
        operations::delete_database_entry(db, name, session.id).await?, // Step 4
    ];

    // Combine optional Zellij operation with required operations
    // Filter + chain pattern: include Zellij op only if Some, then chain required ops
    Ok(zellij_op.into_iter().chain(required_ops).collect())
}

/// Handle user cancellation
fn handle_cancellation(name: &str, json: bool) -> Result<()> {
    if json {
        let output = RemoveOutput {
            success: false,
            session: name.to_string(),
            operations: None,
            message: Some("Removal cancelled".to_string()),
            error: None,
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("Removal cancelled");
    }
    Ok(())
}

/// Output success message
fn output_success(name: &str, operations: Vec<RemoveOperation>, json: bool) -> Result<()> {
    if json {
        let output = RemoveOutput {
            success: true,
            session: name.to_string(),
            operations: Some(operations),
            message: None,
            error: None,
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("Removed session '{name}'");
    }
    Ok(())
}

/// Output JSON error and exit
fn output_json_error(name: &str, e: &anyhow::Error) -> ! {
    let output = RemoveOutput {
        success: false,
        session: name.to_string(),
        operations: None,
        message: None,
        error: Some(e.to_string()),
    };
    if let Ok(json_str) = serde_json::to_string(&output) {
        println!("{json_str}");
    }
    // Determine appropriate exit code based on error type
    let exit_code = e
        .downcast_ref::<zjj_core::Error>()
        .map_or(2, zjj_core::Error::exit_code);
    process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_options_default() {
        let opts = RemoveOptions::default();
        assert!(!opts.force);
        assert!(!opts.merge);
        assert!(!opts.keep_branch);
        assert!(!opts.dry_run);
    }
}
