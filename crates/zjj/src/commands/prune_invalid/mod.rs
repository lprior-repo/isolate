//! Prune invalid sessions command
//!
//! Bulk cleanup primitive to remove all invalid session records in one deterministic command.
//! Invalid sessions are those where:
//! - The workspace directory no longer exists
//! - The session record exists in database but references missing paths
//!
//! Supports --yes flag to skip confirmation for scripting/CI use.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::Write;

use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use zjj_core::{
    output::{emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Message, OutputLine, Summary, SummaryType},
    OutputFormat,
};

use crate::commands::get_session_db;

#[derive(Debug, Clone, Default)]
pub struct PruneInvalidOptions {
    pub yes: bool,
    pub dry_run: bool,
    pub format: OutputFormat,
}

pub async fn run(options: &PruneInvalidOptions) -> Result<()> {
    let db = get_session_db().await?;

    let sessions = db.list(None).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let invalid_sessions: Vec<_> = futures::stream::iter(sessions)
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

    if invalid_sessions.is_empty() {
        output_no_invalid(options.format)?;
        return Ok(());
    }

    let invalid_names: Vec<String> = invalid_sessions.iter().map(|s| s.name.clone()).collect();

    if options.dry_run {
        output_dry_run(&invalid_names, options.format)?;
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
            output_cancelled(&invalid_names, options.format)?;
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

    output_result(removed_count, &invalid_names, options.format)?;

    Ok(())
}

fn output_no_invalid(format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let summary = Summary::new(
            SummaryType::Status,
            Message::new("No invalid sessions found")?,
        )?;
        emit_stdout(&OutputLine::Summary(summary))?;
    } else {
        println!("No invalid sessions found");
        println!("  All sessions have valid workspaces");
    }
    Ok(())
}

fn output_dry_run(invalid_names: &[String], format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let summary = Summary::new(
            SummaryType::Info,
            Message::new(format!(
                "Found {} invalid session(s) (dry-run)",
                invalid_names.len()
            ))?,
        )?;
        emit_stdout(&OutputLine::Summary(summary))?;

        for name in invalid_names {
            let action = Action::new(
                ActionVerb::new("discovered").map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
                ActionTarget::new(name).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
                ActionStatus::Pending,
            );
            emit_stdout(&OutputLine::Action(action))?;
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
    Ok(())
}

fn output_cancelled(invalid_names: &[String], format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let summary = Summary::new(SummaryType::Status, Message::new("Prune cancelled")?)?;
        emit_stdout(&OutputLine::Summary(summary))?;

        for name in invalid_names {
            let action = Action::new(
                ActionVerb::new("remove").map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
                ActionTarget::new(name).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
                ActionStatus::Skipped,
            );
            emit_stdout(&OutputLine::Action(action))?;
        }
    } else {
        println!("Prune cancelled");
    }
    Ok(())
}

fn output_result(
    removed_count: usize,
    invalid_names: &[String],
    format: OutputFormat,
) -> Result<()> {
    if format.is_json() {
        let summary = Summary::new(
            SummaryType::Status,
            Message::new(format!("Removed {removed_count} invalid session(s)"))?,
        )?;
        emit_stdout(&OutputLine::Summary(summary))?;

        for name in invalid_names {
            let action = Action::new(
                ActionVerb::new("remove").map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
                ActionTarget::new(name).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
                ActionStatus::Completed,
            );
            emit_stdout(&OutputLine::Action(action))?;
        }
    } else {
        println!("Removed {removed_count} invalid session(s)");
        for name in invalid_names {
            println!("  - {name}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_invalid_options_default_values() {
        let opts = PruneInvalidOptions::default();
        assert!(!opts.yes);
        assert!(!opts.dry_run);
        assert!(opts.format.is_json());
    }
}
