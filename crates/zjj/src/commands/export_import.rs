//! Export/Import commands - Export and import session configurations
//!
//! Allows saving and restoring session state for backup or transfer.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::commands::get_session_db;

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
    /// Include workspace files
    #[allow(dead_code)]
    pub include_files: bool,
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
pub fn run_export(options: &ExportOptions) -> Result<()> {
    let db = get_session_db()?;

    let sessions: Vec<ExportedSession> = if let Some(session_name) = &options.session {
        let session = db
            .get_blocking(session_name)?
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
        db.list_blocking(None)?
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
        std::fs::write(output_path, &json_output)?;
        if options.format.is_json() {
            let response = serde_json::json!({
                "success": true,
                "output_file": output_path,
                "sessions_exported": result.count,
            });
            let envelope = SchemaEnvelope::new("export-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("✓ Exported {} sessions to {}", result.count, output_path);
        }
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
    pub failed: usize,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub imported_sessions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub skipped_sessions: Vec<String>,
}

/// Run the import command
pub fn run_import(options: &ImportOptions) -> Result<()> {
    let content = std::fs::read_to_string(&options.input)?;
    let export_data: ExportResult = serde_json::from_str(&content)?;

    let db = get_session_db()?;

    let mut result = ImportResult {
        success: true,
        imported: 0,
        skipped: 0,
        failed: 0,
        dry_run: options.dry_run,
        errors: vec![],
        imported_sessions: vec![],
        skipped_sessions: vec![],
    };

    for session in export_data.sessions {
        // Check if session already exists
        if db.get_blocking(&session.name)?.is_some() {
            if options.skip_existing {
                result.skipped += 1;
                result.skipped_sessions.push(session.name.clone());
                continue;
            }
            result.failed += 1;
            result
                .errors
                .push(format!("Session '{}' already exists", session.name));
            result.success = false;
            continue;
        }

        if options.dry_run {
            result.imported += 1;
            result.imported_sessions.push(session.name.clone());
            continue;
        }

        // Create the session
        match db.create_blocking(
            &session.name,
            match session.workspace_path.as_deref() {
                Some(value) => value,
                None => "",
            },
        ) {
            Ok(_) => {
                result.imported += 1;
                result.imported_sessions.push(session.name.clone());
            }
            Err(e) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("Failed to import '{}': {}", session.name, e));
                result.success = false;
            }
        }
    }

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("import-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        if options.dry_run {
            println!("[dry-run] Would import {} sessions", result.imported);
        } else {
            println!(
                "✓ Imported {} sessions, skipped {}, failed {}",
                result.imported, result.skipped, result.failed
            );
        }

        if !result.imported_sessions.is_empty() {
            println!("  Imported: {}", result.imported_sessions.join(", "));
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
            failed: 0,
            dry_run: false,
            errors: vec![],
            imported_sessions: vec!["s1".to_string(), "s2".to_string()],
            skipped_sessions: vec!["s3".to_string()],
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
}
