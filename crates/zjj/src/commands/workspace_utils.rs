use std::path::{Path, PathBuf};

use tokio::fs;

/// Resolve the workspace base directory from the repo root and config value.
pub fn resolve_workspace_base(root: &Path, workspace_dir: &str) -> PathBuf {
    let workspace_path = Path::new(workspace_dir);
    if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        root.join(workspace_path)
    }
}

/// Candidate roots to search for relocated workspaces.
/// Includes the configured workspace_dir and common fallbacks.
pub fn candidate_workspace_roots(root: &Path, workspace_dir: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    roots.push(resolve_workspace_base(root, workspace_dir));

    let fallback = root.join(".zjj").join("workspaces");
    if !roots.contains(&fallback) {
        roots.push(fallback);
    }

    let alt = root.join("workspaces");
    if !roots.contains(&alt) {
        roots.push(alt);
    }

    roots
}

/// Search for an existing workspace directory under known roots.
/// Returns the first matching path whose `.jj` directory exists.
pub async fn find_relocated_workspace(workspace_name: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    for root in roots {
        let candidate = root.join(workspace_name);
        if fs::try_exists(&candidate).await.unwrap_or(false) {
            let jj_path = candidate.join(".jj");
            if fs::try_exists(&jj_path).await.unwrap_or(false) {
                return Some(candidate);
            }
        }
    }
    None
}
