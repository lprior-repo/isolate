#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Beads issue dependency and critical path analysis
//!
//! This module provides functions for analyzing issue dependencies,
//! finding blockers, blocked issues, ready work, and calculating
//! dependency graphs and critical paths.
//!
//! For other analysis operations, see:
//! - `similarity` module for duplicate detection
//! - `trending` module for temporal analysis
//! - `categorization` module for issue extraction and classification

use im::{HashMap, Vector};
use itertools::Itertools;

use super::types::BeadIssue;

/// Find all issues that are blocking other issues
///
/// Returns issues that appear in the `blocked_by` field of other issues.
#[must_use]
pub fn find_blockers(issues: &[BeadIssue]) -> Vector<BeadIssue> {
    let blocked_ids: std::collections::HashSet<_> = issues
        .iter()
        .filter(|i| i.is_blocked())
        .flat_map(|i| i.blocked_by.iter().flatten())
        .cloned()
        .collect();

    issues
        .iter()
        .filter(|i| blocked_ids.contains(&i.id))
        .cloned()
        .collect()
}

/// Find all issues that are currently blocked
///
/// Returns issues with status Blocked or non-empty `blocked_by` field.
#[must_use]
pub fn find_blocked(issues: &[BeadIssue]) -> Vector<BeadIssue> {
    issues.iter().filter(|i| i.is_blocked()).cloned().collect()
}

/// Get the dependency graph showing which issues depend on which
///
/// Returns a map where keys are issue IDs and values are lists of issues
/// that depend on that key issue.
#[must_use]
pub fn get_dependency_graph(issues: &[BeadIssue]) -> HashMap<String, Vec<String>> {
    issues
        .iter()
        .filter_map(|issue| {
            issue.depends_on.as_ref().map(|deps| {
                deps.iter()
                    .map(move |dep| (dep.as_str(), issue.id.as_str()))
            })
        })
        .flatten()
        .into_group_map()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_iter().map(str::to_string).collect()))
        .collect()
}

/// Find all issues that are ready to work on
///
/// Returns open issues that are not blocked and not blocking anything.
#[must_use]
pub fn find_ready(issues: &[BeadIssue]) -> Vector<BeadIssue> {
    let blocked_ids: std::collections::HashSet<_> = issues
        .iter()
        .filter(|i| i.is_blocked())
        .flat_map(|i| i.blocked_by.iter().flatten())
        .cloned()
        .collect();

    issues
        .iter()
        .filter(|i| i.is_open() && !blocked_ids.contains(&i.id))
        .cloned()
        .collect()
}

/// Calculate the critical path through the dependency graph
///
/// Returns the longest chain of dependent issues using depth-first search.
#[must_use]
pub fn calculate_critical_path(issues: &[BeadIssue]) -> Vector<BeadIssue> {
    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        path: &mut Vec<BeadIssue>,
        visited: &mut std::collections::HashSet<String>,
        all_issues: &[BeadIssue],
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        if let Some(issue) = all_issues.iter().find(|i| i.id == node) {
            path.push(issue.clone());
        }

        if let Some(deps) = graph.get(node) {
            // Iterative pattern acceptable: DFS recursive graph traversal
            // Functional patterns would obscure the graph algorithm
            for dep in deps {
                dfs(dep, graph, path, visited, all_issues);
            }
        }
    }

    let graph = get_dependency_graph(issues);

    // Functional approach: collect all paths and find longest
    issues
        .iter()
        .filter_map(|issue| {
            let mut path = Vec::new();
            let mut visited = std::collections::HashSet::new();
            dfs(&issue.id, &graph, &mut path, &mut visited, issues);
            (!path.is_empty()).then_some(path)
        })
        .max_by_key(std::vec::Vec::len)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

#[cfg(test)]
use super::types::IssueStatus;
#[cfg(test)]
use im::vector;

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_find_blockers() {
        let issues = vec![
            BeadIssue {
                id: "blocker".to_string(),
                title: "Blocker".to_string(),
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
                id: "blocked".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vector!["blocker".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "unrelated".to_string(),
                title: "Unrelated".to_string(),
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

        let blockers = find_blockers(&issues);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].id, "blocker");
    }

    #[test]
    fn test_find_blocked() {
        let issues = vec![
            BeadIssue {
                id: "open".to_string(),
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
                id: "blocked".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vector!["other".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let blocked = find_blocked(&issues);

        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].id, "blocked");
    }

    #[test]
    fn test_find_ready() {
        let issues = vec![
            BeadIssue {
                id: "ready".to_string(),
                title: "Ready".to_string(),
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
                id: "blocked".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vector!["other".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "in-progress".to_string(),
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
        ];

        let ready = find_ready(&issues);

        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|i| i.id == "ready"));
        assert!(ready.iter().any(|i| i.id == "in-progress"));
        assert!(!ready.iter().any(|i| i.id == "blocked"));
    }

    #[test]
    fn test_get_dependency_graph() {
        let issues = vec![
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
                depends_on: Some(vector!["2".to_string()]),
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
        ];

        let graph = get_dependency_graph(&issues);

        assert!(graph
            .get("2")
            .map(|v| v.contains(&"1".to_string()))
            .unwrap_or(false));
    }
}
