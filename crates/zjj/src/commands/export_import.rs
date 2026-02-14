//! Export/Import commands - Export and import session configurations
//!
//! Allows saving and restoring session state for backup or transfer.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::{commands::get_session_db, db::SessionDb, session::Session};

// ═══════════════════════════════════════════════════════════════════════════
// TIMESTAMP VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

/// Validate and parse a timestamp string
///
/// Accepts RFC3339-formatted timestamps (e.g., "2025-01-15T12:30:45Z").
/// Returns None if the input is None or empty string, indicating the current
/// timestamp should be used instead.
///
/// # Errors
///
/// Returns an error if the timestamp string is not empty and not valid RFC3339.
fn validate_and_parse_timestamp(timestamp: Option<&String>) -> Result<Option<DateTime<Utc>>> {
    timestamp
        .filter(|s| !s.is_empty())
        .map(|ts| {
            DateTime::parse_from_rfc3339(ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| anyhow::anyhow!("invalid timestamp format '{ts}': {e}"))
        })
        .transpose()
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORT
// ═══════════════════════════════════════════════════════════════════════════

/// Options for the export command
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Session to export (or all if None)
    pub session: Option<String>,
    /// Output file path (stdout if None)
    pub output: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Exported session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSession {
    /// Session name
    pub name: String,
    /// Session status
    pub status: String,
    /// Associated bead ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead_id: Option<String>,
    /// Workspace path (relative)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Owner agent ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Created timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// JJ commits in this workspace
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub commits: Vec<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Export version
    pub version: String,
    /// Export timestamp
    pub exported_at: String,
    /// Exported sessions
    pub sessions: Vec<ExportedSession>,
    /// Number of sessions exported
    pub count: usize,
}

/// Run the export command
///
/// # JSON Output Format
///
/// When `--format json` is specified, the response is wrapped in a `SchemaEnvelope`
/// which includes a `success` field. Therefore, response data MUST NOT include its own
/// `success` field to avoid duplication.
///
/// ## Example (correct)
/// ```ignore
/// let response = serde_json::json!({
///     "output_file": "/tmp/export.json",
///     "sessions_exported": 3,
///     // NO "success" field here - SchemaEnvelope adds it
/// });
/// let envelope = SchemaEnvelope::new("export-response", "single", response);
/// ```
///
/// # Bug Fix (zjj-1ube)
///
/// Previously, the response included `"success": true`, which resulted in duplicate
/// `success` fields when wrapped in `SchemaEnvelope`. This has been fixed.
pub async fn run_export(options: &ExportOptions) -> Result<()> {
    let db = get_session_db().await?;

    let sessions: Vec<ExportedSession> = if let Some(session_name) = &options.session {
        let session = db
            .get(session_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

        vec![ExportedSession {
            name: session.name,
            status: session.status.to_string(),
            bead_id: None, // bead_id not in Session struct
            workspace_path: Some(session.workspace_path),
            owner: None,
            created_at: Some(format!("{}", session.created_at)),
            commits: vec![],
            metadata: session.metadata,
        }]
    } else {
        db.list(None)
            .await?
            .into_iter()
            .map(|s| ExportedSession {
                name: s.name,
                status: s.status.to_string(),
                bead_id: None,
                workspace_path: Some(s.workspace_path),
                owner: None,
                created_at: Some(format!("{}", s.created_at)),
                commits: vec![],
                metadata: s.metadata,
            })
            .collect()
    };

    let result = ExportResult {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        count: sessions.len(),
        sessions,
    };

    let json_output = serde_json::to_string_pretty(&result)?;

    if let Some(output_path) = &options.output {
        tokio::fs::write(output_path, &json_output).await?;
        if options.format.is_json() {
            let response = serde_json::json!({
                "output_file": output_path,
                "sessions_exported": result.count,
            });
            let envelope = SchemaEnvelope::new("export-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("✓ Exported {} sessions to {}", result.count, output_path);
        }
    } else if options.format.is_json() {
        let envelope = SchemaEnvelope::new("export-response", "single", result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("{json_output}");
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// IMPORT
// ═══════════════════════════════════════════════════════════════════════════

/// Options for the import command
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Input file path
    pub input: String,
    /// Overwrite existing sessions
    pub force: bool,
    /// Skip existing sessions instead of erroring
    pub skip_existing: bool,
    /// Dry-run mode
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Import result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub imported: usize,
    pub skipped: usize,
    pub overwritten: usize,
    pub failed: usize,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub imported_sessions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub skipped_sessions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub overwritten_sessions: Vec<String>,
}

/// Validate and parse import file
///
/// Reads the import file and validates it's properly formatted JSON
/// matching the `ExportResult` schema. Also validates all timestamp fields.
async fn validate_and_parse_import(input_path: &str) -> Result<ExportResult> {
    let content = tokio::fs::read_to_string(input_path).await?;
    let export_data: ExportResult = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid import file format: {e}"))?;

    // Validate all session timestamps
    for session in &export_data.sessions {
        if let Some(ref ts_str) = session.created_at {
            // Validate timestamp format - will error if invalid
            validate_and_parse_timestamp(Some(ts_str)).map_err(|e| {
                anyhow::anyhow!("Invalid timestamp for session '{}': {e}", session.name)
            })?;
        }
    }

    Ok(export_data)
}

/// Check if a session exists and determine the action to take
///
/// Returns:
/// - Ok(Some(true)): session exists and should be overwritten
/// - Ok(Some(false)): session exists and should be skipped
/// - Ok(None): session doesn't exist, proceed with import
/// - Err: session exists and action not allowed
async fn check_session_conflict(
    db: &SessionDb,
    session_name: &str,
    force: bool,
    skip_existing: bool,
) -> Result<Option<bool>> {
    let session_exists: Option<Session> = db.get(session_name).await?;
    let session_exists = session_exists.is_some();

    if !session_exists {
        return Ok(None);
    }

    if force {
        Ok(Some(true))
    } else if skip_existing {
        Ok(Some(false))
    } else {
        anyhow::bail!("Session '{session_name}' already exists (use --force to overwrite)")
    }
}

/// Delete existing session for overwrite
///
/// Attempts to remove the existing session before importing a new version.
async fn delete_existing_session(db: &SessionDb, session_name: &str) -> Result<()> {
    let _deleted: bool = db
        .delete(session_name)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to delete existing session '{session_name}': {e}"))?;
    Ok(())
}

/// Import a single session
///
/// Creates a new session in the database, handling both fresh imports and overwrites.
/// Preserves the original creation timestamp if provided and valid.
async fn import_session(
    db: &SessionDb,
    session: &ExportedSession,
    _was_overwritten: bool,
) -> Result<()> {
    let workspace_path = session.workspace_path.as_deref().map_or("", |value| value);
    let name = &session.name;

    // Validate and parse the timestamp, using current time if not provided
    let created_timestamp =
        validate_and_parse_timestamp(session.created_at.as_ref())?.unwrap_or_else(chrono::Utc::now);

    // Convert DateTime to unix timestamp
    let created_ts = u64::try_from(created_timestamp.timestamp()).unwrap_or(u64::MAX);

    let _created: Session = db
        .create_with_timestamp(name, workspace_path, created_ts)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to import '{name}': {e}"))?;

    Ok(())
}

/// Process a single session import
///
/// Handles the full import flow for one session:
/// - Check for conflicts
/// - Delete if overwriting
/// - Import the session
/// - Track results
async fn process_single_session(
    db: &SessionDb,
    session: &ExportedSession,
    options: &ImportOptions,
    result: &mut ImportResult,
) -> Result<()> {
    let session_name = &session.name;

    // Check if session exists and determine action
    let action =
        check_session_conflict(db, session_name, options.force, options.skip_existing).await?;

    match action {
        Some(true) => {
            // Overwrite mode: delete existing session
            delete_existing_session(db, session_name).await?;
            let was_overwritten = true;

            if options.dry_run {
                result.overwritten += 1;
                result.overwritten_sessions.push(session_name.clone());
                return Ok(());
            }

            import_session(db, session, was_overwritten).await?;
            result.overwritten += 1;
            result.overwritten_sessions.push(session_name.clone());
        }
        Some(false) => {
            // Skip mode: skip existing session
            result.skipped += 1;
            result.skipped_sessions.push(session_name.clone());
        }
        None => {
            // New session: import it
            if options.dry_run {
                result.imported += 1;
                result.imported_sessions.push(session_name.clone());
                return Ok(());
            }

            import_session(db, session, false).await?;
            result.imported += 1;
            result.imported_sessions.push(session_name.clone());
        }
    }

    Ok(())
}

/// Display import results in human-readable format
fn display_import_results(result: &ImportResult) {
    if result.dry_run {
        println!("[dry-run] Would import {} sessions", result.imported);
    } else {
        println!(
            "✓ Imported {} sessions, skipped {}, overwritten {}, failed {}",
            result.imported, result.skipped, result.overwritten, result.failed
        );
    }

    if !result.imported_sessions.is_empty() {
        println!("  Imported: {}", result.imported_sessions.join(", "));
    }

    if !result.overwritten_sessions.is_empty() {
        println!("  Overwritten: {}", result.overwritten_sessions.join(", "));
    }

    if !result.skipped_sessions.is_empty() {
        println!("  Skipped: {}", result.skipped_sessions.join(", "));
    }

    if !result.errors.is_empty() {
        eprintln!("Errors:");
        for err in &result.errors {
            eprintln!("  - {err}");
        }
    }
}

/// Run the import command
pub async fn run_import(options: &ImportOptions) -> Result<()> {
    let export_data = validate_and_parse_import(&options.input).await?;
    let db = get_session_db().await?;

    let mut result = ImportResult {
        success: true,
        imported: 0,
        skipped: 0,
        overwritten: 0,
        failed: 0,
        dry_run: options.dry_run,
        errors: vec![],
        imported_sessions: vec![],
        skipped_sessions: vec![],
        overwritten_sessions: vec![],
    };

    // Process each session in the export
    for session in &export_data.sessions {
        if let Err(e) = process_single_session(&db, session, options, &mut result).await {
            result.failed += 1;
            result.errors.push(e.to_string());
            result.success = false;
        }
    }

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("import-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        display_import_results(&result);
    }

    if result.success {
        Ok(())
    } else {
        anyhow::bail!("Import completed with errors")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_result_serialization() -> anyhow::Result<()> {
        let result = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 2,
            sessions: vec![],
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"count\":2"));
        Ok(())
    }

    #[test]
    fn test_exported_session_serialization() -> anyhow::Result<()> {
        let session = ExportedSession {
            name: "test".to_string(),
            status: "active".to_string(),
            bead_id: Some("zjj-1234".to_string()),
            workspace_path: Some("/path".to_string()),
            owner: None,
            created_at: None,
            commits: vec![],
            metadata: None,
        };

        let json = serde_json::to_string(&session)?;
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"bead_id\":\"zjj-1234\""));
        Ok(())
    }

    #[test]
    fn test_import_result_serialization() -> anyhow::Result<()> {
        let result = ImportResult {
            success: true,
            imported: 2,
            skipped: 1,
            overwritten: 0,
            failed: 0,
            dry_run: false,
            errors: vec![],
            imported_sessions: vec!["s1".to_string(), "s2".to_string()],
            skipped_sessions: vec!["s3".to_string()],
            overwritten_sessions: vec![],
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"imported\":2"));
        assert!(json.contains("\"skipped\":1"));
        Ok(())
    }

    #[test]
    fn test_export_import_roundtrip() -> anyhow::Result<()> {
        let original = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 1,
            sessions: vec![ExportedSession {
                name: "test".to_string(),
                status: "active".to_string(),
                bead_id: None,
                workspace_path: None,
                owner: None,
                created_at: None,
                commits: vec![],
                metadata: None,
            }],
        };

        let json = serde_json::to_string(&original)?;
        let parsed: ExportResult = serde_json::from_str(&json)?;

        assert_eq!(parsed.count, original.count);
        assert_eq!(parsed.sessions.len(), original.sessions.len());
        Ok(())
    }

    #[test]
    fn test_validate_and_parse_timestamp_valid_rfc3339() -> anyhow::Result<()> {
        let valid_ts = "2025-01-15T12:30:45Z";
        let result = validate_and_parse_timestamp(Some(&valid_ts.to_string()))?;
        assert!(
            result.is_some(),
            "Valid timestamp should parse successfully"
        );
        Ok(())
    }

    #[test]
    fn test_validate_and_parse_timestamp_with_offset() -> anyhow::Result<()> {
        let valid_ts = "2025-01-15T12:30:45+08:00";
        let result = validate_and_parse_timestamp(Some(&valid_ts.to_string()))?;
        assert!(
            result.is_some(),
            "Timestamp with offset should parse successfully"
        );
        Ok(())
    }

    #[test]
    fn test_validate_and_parse_timestamp_none_returns_none() -> anyhow::Result<()> {
        let result = validate_and_parse_timestamp(None)?;
        assert!(result.is_none(), "None input should return None");
        Ok(())
    }

    #[test]
    fn test_validate_and_parse_timestamp_empty_string_returns_none() -> anyhow::Result<()> {
        let result = validate_and_parse_timestamp(Some(&String::new()))?;
        assert!(result.is_none(), "Empty string should return None");
        Ok(())
    }

    #[test]
    fn test_validate_and_parse_timestamp_invalid_format() {
        let invalid_ts = "not-a-timestamp";
        let result = validate_and_parse_timestamp(Some(&invalid_ts.to_string()));
        assert!(
            result.is_err(),
            "Invalid timestamp format should return an error"
        );
        let Err(err) = result else {
            panic!("Expected error but got Ok");
        };
        assert!(err.to_string().contains("invalid timestamp format"));
    }

    #[test]
    fn test_validate_and_parse_timestamp_iso8601_without_tz() {
        // ISO 8601 without timezone is not valid RFC3339
        let invalid_ts = "2025-01-15T12:30:45";
        let result = validate_and_parse_timestamp(Some(&invalid_ts.to_string()));
        assert!(
            result.is_err(),
            "Timestamp without timezone should return an error"
        );
    }
}
