#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Temporal analysis and trending for issue tracking
//!
//! This module provides time-based analysis of issues, including staleness
//! detection and temporal trend analysis.

use chrono::Utc;
use im::Vector;

use super::types::{BeadIssue, IssueStatus};

/// Find issues that haven't been updated in the specified number of days
///
/// Returns issues with `updated_at` older than the cutoff and not closed.
/// This function helps identify stale or abandoned work items that may need
/// attention or cleanup.
///
/// # Arguments
///
/// * `issues` - Slice of issues to analyze
/// * `days` - Number of days of inactivity to consider an issue stale
///
/// # Example
///
/// ```ignore
/// let issues = vec![/* BeadIssue instances */];
/// let stale = find_stale(&issues, 7);  // Find issues not updated in 7+ days
/// ```
#[must_use]
#[allow(clippy::arithmetic_side_effects, clippy::cast_possible_wrap)]
pub fn find_stale(issues: &[BeadIssue], days: u64) -> Vector<BeadIssue> {
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);

    issues
        .iter()
        .filter(|i| i.updated_at < cutoff && i.status != IssueStatus::Closed)
        .cloned()
        .collect()
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn create_issue_with_update(id: &str, title: &str, days_ago: i64) -> BeadIssue {
        let updated_at = if days_ago > 0 {
            Utc::now() - chrono::Duration::days(days_ago)
        } else {
            Utc::now()
        };

        BeadIssue {
            id: id.to_string(),
            title: title.to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: updated_at,
            updated_at,
            closed_at: None,
        }
    }

    #[test]
    fn test_find_stale() {
        let recent = create_issue_with_update("recent", "Recent", 0);
        let stale = create_issue_with_update("stale", "Stale", 30);

        let issues = vec![recent.clone(), stale.clone()];
        let stale_issues = find_stale(&issues, 7);

        assert_eq!(stale_issues.len(), 1);
        assert_eq!(stale_issues[0].id, "stale");
    }

    #[test]
    fn test_find_stale_ignores_closed() {
        let closed = BeadIssue {
            id: "closed".to_string(),
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
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now() - chrono::Duration::days(30),
            closed_at: Some(Utc::now() - chrono::Duration::days(30)),
        };

        let issues = vec![closed];
        let stale_issues = find_stale(&issues, 7);

        assert_eq!(stale_issues.len(), 0);
    }

    #[test]
    fn test_find_stale_boundary() {
        let just_stale = create_issue_with_update("just_stale", "Just Stale", 7);
        let not_stale = create_issue_with_update("not_stale", "Not Stale", 6);

        let issues = vec![just_stale.clone(), not_stale.clone()];
        let stale_issues = find_stale(&issues, 6);

        assert_eq!(stale_issues.len(), 1);
        assert_eq!(stale_issues[0].id, "just_stale");
    }
}
