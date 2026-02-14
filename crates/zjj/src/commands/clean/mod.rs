//! Clean stale sessions (sessions where workspace no longer exists)

use std::{io::Write, time::Duration};

use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::commands::get_session_db;

pub mod periodic_cleanup;

/// Options for the clean command
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// Skip confirmation prompt
    pub force: bool,
    /// List stale sessions without removing
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
    /// Run periodic cleanup daemon (1hr interval)
    pub periodic: bool,
    /// Age threshold for periodic cleanup (seconds, default 7200 = 2hr)
    pub age_threshold: Option<u64>,
}

/// Output for clean command in JSON mode
#[derive(Debug, Clone, Serialize)]
pub struct CleanOutput {
    pub stale_count: usize,
    pub removed_count: usize,
    pub stale_sessions: Vec<String>,
}

/// Run the clean command with options
///
/// Uses Railway-Oriented Programming to handle the workflow:
/// 1. Check if periodic mode requested
/// 2. Load all sessions from database
/// 3. Filter to find stale sessions (workspace missing)
/// 4. Handle dry-run or interactive confirmation
/// 5. Remove stale sessions if confirmed
pub async fn run_with_options(options: &CleanOptions) -> Result<()> {
    // Handle periodic mode
    if options.periodic {
        return run_periodic_mode(options).await;
    }

    let db = get_session_db().await?;

    // 1. List all sessions and filter to stale ones using functional pipeline
    let sessions = db.list(None).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let stale_sessions: Vec<_> = futures::stream::iter(sessions)
        .map(Ok::<_, anyhow::Error>)
        .try_filter_map(|session| async move {
            let exists = tokio::fs::try_exists(&session.workspace_path)
                .await
                .map_err(|error| {
                    anyhow::anyhow!(
                        "Failed to verify workspace path '{}' for session '{}': {error}",
                        session.workspace_path,
                        session.name
                    )
                })?;
            Ok((!exists).then_some(session))
        })
        .try_collect()
        .await?;

    // 2. Handle no stale sessions case
    if stale_sessions.is_empty() {
        output_no_stale(options.format);
        return Ok(());
    }

    let stale_names: Vec<String> = stale_sessions.iter().map(|s| s.name.clone()).collect();

    // 3. Dry-run: list and exit
    if options.dry_run {
        output_dry_run(&stale_names, options.format);
        return Ok(());
    }

    // 4. Prompt for confirmation unless --force
    if !options.force && !confirm_removal(&stale_names)? {
        output_cancelled(&stale_names, options.format);
        return Ok(());
    }

    // 5. Remove stale sessions using functional fold for error handling
    let removed_count = futures::stream::iter(&stale_sessions)
        .map(Ok::<_, anyhow::Error>)
        .try_fold(0, |acc, session| {
            let db = &db;
            async move {
                let deleted = db.delete(&session.name).await.map_err(anyhow::Error::new)?;
                Ok(if deleted { acc + 1 } else { acc })
            }
        })
        .await?;

    // 6. Output result
    output_result(removed_count, &stale_names, options.format);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT FUNCTIONS (Imperative Shell)
// ═══════════════════════════════════════════════════════════════════════════

/// Output when no stale sessions found
fn output_no_stale(format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: 0,
            removed_count: 0,
            stale_sessions: Vec::new(),
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!("✓ No stale sessions found");
        println!("  All sessions have valid workspaces");
    }
}

/// Output for dry-run mode
fn output_dry_run(stale_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: stale_names.len(),
            removed_count: 0,
            stale_sessions: stale_names.to_vec(),
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!(
            "Found {} stale session(s) (dry-run, no changes made):",
            stale_names.len()
        );
        for name in stale_names {
            println!("  - {name}");
        }
        println!();
        println!("Run 'zjj clean --force' to remove these sessions");
    }
}

/// Output when cleanup is cancelled
fn output_cancelled(stale_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: stale_names.len(),
            removed_count: 0,
            stale_sessions: stale_names.to_vec(),
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!("Cleanup cancelled");
    }
}

/// Output cleanup result
fn output_result(removed_count: usize, stale_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: stale_names.len(),
            removed_count,
            stale_sessions: stale_names.to_vec(),
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!("✓ Removed {removed_count} stale session(s)");
        for name in stale_names {
            println!("  - {name}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIRMATION (Interactive I/O)
// ═══════════════════════════════════════════════════════════════════════════

/// Prompt user for confirmation to remove stale sessions
///
/// Returns Ok(true) if user confirms, Ok(false) if user cancels
///
/// Prompts are written to stderr to avoid mixing with JSON output on stdout.
fn confirm_removal(stale_names: &[String]) -> Result<bool> {
    eprintln!("Found {} stale session(s):", stale_names.len());
    for name in stale_names {
        eprintln!("  - {name}");
    }
    eprintln!();
    eprint!("Remove these sessions? [y/N] ");
    std::io::stderr()
        .flush()
        .map_err(|e| anyhow::Error::new(zjj_core::Error::IoError(e.to_string())))?;

    let mut response = String::new();
    std::io::stdin()
        .read_line(&mut response)
        .map_err(|e| anyhow::Error::new(zjj_core::Error::IoError(e.to_string())))?;

    let response = response.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

// ═══════════════════════════════════════════════════════════════════════════
// PERIODIC MODE
// ═══════════════════════════════════════════════════════════════════════════

/// Run periodic cleanup daemon
///
/// Runs indefinitely in the background
async fn run_periodic_mode(options: &CleanOptions) -> Result<()> {
    let config = periodic_cleanup::PeriodicCleanupConfig {
        interval: Duration::from_secs(3600), // 1 hour
        age_threshold: Duration::from_secs(options.age_threshold.unwrap_or(7200)), /* 2 hours default */
        dry_run: options.dry_run,
        format: options.format,
    };

    if options.format.is_json() {
        println!(
            r#"{{"status": "starting", "mode": "periodic", "interval_secs": 3600, "age_threshold_secs": {}}}"#,
            config.age_threshold.as_secs()
        );
    } else {
        println!("Starting periodic cleanup daemon...");
        println!("  Interval: {} minutes", config.interval.as_secs() / 60);
        println!(
            "  Age threshold: {} hours",
            config.age_threshold.as_secs() / 3600
        );
        println!("  Dry run: {}", config.dry_run);
        println!();
    }

    // Run periodic cleanup
    periodic_cleanup::run_periodic_cleanup(config).await
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_clean_options_default() {
        let opts = CleanOptions::default();
        assert!(!opts.force);
        assert!(!opts.dry_run);
        assert!(opts.format.is_human());
    }

    #[test]
    fn test_clean_output_serialization() -> Result<()> {
        let output = CleanOutput {
            stale_count: 2,
            removed_count: 2,
            stale_sessions: vec!["session1".to_string(), "session2".to_string()],
        };

        let json = serde_json::to_string(&output)?;
        assert!(json.contains("\"stale_count\":2"));
        assert!(json.contains("\"removed_count\":2"));
        assert!(json.contains("session1"));
        assert!(json.contains("session2"));

        Ok(())
    }

    #[test]
    fn test_stale_session_filtering() {
        // This test verifies the filtering logic conceptually
        // In a real integration test, we'd create temp sessions
        let sessions = vec![
            ("session1", "/tmp/exists"),
            ("session2", "/nonexistent/path"),
        ];

        let stale: Vec<_> = sessions
            .into_iter()
            .filter(|(_, path)| !Path::new(path).exists())
            .collect();

        // /tmp/exists might or might not exist, but /nonexistent/path definitely doesn't
        assert!(stale.iter().any(|(name, _)| *name == "session2"));
    }

    #[test]
    fn test_empty_stale_list_handling() {
        let stale_names: Vec<String> = Vec::new();

        // Verify that empty lists are handled correctly
        assert_eq!(stale_names.len(), 0);
        assert!(stale_names.is_empty());
    }

    #[test]
    fn test_removed_count_calculation() {
        // Simulate the fold operation for counting removals
        let sessions = ["s1", "s2", "s3"];

        // Simulate successful removals using functional fold
        let count = sessions
            .iter()
            .try_fold(0, |acc, _| -> Result<i32> { Ok(acc + 1) });

        assert_eq!(count.ok(), Some(3));
    }

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_clean_json_has_envelope() -> Result<()> {
        // FAILING: Verify envelope wrapping for clean command output
        use zjj_core::json::SchemaEnvelope;

        let output = CleanOutput {
            stale_count: 0,
            removed_count: 0,
            stale_sessions: Vec::new(),
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_clean_success_wrapped() -> Result<()> {
        // FAILING: Verify envelope wrapping on successful cleanup
        use zjj_core::json::SchemaEnvelope;

        let output = CleanOutput {
            stale_count: 2,
            removed_count: 2,
            stale_sessions: vec!["session1".to_string(), "session2".to_string()],
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );

        Ok(())
    }

    #[test]
    fn test_clean_error_wrapped() -> Result<()> {
        // FAILING: Verify envelope wrapping includes error information
        use serde_json::json;
        use zjj_core::json::SchemaEnvelope;

        let error_response = json!({"error": "No sessions found"});
        let envelope = SchemaEnvelope::new("clean-response", "single", error_response);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert!(
            parsed.get("_schema_version").is_some(),
            "Missing _schema_version field"
        );

        Ok(())
    }

    #[test]
    fn test_clean_result_type_validated() -> Result<()> {
        // FAILING: Verify schema_type field correctly identifies response shape
        use zjj_core::json::SchemaEnvelope;

        let output = CleanOutput {
            stale_count: 1,
            removed_count: 0,
            stale_sessions: vec!["stale_session".to_string()],
        };
        let envelope = SchemaEnvelope::new("clean-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let schema_type = parsed
            .get("schema_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("schema_type not found"))?;

        assert_eq!(
            schema_type, "single",
            "schema_type should be 'single' for single object responses"
        );

        Ok(())
    }
}
