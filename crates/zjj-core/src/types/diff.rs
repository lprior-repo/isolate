//! Diff statistics and summary types

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    contracts::{Constraint, FieldContract, HasContract, TypeContract},
    Error, Result,
};

use super::changes::FileStatus;

/// Diff statistics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffStat {
    /// File path
    pub path: PathBuf,

    /// Lines inserted
    pub insertions: usize,

    /// Lines deleted
    pub deletions: usize,

    /// File status (`A`/`M`/`D`/`R`)
    pub status: FileStatus,
}

/// Summary of diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of lines inserted
    pub insertions: usize,

    /// Number of lines deleted
    pub deletions: usize,

    /// Number of files changed
    pub files_changed: usize,

    /// Per-file statistics
    pub files: Vec<FileDiffStat>,
}

impl HasContract for DiffSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("DiffSummary")
            .description("Summary of differences between commits or workspace state")
            .field(
                "insertions",
                FieldContract::builder("insertions", "usize")
                    .required()
                    .description("Total number of lines inserted")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .field(
                "deletions",
                FieldContract::builder("deletions", "usize")
                    .required()
                    .description("Total number of lines deleted")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .field(
                "files_changed",
                FieldContract::builder("files_changed", "usize")
                    .required()
                    .description("Number of files changed")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .build()
    }

    fn validate(&self) -> Result<()> {
        if self.files.len() != self.files_changed {
            return Err(Error::validation_error(format!(
                "files_changed ({}) does not match files array length ({})",
                self.files_changed,
                self.files.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_summary_validation() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 2,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("file1.txt"),
                    insertions: 5,
                    deletions: 2,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("file2.txt"),
                    insertions: 5,
                    deletions: 3,
                    status: FileStatus::Added,
                },
            ],
        };

        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_mismatch() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 5, // Mismatch!
            files: vec![FileDiffStat {
                path: PathBuf::from("file1.txt"),
                insertions: 5,
                deletions: 2,
                status: FileStatus::Modified,
            }],
        };

        assert!(diff.validate().is_err());
    }
}
