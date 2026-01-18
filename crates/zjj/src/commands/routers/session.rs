//! Session commands router
//!
//! Routes session management commands (add, remove, focus, list, status, add-batch)
//! All session commands interact with the session database and may interact with
//! JJ workspaces and Zellij tabs.

use anyhow::Result;

use crate::commands::{add, add_batch, focus, list, remove, status};

/// Handle session management commands
///
/// Routes session-related commands to their appropriate handlers.
/// Session commands manage the lifecycle of ZJJ sessions.
///
/// # Errors
///
/// Returns an error if the command execution fails
pub async fn handle_session_cmd(cmd: &str, sub_m: &clap::ArgMatches) -> Result<()> {
    match cmd {
        "add" => handle_add_cmd(sub_m).await,
        "add-batch" => handle_add_batch_cmd(sub_m).await,
        "list" => handle_list_cmd(sub_m).await,
        "remove" => handle_remove_cmd(sub_m).await,
        "focus" => handle_focus_cmd(sub_m).await,
        "status" => handle_status_cmd(sub_m).await,
        _ => Err(anyhow::anyhow!("Unknown session command: {cmd}")),
    }
}

/// Handle the 'add' command
async fn handle_add_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let options = add::AddOptions {
        name: name.clone(),
        no_hooks: sub_m.get_flag("no-hooks"),
        template: sub_m.get_one::<String>("template").cloned(),
        no_open: sub_m.get_flag("no-open"),
        json: sub_m.get_flag("json"),
        dry_run: sub_m.get_flag("dry-run"),
        bead: sub_m.get_one::<String>("bead").cloned(),
        revision: sub_m.get_one::<String>("revision").cloned(),
    };
    add::run_with_options(&options).await
}

/// Handle the 'add-batch' command
async fn handle_add_batch_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let options = add_batch::AddBatchOptions {
        beads_stdin: sub_m.get_flag("beads-stdin"),
        json: sub_m.get_flag("json"),
        no_open: sub_m.get_flag("no-open"),
        no_hooks: sub_m.get_flag("no-hooks"),
        template: sub_m.get_one::<String>("template").cloned(),
    };
    add_batch::run_with_options(&options).await
}

/// Handle the 'list' command
async fn handle_list_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let filter = list::ListFilter {
        bead_id: sub_m.get_one::<String>("filter-by-bead").cloned(),
        agent_id: sub_m.get_one::<String>("filter-by-agent").cloned(),
        with_beads: sub_m.get_flag("with-beads"),
        with_agents: sub_m.get_flag("with-agents"),
    };

    list::run(
        sub_m.get_flag("all"),
        sub_m.get_flag("json"),
        sub_m.get_flag("silent"),
        filter,
    )
    .await
}

/// Handle the 'remove' command
async fn handle_remove_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let options = remove::RemoveOptions {
        force: sub_m.get_flag("force"),
        merge: sub_m.get_flag("merge"),
        keep_branch: sub_m.get_flag("keep-branch"),
        json: sub_m.get_flag("json"),
        dry_run: sub_m.get_flag("dry-run"),
    };
    remove::run_with_options(name, &options).await
}

/// Handle the 'focus' command
async fn handle_focus_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    focus::run_with_options(
        name,
        &focus::FocusOptions {
            json: sub_m.get_flag("json"),
        },
    )
    .await
}

/// Handle the 'status' command
async fn handle_status_cmd(sub_m: &clap::ArgMatches) -> Result<()> {
    status::run(
        sub_m.get_one::<String>("name").map(String::as_str),
        sub_m.get_flag("json"),
        sub_m.get_flag("watch"),
    )
    .await
}
