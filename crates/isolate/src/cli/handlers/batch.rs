//! Batch and events handlers

use anyhow::Result;
use clap::ArgMatches;
use futures::{StreamExt, TryStreamExt};
use isolate_core::OutputFormat;

use super::json_format::get_format;
use crate::commands::{batch, events, get_session_db};

pub async fn handle_batch(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let file = sub_m.get_one::<String>("file").cloned();
    let atomic = sub_m.get_flag("atomic");
    let stop_on_error = sub_m.get_flag("stop-on-error");
    let dry_run = sub_m.get_flag("dry-run");

    if atomic {
        return handle_atomic_batch(sub_m, format, file, stop_on_error, dry_run).await;
    }

    let commands = if let Some(file_path) = file {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file: {e}"))?;
        parse_legacy_batch_commands(&content)?
    } else {
        let raw_commands: Vec<String> = sub_m
            .get_many::<String>("commands")
            .map(|v| v.cloned().collect())
            .unwrap_or_default();
        if raw_commands.is_empty() {
            anyhow::bail!("No commands provided. Use --file or provide commands as arguments");
        }
        parse_legacy_batch_commands(&raw_commands.join("\n"))?
    };

    futures::stream::iter(commands.iter().enumerate())
        .map(Ok)
        .try_fold((), |(), (index, command_str)| async move {
            let parts: Vec<&str> = command_str.split_whitespace().collect();
            if parts.is_empty() {
                return Ok(());
            }

            let (cmd, args) = if parts[0] == "isolate" {
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!(
                        "Empty command after 'isolate' at index {index}"
                    ));
                }
                (parts[1], &parts[2..])
            } else {
                (parts[0], &parts[1..])
            };

            if dry_run {
                println!(
                    "Would execute command {index}: isolate {} {}",
                    cmd,
                    args.join(" ")
                );
                return Ok(());
            }

            let output = tokio::process::Command::new("isolate")
                .arg(cmd)
                .args(args)
                .output()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to execute: {e}"))?;

            if output.status.success() {
                println!(
                    "Command {index}: {}",
                    String::from_utf8_lossy(&output.stdout).trim()
                );
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let error_msg = if stderr.is_empty() { stdout } else { stderr };
                eprintln!("Command {index} failed: {error_msg}");
                if stop_on_error {
                    Err(anyhow::anyhow!("Batch failed at command {index}"))
                } else {
                    Ok(())
                }
            }
        })
        .await
}

async fn handle_atomic_batch(
    sub_m: &ArgMatches,
    format: OutputFormat,
    file: Option<String>,
    _stop_on_error: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    use crate::commands::batch::execute_batch;
    let db = get_session_db().await?;
    let request = if let Some(file_path) = file {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file: {e}"))?;
        let mut req = serde_json::from_str::<batch::BatchRequest>(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse batch request: {e}"))?;
        // CLI flag overrides file content
        if dry_run {
            req.dry_run = true;
        }
        req
    } else {
        let raw_commands: Vec<String> = sub_m
            .get_many::<String>("commands")
            .map(|v| v.cloned().collect())
            .unwrap_or_default();
        if raw_commands.is_empty() {
            anyhow::bail!("No commands provided. Use --file or provide commands as arguments");
        }
        let operations = raw_commands
            .iter()
            .enumerate()
            .filter_map(|(index, cmd_str)| {
                let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }
                let cmd = parts[0];
                let args: Vec<String> = parts[1..]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                Some(batch::BatchOperation {
                    command: cmd.to_string(),
                    args,
                    id: Some(format!("op-{}", index + 1)),
                    optional: false,
                })
            })
            .collect();
        batch::BatchRequest {
            atomic: true,
            dry_run,
            operations,
        }
    };
    execute_batch(request, db.pool(), format).await?;
    Ok(())
}

fn parse_legacy_batch_commands(input: &str) -> anyhow::Result<Vec<String>> {
    let commands: Vec<String> = input
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .map(|line| line.trim().to_string())
        .collect();
    if commands.is_empty() {
        anyhow::bail!("No valid commands found");
    }
    Ok(commands)
}

pub async fn handle_events(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m.get_one::<String>("session").cloned();
    let event_type = sub_m.get_one::<String>("type").cloned();
    let limit = sub_m.get_one::<usize>("limit").copied();
    let follow = sub_m.get_flag("follow");
    let options = events::EventsOptions {
        session,
        event_type,
        limit,
        follow,
        since: None,
        format,
    };
    events::run(&options).await
}
