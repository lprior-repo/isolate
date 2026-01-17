//! Pure parsing functions for JJ output (no I/O operations)

use std::path::PathBuf;

use crate::Result;

use super::types::{DiffSummary, Status, WorkspaceInfo};

/// Parse output from 'jj workspace list'
pub fn parse_workspace_list(output: &str) -> Result<Vec<WorkspaceInfo>> {
    use crate::Error;

    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            // Format: "workspace_name: /path/to/workspace"
            // or "workspace_name: /path/to/workspace (stale)"
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(Error::parse_error(format!(
                    "Invalid workspace list format: {line}"
                )));
            }

            let name = parts[0].trim().to_string();
            let rest = parts[1].trim();

            let (path_str, is_stale) = rest
                .strip_suffix("(stale)")
                .map_or((rest, false), |path_part| (path_part.trim(), true));

            Ok(WorkspaceInfo {
                name,
                path: PathBuf::from(path_str),
                is_stale,
            })
        })
        .collect()
}

/// Parse output from 'jj status'
pub fn parse_status(output: &str) -> Status {
    #[derive(Debug)]
    enum StatusLine {
        Modified(PathBuf),
        Added(PathBuf),
        Deleted(PathBuf),
        Renamed(PathBuf, PathBuf),
        Unknown(PathBuf),
    }
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            // Parse status markers: M, A, D, R, ?
            #[allow(clippy::option_if_let_else)]
            if let Some(rest) = line.strip_prefix('M') {
                Some(StatusLine::Modified(PathBuf::from(rest.trim())))
            } else if let Some(rest) = line.strip_prefix('A') {
                Some(StatusLine::Added(PathBuf::from(rest.trim())))
            } else if let Some(rest) = line.strip_prefix('D') {
                Some(StatusLine::Deleted(PathBuf::from(rest.trim())))
            } else if let Some(rest) = line.strip_prefix('R') {
                // Renamed: "R old_path => new_path"
                rest.split_once("=>").map(|(old, new)| {
                    StatusLine::Renamed(PathBuf::from(old.trim()), PathBuf::from(new.trim()))
                })
            } else {
                line.strip_prefix('?')
                    .map(|rest| StatusLine::Unknown(PathBuf::from(rest.trim())))
            }
        })
        .fold(
            Status {
                modified: Vec::new(),
                added: Vec::new(),
                deleted: Vec::new(),
                renamed: Vec::new(),
                unknown: Vec::new(),
            },
            |mut status, line| {
                match line {
                    StatusLine::Modified(path) => status.modified.push(path),
                    StatusLine::Added(path) => status.added.push(path),
                    StatusLine::Deleted(path) => status.deleted.push(path),
                    StatusLine::Renamed(old, new) => status.renamed.push((old, new)),
                    StatusLine::Unknown(path) => status.unknown.push(path),
                }
                status
            },
        )
}

/// Parse output from 'jj diff --stat'
pub fn parse_diff_stat(output: &str) -> DiffSummary {
    // Look for summary line like: "5 files changed, 123 insertions(+), 45 deletions(-)"
    let summary_line = output
        .lines()
        .find(|line| line.contains("insertion") || line.contains("deletion"))
        .unwrap_or("");

    // Functional parsing: chain Option operations to extract numbers
    let insertions = summary_line
        .split("insertion")
        .next()
        .and_then(|ins_str| ins_str.split_whitespace().last())
        .and_then(|num_str| num_str.parse().ok())
        .unwrap_or(0);

    let deletions = summary_line
        .split("deletion")
        .next()
        .and_then(|del_str| del_str.rsplit(',').next())
        .and_then(|s| s.split_whitespace().next())
        .and_then(|num_str| num_str.parse().ok())
        .unwrap_or(0);

    DiffSummary {
        insertions,
        deletions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace_list() {
        let output = "default: /home/user/repo\nfeature: /home/user/repo/.jjz/workspaces/feature\nstale-ws: /home/user/old (stale)";
        let result = parse_workspace_list(output);
        assert!(result.is_ok());

        let workspaces = result.unwrap_or_else(|_| Vec::new());
        assert_eq!(workspaces.len(), 3);
        assert_eq!(workspaces[0].name, "default");
        assert!(!workspaces[0].is_stale);
        assert_eq!(workspaces[2].name, "stale-ws");
        assert!(workspaces[2].is_stale);
    }

    #[test]
    fn test_parse_status() {
        let output = "M file1.rs\nA file2.rs\nD file3.rs\n? unknown.txt";
        let status = parse_status(output);
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.added.len(), 1);
        assert_eq!(status.deleted.len(), 1);
        assert_eq!(status.unknown.len(), 1);
        assert!(!status.is_clean());
        assert_eq!(status.change_count(), 3);
    }

    #[test]
    fn test_parse_diff_stat() {
        let output = "file1.rs | 10 +++++++---\nfile2.rs | 5 ++---\n2 files changed, 12 insertions(+), 3 deletions(-)";
        let summary = parse_diff_stat(output);
        assert_eq!(summary.insertions, 12);
        assert_eq!(summary.deletions, 3);
    }
}
