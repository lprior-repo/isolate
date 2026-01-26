//! Show detailed session status

use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use zjj_core::json::SchemaEnvelope;

use crate::{commands::get_session_db, session::Session};

/// Detailed session status information
#[derive(Debug, Clone, Serialize)]
pub struct SessionStatusInfo {
    pub name: String,
    pub status: String,
    pub workspace_path: String,
    pub branch: String,
    pub changes: FileChanges,
    pub diff_stats: DiffStats,
    pub beads: BeadStats,
    #[serde(flatten)]
    pub session: Session,
}

/// File changes in the workspace
#[derive(Debug, Clone, Default, Serialize)]
pub struct FileChanges {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub unknown: usize,
}

impl FileChanges {
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    pub const fn is_clean(&self) -> bool {
        self.total() == 0
    }
}

impl std::fmt::Display for FileChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_clean() {
            write!(f, "clean")
        } else {
            write!(
                f,
                "M:{} A:{} D:{} R:{}",
                self.modified, self.added, self.deleted, self.renamed
            )
        }
    }
}

/// Diff statistics (insertions/deletions)
#[derive(Debug, Clone, Default, Serialize)]
pub struct DiffStats {
    pub insertions: usize,
    pub deletions: usize,
}

impl std::fmt::Display for DiffStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.insertions, self.deletions)
    }
}

/// Beads issue statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct BeadStats {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub closed: usize,
}

impl std::fmt::Display for BeadStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "O:{} P:{} B:{} C:{}",
            self.open, self.in_progress, self.blocked, self.closed
        )
    }
}

/// Run the status command
pub fn run(name: Option<&str>, json: bool, watch: bool) -> Result<()> {
    if watch {
        run_watch_mode(name, json)
    } else {
        run_once(name, json)
    }
}

/// Run status once
fn run_once(name: Option<&str>, json: bool) -> Result<()> {
    let db = get_session_db()?;

    let sessions = if let Some(session_name) = name {
        // Get single session
        // Return zjj_core::Error::NotFound to get exit code 2 (not found)
        let session = db.get(session_name)?.ok_or_else(|| {
            anyhow::Error::new(zjj_core::Error::NotFound(format!(
                "Session '{session_name}' not found"
            )))
        })?;
        vec![session]
    } else {
        // Get all sessions
        db.list(None)?
    };

    if sessions.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No sessions found.");
            println!("Use 'zjj add <name>' to create a session.");
        }
        return Ok(());
    }

    // Gather status for all sessions
    let statuses: Vec<SessionStatusInfo> = sessions
        .into_iter()
        .map(|session| gather_session_status(&session))
        .collect::<Result<Vec<_>>>()?;

    if json {
        output_json(&statuses)?;
    } else {
        output_table(&statuses);
    }

    Ok(())
}

/// Run status in watch mode (continuous updates)
fn run_watch_mode(name: Option<&str>, json: bool) -> Result<()> {
    use std::{io::Write, thread, time::Duration};

    loop {
        // Clear screen (ANSI escape code)
        if !json {
            print!("\x1B[2J\x1B[1;1H");
            std::io::stdout().flush()?;
        }

        // Run status once
        if let Err(e) = run_once(name, json) {
            if !json {
                eprintln!("Error: {e}");
            }
        }

        // Wait 1 second
        thread::sleep(Duration::from_secs(1));
    }
}

/// Gather detailed status for a session
fn gather_session_status(session: &Session) -> Result<SessionStatusInfo> {
    let workspace_path = Path::new(&session.workspace_path);

    // Get file changes
    let changes = get_file_changes(workspace_path);

    // Get diff stats
    let diff_stats = get_diff_stats(workspace_path);

    // Get beads stats
    let beads = get_beads_stats()?;

    Ok(SessionStatusInfo {
        name: session.name.clone(),
        status: session.status.to_string(),
        workspace_path: session.workspace_path.clone(),
        branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
        changes,
        diff_stats,
        beads,
        session: session.clone(),
    })
}

/// Get file changes from JJ status
fn get_file_changes(workspace_path: &Path) -> FileChanges {
    if !workspace_path.exists() {
        return FileChanges::default();
    }

    match zjj_core::jj::workspace_status(workspace_path) {
        Ok(status) => FileChanges {
            modified: status.modified.len(),
            added: status.added.len(),
            deleted: status.deleted.len(),
            renamed: status.renamed.len(),
            unknown: status.unknown.len(),
        },
        Err(_) => FileChanges::default(),
    }
}

/// Get diff statistics from JJ diff
fn get_diff_stats(workspace_path: &Path) -> DiffStats {
    if !workspace_path.exists() {
        return DiffStats::default();
    }

    zjj_core::jj::workspace_diff(workspace_path)
        .map(|summary| DiffStats {
            insertions: summary.insertions,
            deletions: summary.deletions,
        })
        .unwrap_or_default()
}

/// Get beads statistics from the repository's beads database
fn get_beads_stats() -> Result<BeadStats> {
    use rusqlite::Connection;

    // Find repository root
    let repo_root = zjj_core::jj::check_in_jj_repo().ok();

    let Some(root) = repo_root else {
        return Ok(BeadStats::default());
    };

    let beads_db_path = root.join(".beads").join("beads.db");

    if !beads_db_path.exists() {
        return Ok(BeadStats::default());
    }

    // Query beads database
    // Map database errors to zjj_core::Error::DatabaseError for exit code 3
    let conn = Connection::open(&beads_db_path).map_err(|e| {
        anyhow::Error::new(zjj_core::Error::DatabaseError(format!(
            "Failed to open beads database: {e}"
        )))
    })?;

    // Count issues by status
    let open: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'open'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let in_progress: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'in_progress'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let blocked: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'blocked'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let closed: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(BeadStats {
        open,
        in_progress,
        blocked,
        closed,
    })
}

/// Wrapper for status response data
#[derive(Debug, Clone, Serialize)]
struct StatusResponseData {
    pub sessions: Vec<SessionStatusInfo>,
}

/// Output sessions as formatted table
fn output_table(items: &[SessionStatusInfo]) {
    println!(
        "{:<20} {:<12} {:<15} {:<20} {:<15} {:<20}",
        "NAME", "STATUS", "BRANCH", "CHANGES", "DIFF", "BEADS"
    );
    println!("{}", "-".repeat(105));

    for item in items {
        println!(
            "{:<20} {:<12} {:<15} {:<20} {:<15} {:<20}",
            item.name, item.status, item.branch, item.changes, item.diff_stats, item.beads
        );
    }
}

/// Output sessions as JSON
fn output_json(items: &[SessionStatusInfo]) -> Result<()> {
    let data = StatusResponseData {
        sessions: items.to_vec(),
    };
    let envelope = SchemaEnvelope::new("status-response", "single", data);
    let json = serde_json::to_string_pretty(&envelope)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, SessionStatus};

    #[test]
    fn test_file_changes_total() {
        let changes = FileChanges {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            unknown: 0,
        };
        assert_eq!(changes.total(), 7);
    }

    #[test]
    fn test_file_changes_is_clean() {
        let clean = FileChanges::default();
        assert!(clean.is_clean());

        let dirty = FileChanges {
            modified: 1,
            ..Default::default()
        };
        assert!(!dirty.is_clean());
    }

    #[test]
    fn test_file_changes_display_clean() {
        let changes = FileChanges::default();
        assert_eq!(changes.to_string(), "clean");
    }

    #[test]
    fn test_file_changes_display_dirty() {
        let changes = FileChanges {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            unknown: 0,
        };
        assert_eq!(changes.to_string(), "M:2 A:3 D:1 R:1");
    }

    #[test]
    fn test_diff_stats_display() {
        let stats = DiffStats {
            insertions: 123,
            deletions: 45,
        };
        assert_eq!(stats.to_string(), "+123 -45");
    }

    #[test]
    fn test_diff_stats_default() {
        let stats = DiffStats::default();
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
        assert_eq!(stats.to_string(), "+0 -0");
    }

    #[test]
    fn test_bead_stats_display() {
        let stats = BeadStats {
            open: 5,
            in_progress: 3,
            blocked: 2,
            closed: 10,
        };
        assert_eq!(stats.to_string(), "O:5 P:3 B:2 C:10");
    }

    #[test]
    fn test_bead_stats_default() {
        let stats = BeadStats::default();
        assert_eq!(stats.open, 0);
        assert_eq!(stats.in_progress, 0);
        assert_eq!(stats.blocked, 0);
        assert_eq!(stats.closed, 0);
    }

    #[test]
    fn test_session_status_info_serialization() -> Result<()> {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let info = SessionStatusInfo {
            name: session.name.clone(),
            status: session.status.to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: FileChanges {
                modified: 2,
                added: 1,
                deleted: 0,
                renamed: 0,
                unknown: 1,
            },
            diff_stats: DiffStats {
                insertions: 50,
                deletions: 10,
            },
            beads: BeadStats {
                open: 3,
                in_progress: 1,
                blocked: 0,
                closed: 5,
            },
            session,
        };

        let json = serde_json::to_string(&info)?;
        assert!(json.contains("\"name\":\"test-session\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"modified\":2"));
        assert!(json.contains("\"insertions\":50"));
        assert!(json.contains("\"open\":3"));
        Ok(())
    }

    #[test]
    fn test_get_file_changes_missing_workspace() {
        let result = get_file_changes(Path::new("/nonexistent/path"));
        assert_eq!(result.modified, 0);
        assert_eq!(result.added, 0);
        assert_eq!(result.deleted, 0);
        assert_eq!(result.renamed, 0);
    }

    #[test]
    fn test_get_diff_stats_missing_workspace() {
        let result = get_diff_stats(Path::new("/nonexistent/path"));
        assert_eq!(result.insertions, 0);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn test_output_table_format() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges {
                modified: 2,
                added: 1,
                deleted: 0,
                renamed: 0,
                unknown: 0,
            },
            diff_stats: DiffStats {
                insertions: 50,
                deletions: 10,
            },
            beads: BeadStats {
                open: 3,
                in_progress: 1,
                blocked: 0,
                closed: 5,
            },
            session,
        }];

        // This test just verifies the function doesn't panic
        output_table(&items);
    }

    #[test]
    fn test_output_json_format() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges::default(),
            diff_stats: DiffStats::default(),
            beads: BeadStats::default(),
            session,
        }];

        let result = output_json(&items);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_changes_with_unknown_files() {
        let changes = FileChanges {
            modified: 1,
            added: 0,
            deleted: 0,
            renamed: 0,
            unknown: 3,
        };
        // Unknown files don't count toward total
        assert_eq!(changes.total(), 1);
        assert!(!changes.is_clean());
    }

    // ========================================================================
    // RED PHASE: Tests for SchemaEnvelope wrapping (should fail initially)
    // ========================================================================

    /// Test that JSON output includes the $schema field
    #[test]
    fn test_status_json_has_envelope() -> Result<()> {
        let session = Session {
            id: Some(1),
            name: "test-envelope".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test-envelope".to_string(),
            zellij_tab: "zjj:test-envelope".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges::default(),
            diff_stats: DiffStats::default(),
            beads: BeadStats::default(),
            session,
        }];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify $schema field exists
        let schema_field = json_value
            .get("$schema")
            .ok_or_else(|| anyhow::anyhow!("Missing $schema field in envelope"))?;

        // Verify it's a string
        assert!(schema_field.is_string(), "$schema field must be a string");

        // Verify the schema format
        let schema_str = schema_field
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("$schema is not a valid string"))?;

        assert_eq!(
            schema_str, "zjj://status-response/v1",
            "Schema format must be zjj://status-response/v1"
        );

        Ok(())
    }

    /// Test that schema_type field is set to "single" for wrapped responses
    #[test]
    fn test_status_schema_type_single() -> Result<()> {
        let session = Session {
            id: Some(2),
            name: "test-schema-type".to_string(),
            status: SessionStatus::Paused,
            workspace_path: "/tmp/test-schema-type".to_string(),
            zellij_tab: "zjj:test-schema-type".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_891,
            last_synced: Some(1_234_567_891),
            metadata: Some(serde_json::json!({"test": "metadata"})),
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "paused".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "feature".to_string(),
            changes: FileChanges {
                modified: 3,
                added: 2,
                deleted: 1,
                renamed: 1,
                unknown: 0,
            },
            diff_stats: DiffStats {
                insertions: 100,
                deletions: 50,
            },
            beads: BeadStats {
                open: 5,
                in_progress: 2,
                blocked: 1,
                closed: 10,
            },
            session,
        }];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify schema_type field exists
        let schema_type = json_value
            .get("schema_type")
            .ok_or_else(|| anyhow::anyhow!("Missing schema_type field in envelope"))?;

        // Verify it's a string with value "single"
        let schema_type_str = schema_type
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("schema_type is not a valid string"))?;

        assert_eq!(
            schema_type_str, "single",
            "schema_type must be 'single' for wrapped responses"
        );

        Ok(())
    }

    /// Test that empty sessions array is properly wrapped in envelope
    #[test]
    fn test_status_empty_sessions_wrapped() -> Result<()> {
        let items: Vec<SessionStatusInfo> = vec![];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify envelope structure exists even for empty data
        let schema_field = json_value
            .get("$schema")
            .ok_or_else(|| anyhow::anyhow!("Missing $schema field for empty response"))?;

        assert!(
            schema_field.is_string(),
            "Empty response must have $schema field"
        );

        // Verify schema_type exists
        let schema_type = json_value
            .get("schema_type")
            .ok_or_else(|| anyhow::anyhow!("Missing schema_type field for empty response"))?;

        assert!(
            schema_type.is_string(),
            "Empty response must have schema_type field"
        );

        // Verify success field exists
        let success = json_value
            .get("success")
            .ok_or_else(|| anyhow::anyhow!("Missing success field for empty response"))?;

        assert!(
            success.is_boolean(),
            "Empty response must have boolean success field"
        );

        let success_bool = success
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("success is not a valid boolean"))?;

        assert!(success_bool, "Empty response should have success=true");

        // Verify sessions field contains empty array
        let sessions = json_value
            .get("sessions")
            .ok_or_else(|| anyhow::anyhow!("Missing sessions field for empty response"))?;

        assert!(sessions.is_array(), "sessions field must be an array");

        let sessions_array = sessions
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("sessions is not a valid array"))?;

        assert!(sessions_array.is_empty(), "sessions array must be empty");

        Ok(())
    }

    /// Test that the schema format is exactly "zjj://status-response/v1"
    #[test]
    fn test_status_schema_format() -> Result<()> {
        let session = Session {
            id: Some(3),
            name: "test-format".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test-format".to_string(),
            zellij_tab: "zjj:test-format".to_string(),
            branch: None,
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "-".to_string(),
            changes: FileChanges::default(),
            diff_stats: DiffStats::default(),
            beads: BeadStats::default(),
            session,
        }];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Extract $schema field
        let schema_field = json_value
            .get("$schema")
            .ok_or_else(|| anyhow::anyhow!("Missing $schema field"))?;

        let schema_str = schema_field
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("$schema is not a string"))?;

        // Verify exact format (no variations allowed)
        assert_eq!(
            schema_str, "zjj://status-response/v1",
            "Schema format must exactly match zjj://status-response/v1"
        );

        // Verify it's not some other variation
        assert!(
            !schema_str.contains("http://"),
            "Schema must use zjj:// protocol"
        );
        assert!(
            !schema_str.contains("https://"),
            "Schema must use zjj:// protocol"
        );
        assert!(
            schema_str.starts_with("zjj://"),
            "Schema must start with zjj://"
        );
        assert!(schema_str.ends_with("/v1"), "Schema must end with /v1");

        Ok(())
    }

    /// Test that _schema_version field exists and is correct
    #[test]
    fn test_status_schema_version_field() -> Result<()> {
        let session = Session {
            id: Some(4),
            name: "test-version".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test-version".to_string(),
            zellij_tab: "zjj:test-version".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges::default(),
            diff_stats: DiffStats::default(),
            beads: BeadStats::default(),
            session,
        }];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify _schema_version field exists
        let version_field = json_value
            .get("_schema_version")
            .ok_or_else(|| anyhow::anyhow!("Missing _schema_version field"))?;

        // Verify it's a string
        assert!(
            version_field.is_string(),
            "_schema_version field must be a string"
        );

        let version_str = version_field
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("_schema_version is not a valid string"))?;

        assert_eq!(version_str, "1.0", "_schema_version must be 1.0");

        Ok(())
    }

    /// Test that success field is present and true for valid responses
    #[test]
    fn test_status_success_field_true() -> Result<()> {
        let session = Session {
            id: Some(5),
            name: "test-success".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test-success".to_string(),
            zellij_tab: "zjj:test-success".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges::default(),
            diff_stats: DiffStats::default(),
            beads: BeadStats::default(),
            session,
        }];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify success field exists
        let success = json_value
            .get("success")
            .ok_or_else(|| anyhow::anyhow!("Missing success field"))?;

        // Verify it's a boolean
        assert!(success.is_boolean(), "success field must be a boolean");

        let success_bool = success
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("success is not a valid boolean"))?;

        assert!(success_bool, "success must be true for valid responses");

        Ok(())
    }

    /// Test that data field contains the actual session array
    #[test]
    fn test_status_data_field_contains_sessions() -> Result<()> {
        let session1 = Session {
            id: Some(6),
            name: "session-1".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/session-1".to_string(),
            zellij_tab: "zjj:session-1".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let session2 = Session {
            id: Some(7),
            name: "session-2".to_string(),
            status: SessionStatus::Paused,
            workspace_path: "/tmp/session-2".to_string(),
            zellij_tab: "zjj:session-2".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_891,
            updated_at: 1_234_567_891,
            last_synced: None,
            metadata: None,
        };

        let items = vec![
            SessionStatusInfo {
                name: session1.name.clone(),
                status: "active".to_string(),
                workspace_path: session1.workspace_path.clone(),
                branch: "main".to_string(),
                changes: FileChanges::default(),
                diff_stats: DiffStats::default(),
                beads: BeadStats::default(),
                session: session1,
            },
            SessionStatusInfo {
                name: session2.name.clone(),
                status: "paused".to_string(),
                workspace_path: session2.workspace_path.clone(),
                branch: "feature".to_string(),
                changes: FileChanges::default(),
                diff_stats: DiffStats::default(),
                beads: BeadStats::default(),
                session: session2,
            },
        ];

        // Wrap in envelope
        let data = StatusResponseData { sessions: items };
        let envelope = SchemaEnvelope::new("status-response", "single", data);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify sessions field exists
        let sessions = json_value
            .get("sessions")
            .ok_or_else(|| anyhow::anyhow!("Missing sessions field"))?;

        // Verify it's an array
        assert!(sessions.is_array(), "sessions field must be an array");

        let sessions_array = sessions
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("sessions is not a valid array"))?;

        // Verify it contains 2 sessions
        assert_eq!(
            sessions_array.len(),
            2,
            "sessions array must contain 2 sessions"
        );

        // Verify first session name
        let first_session = &sessions_array[0];
        let first_name = first_session
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("First session missing name"))?;

        assert_eq!(
            first_name, "session-1",
            "First session name must be session-1"
        );

        // Verify second session name
        let second_session = &sessions_array[1];
        let second_name = second_session
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Second session missing name"))?;

        assert_eq!(
            second_name, "session-2",
            "Second session name must be session-2"
        );

        Ok(())
    }
}
