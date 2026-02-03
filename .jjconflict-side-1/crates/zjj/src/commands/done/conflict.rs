//! Pre-merge conflict detection for the done command

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::executor::{ExecutorError, JjExecutor};

/// Errors during conflict detection
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConflictError {
    #[error("Failed to check workspace status: {0}")]
    StatusFailed(String),
    #[error("Failed to find merge base: {0}")]
    MergeBaseFailed(String),
    #[error("Failed to get diff: {0}")]
    DiffFailed(String),
    #[error("JJ command failed: {0}")]
    JjFailed(String),
    #[error("Invalid JJ output: {0}")]
    InvalidOutput(String),
}

impl From<ExecutorError> for ConflictError {
    fn from(err: ExecutorError) -> Self {
        Self::JjFailed(err.to_string())
    }
}

/// Result of conflict detection
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictDetectionResult {
    pub has_existing_conflicts: bool,
    pub existing_conflicts: Vec<String>,
    pub overlapping_files: Vec<String>,
    pub workspace_only: Vec<String>,
    pub main_only: Vec<String>,
    pub merge_likely_safe: bool,
    pub summary: String,
    pub merge_base: Option<String>,
    pub files_analyzed: usize,
}

impl ConflictDetectionResult {
    pub fn no_conflicts() -> Self {
        Self {
            merge_likely_safe: true,
            summary: "No conflicts detected - merge is safe".to_string(),
            ..Default::default()
        }
    }

    pub fn has_conflicts(&self) -> bool {
        self.has_existing_conflicts || !self.overlapping_files.is_empty()
    }

    pub fn conflict_count(&self) -> usize {
        self.existing_conflicts.len() + self.overlapping_files.len()
    }
}

/// Trait for conflict detection
pub trait ConflictDetector {
    fn detect_conflicts(&self) -> Result<ConflictDetectionResult, ConflictError>;
    fn has_existing_conflicts(&self) -> Result<bool, ConflictError>;
}

/// JJ-based conflict detector
pub struct JjConflictDetector<'a, E: JjExecutor> {
    executor: &'a E,
}

impl<'a, E: JjExecutor> JjConflictDetector<'a, E> {
    pub fn new(executor: &'a E) -> Self {
        Self { executor }
    }

    fn check_existing_conflicts(&self) -> Result<Vec<String>, ConflictError> {
        let output = self
            .executor
            .run(&[
                "log",
                "-r",
                "@",
                "--no-graph",
                "-T",
                r#"if(conflict, "CONFLICT\n", "")"#,
            ])
            .map_err(|e| ConflictError::StatusFailed(e.to_string()))?;

        if output.as_str().contains("CONFLICT") {
            let resolve_output = self
                .executor
                .run(&["resolve", "--list"])
                .map_err(|e| ConflictError::StatusFailed(e.to_string()))?;

            let conflicts: Vec<String> = resolve_output
                .as_str()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    line.split_whitespace()
                        .next()
                        .unwrap_or(line.trim())
                        .to_string()
                })
                .collect();

            Ok(conflicts)
        } else {
            Ok(Vec::new())
        }
    }

    fn find_merge_base(&self) -> Result<Option<String>, ConflictError> {
        let output = self
            .executor
            .run(&[
                "log",
                "-r",
                "heads(::@ & ::trunk())",
                "--no-graph",
                "-T",
                "commit_id ++ \"\\n\"",
                "--limit",
                "1",
            ])
            .map_err(|e| ConflictError::MergeBaseFailed(e.to_string()))?;

        let commit_id = output.as_str().trim();
        if commit_id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(commit_id.to_string()))
        }
    }

    fn get_workspace_modified_files(&self) -> Result<HashSet<String>, ConflictError> {
        let output = self
            .executor
            .run(&["diff", "--from", "trunk()", "--to", "@", "--summary"])
            .map_err(|e| ConflictError::DiffFailed(e.to_string()))?;

        Self::parse_diff_summary(output.as_str())
    }

    fn get_trunk_modified_files(&self, merge_base: &str) -> Result<HashSet<String>, ConflictError> {
        let output = self
            .executor
            .run(&["diff", "--from", merge_base, "--to", "trunk()", "--summary"])
            .map_err(|e| ConflictError::DiffFailed(e.to_string()))?;

        Self::parse_diff_summary(output.as_str())
    }

    fn parse_diff_summary(output: &str) -> Result<HashSet<String>, ConflictError> {
        let files: HashSet<String> = output
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() >= 2 {
                    let file_part = parts.get(1).copied().unwrap_or("");
                    if file_part.contains(" -> ") {
                        file_part.split(" -> ").last().map(|s| s.to_string())
                    } else {
                        Some(file_part.to_string())
                    }
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();

        Ok(files)
    }
}

impl<'a, E: JjExecutor> ConflictDetector for JjConflictDetector<'a, E> {
    fn detect_conflicts(&self) -> Result<ConflictDetectionResult, ConflictError> {
        let existing_conflicts = self.check_existing_conflicts()?;
        let has_existing = !existing_conflicts.is_empty();
        let merge_base = self.find_merge_base()?;
        let workspace_files = self.get_workspace_modified_files()?;

        let trunk_files = if let Some(ref base) = merge_base {
            self.get_trunk_modified_files(base)?
        } else {
            let output = self
                .executor
                .run(&["diff", "--from", "@", "--to", "trunk()", "--summary"])
                .map_err(|e| ConflictError::DiffFailed(e.to_string()))?;
            Self::parse_diff_summary(output.as_str())?
        };

        let overlapping: Vec<String> = workspace_files
            .intersection(&trunk_files)
            .cloned()
            .collect();
        let workspace_only: Vec<String> =
            workspace_files.difference(&trunk_files).cloned().collect();
        let main_only: Vec<String> = trunk_files.difference(&workspace_files).cloned().collect();
        let merge_likely_safe = !has_existing && overlapping.is_empty();

        let summary = if has_existing {
            format!(
                "Existing conflicts in {} files - resolve before merging",
                existing_conflicts.len()
            )
        } else if !overlapping.is_empty() {
            format!(
                "Potential conflicts in {} files: {}",
                overlapping.len(),
                overlapping.join(", ")
            )
        } else {
            "No conflicts detected - merge is safe".to_string()
        };

        Ok(ConflictDetectionResult {
            has_existing_conflicts: has_existing,
            existing_conflicts,
            overlapping_files: overlapping,
            workspace_only,
            main_only,
            merge_likely_safe,
            summary,
            merge_base,
            files_analyzed: workspace_files.len() + trunk_files.len(),
        })
    }

    fn has_existing_conflicts(&self) -> Result<bool, ConflictError> {
        Ok(!self.check_existing_conflicts()?.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflicts_result() {
        let result = ConflictDetectionResult::no_conflicts();
        assert!(!result.has_conflicts());
        assert!(result.merge_likely_safe);
        assert_eq!(result.conflict_count(), 0);
    }

    #[test]
    fn test_has_conflicts_with_overlapping() {
        let result = ConflictDetectionResult {
            overlapping_files: vec!["shared.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
        assert_eq!(result.conflict_count(), 1);
    }

    #[test]
    fn test_has_conflicts_with_existing() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["conflicted.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
        assert_eq!(result.conflict_count(), 1);
    }

    #[test]
    fn test_parse_diff_summary() {
        let output =
            "M src/lib.rs\nA src/new.rs\nD src/old.rs\nR src/renamed.rs -> src/new_name.rs";
        let files =
            JjConflictDetector::<super::super::executor::RealJjExecutor>::parse_diff_summary(
                output,
            );
        assert!(files.is_ok());
        let files = files.unwrap_or_default();
        assert!(files.contains("src/lib.rs"));
        assert!(files.contains("src/new.rs"));
        assert!(files.contains("src/old.rs"));
        assert!(files.contains("src/new_name.rs"));
    }
}
