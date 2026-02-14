//! Prune invalid sessions command
//!
//! Bulk cleanup primitive to remove all invalid session records in one deterministic command.
//! Invalid sessions are those where:
//! - The workspace directory no longer exists
//! - The session record exists in database but references missing paths
//!
//! Supports --yes flag to skip confirmation for scripting/CI use.

use std::io::Write;

use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::commands::get_session_db;

#[derive(Debug, Clone, Default)]
pub struct PruneInvalidOptions {
    pub yes: bool,
    pub dry_run: bool,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Serialize)]
pub struct PruneInvalidOutput {
    pub invalid_count: usize,
    pub removed_count: usize,
    pub invalid_sessions: Vec<String>,
}

pub async fn run(options: &PruneInvalidOptions) -> Result<()> {
    let db = get_session_db().await?;

    let sessions = db.list(None).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let invalid_sessions: Vec<_> = futures::stream::iter(sessions)
        .then(|session| async move {
            let exists = tokio::fs::try_exists(&session.workspace_path)
                .await
                .map_err(anyhow::Error::new)?;
            Ok(if exists { None } else { Some(session) })
        })
        .filter_map(|res: Result<Option<_>, anyhow::Error>| async move { res.ok().flatten() })
        .collect()
        .await;

    if invalid_sessions.is_empty() {
        output_no_invalid(options.format);
        return Ok(());
    }

    let invalid_names: Vec<String> = invalid_sessions.iter().map(|s| s.name.clone()).collect();

    if options.dry_run {
        output_dry_run(&invalid_names, options.format);
        return Ok(());
    }

    if !options.yes {
        eprintln!("Found {} invalid session(s):", invalid_names.len());
        for name in &invalid_names {
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
        if response != "y" && response != "yes" {
            output_cancelled(&invalid_names, options.format);
            return Ok(());
        }
    }

    let removed_count = futures::stream::iter(&invalid_sessions)
        .map(Ok::<_, anyhow::Error>)
        .try_fold(0, |acc, session| {
            let db = &db;
            async move {
                let deleted = db.delete(&session.name).await.map_err(anyhow::Error::new)?;
                Ok(if deleted { acc + 1 } else { acc })
            }
        })
        .await?;

    output_result(removed_count, &invalid_names, options.format);

    Ok(())
}

fn output_no_invalid(format: OutputFormat) {
    if format.is_json() {
        let output = PruneInvalidOutput {
            invalid_count: 0,
            removed_count: 0,
            invalid_sessions: Vec::new(),
        };
        let envelope = SchemaEnvelope::new("prune-invalid-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!("✓ No invalid sessions found");
        println!("  All sessions have valid workspaces");
    }
}

fn output_dry_run(invalid_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = PruneInvalidOutput {
            invalid_count: invalid_names.len(),
            removed_count: 0,
            invalid_sessions: invalid_names.to_vec(),
        };
        let envelope = SchemaEnvelope::new("prune-invalid-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!(
            "Found {} invalid session(s) (dry-run, no changes made):",
            invalid_names.len()
        );
        for name in invalid_names {
            println!("  - {name}");
        }
        println!();
        println!("Run 'zjj prune-invalid --yes' to remove these sessions");
    }
}

fn output_cancelled(invalid_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = PruneInvalidOutput {
            invalid_count: invalid_names.len(),
            removed_count: 0,
            invalid_sessions: invalid_names.to_vec(),
        };
        let envelope = SchemaEnvelope::new("prune-invalid-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!("Prune cancelled");
    }
}

fn output_result(removed_count: usize, invalid_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = PruneInvalidOutput {
            invalid_count: invalid_names.len(),
            removed_count,
            invalid_sessions: invalid_names.to_vec(),
        };
        let envelope = SchemaEnvelope::new("prune-invalid-response", "single", output);
        if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
            println!("{json_str}");
        }
    } else {
        println!("✓ Removed {removed_count} invalid session(s)");
        for name in invalid_names {
            println!("  - {name}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_invalid_options_default_values() {
        let opts = PruneInvalidOptions::default();
        assert!(!opts.yes);
        assert!(!opts.dry_run);
        assert!(opts.format.is_human());
    }

    #[test]
    fn test_prune_invalid_output_structure() {
        let output = PruneInvalidOutput {
            invalid_count: 2,
            removed_count: 2,
            invalid_sessions: vec!["a".to_string(), "b".to_string()],
        };
        assert_eq!(output.invalid_count, 2);
        assert_eq!(output.removed_count, 2);
        assert_eq!(output.invalid_sessions.len(), 2);
    }
}
