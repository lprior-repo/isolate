//! Pre-merge conflict detection for the done command
//!
//! This module provides comprehensive conflict detection before merging
//! a workspace to main. It follows these principles:
//!
//! - **Zero false negatives**: Catches ALL potential conflicts
//! - **Sub-second detection**: Fast analysis through efficient JJ queries
//! - **JJ integration**: Uses JJ's native conflict and diff capabilities
//! - **Railway-Oriented Programming**: All operations return Result
//!
//! ## Exit Codes
//!
//! - 0: Merge is safe (no conflicts detected)
//! - 1: Conflicts detected (potential or existing)
//! - 3: Error during detection
//!
//! ## Detection Strategy
//!
//! 1. Check for existing JJ conflicts in the workspace
//! 2. Find the merge base (common ancestor of workspace and trunk)
//! 3. Get files modified in workspace since merge base
//! 4. Get files modified in trunk since merge base
//! 5. Identify overlapping files (potential conflicts)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::executor::{ExecutorError, JjExecutor};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during conflict detection
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConflictError {
    /// Failed to check workspace status
    #[error("Failed to check workspace status: {0}")]
    StatusFailed(String),

    /// Failed to find merge base between workspace and trunk
    #[error("Failed to find merge base: {0}")]
    MergeBaseFailed(String),

    /// Failed to get diff information
    #[error("Failed to get diff: {0}")]
    DiffFailed(String),

    /// JJ command execution failed
    #[error("JJ command failed: {0}")]
    JjFailed(String),

    /// Invalid output from JJ command
    #[allow(dead_code)] // Reserved for future validation
    #[error("Invalid JJ output: {0}")]
    InvalidOutput(String),

    /// Workspace is not in a valid state for detection
    #[allow(dead_code)] // Reserved for future validation
    #[error("Invalid workspace state: {0}")]
    InvalidState(String),
}

impl From<ExecutorError> for ConflictError {
    fn from(err: ExecutorError) -> Self {
        Self::JjFailed(err.to_string())
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Exit codes for conflict detection
#[allow(dead_code)] // Used in tests and for future CLI subcommand
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ConflictExitCode {
    /// Merge is safe - no conflicts detected
    Safe = 0,
    /// Conflicts detected - merge may fail
    Conflicts = 1,
    /// Error during detection
    Error = 3,
}

impl ConflictExitCode {
    /// Convert to i32 for process exit
    #[must_use]
    #[allow(dead_code)] // For future CLI exit code handling
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Comprehensive result of conflict detection
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictDetectionResult {
    /// Whether there are existing JJ conflicts in the workspace
    pub has_existing_conflicts: bool,

    /// List of files with existing JJ conflicts
    pub existing_conflicts: Vec<String>,

    /// Files modified in both workspace and trunk (potential conflicts)
    pub overlapping_files: Vec<String>,

    /// Files modified only in workspace
    pub workspace_only: Vec<String>,

    /// Files modified only in trunk/main
    pub main_only: Vec<String>,

    /// Whether the merge is likely to succeed without conflicts
    pub merge_likely_safe: bool,

    /// Human-readable summary of the detection result
    pub summary: String,

    /// The merge base commit (common ancestor)
    pub merge_base: Option<String>,

    /// Total number of files analyzed
    pub files_analyzed: usize,

    /// Time taken for detection in milliseconds
    pub detection_time_ms: u64,
}

impl ConflictDetectionResult {
    /// Create a result indicating no conflicts were detected
    #[cfg(test)]
    #[must_use]
    pub fn no_conflicts() -> Self {
        Self {
            merge_likely_safe: true,
            summary: "No conflicts detected - merge is safe".to_string(),
            ..Default::default()
        }
    }

    /// Check if any conflicts (existing or potential) were found
    #[must_use]
    pub const fn has_conflicts(&self) -> bool {
        self.has_existing_conflicts || !self.overlapping_files.is_empty()
    }

    /// Get total count of conflicts (existing + potential)
    #[cfg(test)]
    #[must_use]
    pub const fn conflict_count(&self) -> usize {
        self.existing_conflicts.len() + self.overlapping_files.len()
    }

    /// Get the appropriate exit code for this result
    #[cfg(test)]
    #[must_use]
    pub const fn exit_code(&self) -> ConflictExitCode {
        if self.has_existing_conflicts || !self.overlapping_files.is_empty() {
            ConflictExitCode::Conflicts
        } else {
            ConflictExitCode::Safe
        }
    }

    /// Format as human-readable text output
    #[must_use]
    pub fn to_text_output(&self) -> String {
        use std::fmt::Write;

        let mut output = String::new();

        if self.merge_likely_safe {
            output.push_str("No conflicts detected - merge is safe\n");
        } else {
            output.push_str("Conflicts detected:\n\n");

            if !self.existing_conflicts.is_empty() {
                output.push_str("  Existing JJ conflicts:\n");
                for file in &self.existing_conflicts {
                    let _ = writeln!(output, "    - {file}");
                }
                output.push('\n');
            }

            if !self.overlapping_files.is_empty() {
                output.push_str("  Files modified in both workspace and main:\n");
                for file in &self.overlapping_files {
                    let _ = writeln!(output, "    - {file}");
                }
                output.push('\n');
            }

            output.push_str("  Resolution hints:\n");
            output.push_str("    - Use 'jj resolve' to resolve existing conflicts\n");
            output.push_str("    - Review overlapping files before merging\n");
            output.push_str("    - Consider rebasing onto trunk first: jj rebase -d trunk()\n");
        }

        if let Some(ref base) = self.merge_base {
            let _ = writeln!(output, "\nMerge base: {base}");
        }

        let _ = writeln!(output, "Files analyzed: {}", self.files_analyzed);
        let _ = writeln!(output, "Detection time: {}ms", self.detection_time_ms);

        output
    }
}

// ============================================================================
// Conflict Detector Trait
// ============================================================================

/// Trait for conflict detection implementations
#[allow(dead_code)] // For future extensibility - allows alternative implementations
pub trait ConflictDetector {
    /// Perform comprehensive conflict detection
    ///
    /// This is the main entry point for conflict detection. It:
    /// 1. Checks for existing JJ conflicts
    /// 2. Finds the merge base
    /// 3. Analyzes file overlap between workspace and trunk
    /// 4. Returns a comprehensive result
    fn detect_conflicts(&self) -> Result<ConflictDetectionResult, ConflictError>;

    /// Quick check for existing JJ conflicts only
    ///
    /// This is faster than full detection when you only need to know
    /// if the workspace has existing unresolved conflicts.
    fn has_existing_conflicts(&self) -> Result<bool, ConflictError>;
}

// ============================================================================
// JJ-Based Implementation
// ============================================================================

/// JJ-based conflict detector implementation
///
/// Uses JJ commands to detect conflicts through:
/// - `jj log` with conflict template to check existing conflicts
/// - `jj resolve --list` to get list of conflicted files
/// - `jj diff --summary` to get modified files
pub struct JjConflictDetector<'a, E: JjExecutor + ?Sized> {
    executor: &'a E,
}

impl<'a, E: JjExecutor + ?Sized> JjConflictDetector<'a, E> {
    /// Create a new JJ conflict detector
    #[must_use]
    pub const fn new(executor: &'a E) -> Self {
        Self { executor }
    }

    /// Check for existing JJ conflicts in the workspace
    fn check_existing_conflicts(&self) -> Result<Vec<String>, ConflictError> {
        // Check if current revision has conflicts
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
            // Get list of conflicted files
            let resolve_output = self
                .executor
                .run(&["resolve", "--list"])
                .map_err(|e| ConflictError::StatusFailed(e.to_string()))?;

            let conflicts: Vec<String> = resolve_output
                .as_str()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    // Extract file path from resolve --list output
                    line.split_whitespace()
                        .next()
                        .unwrap_or_else(|| line.trim())
                        .to_string()
                })
                .collect();

            Ok(conflicts)
        } else {
            Ok(Vec::new())
        }
    }

    /// Find the merge base (common ancestor) between workspace and trunk
    fn find_merge_base(&self) -> Result<Option<String>, ConflictError> {
        // Find the most recent common ancestor of @ and trunk()
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

    /// Get files modified in workspace since branching from trunk
    fn get_workspace_modified_files(&self) -> Result<HashSet<String>, ConflictError> {
        let output = self
            .executor
            .run(&["diff", "--from", "trunk()", "--to", "@", "--summary"])
            .map_err(|e| ConflictError::DiffFailed(e.to_string()))?;

        Ok(Self::parse_diff_summary(output.as_str()))
    }

    /// Get files modified in trunk since the merge base
    fn get_trunk_modified_files(&self, merge_base: &str) -> Result<HashSet<String>, ConflictError> {
        let output = self
            .executor
            .run(&["diff", "--from", merge_base, "--to", "trunk()", "--summary"])
            .map_err(|e| ConflictError::DiffFailed(e.to_string()))?;

        Ok(Self::parse_diff_summary(output.as_str()))
    }

    /// Parse JJ diff --summary output to extract file paths
    ///
    /// Format: "M path/to/file" or "A path" or "D path" or "R old -> new"
    fn parse_diff_summary(output: &str) -> HashSet<String> {
        output
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }

                // Split on first whitespace to separate status from path
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() >= 2 {
                    let file_part = parts.get(1).copied().unwrap_or("");
                    // Handle rename format: "old_path -> new_path"
                    if file_part.contains(" -> ") {
                        // For renames, consider the destination file
                        file_part
                            .split(" -> ")
                            .last()
                            .map(std::string::ToString::to_string)
                    } else {
                        Some(file_part.to_string())
                    }
                } else {
                    // Fallback: use the whole line if format is unexpected
                    Some(trimmed.to_string())
                }
            })
            .collect()
    }
}

impl<E: JjExecutor + ?Sized> ConflictDetector for JjConflictDetector<'_, E> {
    fn detect_conflicts(&self) -> Result<ConflictDetectionResult, ConflictError> {
        let start = std::time::Instant::now();

        // Step 1: Check for existing conflicts
        let existing_conflicts = self.check_existing_conflicts()?;
        let has_existing = !existing_conflicts.is_empty();

        // Step 2: Find merge base
        let merge_base = self.find_merge_base()?;

        // Step 3: Get workspace modified files
        let workspace_files = self.get_workspace_modified_files()?;

        // Step 4: Get trunk modified files
        let trunk_files = if let Some(ref base) = merge_base {
            self.get_trunk_modified_files(base)?
        } else {
            // If no merge base found, compare directly to trunk
            let output = self
                .executor
                .run(&["diff", "--from", "@", "--to", "trunk()", "--summary"])
                .map_err(|e| ConflictError::DiffFailed(e.to_string()))?;
            Self::parse_diff_summary(output.as_str())
        };

        // Step 5: Compute overlapping files
        let overlapping: Vec<String> = workspace_files
            .intersection(&trunk_files)
            .cloned()
            .collect();

        let workspace_only: Vec<String> =
            workspace_files.difference(&trunk_files).cloned().collect();

        let main_only: Vec<String> = trunk_files.difference(&workspace_files).cloned().collect();

        // Step 6: Determine if merge is safe
        let merge_likely_safe = !has_existing && overlapping.is_empty();

        // Step 7: Generate summary
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

        #[allow(clippy::cast_possible_truncation)]
        let detection_time_ms = start.elapsed().as_millis() as u64;

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
            detection_time_ms,
        })
    }

    fn has_existing_conflicts(&self) -> Result<bool, ConflictError> {
        Ok(!self.check_existing_conflicts()?.is_empty())
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Run conflict detection with the given executor
///
/// This is the main entry point for conflict detection.
pub fn run_conflict_detection<E: JjExecutor + ?Sized>(
    executor: &E,
) -> Result<ConflictDetectionResult, ConflictError> {
    let detector = JjConflictDetector::new(executor);
    detector.detect_conflicts()
}

/// Quick check for existing conflicts only
#[allow(dead_code)] // Reserved for future use in quick conflict checks
pub fn has_conflicts<E: JjExecutor + ?Sized>(executor: &E) -> Result<bool, ConflictError> {
    let detector = JjConflictDetector::new(executor);
    detector.has_existing_conflicts()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConflictDetectionResult Tests ──────────────────────────────────────

    #[test]
    fn test_no_conflicts_result() {
        let result = ConflictDetectionResult::no_conflicts();
        assert!(!result.has_conflicts());
        assert!(result.merge_likely_safe);
        assert_eq!(result.conflict_count(), 0);
        assert_eq!(result.exit_code(), ConflictExitCode::Safe);
    }

    #[test]
    fn test_has_conflicts_with_overlapping() {
        let result = ConflictDetectionResult {
            overlapping_files: vec!["shared.rs".to_string()],
            ..Default::default()
        };
        assert!(result.has_conflicts());
        assert_eq!(result.conflict_count(), 1);
        assert_eq!(result.exit_code(), ConflictExitCode::Conflicts);
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
        assert_eq!(result.exit_code(), ConflictExitCode::Conflicts);
    }

    #[test]
    fn test_conflict_count_combined() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["a.rs".to_string(), "b.rs".to_string()],
            overlapping_files: vec!["c.rs".to_string()],
            ..Default::default()
        };
        assert_eq!(result.conflict_count(), 3);
    }

    // ── ConflictExitCode Tests ────────────────────────────────────────────

    #[test]
    fn test_exit_codes() {
        assert_eq!(ConflictExitCode::Safe.as_i32(), 0);
        assert_eq!(ConflictExitCode::Conflicts.as_i32(), 1);
        assert_eq!(ConflictExitCode::Error.as_i32(), 3);
    }

    // ── Diff Parsing Tests ────────────────────────────────────────────────

    #[test]
    fn test_parse_diff_summary_basic() {
        let output = "M src/lib.rs\nA src/new.rs\nD src/old.rs";
        let files =
            JjConflictDetector::<super::super::executor::RealJjExecutor>::parse_diff_summary(
                output,
            );
        assert!(files.contains("src/lib.rs"));
        assert!(files.contains("src/new.rs"));
        assert!(files.contains("src/old.rs"));
    }

    #[test]
    fn test_parse_diff_summary_with_rename() {
        let output = "R src/old_name.rs -> src/new_name.rs";
        let files =
            JjConflictDetector::<super::super::executor::RealJjExecutor>::parse_diff_summary(
                output,
            );
        // Should contain the destination (new name)
        assert!(files.contains("src/new_name.rs"));
    }

    #[test]
    fn test_parse_diff_summary_empty() {
        let output = "";
        let files =
            JjConflictDetector::<super::super::executor::RealJjExecutor>::parse_diff_summary(
                output,
            );
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_diff_summary_with_whitespace() {
        let output = "\n  M src/lib.rs  \n\nA src/new.rs\n  \n";
        let files =
            JjConflictDetector::<super::super::executor::RealJjExecutor>::parse_diff_summary(
                output,
            );
        assert_eq!(files.len(), 2);
    }

    // ── ConflictError Tests ───────────────────────────────────────────────

    #[test]
    fn test_error_display() {
        let err = ConflictError::StatusFailed("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = ConflictError::MergeBaseFailed("base error".to_string());
        assert!(err.to_string().contains("base error"));

        let err = ConflictError::DiffFailed("diff error".to_string());
        assert!(err.to_string().contains("diff error"));
    }

    #[test]
    fn test_error_from_executor() {
        let exec_err = ExecutorError::CommandFailed {
            code: 1,
            stderr: "command failed".to_string(),
        };
        let conflict_err: ConflictError = exec_err.into();
        assert!(matches!(conflict_err, ConflictError::JjFailed(_)));
    }

    // ── Text Output Tests ─────────────────────────────────────────────────

    #[test]
    fn test_text_output_safe() {
        let result = ConflictDetectionResult::no_conflicts();
        let output = result.to_text_output();
        assert!(output.contains("No conflicts detected"));
        assert!(output.contains("merge is safe"));
    }

    #[test]
    fn test_text_output_with_conflicts() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: true,
            existing_conflicts: vec!["file.rs".to_string()],
            merge_likely_safe: false,
            ..Default::default()
        };
        let output = result.to_text_output();
        assert!(output.contains("Conflicts detected"));
        assert!(output.contains("file.rs"));
        assert!(output.contains("jj resolve"));
    }

    #[test]
    fn test_text_output_with_overlapping() {
        let result = ConflictDetectionResult {
            overlapping_files: vec!["shared.rs".to_string()],
            merge_likely_safe: false,
            ..Default::default()
        };
        let output = result.to_text_output();
        assert!(output.contains("modified in both"));
        assert!(output.contains("shared.rs"));
    }

    // ── Serialization Tests ───────────────────────────────────────────────

    #[test]
    fn test_result_serialization() {
        let result = ConflictDetectionResult {
            has_existing_conflicts: false,
            existing_conflicts: vec![],
            overlapping_files: vec!["test.rs".to_string()],
            workspace_only: vec!["new.rs".to_string()],
            main_only: vec!["main.rs".to_string()],
            merge_likely_safe: false,
            summary: "Test summary".to_string(),
            merge_base: Some("abc123".to_string()),
            files_analyzed: 3,
            detection_time_ms: 42,
        };

        let json = serde_json::to_string(&result);
        assert!(json.is_ok());

        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("overlapping_files"));
        assert!(json_str.contains("test.rs"));
        assert!(json_str.contains("merge_base"));
        assert!(json_str.contains("abc123"));
    }

    #[test]
    fn test_result_deserialization() {
        let json = r#"{"has_existing_conflicts":false,"existing_conflicts":[],"overlapping_files":["test.rs"],"workspace_only":[],"main_only":[],"merge_likely_safe":false,"summary":"Test","merge_base":"abc","files_analyzed":1,"detection_time_ms":10}"#;

        let result: Result<ConflictDetectionResult, _> = serde_json::from_str(json);
        assert!(result.is_ok());

        let result = result.unwrap_or_default();
        assert_eq!(result.overlapping_files, vec!["test.rs"]);
        assert_eq!(result.merge_base, Some("abc".to_string()));
    }
}
