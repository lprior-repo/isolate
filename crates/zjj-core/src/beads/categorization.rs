#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Issue categorization and extraction utilities
//!
//! This module provides operations for extracting, retrieving, and organizing
//! issues by various criteria including IDs, titles, and labels.

use im::Vector;
use itertools::Itertools;

use super::types::BeadIssue;

/// Get a single issue by ID
///
/// Returns `None` if no issue with the given ID exists.
///
/// # Arguments
///
/// * `issues` - Slice of issues to search
/// * `id` - The issue ID to find
///
/// # Example
///
/// ```ignore
/// let issues = vec![/* BeadIssue instances */];
/// if let Some(issue) = get_issue(&issues, "zjj-001") {
///     println!("Found: {}", issue.title);
/// }
/// ```
pub fn get_issue(issues: &[BeadIssue], id: &str) -> Option<BeadIssue> {
    issues.iter().find(|i| i.id == id).cloned()
}

/// Get multiple issues by their IDs
///
/// Returns all issues whose IDs are in the provided list, in no guaranteed order.
///
/// # Arguments
///
/// * `issues` - Slice of issues to search
/// * `ids` - List of issue IDs to retrieve
///
/// # Example
///
/// ```ignore
/// let ids = vec!["zjj-001".to_string(), "zjj-002".to_string()];
/// let results = get_issues_by_id(&issues, &ids);
/// ```
#[must_use]
pub fn get_issues_by_id(issues: &[BeadIssue], ids: &[String]) -> Vector<BeadIssue> {
    let id_set: std::collections::HashSet<_> = ids.iter().collect();
    issues
        .iter()
        .filter(|i| id_set.contains(&i.id))
        .cloned()
        .collect()
}

/// Extract all issue IDs from a collection
///
/// Returns a vector of issue IDs in order.
///
/// # Arguments
///
/// * `issues` - Slice of issues to extract IDs from
///
/// # Example
///
/// ```ignore
/// let ids = to_ids(&issues);
/// ```
#[must_use]
pub fn to_ids(issues: &[BeadIssue]) -> Vector<String> {
    issues.iter().map(|i| i.id.as_str().to_string()).collect()
}

/// Extract all issue titles from a collection
///
/// Returns a vector of issue titles in order.
///
/// # Arguments
///
/// * `issues` - Slice of issues to extract titles from
///
/// # Example
///
/// ```ignore
/// let titles = to_titles(&issues);
/// ```
#[must_use]
pub fn to_titles(issues: &[BeadIssue]) -> Vector<String> {
    issues
        .iter()
        .map(|i| i.title.as_str().to_string())
        .collect()
}

/// Extract all unique labels from a collection of issues
///
/// Returns a vector of unique labels across all issues.
///
/// # Arguments
///
/// * `issues` - Slice of issues to extract labels from
///
/// # Example
///
/// ```ignore
/// let labels = extract_labels(&issues);
/// ```
#[must_use]
pub fn extract_labels(issues: &[BeadIssue]) -> Vector<String> {
    issues
        .iter()
        .filter_map(|i| i.labels.as_ref())
        .flatten()
        .unique()
        .cloned()
        .collect()
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use chrono::Utc;
    use im::vector;

    use super::*;

    fn create_test_issue(id: &str, title: &str) -> BeadIssue {
        BeadIssue {
            id: id.to_string(),
            title: title.to_string(),
            status: super::super::types::IssueStatus::Open,
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
        }
    }

    #[test]
    fn test_get_issue() {
        let issues = vec![
            create_test_issue("zjj-001", "Issue 1"),
            create_test_issue("zjj-002", "Issue 2"),
        ];

        let found = get_issue(&issues, "zjj-001");
        let not_found = get_issue(&issues, "nonexistent");

        assert!(found.is_some(), "Should find issue zjj-001");
        if let Some(found_issue) = found {
            assert_eq!(found_issue.id, "zjj-001");
        }
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_issues_by_id() {
        let issues = vec![
            create_test_issue("1", "Issue 1"),
            create_test_issue("2", "Issue 2"),
            create_test_issue("3", "Issue 3"),
        ];

        let ids = vec!["1".to_string(), "3".to_string()];
        let result = get_issues_by_id(&issues, &ids);

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|i| i.id == "1"));
        assert!(result.iter().any(|i| i.id == "3"));
    }

    #[test]
    fn test_to_ids() {
        let issues = vec![
            create_test_issue("1", "Issue 1"),
            create_test_issue("2", "Issue 2"),
        ];

        let ids = to_ids(&issues);

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"1".to_string()));
        assert!(ids.contains(&"2".to_string()));
    }

    #[test]
    fn test_to_titles() {
        let issues = vec![
            create_test_issue("1", "First Issue"),
            create_test_issue("2", "Second Issue"),
        ];

        let titles = to_titles(&issues);

        assert_eq!(titles.len(), 2);
        assert!(titles.contains(&"First Issue".to_string()));
        assert!(titles.contains(&"Second Issue".to_string()));
    }

    #[test]
    fn test_extract_labels() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
                status: super::super::types::IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: Some(vector!["urgent".to_string(), "bug".to_string()]),
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
                status: super::super::types::IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: Some(vector!["urgent".to_string(), "feature".to_string()]),
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let labels = extract_labels(&issues);

        assert_eq!(labels.len(), 3);
        assert!(labels.contains(&"urgent".to_string()));
        assert!(labels.contains(&"bug".to_string()));
        assert!(labels.contains(&"feature".to_string()));
    }

    #[test]
    fn test_extract_labels_empty() {
        let issues = vec![create_test_issue("1", "Issue 1")];

        let labels = extract_labels(&issues);

        assert_eq!(labels.len(), 0);
    }
}
