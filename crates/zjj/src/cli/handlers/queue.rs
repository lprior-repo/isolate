//! Queue command handlers

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::{queue, queue_worker};

fn has_flag_with_optional_value(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))
}

pub async fn handle_queue(sub_m: &ArgMatches) -> Result<()> {
    if let Some((subcommand_name, subcommand_matches)) = sub_m.subcommand() {
        return match subcommand_name {
            "list" => handle_queue_list(subcommand_matches).await,
            "worker" => handle_queue_worker(subcommand_matches).await,
            _ => Err(anyhow::anyhow!(
                "Unknown queue subcommand: {}",
                subcommand_name
            )),
        };
    }

    let format = get_format(sub_m);
    let add = sub_m.get_one::<String>("add").cloned();
    let bead_id = sub_m.get_one::<String>("bead").cloned();
    let agent_id = sub_m.get_one::<String>("agent").cloned();
    let priority = sub_m.get_one::<i32>("priority").copied().unwrap_or(5);

    let used_add_only_flags_without_add = add.is_none()
        && ["--bead", "--priority", "--agent"]
            .into_iter()
            .any(has_flag_with_optional_value);

    if used_add_only_flags_without_add {
        anyhow::bail!("--bead, --priority, and --agent require --add");
    }

    let options = queue::QueueOptions {
        format,
        add,
        bead_id,
        priority,
        agent_id,
        list: sub_m.get_flag("list"),
        process: sub_m.get_flag("process"),
        next: sub_m.get_flag("next"),
        remove: sub_m.get_one::<String>("remove").cloned(),
        status: sub_m.get_one::<String>("status").cloned(),
        stats: sub_m.get_flag("stats"),
        status_id: sub_m.get_one::<i64>("status-id").copied(),
        retry: sub_m.get_one::<i64>("retry").copied(),
        cancel: sub_m.get_one::<i64>("cancel").copied(),
        reclaim_stale: sub_m.get_one::<i64>("reclaim-stale").copied(),
    };
    queue::run_with_options(&options).await
}

pub async fn handle_queue_list(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let options = queue::QueueOptions {
        format,
        add: None,
        bead_id: None,
        priority: 5,
        agent_id: None,
        list: true,
        process: false,
        next: false,
        remove: None,
        status: None,
        stats: false,
        status_id: None,
        retry: None,
        cancel: None,
        reclaim_stale: None,
    };
    queue::run_with_options(&options).await
}

pub async fn handle_queue_worker(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let options = queue_worker::WorkerOptions {
        loop_mode: sub_m.get_flag("loop"),
        once: sub_m.get_flag("once"),
        interval_secs: sub_m.get_one::<u64>("interval").copied().unwrap_or(10),
        worker_id: sub_m.get_one::<String>("worker-id").cloned(),
        format,
    };

    let exit_code = queue_worker::run_with_options(&options).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}
