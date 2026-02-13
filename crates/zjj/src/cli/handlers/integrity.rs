//! Integrity, doctor, clean, and prune handlers

use anyhow::Result;
use clap::ArgMatches;
use zjj_core::OutputFormat;

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
            let json = validate_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
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
            let json = repair_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
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
                let json = list_m.get_flag("json");
                let format = OutputFormat::from_json_flag(json);
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
                let json = restore_m.get_flag("json");
                let format = OutputFormat::from_json_flag(json);
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

pub async fn handle_doctor(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let fix = sub_m.get_flag("fix");
    let dry_run = sub_m.get_flag("dry-run");
    let verbose = sub_m.get_flag("verbose");
    doctor::run(format, fix, dry_run, verbose).await
}

pub async fn handle_clean(sub_m: &ArgMatches) -> Result<()> {
    let force = sub_m.get_flag("force");
    let dry_run = sub_m.get_flag("dry-run");
    let periodic = sub_m.get_flag("periodic");
    let json = sub_m.get_flag("json");
    let age_threshold = sub_m
        .get_one::<String>("age-threshold")
        .and_then(|s| s.parse::<u64>().ok());
    let format = OutputFormat::from_json_flag(json);
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
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = prune_invalid::PruneInvalidOptions {
        yes,
        dry_run,
        format,
    };
    prune_invalid::run(&options).await
}
