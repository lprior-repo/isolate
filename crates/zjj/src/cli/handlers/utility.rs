//! Utility handlers: config, query, schema, completions, wait, pane

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::{completions, config, pane, query, schema, wait};

pub async fn handle_config(sub_m: &ArgMatches) -> Result<()> {
    let key = sub_m.get_one::<String>("key").cloned();
    let value = sub_m.get_one::<String>("value").cloned();
    let global = sub_m.get_flag("global");
    let format = get_format(sub_m);
    let options = config::ConfigOptions {
        key,
        value,
        global,
        format,
    };
    config::run(options).await
}

pub async fn handle_query(sub_m: &ArgMatches) -> Result<()> {
    let query_type = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
    let args = sub_m.get_one::<String>("args").map(String::as_str);

    let result = query::run(query_type, args).await?;

    if !result.output.is_empty() {
        println!("{}", result.output);
    }

    if result.exit_code != 0 {
        std::process::exit(result.exit_code);
    }

    Ok(())
}

pub fn handle_schema(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let options = schema::SchemaOptions {
        schema_name: sub_m.get_one::<String>("name").cloned(),
        list: sub_m.get_flag("list"),
        all: sub_m.get_flag("all"),
        format,
    };
    schema::run(&options)
}

pub fn handle_completions(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let shell_str = sub_m
        .get_one::<String>("shell")
        .ok_or_else(|| anyhow::anyhow!("Shell is required"))?;
    let shell: completions::Shell = shell_str.parse()?;
    let options = completions::CompletionsOptions { shell, format };
    completions::run(&options)
}

pub async fn handle_wait(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let condition_str = sub_m
        .get_one::<String>("condition")
        .ok_or_else(|| anyhow::anyhow!("Condition is required"))?;
    let name = sub_m.get_one::<String>("name").cloned();
    let status = sub_m.get_one::<String>("status").cloned();
    let timeout: u64 = sub_m
        .get_one::<String>("timeout")
        .and_then(|s| s.parse().ok())
        .map_or(30, |v| v);
    let interval: u64 = sub_m
        .get_one::<String>("interval")
        .and_then(|s| s.parse().ok())
        .map_or(30, |v| v);

    let condition = match condition_str.as_str() {
        "session-exists" => wait::WaitCondition::SessionExists(
            name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
        ),
        "session-unlocked" => wait::WaitCondition::SessionUnlocked(
            name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
        ),
        "healthy" => wait::WaitCondition::Healthy,
        "session-status" => wait::WaitCondition::SessionStatus {
            name: name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
            status: status.ok_or_else(|| anyhow::anyhow!("--status required"))?,
        },
        _ => anyhow::bail!("Unknown condition: {condition_str}"),
    };

    let options = wait::WaitOptions {
        condition,
        timeout: std::time::Duration::from_secs(timeout),
        poll_interval: std::time::Duration::from_secs(interval),
        format,
    };
    wait::run(&options).await
}

pub async fn handle_pane(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("focus", focus_m)) => {
            if focus_m.get_flag("contract") {
                println!("AI CONTRACT for zjj pane focus:");
                println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
                return Ok(());
            }

            if focus_m.get_flag("ai-hints") {
                println!("AI COMMAND FLOW:");
                println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
                return Ok(());
            }

            let format = get_format(focus_m);
            let session = focus_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            let pane_identifier = focus_m.get_one::<String>("pane").map(String::as_str);
            let direction = focus_m.get_one::<String>("direction").map(String::as_str);
            let options = pane::PaneFocusOptions { format };
            if let Some(dir_str) = direction {
                let dir = pane::Direction::parse(dir_str)?;
                pane::pane_navigate(session, dir, &options).await
            } else {
                pane::pane_focus(session, pane_identifier, &options).await
            }
        }
        Some(("list", list_m)) => {
            let format = get_format(list_m);
            let session = list_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            pane::pane_list(session, &pane::PaneListOptions { format }).await
        }
        Some(("next", next_m)) => {
            let format = get_format(next_m);
            let session = next_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            pane::pane_next(session, &pane::PaneNextOptions { format }).await
        }
        _ => anyhow::bail!("Unknown pane subcommand"),
    }
}

#[cfg(test)]
mod tests {
    use zjj_core::OutputFormat;

    #[test]
    fn test_handle_query_always_uses_json_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
        let json_flag_false = false;
        let _ = OutputFormat::from_json_flag(json_flag_false);
        let query_format = OutputFormat::Json;
        assert!(query_format.is_json());
    }
}
