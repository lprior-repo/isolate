//! Queue command handlers

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::str::FromStr;

use anyhow::Result;
use clap::ArgMatches;

use super::{
    domain::{AgentId, BeadId, Priority, QueueAction, QueueId, SessionName},
    json_format::get_format,
};
use crate::commands::{queue, queue_worker};

/// Parse clap arguments into a validated `QueueAction`
///
/// This function implements "parse at boundaries" - all validation happens here,
/// converting raw strings into validated domain types.
///
/// # Errors
///
/// Returns `anyhow::Error` if parsing or validation fails.
fn parse_queue_action(matches: &ArgMatches) -> Result<QueueAction> {
    // Check for explicit actions first
    if matches.get_flag("list") {
        return Ok(QueueAction::List);
    }

    if matches.get_flag("stats") {
        return Ok(QueueAction::Stats);
    }

    if matches.get_flag("next") {
        return Ok(QueueAction::Next);
    }

    if matches.get_flag("process") {
        return Ok(QueueAction::Process);
    }

    // Parse ID-based actions
    if let Some(id_str) = matches.get_one::<String>("status-id") {
        let id = QueueId::from_str(id_str)
            .map_err(|e| anyhow::anyhow!("Invalid status-id: {e}"))?;
        return Ok(QueueAction::StatusId { id });
    }

    if let Some(id_str) = matches.get_one::<String>("retry") {
        let id =
            QueueId::from_str(id_str).map_err(|e| anyhow::anyhow!("Invalid retry ID: {e}"))?;
        return Ok(QueueAction::Retry { id });
    }

    if let Some(id_str) = matches.get_one::<String>("cancel") {
        let id =
            QueueId::from_str(id_str).map_err(|e| anyhow::anyhow!("Invalid cancel ID: {e}"))?;
        return Ok(QueueAction::Cancel { id });
    }

    if let Some(id_str) = matches.get_one::<String>("reclaim-stale") {
        let id = QueueId::from_str(id_str)
            .map_err(|e| anyhow::anyhow!("Invalid reclaim-stale ID: {e}"))?;
        return Ok(QueueAction::ReclaimStale { id });
    }

    // Parse string-based actions
    if let Some(remove_str) = matches.get_one::<String>("remove") {
        let session = SessionName::from_str(remove_str)
            .map_err(|e| anyhow::anyhow!("Invalid remove session: {e}"))?;
        return Ok(QueueAction::Remove { session });
    }

    if let Some(status_str) = matches.get_one::<String>("status") {
        let session = SessionName::from_str(status_str)
            .map_err(|e| anyhow::anyhow!("Invalid status session: {e}"))?;
        return Ok(QueueAction::Status {
            session: Some(session),
        });
    }

    // Parse add action with validation
    if let Some(add_str) = matches.get_one::<String>("add") {
        let session =
            SessionName::from_str(add_str).map_err(|e| anyhow::anyhow!("Invalid add session: {e}"))?;

        let bead = matches
            .get_one::<String>("bead")
            .map(|s| BeadId::parse(s.as_str()))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid bead ID: {e}"))?;

        let priority = matches
            .get_one::<i32>("priority")
            .copied()
            .map(Priority::new)
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid priority: {e}"))?
            .unwrap_or_default();

        let agent = matches
            .get_one::<String>("agent")
            .map(|s| AgentId::from_str(s))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid agent ID: {e}"))?;

        return Ok(QueueAction::Add {
            session,
            bead,
            priority,
            agent,
        });
    }

    // If no explicit action but --status was used without a value, show stats
    if matches.contains_id("status") && matches.get_one::<String>("status").is_none() {
        return Ok(QueueAction::Status { session: None });
    }

    // Default to showing stats
    Ok(QueueAction::Stats)
}

/// Convert a validated `QueueAction` into legacy `QueueOptions`
///
/// This is a bridge function to maintain compatibility with the existing
/// command implementation. Eventually, the command implementation should
/// accept `QueueAction` directly.
fn queue_action_to_options(
    action: &QueueAction,
    format: zjj_core::OutputFormat,
) -> queue::QueueOptions {
    match action {
        QueueAction::List => queue::QueueOptions {
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
        },
        QueueAction::Stats => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: None,
            status: None,
            stats: true,
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Add {
            session,
            bead,
            priority,
            agent,
        } => queue::QueueOptions {
            format,
            add: Some(session.as_str().to_string()),
            bead_id: bead.as_ref().map(|b| b.as_str().to_string()),
            priority: priority.value(),
            agent_id: agent.as_ref().map(|a| a.as_str().to_string()),
            list: false,
            process: false,
            next: false,
            remove: None,
            status: None,
            stats: false,
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Remove { session } => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: Some(session.as_str().to_string()),
            status: None,
            stats: false,
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Status { session } => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: None,
            status: session.as_ref().map(|s| s.as_str().to_string()),
            stats: session.is_none(),
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Next => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: true,
            remove: None,
            status: None,
            stats: false,
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Process => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: true,
            next: false,
            remove: None,
            status: None,
            stats: false,
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Retry { id } => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: None,
            status: None,
            stats: false,
            status_id: None,
            retry: Some(id.value()),
            cancel: None,
            reclaim_stale: None,
        },
        QueueAction::Cancel { id } => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: None,
            status: None,
            stats: false,
            status_id: None,
            retry: None,
            cancel: Some(id.value()),
            reclaim_stale: None,
        },
        QueueAction::ReclaimStale { id } => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: None,
            status: None,
            stats: false,
            status_id: None,
            retry: None,
            cancel: None,
            reclaim_stale: Some(id.value()),
        },
        QueueAction::StatusId { id } => queue::QueueOptions {
            format,
            add: None,
            bead_id: None,
            priority: 5,
            agent_id: None,
            list: false,
            process: false,
            next: false,
            remove: None,
            status: None,
            stats: false,
            status_id: Some(id.value()),
            retry: None,
            cancel: None,
            reclaim_stale: None,
        },
    }
}

pub async fn handle_queue(sub_m: &ArgMatches) -> Result<()> {
    if let Some((subcommand_name, subcommand_matches)) = sub_m.subcommand() {
        return match subcommand_name {
            "list" => handle_queue_list(subcommand_matches).await,
            "worker" => handle_queue_worker(subcommand_matches).await,
            "enqueue" => handle_queue_enqueue(subcommand_matches).await,
            "dequeue" => handle_queue_dequeue(subcommand_matches).await,
            "status" => handle_queue_status(subcommand_matches).await,
            "process" => handle_queue_process(subcommand_matches).await,
            _ => Err(anyhow::anyhow!(
                "Unknown queue subcommand: {subcommand_name}",
            )),
        };
    }

    let format = get_format(sub_m);
    let action = parse_queue_action(sub_m)?;
    let options = queue_action_to_options(&action, format);
    queue::run_with_options(&options).await
}

pub async fn handle_queue_list(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let action = QueueAction::List;
    let options = queue_action_to_options(&action, format);
    queue::run_with_options(&options).await
}

pub async fn handle_queue_enqueue(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session_str = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("session name is required for enqueue"))?;

    let session = SessionName::from_str(session_str)
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))?;

    let action = QueueAction::Add {
        session,
        bead: None,
        priority: Priority::default(),
        agent: None,
    };
    let options = queue_action_to_options(&action, format);
    queue::run_with_options(&options).await
}

pub async fn handle_queue_dequeue(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session_str = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("session name is required for dequeue"))?;

    let session = SessionName::from_str(session_str)
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))?;

    let action = QueueAction::Remove { session };
    let options = queue_action_to_options(&action, format);
    queue::run_with_options(&options).await
}

pub async fn handle_queue_status(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session_opt = sub_m
        .get_one::<String>("session")
        .map(|s| SessionName::from_str(s))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))?;

    let action = session_opt.map_or(
        QueueAction::Stats,
        |session| QueueAction::Status {
            session: Some(session),
        },
    );
    let options = queue_action_to_options(&action, format);
    queue::run_with_options(&options).await
}

pub async fn handle_queue_process(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let dry_run = sub_m.get_flag("dry-run");

    let action = if dry_run {
        QueueAction::Next
    } else {
        QueueAction::Process
    };
    let options = queue_action_to_options(&action, format);
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
