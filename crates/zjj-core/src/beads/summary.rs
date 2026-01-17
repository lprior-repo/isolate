#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Beads summary aggregation functionality

use im::HashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::types::{BeadIssue, IssueStatus, IssueType};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeadsSummary {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub deferred: usize,
    pub closed: usize,
}

impl BeadsSummary {
    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn from_issues(issues: &[BeadIssue]) -> Self {
        issues.iter().fold(Self::default(), |mut acc, issue| {
            acc.total += 1;
            match issue.status {
                IssueStatus::Open => acc.open += 1,
                IssueStatus::InProgress => acc.in_progress += 1,
                IssueStatus::Blocked => acc.blocked += 1,
                IssueStatus::Deferred => acc.deferred += 1,
                IssueStatus::Closed => acc.closed += 1,
            }
            acc
        })
    }

    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

/// Create a summary of issues
#[must_use]
pub fn summarize(issues: &[BeadIssue]) -> BeadsSummary {
    BeadsSummary::from_issues(issues)
}

/// Group issues by their status
#[must_use]
pub fn group_by_status(issues: &[BeadIssue]) -> HashMap<IssueStatus, Vec<BeadIssue>> {
    issues
        .iter()
        .map(|issue| (issue.status, issue))
        .into_group_map_by(|(status, _)| *status)
        .into_iter()
        .map(|(status, issues)| {
            (
                status,
                issues
                    .into_iter()
                    .map(|(_, issue)| issue)
                    .cloned()
                    .collect(),
            )
        })
        .collect()
}

/// Group issues by their type
#[must_use]
pub fn group_by_type(issues: &[BeadIssue]) -> HashMap<Option<IssueType>, Vec<BeadIssue>> {
    issues
        .iter()
        .map(|issue| (issue.issue_type.clone(), issue))
        .into_group_map()
        .into_iter()
        .map(|(issue_type, issues)| (issue_type, issues.into_iter().cloned().collect()))
        .collect()
}

/// Count issues by status using iterator fold
#[must_use]
pub fn count_by_status(issues: &[BeadIssue]) -> HashMap<IssueStatus, usize> {
    issues
        .iter()
        .map(|issue| issue.status)
        .counts()
        .into_iter()
        .collect()
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_beads_summary_from_issues() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "In Progress".to_string(),
                status: IssueStatus::InProgress,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "4".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let summary = BeadsSummary::from_issues(&issues);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.open, 1);
        assert_eq!(summary.in_progress, 1);
        assert_eq!(summary.blocked, 1);
        assert_eq!(summary.closed, 1);
        assert_eq!(summary.active(), 2);
        assert!(summary.has_blockers());
    }

    #[test]
    fn test_summarize_function() {
        let issues = vec![BeadIssue {
            id: "1".to_string(),
            title: "Open".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        }];

        let summary = summarize(&issues);

        assert_eq!(summary.total, 1);
        assert_eq!(summary.open, 1);
    }

    #[test]
    fn test_group_by_status() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Another Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let grouped = group_by_status(&issues);

        assert_eq!(grouped.get(&IssueStatus::Open).map(Vec::len), Some(2));
        assert_eq!(grouped.get(&IssueStatus::Closed).map(Vec::len), Some(1));
    }

    #[test]
    fn test_count_by_status() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Another Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let counts = count_by_status(&issues);

        assert_eq!(counts.get(&IssueStatus::Open), Some(&2));
        assert_eq!(counts.get(&IssueStatus::Closed), Some(&1));
    }

    #[test]
    fn test_group_by_type() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Bug 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: Some(IssueType::Bug),
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Feature 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: Some(IssueType::Feature),
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let grouped = group_by_type(&issues);

        assert_eq!(grouped.get(&Some(IssueType::Bug)).map(Vec::len), Some(1));
        assert_eq!(
            grouped.get(&Some(IssueType::Feature)).map(Vec::len),
            Some(1)
        );
    }
}
