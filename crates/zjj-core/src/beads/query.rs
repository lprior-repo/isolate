//! Beads JSONL query operations

use std::path::Path;

use im::Vector;

use super::types::{BeadIssue, BeadsError};

/// Query beads from the workspace JSONL file
///
/// Reads all issues from `.beads/issues.jsonl`, parsing each line as a JSON object.
/// Returns empty vector if file doesn't exist (valid case for uninitialized repos).
///
/// # Errors
///
/// Returns `BeadsError::FileReadFailed` if file cannot be read.
/// Returns `BeadsError::JsonParseFailed` if any line contains invalid JSON.
pub async fn query_beads(
    workspace_path: &Path,
) -> std::result::Result<Vector<BeadIssue>, BeadsError> {
    let jsonl_path = workspace_path.join(".beads/issues.jsonl");

    // Early return if file doesn't exist (not an error)
    if !jsonl_path.exists() {
        return Ok(Vector::new());
    }

    // Read file contents
    let content =
        std::fs::read_to_string(&jsonl_path).map_err(|source| BeadsError::FileReadFailed {
            path: jsonl_path.clone(),
            source,
        })?;

    // Parse each line as JSON, filtering out empty lines
    let issues: Vector<BeadIssue> = content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str::<BeadIssue>(line).map_err(|source| BeadsError::JsonParseFailed {
                line: index.saturating_add(1),
                source,
            })
        })
        .collect::<Result<Vec<_>, BeadsError>>()?
        .into_iter()
        .collect();

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use im::vector;

    use super::*;
    use crate::beads::{
        apply_query, BeadFilter, BeadQuery, BeadSort, IssueStatus, IssueType, Priority,
        SortDirection,
    };

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
}
