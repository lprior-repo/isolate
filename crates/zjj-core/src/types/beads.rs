//! Beads issue tracking types

use serde::{Deserialize, Serialize};

use crate::{
    contracts::{ContextualHint, FieldContract, HasContract, HintType, TypeContract},
    Result,
};

/// Issue status from beads
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

/// A beads issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsIssue {
    /// Issue ID (e.g., "zjj-abc")
    pub id: String,

    /// Issue title
    pub title: String,

    /// Issue status
    pub status: IssueStatus,

    /// Priority (e.g., "P1", "P2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,

    /// Issue type (e.g., "task", "bug", "feature")
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
}

/// Summary of beads issues
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeadsSummary {
    /// Number of open issues
    pub open: usize,

    /// Number of in-progress issues
    pub in_progress: usize,

    /// Number of blocked issues
    pub blocked: usize,

    /// Number of closed issues
    pub closed: usize,
}

impl BeadsSummary {
    /// Total number of issues
    #[must_use]
    pub const fn total(&self) -> usize {
        self.open
            .saturating_add(self.in_progress)
            .saturating_add(self.blocked)
            .saturating_add(self.closed)
    }

    /// Number of active issues (open + `in_progress`)
    #[must_use]
    pub const fn active(&self) -> usize {
        self.open.saturating_add(self.in_progress)
    }

    /// Has blocking issues?
    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

impl HasContract for BeadsSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("BeadsSummary")
            .description("Summary of beads issues in a workspace")
            .field(
                "open",
                FieldContract::builder("open", "usize")
                    .required()
                    .description("Number of open issues")
                    .default("0")
                    .build(),
            )
            .field(
                "in_progress",
                FieldContract::builder("in_progress", "usize")
                    .required()
                    .description("Number of in-progress issues")
                    .default("0")
                    .build(),
            )
            .field(
                "blocked",
                FieldContract::builder("blocked", "usize")
                    .required()
                    .description("Number of blocked issues")
                    .default("0")
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::Warning,
                message: "Blocked issues prevent progress - resolve blockers first".to_string(),
                condition: Some("blocked > 0".to_string()),
                related_to: Some("blocked".to_string()),
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_summary_active() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 1,
            closed: 5,
        };

        assert_eq!(summary.total(), 11);
        assert_eq!(summary.active(), 5);
        assert!(summary.has_blockers());
    }

    #[test]
    fn test_beads_summary_no_blockers() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 0,
            closed: 5,
        };

        assert!(!summary.has_blockers());
    }
}
