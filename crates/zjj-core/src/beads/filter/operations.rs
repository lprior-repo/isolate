#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Complex filtering operations for beads
//!
//! This module provides the main operations for filtering, sorting, and paginating
//! bead issues. These are higher-level functions that compose predicates into
//! complete query pipelines.

use std::cmp::Reverse;

use chrono::DateTime;
use chrono::Utc;
use im::Vector;
use itertools::Itertools;
use tap::Pipe;

use crate::beads::types::BeadIssue;

use super::predicates::matches_filter;
use super::{BeadFilter, BeadQuery, BeadSort, SortDirection};

/// Extract sort key from issue based on sort field
fn extract_sort_key(issue: &BeadIssue, sort: BeadSort) -> SortKey {
    match sort {
        BeadSort::Priority => {
            // Invert priority so P0 (most important) sorts before P4 (least important)
            // P0 → 0 → inverted to 4
            // P4 → 4 → inverted to 0
            // No priority → 5 (unchanged, sorts last)
            SortKey::Priority(
                issue
                    .priority
                    .map_or(5, |p| 4_u32.saturating_sub(p.to_u32())),
                issue.updated_at,
            )
        }
        BeadSort::Created => SortKey::DateTime(issue.created_at),
        BeadSort::Updated => SortKey::DateTime(issue.updated_at),
        BeadSort::Closed => SortKey::OptionalDateTime(issue.closed_at),
        BeadSort::Status => SortKey::Status(issue.status),
        BeadSort::Title => SortKey::Text(issue.title.to_lowercase()),
        BeadSort::Id => SortKey::Text(issue.id.to_lowercase()),
    }
}

/// Unified sort key type for type-safe sorting
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SortKey {
    /// Priority with `updated_at` for tiebreaking
    Priority(u32, DateTime<Utc>),
    /// Simple timestamp
    DateTime(DateTime<Utc>),
    /// Optional timestamp (`closed_at`)
    OptionalDateTime(Option<DateTime<Utc>>),
    /// Issue status
    Status(IssueStatus),
    /// Lowercase text for case-insensitive sorting
    Text(String),
}

/// Apply sort direction to sort key
fn apply_direction(direction: SortDirection) -> impl Fn(SortKey) -> SortKeyWithDirection {
    move |key| match direction {
        SortDirection::Asc => SortKeyWithDirection::Asc(key),
        SortDirection::Desc => SortKeyWithDirection::Desc(Reverse(key)),
    }
}

/// Sort key with direction applied
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SortKeyWithDirection {
    Asc(SortKey),
    Desc(Reverse<SortKey>),
}

/// Filter issues by criteria
#[must_use]
pub fn filter_issues(issues: &Vector<BeadIssue>, filter: &BeadFilter) -> Vector<BeadIssue> {
    issues
        .iter()
        .filter(|issue| matches_filter(issue, filter))
        .cloned()
        .collect()
}

/// Sort issues by field and direction using functional approach
#[must_use]
pub fn sort_issues(
    issues: &Vector<BeadIssue>,
    sort: BeadSort,
    direction: SortDirection,
) -> Vector<BeadIssue> {
    let direction_fn = apply_direction(direction);

    issues
        .iter()
        .map(|issue| (issue, direction_fn(extract_sort_key(issue, sort))))
        .sorted_by(|(_, key_a), (_, key_b)| key_a.cmp(key_b))
        .map(|(issue, _)| issue)
        .cloned()
        .collect()
}

/// Paginate issues with offset and limit
#[must_use]
pub fn paginate(
    issues: &Vector<BeadIssue>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Vector<BeadIssue> {
    issues
        .iter()
        .skip(offset.unwrap_or(0))
        .take(limit.unwrap_or_else(|| issues.len()))
        .cloned()
        .collect()
}

/// Apply complete query: filter, sort, and paginate
#[must_use]
pub fn apply_query(issues: &Vector<BeadIssue>, query: &BeadQuery) -> Vector<BeadIssue> {
    issues
        .pipe(|i| filter_issues(i, &query.filter))
        .pipe(|i| sort_issues(&i, query.sort, query.direction))
        .pipe(|i| paginate(&i, query.filter.offset, query.filter.limit))
}

/// Check if any issue matches the filter
#[must_use]
pub fn any_match(issues: &Vector<BeadIssue>, filter: &BeadFilter) -> bool {
    issues.iter().any(|i| matches_filter(i, filter))
}

/// Check if all issues match the filter
#[must_use]
pub fn all_match(issues: &Vector<BeadIssue>, filter: &BeadFilter) -> bool {
    issues.iter().all(|i| matches_filter(i, filter))
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use super::*;
    use crate::beads::types::IssueType;
    use chrono::Utc;
    use im::vector;

    #[test]
    fn test_filter_issues_by_status() {
        let issues = vector![
            BeadIssue {
                id: "1".to_string(),
                title: "Open Issue".to_string(),
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
                title: "Closed Issue".to_string(),
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

        let filter = BeadFilter::new().with_status(IssueStatus::Open);
        let filtered = filter_issues(&issues, &filter);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn test_sort_issues_by_priority() {
        let issues = vector![
            BeadIssue {
                id: "p3".to_string(),
                title: "P3".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P3),
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
                id: "p0".to_string(),
                title: "P0".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P0),
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
                id: "p2".to_string(),
                title: "P2".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P2),
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
        ];

        let sorted = sort_issues(&issues, BeadSort::Priority, SortDirection::Desc);

        assert_eq!(sorted[0].id, "p0");
        assert_eq!(sorted[1].id, "p2");
        assert_eq!(sorted[2].id, "p3");
    }

    #[test]
    fn test_paginate() {
        let issues = vector![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
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
                title: "Issue 2".to_string(),
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
                title: "Issue 3".to_string(),
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
        ];

        let page = paginate(&issues, Some(1), Some(1));

        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, "2");
    }

    #[test]
    fn test_apply_query() {
        let issues = vector![
            BeadIssue {
                id: "1".to_string(),
                title: "Open Bug".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P0),
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
                title: "Open Feature".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P1),
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
            BeadIssue {
                id: "3".to_string(),
                title: "Closed Bug".to_string(),
                status: IssueStatus::Closed,
                priority: Some(Priority::P2),
                issue_type: Some(IssueType::Bug),
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

        let query = BeadQuery::new()
            .filter(BeadFilter::new().with_type(IssueType::Bug))
            .sort_by(BeadSort::Priority)
            .direction(SortDirection::Desc);

        let result = apply_query(&issues, &query);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "1");
        assert_eq!(result[1].id, "3");
    }

    #[test]
    fn test_any_match() {
        let issues = vector![
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

        let open_filter = BeadFilter::new().with_status(IssueStatus::Open);
        let bug_filter = BeadFilter::new().with_type(IssueType::Bug);

        assert!(any_match(&issues, &open_filter));
        assert!(!any_match(&issues, &bug_filter));
    }

    #[test]
    fn test_all_match() {
        let issues = vector![
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
                title: "Open Too".to_string(),
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
        ];

        let open_filter = BeadFilter::new().with_status(IssueStatus::Open);
        let closed_filter = BeadFilter::new().with_status(IssueStatus::Closed);

        assert!(all_match(&issues, &open_filter));
        assert!(!all_match(&issues, &closed_filter));
    }
}
