//! Integrity, doctor, clean, and prune handlers

use std::path::Path;

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::{clean, doctor, integrity, prune_invalid};

pub async fn handle_integrity(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("validate", validate_m)) => {
            let workspace = validate_m
                .get_one::<String>("workspace")
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("Workspace argument is required for validate command")
                })?;
            let format = get_format(validate_m);
            integrity::run(&integrity::IntegrityOptions {
                subcommand: integrity::IntegritySubcommand::Validate { workspace },
                format,
            })
            .await
        }
        Some(("repair", repair_m)) => {
            let workspace = repair_m
                .get_one::<String>("workspace")
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("Workspace argument is required for repair command")
                })?;
            let force = repair_m.get_flag("force");
            let rebind = repair_m.get_flag("rebind");
            let format = get_format(repair_m);
            integrity::run(&integrity::IntegrityOptions {
                subcommand: integrity::IntegritySubcommand::Repair {
                    workspace,
                    force,
                    rebind,
                },
                format,
            })
            .await
        }
        Some(("backup", backup_m)) => match backup_m.subcommand() {
            Some(("list", list_m)) => {
                let format = get_format(list_m);
                integrity::run(&integrity::IntegrityOptions {
                    subcommand: integrity::IntegritySubcommand::BackupList,
                    format,
                })
                .await
            }
            Some(("restore", restore_m)) => {
                let backup_id = restore_m
                    .get_one::<String>("backup_id")
                    .ok_or_else(|| anyhow::anyhow!("Backup ID is required"))?
                    .clone();
                let force = restore_m.get_flag("force");
                let format = get_format(restore_m);
                integrity::run(&integrity::IntegrityOptions {
                    subcommand: integrity::IntegritySubcommand::BackupRestore { backup_id, force },
                    format,
                })
                .await
            }
            _ => Err(anyhow::anyhow!("Unknown backup subcommand")),
        },
        _ => Err(anyhow::anyhow!("Unknown integrity subcommand")),
    }
}

/// Main doctor command dispatcher - routes to subcommands
pub async fn handle_doctor(sub_m: &ArgMatches) -> Result<()> {
    // Match on subcommand first
    match sub_m.subcommand() {
        // isolate doctor check - run all health checks
        Some(("check", check_m)) => {
            let format = get_format(check_m);
            doctor::run(format.is_json(), false, false, false).await
        }
        // isolate doctor fix - auto-fix issues
        Some(("fix", fix_m)) => {
            let format = get_format(fix_m);
            let dry_run = fix_m.get_flag("dry-run");
            let verbose = fix_m.get_flag("verbose");
            doctor::run(format.is_json(), true, dry_run, verbose).await
        }
        // isolate doctor integrity - run database integrity check
        Some(("integrity", integrity_m)) => {
            let format = get_format(integrity_m);
            run_db_integrity_check(format.is_json()).await
        }
        // isolate doctor clean - remove stale sessions
        Some(("clean", clean_m)) => {
            let format = get_format(clean_m);
            let force = clean_m.get_flag("force");
            let dry_run = clean_m.get_flag("dry-run");
            let options = clean::CleanOptions {
                force,
                dry_run,
                format,
                periodic: false,
                age_threshold: None,
            };
            clean::run_with_options(&options).await
        }
        // No subcommand - legacy mode (doctor with flags)
        None => {
            // Check if any legacy flags are present
            let format = get_format(sub_m);
            let fix = sub_m.get_flag("fix");
            let dry_run = sub_m.get_flag("dry-run");
            let verbose = sub_m.get_flag("verbose");

            // If no flags, run check mode
            if !fix && !dry_run && !verbose {
                doctor::run(format.is_json(), false, false, false).await
            } else {
                doctor::run(format.is_json(), fix, dry_run, verbose).await
            }
        }
        // Unknown subcommand
        _ => {
            let available = ["check", "fix", "integrity", "clean"];
            Err(anyhow::anyhow!(
                "Unknown doctor subcommand. Available: {}",
                available.join(", ")
            ))
        }
    }
}

/// Run database integrity check only (PRAGMA integrity_check)
async fn run_db_integrity_check(json_output: bool) -> Result<()> {
    use isolate_core::output::{emit_stdout, Issue, IssueId, IssueKind, IssueSeverity, IssueTitle};

    let db_path = Path::new(".isolate/state.db");

    // Check if database exists
    let exists = tokio::fs::try_exists(db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check database file: {}", e))?;

    if !exists {
        return Err(anyhow::anyhow!(
            "Database file not found at {}. Run 'isolate init' first.",
            db_path.display()
        ));
    }

    // Run integrity check
    let result = doctor::run_integrity_check(db_path).await;

    match result {
        Ok(details) => {
            let message = format!("Database integrity: {}", details);
            let issue = Issue::new(
                IssueId::new("db_integrity")?,
                IssueTitle::new(&message)?,
                IssueKind::Validation,
                IssueSeverity::Hint,
            )?;
            emit_stdout(&isolate_core::output::OutputLine::Issue(issue))?;

            if json_output {
                println!(r#"{{"status":"pass","message":"{}"}}"#, details);
            } else {
                println!("✓ Database integrity check passed");
            }
            Ok(())
        }
        Err(error_msg) => {
            let issue = Issue::new(
                IssueId::new("db_integrity")?,
                IssueTitle::new(format!("Database integrity: {}", error_msg))?,
                IssueKind::StateConflict,
                IssueSeverity::Error,
            )?
            .with_suggestion("Run 'isolate doctor fix' to attempt recovery".to_string());
            emit_stdout(&isolate_core::output::OutputLine::Issue(issue))?;

            Err(anyhow::anyhow!(
                "Database integrity check failed: {}",
                error_msg
            ))
        }
    }
}

pub async fn handle_clean(sub_m: &ArgMatches) -> Result<()> {
    let force = sub_m.get_flag("force");
    let dry_run = sub_m.get_flag("dry-run");
    let periodic = sub_m.get_flag("periodic");
    let format = get_format(sub_m);
    let age_threshold = sub_m.get_one::<u64>("age-threshold").copied();
    let options = clean::CleanOptions {
        force,
        dry_run,
        format,
        periodic,
        age_threshold,
    };
    clean::run_with_options(&options).await
}

pub async fn handle_prune_invalid(sub_m: &ArgMatches) -> Result<()> {
    let yes = sub_m.get_flag("yes");
    let dry_run = sub_m.get_flag("dry-run");
    let format = get_format(sub_m);
    let options = prune_invalid::PruneInvalidOptions {
        yes,
        dry_run,
        format,
    };
    prune_invalid::run(&options).await
}
