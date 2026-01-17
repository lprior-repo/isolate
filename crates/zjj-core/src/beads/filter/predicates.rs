#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Predicate functions for filtering beads
//!
//! This module contains all the individual filter predicates that check
//! if a bead issue matches specific filter criteria.

use crate::beads::types::BeadIssue;

use super::BeadFilter;

/// Check if an issue matches filter criteria (main coordinator)
pub(super) fn matches_filter(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    matches_status(issue, filter)
        && matches_issue_type(issue, filter)
        && matches_priority_range(issue, filter)
        && matches_labels(issue, filter)
        && matches_assignee(issue, filter)
        && matches_parent(issue, filter)
        && matches_blocked(issue, filter)
        && matches_search(issue, filter)
}

/// Check if issue matches status filter
pub(super) fn matches_status(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter.status.is_empty() || filter.status.contains(&issue.status)
}

/// Check if issue matches type filter
pub(super) fn matches_issue_type(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter.issue_type.is_empty()
        || issue
            .issue_type
            .as_ref()
            .is_some_and(|t| filter.issue_type.contains(t))
}

/// Check if issue matches priority range filter
pub(super) fn matches_priority_range(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    matches_min_priority(issue, filter) && matches_max_priority(issue, filter)
}

/// Check if issue meets minimum priority
pub(super) fn matches_min_priority(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter
        .priority_min
        .is_none_or(|min| issue.priority.is_none_or(|p| p >= min))
}

/// Check if issue meets maximum priority
pub(super) fn matches_max_priority(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter
        .priority_max
        .is_none_or(|max| issue.priority.is_none_or(|p| p <= max))
}

/// Check if issue matches labels filter
pub(super) fn matches_labels(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter.labels.is_empty()
        || issue
            .labels
            .as_ref()
            .is_some_and(|issue_labels| filter.labels.iter().all(|l| issue_labels.contains(l)))
}

/// Check if issue matches assignee filter
pub(super) fn matches_assignee(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter
        .assignee
        .as_ref()
        .is_none_or(|assignee| issue.assignee.as_ref().is_some_and(|a| a == assignee))
}

/// Check if issue matches parent filter
pub(super) fn matches_parent(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    let matches_parent_id = filter
        .parent
        .as_ref()
        .is_none_or(|parent| issue.parent.as_ref().is_some_and(|p| p == parent));

    let matches_has_parent = !filter.has_parent || issue.parent.is_some();

    matches_parent_id && matches_has_parent
}

/// Check if issue matches blocked filter
pub(super) fn matches_blocked(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    !filter.blocked_only || issue.is_blocked()
}

/// Check if issue matches search text filter
pub(super) fn matches_search(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    filter
        .search_text
        .as_ref()
        .is_none_or(|text| search_matches_issue(text, issue))
}

/// Check if search text matches issue title or description
pub(super) fn search_matches_issue(text: &str, issue: &BeadIssue) -> bool {
    let text_lower = text.to_lowercase();
    search_matches_title(&text_lower, issue) || search_matches_description(&text_lower, issue)
}

/// Check if search text matches issue title
pub(super) fn search_matches_title(text_lower: &str, issue: &BeadIssue) -> bool {
    issue.title.to_lowercase().contains(text_lower)
}

/// Check if search text matches issue description
pub(super) fn search_matches_description(text_lower: &str, issue: &BeadIssue) -> bool {
    issue
        .description
        .as_ref()
        .is_some_and(|d| d.to_lowercase().contains(text_lower))
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_matches_status() {
        let issue = BeadIssue {
            id: "1".to_string(),
            title: "Test".to_string(),
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
        };

        let filter_open = BeadFilter::new().with_status(IssueStatus::Open);
        let filter_closed = BeadFilter::new().with_status(IssueStatus::Closed);
        let filter_empty = BeadFilter::new();

        assert!(matches_status(&issue, &filter_open));
        assert!(!matches_status(&issue, &filter_closed));
        assert!(matches_status(&issue, &filter_empty));
    }

    #[test]
    fn test_search_matches_title() {
        let issue = BeadIssue {
            id: "1".to_string(),
            title: "Fix Bug in Parser".to_string(),
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
        };

        assert!(search_matches_title("bug", &issue));
        assert!(search_matches_title("BUG", &issue));
        assert!(!search_matches_title("feature", &issue));
    }
}
