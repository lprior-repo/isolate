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

    /// Helper: Assert issues count using functional pattern
    fn assert_issue_count(
        result: &std::result::Result<Vector<BeadIssue>, BeadsError>,
        count: usize,
    ) {
        if let Ok(issues) = result {
            assert_eq!(issues.len(), count);
        }
    }

    /// Helper: Test first and second issues in collection
    fn assert_first_second_issues(issues: &Vector<BeadIssue>, first_id: &str, second_id: &str) {
        if let Some(first) = issues.iter().next() {
            assert_eq!(first.id, first_id);
        }
        if let Some(second) = issues.iter().nth(1) {
            assert_eq!(second.id, second_id);
        }
    }

    /// Helper: Setup test directory with JSONL file
    fn setup_test_dir_with_jsonl(
        content: &str,
    ) -> std::result::Result<tempfile::TempDir, std::io::Error> {
        use std::fs;
        let dir = tempfile::tempdir()?;
        let beads_dir = dir.path().join(".beads");
        fs::create_dir(&beads_dir)?;
        let jsonl_path = beads_dir.join("issues.jsonl");
        fs::write(&jsonl_path, content)?;
        Ok(dir)
    }

    #[tokio::test]
    async fn test_query_beads_empty_path() {
        let result = query_beads(std::path::Path::new("/tmp")).await;
        assert!(result.is_ok());
        assert_issue_count(&result, 0);
    }

    #[tokio::test]
    async fn test_query_beads_jsonl_parsing() {
        let test_data = r#"{"id":"test-1","title":"Test Issue","status":"open","priority":0,"issue_type":"bug","created_at":"2026-01-17T10:00:00Z","updated_at":"2026-01-17T10:00:00Z"}
{"id":"test-2","title":"Another Issue","status":"closed","priority":1,"issue_type":"feature","created_at":"2026-01-17T09:00:00Z","updated_at":"2026-01-17T09:30:00Z","closed_at":"2026-01-17T09:30:00Z"}"#;

        // Functional approach: map_err to skip test on setup failure
        if let Ok(dir) = setup_test_dir_with_jsonl(test_data) {
            let result = query_beads(dir.path()).await;
            assert!(result.is_ok());
            assert_issue_count(&result, 2);

            if let Ok(issues) = result {
                assert_first_second_issues(&issues, "test-1", "test-2");

                // Validate first issue properties
                if let Some(first) = issues.iter().next() {
                    assert_eq!(first.title, "Test Issue");
                    assert_eq!(first.status, IssueStatus::Open);
                    assert_eq!(first.priority, Some(Priority::P0));
                    assert_eq!(first.issue_type, Some(IssueType::Bug));
                }

                // Validate second issue properties
                if let Some(second) = issues.iter().nth(1) {
                    assert_eq!(second.status, IssueStatus::Closed);
                    assert!(second.closed_at.is_some());
                }
            }
        }
    }

    #[tokio::test]
    async fn test_query_beads_with_extra_fields() {
        let test_data = r#"{"id":"zjj-test","title":"Real Issue Format","description":"Test with all fields","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-17T09:00:00Z","created_by":"test","updated_at":"2026-01-17T10:00:00Z","dependencies":[{"issue_id":"zjj-test","depends_on_id":"zjj-other","type":"blocks"}],"close_reason":null}"#;

        if let Ok(dir) = setup_test_dir_with_jsonl(test_data) {
            let result = query_beads(dir.path()).await;
            assert!(result.is_ok());
            assert_issue_count(&result, 1);

            if let Ok(issues) = result {
                if let Some(issue) = issues.iter().next() {
                    assert_eq!(issue.id, "zjj-test");
                    assert_eq!(issue.title, "Real Issue Format");
                    assert_eq!(issue.description, Some("Test with all fields".to_string()));
                    assert_eq!(issue.status, IssueStatus::Open);
                    assert_eq!(issue.priority, Some(Priority::P2));
                    assert_eq!(issue.issue_type, Some(IssueType::Task));
                }
            }
        }
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
}
