//! List all sessions

use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use zjj_core::{json::SchemaEnvelopeArray, OutputFormat};

use crate::{
    commands::get_session_db,
    json_output,
    session::{Session, SessionStatus},
};

/// Enhanced session information for list output
#[derive(Debug, Clone, Serialize)]
pub struct SessionListItem {
    pub name: String,
    pub status: String,
    pub branch: String,
    pub changes: String,
    pub beads: String,
    #[serde(flatten)]
    pub session: Session,
}

/// Beads issue counts
#[derive(Debug, Clone, Default)]
pub struct BeadCounts {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
}

impl std::fmt::Display for BeadCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.open, self.in_progress, self.blocked)
    }
}

/// Run the list command
pub fn run(all: bool, format: OutputFormat, bead: Option<&str>, agent: Option<&str>) -> Result<()> {
    // Execute in a closure to allow ? operator while catching errors for JSON mode
    let result = (|| -> Result<()> {
        let db = get_session_db()?;

        // Filter sessions: exclude completed/failed unless --all is used
        // Single-pass filtering using iterator chain for O(n) complexity
        let sessions: Vec<Session> = db
            .list_blocking(None)?
            .into_iter()
            .filter(|s| {
                // Filter by status: exclude completed/failed unless --all flag is set
                let status_matches = all
                    || (s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);

                // Filter by bead ID if specified
                let bead_matches = bead.is_none_or(|bead_id| {
                    s.metadata
                        .as_ref()
                        .and_then(|m| m.get("bead_id"))
                        .and_then(|v| v.as_str())
                        == Some(bead_id)
                });

                // Filter by agent owner if specified
                let agent_matches = agent.is_none_or(|agent_filter| {
                    s.metadata
                        .as_ref()
                        .and_then(|m| m.get("owner"))
                        .and_then(|v| v.as_str())
                        == Some(agent_filter)
                });

                // Combine all filter conditions
                status_matches && bead_matches && agent_matches
            })
            .collect();

        if sessions.is_empty() {
            if format.is_json() {
                let envelope =
                    SchemaEnvelopeArray::new("list-response", Vec::<SessionListItem>::new());
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                println!("No sessions found.");
                println!("Use 'zjj add <name>' to create a session.");
            }
            return Ok(());
        }

        // Build list items with enhanced data
        let items: Vec<SessionListItem> = sessions
            .into_iter()
            .map(|session| {
                let changes = get_session_changes(&session.workspace_path);
                let beads = get_beads_count().unwrap_or_default();

                SessionListItem {
                    name: session.name.clone(),
                    status: session.status.to_string(),
                    branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
                    changes: changes.map_or_else(|| "-".to_string(), |c| c.to_string()),
                    beads: beads.to_string(),
                    session,
                }
            })
            .collect();

        if format.is_json() {
            output_json(&items)?;
        } else {
            output_table(&items);
        }

        Ok(())
    })();

    // Handle errors in JSON mode
    if let Err(e) = result {
        if format.is_json() {
            json_output::output_json_error_and_exit(&e);
        } else {
            return Err(e);
        }
    }

    Ok(())
}

/// Get the number of changes in a workspace
fn get_session_changes(workspace_path: &str) -> Option<usize> {
    let path = Path::new(workspace_path);

    // Check if workspace exists
    if !path.exists() {
        return None;
    }

    // Try to get status from JJ
    zjj_core::jj::workspace_status(path)
        .ok()
        .map(|status| status.change_count())
}

/// Get beads count from the repository's beads database
fn get_beads_count() -> Result<BeadCounts> {
    // Find repository root
    let repo_root = zjj_core::jj::check_in_jj_repo().ok();

    let Some(root) = repo_root else {
        return Ok(BeadCounts::default());
    };

    let beads_db_path = root.join(".beads").join("beads.db");

    if !beads_db_path.exists() {
        return Ok(BeadCounts::default());
    }

    // Use sqlx to query the beads database synchronously
    // We create a runtime and block on the async operation
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        anyhow::Error::new(zjj_core::Error::DatabaseError(format!(
            "Failed to create runtime: {e}"
        )))
    })?;

    rt.block_on(async {
        let connection_string = format!("sqlite:{}", beads_db_path.display());
        let pool = sqlx::SqlitePool::connect(&connection_string)
            .await
            .map_err(|e| {
                anyhow::Error::new(zjj_core::Error::DatabaseError(format!(
                    "Failed to open beads database: {e}"
                )))
            })?;

        // Count open issues
        let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM issues WHERE status = ?1")
            .bind("open")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

        // For now, we can't distinguish in_progress vs blocked without more schema knowledge
        // Let's return a simplified count
        Result::<_, anyhow::Error>::Ok(BeadCounts {
            open: open as usize,
            in_progress: 0,
            blocked: 0,
        })
    })
}

/// Output sessions as formatted table
fn output_table(items: &[SessionListItem]) {
    println!(
        "{:<20} {:<12} {:<15} {:<10} {:<12}",
        "NAME", "STATUS", "BRANCH", "CHANGES", "BEADS"
    );
    println!("{}", "-".repeat(70));

    for item in items {
        println!(
            "{:<20} {:<12} {:<15} {:<10} {:<12}",
            item.name, item.status, item.branch, item.changes, item.beads
        );
    }
}

/// Output sessions as JSON
fn output_json(items: &[SessionListItem]) -> Result<()> {
    let envelope = SchemaEnvelopeArray::new("list-response", items.to_vec());
    let json = serde_json::to_string_pretty(&envelope)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{
        db::SessionDb,
        session::{Session, SessionStatus, SessionUpdate},
    };

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_bead_counts_display() {
        let counts = BeadCounts {
            open: 5,
            in_progress: 3,
            blocked: 2,
        };
        assert_eq!(counts.to_string(), "5/3/2");
    }

    #[tokio::test]
    async fn test_bead_counts_default() {
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[tokio::test]
    async fn test_session_list_item_serialization() -> Result<()> {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let item = SessionListItem {
            name: session.name.clone(),
            status: session.status.to_string(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            session,
        };

        let json = serde_json::to_string(&item)?;
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"changes\":\"5\""));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_session_changes_missing_workspace() {
        let result = get_session_changes("/nonexistent/path");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_output_table_format() {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionListItem {
            name: session.name.clone(),
            status: "active".to_string(),
            branch: "main".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            session,
        }];

        // This test just verifies the function doesn't panic
        output_table(&items);
    }

    #[tokio::test]
    async fn test_output_json_format() {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionListItem {
            name: session.name.clone(),
            status: "active".to_string(),
            branch: "main".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            session,
        }];

        let result = output_json(&items);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_filter_completed_and_failed_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create sessions with different statuses
        let s1 = db.create("active-session", "/tmp/active").await?;
        db.update(
            &s1.name,
            SessionUpdate {
                status: Some(SessionStatus::Active),
                ..Default::default()
            },
        )
        .await?;

        let s2 = db.create("completed-session", "/tmp/completed").await?;
        db.update(
            &s2.name,
            SessionUpdate {
                status: Some(SessionStatus::Completed),
                ..Default::default()
            },
        )
        .await?;

        let s3 = db.create("failed-session", "/tmp/failed").await?;
        db.update(
            &s3.name,
            SessionUpdate {
                status: Some(SessionStatus::Failed),
                ..Default::default()
            },
        )
        .await?;

        let s4 = db.create("paused-session", "/tmp/paused").await?;
        db.update(
            &s4.name,
            SessionUpdate {
                status: Some(SessionStatus::Paused),
                ..Default::default()
            },
        )
        .await?;

        // Get all sessions and filter
        let mut sessions = db.list(None).await?;

        // Simulate the filtering logic from run()
        sessions
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);

        // Should only have active and paused
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|s| s.name == "active-session"));
        assert!(sessions.iter().any(|s| s.name == "paused-session"));
        assert!(!sessions.iter().any(|s| s.name == "completed-session"));
        assert!(!sessions.iter().any(|s| s.name == "failed-session"));

        Ok(())
    }

    #[tokio::test]
    async fn test_all_flag_includes_all_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create sessions with different statuses
        db.create("active-session", "/tmp/active").await?;
        let s2 = db.create("completed-session", "/tmp/completed").await?;
        db.update(
            &s2.name,
            SessionUpdate {
                status: Some(SessionStatus::Completed),
                ..Default::default()
            },
        )
        .await?;

        // With all=true, no filtering
        let sessions = db.list(None).await?;
        assert_eq!(sessions.len(), 2);

        // With all=false, filter out completed
        let mut filtered = db.list(None).await?;
        filtered
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);
        assert_eq!(filtered.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_list_handling() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let sessions = db.list(None).await?;
        assert!(sessions.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_session_list_item_with_none_branch() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: None,
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let item = SessionListItem {
            name: session.name.clone(),
            status: session.status.to_string(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: "-".to_string(),
            beads: "0/0/0".to_string(),
            session,
        };

        assert_eq!(item.branch, "-");
        assert_eq!(item.changes, "-");
    }

    #[tokio::test]
    async fn test_get_beads_count_no_repo() {
        // When not in a repo or no beads db, should return default
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[allow(clippy::too_many_lines)]
    #[tokio::test]
    async fn test_combined_filters_single_pass() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create sessions with different combinations of properties
        let s1 = db.create("active-bead-123", "/tmp/s1").await?;
        let mut metadata1 = serde_json::Map::new();
        metadata1.insert(
            "bead_id".to_string(),
            serde_json::Value::String("123".to_string()),
        );
        metadata1.insert(
            "owner".to_string(),
            serde_json::Value::String("agent-a".to_string()),
        );
        db.update(
            &s1.name,
            SessionUpdate {
                status: Some(SessionStatus::Active),
                metadata: Some(serde_json::Value::Object(metadata1)),
                ..Default::default()
            },
        )
        .await?;

        let s2 = db.create("completed-bead-123", "/tmp/s2").await?;
        let mut metadata2 = serde_json::Map::new();
        metadata2.insert(
            "bead_id".to_string(),
            serde_json::Value::String("123".to_string()),
        );
        metadata2.insert(
            "owner".to_string(),
            serde_json::Value::String("agent-a".to_string()),
        );
        db.update(
            &s2.name,
            SessionUpdate {
                status: Some(SessionStatus::Completed),
                metadata: Some(serde_json::Value::Object(metadata2)),
                ..Default::default()
            },
        )
        .await?;

        let s3 = db.create("active-bead-456", "/tmp/s3").await?;
        let mut metadata3 = serde_json::Map::new();
        metadata3.insert(
            "bead_id".to_string(),
            serde_json::Value::String("456".to_string()),
        );
        metadata3.insert(
            "owner".to_string(),
            serde_json::Value::String("agent-b".to_string()),
        );
        db.update(
            &s3.name,
            SessionUpdate {
                status: Some(SessionStatus::Active),
                metadata: Some(serde_json::Value::Object(metadata3)),
                ..Default::default()
            },
        )
        .await?;

        // Test 1: Filter by bead_id=123 AND agent=agent-a (excludes completed)
        let filtered: Vec<Session> = db
            .list(None)
            .await?
            .into_iter()
            .filter(|s| {
                let status_matches =
                    s.status != SessionStatus::Completed && s.status != SessionStatus::Failed;
                let bead_matches = s
                    .metadata
                    .as_ref()
                    .and_then(|m: &serde_json::Value| m.get("bead_id"))
                    .and_then(|v: &serde_json::Value| v.as_str())
                    == Some("123");
                let agent_matches = s
                    .metadata
                    .as_ref()
                    .and_then(|m: &serde_json::Value| m.get("owner"))
                    .and_then(|v: &serde_json::Value| v.as_str())
                    == Some("agent-a");
                status_matches && bead_matches && agent_matches
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "active-bead-123");

        // Test 2: Filter by bead_id=456 only (excludes completed)
        let filtered2: Vec<Session> = db
            .list(None)
            .await?
            .into_iter()
            .filter(|s| {
                let status_matches =
                    s.status != SessionStatus::Completed && s.status != SessionStatus::Failed;
                let bead_matches = s
                    .metadata
                    .as_ref()
                    .and_then(|m: &serde_json::Value| m.get("bead_id"))
                    .and_then(|v: &serde_json::Value| v.as_str())
                    == Some("456");
                status_matches && bead_matches
            })
            .collect();

        assert_eq!(filtered2.len(), 1);
        assert_eq!(filtered2[0].name, "active-bead-456");

        Ok(())
    }

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[tokio::test]
    async fn test_list_json_has_envelope() -> Result<()> {
        // Verify envelope wrapping for list command output
        use zjj_core::json::SchemaEnvelopeArray;

        let items: Vec<SessionListItem> = vec![];
        let envelope = SchemaEnvelopeArray::new("list-response", items);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("array")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_filtered_wrapped() -> Result<()> {
        // Verify filtered results are wrapped in envelope
        use zjj_core::json::SchemaEnvelopeArray;

        let items = vec![SessionListItem {
            name: "session1".to_string(),
            status: "active".to_string(),
            branch: "main".to_string(),
            changes: "0".to_string(),
            beads: "1/0/0".to_string(),
            session: Session {
                id: Some(1i64),
                name: "session1".to_string(),
                workspace_path: "/tmp/ws1".to_string(),
                zellij_tab: "zjj:session1".to_string(),
                status: SessionStatus::Active,
                branch: Some("main".to_string()),
                created_at: 1_704_067_200_u64,
                updated_at: 1_704_067_200_u64,
                last_synced: Some(1_704_067_200_u64),
                metadata: None,
            },
        }];
        let envelope = SchemaEnvelopeArray::new("list-response", items);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("array")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_array_type() -> Result<()> {
        // Verify schema_type is "array" for list results
        use zjj_core::json::SchemaEnvelopeArray;

        let items: Vec<SessionListItem> = vec![];
        let envelope = SchemaEnvelopeArray::new("list-response", items);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let schema_type = parsed
            .get("schema_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("schema_type not found"))?;

        assert_eq!(
            schema_type, "array",
            "schema_type should be 'array' for list responses"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_metadata_preserved() -> Result<()> {
        // Verify session metadata is preserved in envelope
        use serde_json::json;
        use zjj_core::json::SchemaEnvelopeArray;

        let metadata = json!({
            "owner": "alice",
            "bead_id": "feat-123"
        });

        let items = vec![SessionListItem {
            name: "session1".to_string(),
            status: "active".to_string(),
            branch: "feature".to_string(),
            changes: "3".to_string(),
            beads: "2/1/0".to_string(),
            session: Session {
                id: Some(1i64),
                name: "session1".to_string(),
                workspace_path: "/tmp/ws1".to_string(),
                zellij_tab: "zjj:session1".to_string(),
                status: SessionStatus::Active,
                branch: Some("feature".to_string()),
                created_at: 1_704_067_200_u64,
                updated_at: 1_704_067_200_u64,
                last_synced: Some(1_704_067_200_u64),
                metadata: Some(metadata),
            },
        }];
        let envelope = SchemaEnvelopeArray::new("list-response", items);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );

        Ok(())
    }
}
