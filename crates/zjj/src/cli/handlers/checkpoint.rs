//! Checkpoint, undo, revert, recover, retry, and rollback handlers

use anyhow::Result;
use clap::ArgMatches;
use zjj_core::OutputFormat;

use crate::commands::{checkpoint, recover, revert, undo};

pub async fn handle_checkpoint(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let action = match sub_m.subcommand() {
        Some(("create", create_m)) => checkpoint::CheckpointAction::Create {
            description: create_m.get_one::<String>("description").cloned(),
        },
        Some(("restore", restore_m)) => checkpoint::CheckpointAction::Restore {
            checkpoint_id: restore_m
                .get_one::<String>("checkpoint_id")
                .ok_or_else(|| anyhow::anyhow!("Checkpoint ID is required"))?
                .clone(),
        },
        Some(("list", _)) => checkpoint::CheckpointAction::List,
        _ => anyhow::bail!("Unknown checkpoint subcommand"),
    };
    let args = checkpoint::CheckpointArgs { action, format };
    checkpoint::run(&args).await
}

pub async fn handle_undo(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let args = undo::UndoArgs {
        dry_run: sub_m.get_flag("dry-run"),
        list: sub_m.get_flag("list"),
        format,
    };
    let options = args.to_options();
    undo::run_with_options(&options)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn handle_revert(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let args = revert::RevertArgs {
        session_name: name.clone(),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    let options = args.to_options();
    revert::run_with_options(&options)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn handle_recover(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);

    let session = sub_m.get_one::<String>("session").cloned();
    let operation = sub_m.get_one::<String>("op").cloned();
    let last = sub_m.get_flag("last");
    let list_ops = sub_m.get_flag("list-ops");

    if session.is_some() || operation.is_some() || last || list_ops {
        let list_only = list_ops || operation.as_ref().is_none_or(|_| false) && !last;
        let options = recover::OpRecoverOptions {
            session,
            operation: operation.clone(),
            last,
            list_only,
            format,
        };
        recover::run_op_recover(&options).await
    } else {
        let options = recover::RecoverOptions {
            diagnose_only: sub_m.get_flag("diagnose"),
            format,
        };
        recover::run_recover(&options).await
    }
}

pub async fn handle_retry(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    recover::run_retry(&recover::RetryOptions { format }).await
}

pub async fn handle_rollback(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m
        .get_one::<String>("session")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
    let checkpoint = sub_m
        .get_one::<String>("to")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Checkpoint name is required"))?;
    let dry_run = sub_m.get_flag("dry-run");
    let options = recover::RollbackOptions {
        session,
        checkpoint,
        dry_run,
        format,
    };
    recover::run_rollback(&options).await
}
