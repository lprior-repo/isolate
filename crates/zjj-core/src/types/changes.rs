//! File change tracking types

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    contracts::{Constraint, ContextualHint, FieldContract, HasContract, HintType, TypeContract},
    Error, Result,
};

/// File modification status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileStatus {
    /// File modified
    #[serde(rename = "M")]
    Modified,
    /// File added
    #[serde(rename = "A")]
    Added,
    /// File deleted
    #[serde(rename = "D")]
    Deleted,
    /// File renamed
    #[serde(rename = "R")]
    Renamed,
    /// File untracked
    #[serde(rename = "?")]
    Untracked,
}

/// A single file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// File path relative to workspace root
    pub path: PathBuf,

    /// Modification status
    pub status: FileStatus,

    /// Original path (only for `Renamed`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
}

impl HasContract for FileChange {
    fn contract() -> TypeContract {
        TypeContract::builder("FileChange")
            .description("Represents a change to a file in the workspace")
            .field(
                "path",
                FieldContract::builder("path", "PathBuf")
                    .required()
                    .description("File path relative to workspace root")
                    .build(),
            )
            .field(
                "status",
                FieldContract::builder("status", "FileStatus")
                    .required()
                    .description("Type of modification")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "M".to_string(),
                            "A".to_string(),
                            "D".to_string(),
                            "R".to_string(),
                            "?".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "old_path",
                FieldContract::builder("old_path", "Option<PathBuf>")
                    .description("Original path for renamed files")
                    .constraint(Constraint::Custom {
                        rule: "required when status is Renamed".to_string(),
                        description: "Must be set when file is renamed".to_string(),
                    })
                    .build(),
            )
            .build()
    }

    fn validate(&self) -> Result<()> {
        if self.status == FileStatus::Renamed && self.old_path.is_none() {
            return Err(Error::validation_error(
                "Renamed files must have old_path set".to_string(),
            ));
        }
        Ok(())
    }
}

/// Summary of changes in a workspace
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangesSummary {
    /// Number of modified files
    pub modified: usize,

    /// Number of added files
    pub added: usize,

    /// Number of deleted files
    pub deleted: usize,

    /// Number of renamed files
    pub renamed: usize,

    /// Number of untracked files
    pub untracked: usize,
}

impl ChangesSummary {
    /// Total number of changed files
    #[must_use]
    pub const fn total(&self) -> usize {
        self.modified
            .saturating_add(self.added)
            .saturating_add(self.deleted)
            .saturating_add(self.renamed)
    }

    /// Has any changes?
    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.total() > 0
    }

    /// Has any tracked changes (excluding untracked)?
    #[must_use]
    pub const fn has_tracked_changes(&self) -> bool {
        self.modified
            .saturating_add(self.added)
            .saturating_add(self.deleted)
            .saturating_add(self.renamed)
            > 0
    }
}

impl HasContract for ChangesSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("ChangesSummary")
            .description("Summary of file changes in a workspace")
            .field(
                "modified",
                FieldContract::builder("modified", "usize")
                    .required()
                    .description("Number of modified files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .field(
                "added",
                FieldContract::builder("added", "usize")
                    .required()
                    .description("Number of added files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .field(
                "deleted",
                FieldContract::builder("deleted", "usize")
                    .required()
                    .description("Number of deleted files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::Example,
                message: "Use total() method to get sum of all changes".to_string(),
                condition: None,
                related_to: None,
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        // All fields are usize, so always valid
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changes_summary_total() {
        let summary = ChangesSummary {
            modified: 5,
            added: 3,
            deleted: 2,
            renamed: 1,
            untracked: 4,
        };

        assert_eq!(summary.total(), 11);
        assert!(summary.has_changes());
        assert!(summary.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_no_changes() {
        let summary = ChangesSummary::default();
        assert_eq!(summary.total(), 0);
        assert!(!summary.has_changes());
    }

    #[test]
    fn test_file_change_renamed_validation() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: None, // Missing old_path!
        };

        assert!(change.validate().is_err());
    }

    #[test]
    fn test_file_change_renamed_valid() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("old/path.txt")),
        };

        assert!(change.validate().is_ok());
    }
}
