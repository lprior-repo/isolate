//! Merge-to-main workflow for session removal

use std::path::Path;

use anyhow::{Context, Result};

use crate::commands::remove::hooks::run_post_merge_hooks;

/// Merge session to main branch
///
/// This function performs a squash-merge workflow:
/// 1. Squash all commits in the workspace
/// 2. Rebase onto main branch
/// 3. Push changes to remote
/// 4. Run `post_merge` hooks
///
/// # Errors
///
/// Returns error if:
/// - Workspace path is invalid
/// - JJ commands fail (squash, rebase, push)
/// - Main branch doesn't exist
/// - Rebase conflicts occur
pub fn merge_to_main(
    name: &str,
    workspace_path: &str,
    config: &zjj_core::config::Config,
) -> Result<()> {
    let workspace_path_buf = Path::new(workspace_path);

    // Get main branch from config or use default (zjj-qf8)
    let main_branch = config.main_branch.as_deref().unwrap_or("trunk()");

    eprintln!("Merging session '{name}' to {main_branch}...");

    // Step 1: Squash commits
    eprintln!("  1. Squashing commits...");
    squash_commits(name, workspace_path_buf, workspace_path)?;

    // Step 2: Rebase onto main
    eprintln!("  2. Rebasing onto {main_branch}...");
    rebase_onto_main(name, workspace_path_buf, workspace_path, main_branch)?;

    // Step 3: Push to remote
    eprintln!("  3. Pushing to remote...");
    push_to_remote(name, workspace_path_buf, workspace_path)?;

    // Step 4: Run `post_merge` hooks
    run_post_merge_hooks(name, workspace_path, config);

    eprintln!("Successfully merged session '{name}' to {main_branch}");
    Ok(())
}

/// Squash all commits in the workspace
fn squash_commits(name: &str, workspace_path_buf: &Path, workspace_path: &str) -> Result<()> {
    zjj_core::jj::workspace_squash(workspace_path_buf).with_context(|| {
        format!(
            "Failed to squash commits in workspace '{name}'\n\
             \n\
             Possible causes:\n\
             • No commits to squash\n\
             • Workspace is not in a valid state\n\
             \n\
             Try:\n\
             • Check workspace status with: cd {workspace_path} && jj status\n\
             • Manually squash with: jj squash"
        )
    })
}

/// Rebase workspace onto main branch
fn rebase_onto_main(
    name: &str,
    workspace_path_buf: &Path,
    workspace_path: &str,
    main_branch: &str,
) -> Result<()> {
    zjj_core::jj::workspace_rebase_onto_main(workspace_path_buf, main_branch).with_context(
        || {
            format!(
                "Failed to rebase workspace '{name}' onto {main_branch}\n\
                 \n\
                 Possible causes:\n\
                 • Rebase conflicts\n\
                 • Main branch doesn't exist\n\
                 • Workspace is not in a valid state\n\
                 \n\
                 Try:\n\
                 • Resolve conflicts manually with: cd {workspace_path} && jj rebase -d {main_branch}\n\
                 • Check if {main_branch} exists"
            )
        },
    )
}

/// Push changes to remote
fn push_to_remote(name: &str, workspace_path_buf: &Path, workspace_path: &str) -> Result<()> {
    zjj_core::jj::workspace_git_push(workspace_path_buf).with_context(|| {
        format!(
            "Failed to push changes from workspace '{name}'\n\
             \n\
             Possible causes:\n\
             • No git remote configured\n\
             • Network error\n\
             • Authentication failed\n\
             • Remote rejected push\n\
             \n\
             Try:\n\
             • Check git remote with: cd {workspace_path} && jj git remote -v\n\
             • Manually push with: jj git push"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_requires_valid_workspace() {
        let config = zjj_core::config::Config::default();
        // This will fail because the workspace doesn't exist
        // but it verifies the function signature is correct
        let result = merge_to_main("test", "/nonexistent/path", &config);
        assert!(result.is_err());
    }
}
