//! Hook execution for remove command

use std::path::Path;

use zjj_core::hooks::{HookRunner, HookType};

/// Run `pre_remove` hooks
///
/// Executes configured `pre_remove` hooks before session removal.
/// If hooks fail, warns but doesn't block removal.
pub fn run_pre_remove_hooks(name: &str, workspace_path: &str, config: &zjj_core::config::Config) {
    let runner = HookRunner::new(config.hooks.clone());
    let workspace_path_buf = Path::new(workspace_path);

    match runner.run(HookType::PreRemove, workspace_path_buf) {
        Ok(_) => {
            // Hooks succeeded or no hooks configured
            eprintln!("Pre-remove hooks completed for session '{name}'");
        }
        Err(e) => {
            // Hook failed - warn but don't block removal
            eprintln!("Warning: pre_remove hook failed: {e}");
            eprintln!("Continuing with removal...");
        }
    }
}

/// Run `post_merge` hooks
///
/// Executes configured `post_merge` hooks after merging to main.
/// If hooks fail, warns but doesn't block removal.
pub fn run_post_merge_hooks(name: &str, workspace_path: &str, config: &zjj_core::config::Config) {
    let runner = HookRunner::new(config.hooks.clone());
    let workspace_path_buf = Path::new(workspace_path);

    match runner.run(HookType::PostMerge, workspace_path_buf) {
        Ok(_) => {
            eprintln!("Post-merge hooks completed for session '{name}'");
        }
        Err(e) => {
            // Hook failed - warn but don't block removal
            eprintln!("Warning: post_merge hook failed: {e}");
            eprintln!("Continuing with removal...");
        }
    }
}
