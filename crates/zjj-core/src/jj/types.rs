//! JJ-specific types and structures

use std::path::PathBuf;

/// Information about a JJ workspace
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Workspace name
    pub name: String,
    /// Workspace path
    pub path: PathBuf,
    /// Whether the workspace is stale (directory doesn't exist)
    pub is_stale: bool,
}

/// Summary of changes in a workspace (JJ-specific)
///
/// Note: This is distinct from `types::DiffSummary` which includes per-file stats.
/// This version is used for simple insertion/deletion counts from jj diff --stat output.
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    /// Number of lines added
    pub insertions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

/// Status of files in a workspace
#[derive(Debug, Clone)]
pub struct Status {
    /// Modified files
    pub modified: Vec<PathBuf>,
    /// Added files
    pub added: Vec<PathBuf>,
    /// Deleted files
    pub deleted: Vec<PathBuf>,
    /// Renamed files (`old_path`, `new_path`)
    pub renamed: Vec<(PathBuf, PathBuf)>,
    /// Unknown files
    pub unknown: Vec<PathBuf>,
}

impl Status {
    /// Check if there are any changes
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.modified.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.renamed.is_empty()
    }

    /// Count total number of changed files
    #[must_use]
    pub const fn change_count(&self) -> usize {
        self.modified
            .len()
            .saturating_add(self.added.len())
            .saturating_add(self.deleted.len())
            .saturating_add(self.renamed.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_is_clean() {
        let clean_status = Status {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(clean_status.is_clean());

        let dirty_status = Status {
            modified: vec![PathBuf::from("file.rs")],
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(!dirty_status.is_clean());
    }
}
