#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Similarity detection for identifying potential duplicate issues
//!
//! This module provides algorithms for finding potentially duplicate issues
//! based on title similarity using word intersection analysis.

use im::Vector;

use super::types::BeadIssue;

/// Find potential duplicate issues based on title similarity
///
/// Returns pairs of (issue, `similar_issues`) where similar issues share
/// at least `threshold` words in their titles.
///
/// Uses a word intersection algorithm: splits titles into words, compares
/// word sets, and counts matches. This preserves the complex similarity
/// detection logic from the original analysis module.
///
/// # Arguments
///
/// * `issues` - Slice of issues to compare
/// * `threshold` - Minimum number of matching words to consider issues similar
///
/// # Example
///
/// ```ignore
/// let issues = vec![/* BeadIssue instances */];
/// let duplicates = find_potential_duplicates(&issues, 2);
/// ```
#[must_use]
pub fn find_potential_duplicates(
    issues: &[BeadIssue],
    threshold: usize,
) -> Vector<(BeadIssue, Vector<BeadIssue>)> {
    issues
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < issues.len().saturating_sub(1))
        .filter_map(|(i, issue)| {
            #[allow(clippy::arithmetic_side_effects)]
            let similar: Vector<BeadIssue> = issues
                .iter()
                .skip(i + 1)
                .filter(|other| {
                    let self_words: std::collections::HashSet<_> =
                        issue.title.split_whitespace().collect();
                    let other_words: std::collections::HashSet<_> =
                        other.title.split_whitespace().collect();
                    self_words.intersection(&other_words).count() >= threshold
                })
                .cloned()
                .collect();

            (!similar.is_empty()).then_some((issue.clone(), similar))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use chrono::Utc;

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
    fn test_find_potential_duplicates() {
        let issues = vec![
            create_test_issue("1", "Fix login button styling"),
            create_test_issue("2", "Fix styling of login button"),
            create_test_issue("3", "Add user profile page"),
        ];

        let duplicates = find_potential_duplicates(&issues, 2);

        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].0.id, "1");
        assert_eq!(duplicates[0].1.len(), 1);
        assert_eq!(duplicates[0].1[0].id, "2");
    }

    #[test]
    fn test_find_potential_duplicates_threshold() {
        let issues = vec![
            create_test_issue("1", "Add feature A"),
            create_test_issue("2", "Add feature B"),
        ];

        let duplicates = find_potential_duplicates(&issues, 2);

        assert_eq!(duplicates.len(), 1);
    }

    #[test]
    fn test_find_potential_duplicates_no_matches() {
        let issues = vec![
            create_test_issue("1", "Fix login bug"),
            create_test_issue("2", "Add profile page"),
        ];

        let duplicates = find_potential_duplicates(&issues, 3);

        assert_eq!(duplicates.len(), 0);
    }
}
