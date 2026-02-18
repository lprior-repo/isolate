//! Checkpoint, undo, revert, recover, retry, and rollback handlers

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::cli::handlers::CommandExit;
use crate::commands::{checkpoint, recover, revert, undo};

pub async fn handle_checkpoint(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
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
    let format = get_format(sub_m);
    let args = undo::UndoArgs {
        dry_run: sub_m.get_flag("dry-run"),
        list: sub_m.get_flag("list"),
        format,
    };
    let options = args.to_options();
    match undo::run_with_options(&options).await {
        Ok(code) if (code as i32) == 0 => Ok(()),
        Ok(code) => Err(anyhow::Error::new(CommandExit::new(code as i32))),
        Err(error) => {
            let exit_code = match error {
                undo::UndoError::AlreadyPushedToRemote { .. } => {
                    undo::UndoExitCode::AlreadyPushed as i32
                }
                undo::UndoError::NoUndoHistory => undo::UndoExitCode::NoHistory as i32,
                undo::UndoError::InvalidState { .. } => undo::UndoExitCode::InvalidState as i32,
                _ => undo::UndoExitCode::OtherError as i32,
            };
            Err(anyhow::Error::new(CommandExit::new(exit_code)))
        }
    }
}

pub async fn handle_revert(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let format = get_format(sub_m);
    let args = revert::RevertArgs {
        session_name: name.clone(),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    let options = args.to_options();
    let exit_code = revert::run_with_options(&options).await? as i32;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

pub async fn handle_recover(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);

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
    let format = get_format(sub_m);
    recover::run_retry(&recover::RetryOptions { format }).await
}

pub async fn handle_rollback(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
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
    let exit_code = recover::run_rollback(&options).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}
