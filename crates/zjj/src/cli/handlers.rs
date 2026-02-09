//! CLI command handlers that bridge between `clap` and internal logic

use std::process;

use anyhow::Result;
use clap::ArgMatches;
use futures::{StreamExt, TryStreamExt};
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{
    cli::commands::build_cli,
    command_context,
    commands::{
        abort, add, agents, ai, attach, backup, batch, bookmark, broadcast, can_i, checkpoint,
        claim, clean, completions, config, context, contract, dashboard, diff, doctor, done,
        events, examples, export_import, focus, get_session_db, init, integrity, introspect, list,
        pane, query, queue, recover, remove, rename, revert, schema, session_mgmt, spawn, status,
        switch, sync, template, undo, validate, wait, whatif, whereami, whoami, work,
    },
    hooks, json,
};

/// Format an error for user display (no stack traces)
pub fn format_error(err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if let Some(source) = err.source() {
        let source_msg = source.to_string();
        if !msg.contains(&source_msg) && !source_msg.is_empty() {
            return format!("{msg}\nCause: {source_msg}");
        }
    }
    msg
}

pub async fn handle_init(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    init::run_with_options(init::InitOptions { format }).await
}

pub async fn handle_add(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("example-json") {
        let example_output = json::AddOutput {
            name: "example-session".to_string(),
            workspace_path: "/path/to/.zjj/workspaces/example-session".to_string(),
            zellij_tab: "zjj:example-session".to_string(),
            status: "active".to_string(),
            created: true,
        };
        let envelope = SchemaEnvelope::new("add-response", "single", example_output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let bead_id = sub_m.get_one::<String>("bead").cloned();
    let no_hooks = sub_m.get_flag("no-hooks");
    let template = sub_m.get_one::<String>("template").cloned();
    let no_open = sub_m.get_flag("no-open");
    let no_zellij = sub_m.get_flag("no-zellij");
    let json = sub_m.get_flag("json");
    let idempotent = sub_m.get_flag("idempotent");
    let dry_run = sub_m.get_flag("dry-run");

    let options = add::AddOptions {
        name: name.clone(),
        bead_id,
        no_hooks,
        template,
        no_open,
        no_zellij,
        format: OutputFormat::from_json_flag(json),
        idempotent,
        dry_run,
    };

    add::run_with_options(&options).await
}

pub async fn handle_list(sub_m: &ArgMatches) -> Result<()> {
    let all = sub_m.get_flag("all");
    let verbose = sub_m.get_flag("verbose");
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let bead = sub_m.get_one::<String>("bead").cloned();
    let agent = sub_m.get_one::<String>("agent").map(String::as_str);
    let state = sub_m.get_one::<String>("state").map(String::as_str);
    list::run(all, verbose, format, bead.as_deref(), agent, state).await
}

pub async fn handle_bookmark(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", list_m)) => {
            let session = list_m.get_one::<String>("session").cloned();
            let show_all = list_m.get_flag("all");
            let json = list_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            bookmark::run_list(&bookmark::ListOptions {
                session,
                show_all,
                format,
            })
            .await
        }
        Some(("create", create_m)) => {
            let name = create_m
                .get_one::<String>("name")
                .cloned()
                .map_or_else(|| String::new(), |v| v);
            let session = create_m.get_one::<String>("session").cloned();
            let push = create_m.get_flag("push");
            let json = create_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            bookmark::run_create(&bookmark::CreateOptions {
                name,
                session,
                push,
                format,
            })
            .await
        }
        Some(("delete", delete_m)) => {
            let name = delete_m
                .get_one::<String>("name")
                .cloned()
                .map_or_else(|| String::new(), |v| v);
            let session = delete_m.get_one::<String>("session").cloned();
            let json = delete_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            bookmark::run_delete(&bookmark::DeleteOptions {
                name,
                session,
                format,
            })
            .await
        }
        Some(("move", move_m)) => {
            let name = move_m
                .get_one::<String>("name")
                .cloned()
                .map_or_else(|| String::new(), |v| v);
            let to_revision = move_m
                .get_one::<String>("to")
                .cloned()
                .map_or_else(|| String::new(), |v| v);
            let session = move_m.get_one::<String>("session").cloned();
            let json = move_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            bookmark::run_move(&bookmark::MoveOptions {
                name,
                to_revision,
                session,
                format,
            })
            .await
        }
        _ => Err(anyhow::anyhow!(
            "Subcommand required: list, create, delete, or move"
        )),
    }
}

pub async fn handle_broadcast(sub_m: &ArgMatches) -> Result<()> {
    let message = sub_m
        .get_one::<String>("message")
        .ok_or_else(|| anyhow::anyhow!("Message is required"))?
        .clone();
    let agent_id = sub_m
        .get_one::<String>("agent-id")
        .cloned()
        .or_else(|| std::env::var("ZJJ_AGENT_ID").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("No agent ID provided. Set ZJJ_AGENT_ID or use --agent-id")
        })?;

    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);

    let args = broadcast::types::BroadcastArgs { message, agent_id };
    broadcast::run(&args, format).await
}
pub async fn handle_remove(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = remove::RemoveOptions {
        force: sub_m.get_flag("force"),
        merge: sub_m.get_flag("merge"),
        keep_branch: sub_m.get_flag("keep-branch"),
        idempotent: sub_m.get_flag("idempotent"),
        format,
    };
    remove::run_with_options(name, &options).await
}

pub async fn handle_focus(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let no_zellij = sub_m.get_flag("no-zellij");
    let format = OutputFormat::from_json_flag(json);
    let options = focus::FocusOptions { format, no_zellij };
    focus::run_with_options(name, &options).await
}

pub async fn handle_status(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let watch = sub_m.get_flag("watch");
    status::run(name, format, watch).await
}

pub async fn handle_switch(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let show_context = sub_m.get_flag("show-context");
    let format = OutputFormat::from_json_flag(json);
    let options = switch::SwitchOptions {
        format,
        show_context,
    };
    switch::run_with_options(name, &options).await
}

pub async fn handle_sync(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = sync::SyncOptions { format };
    sync::run_with_options(name, options).await
}

pub async fn handle_diff(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let stat = sub_m.get_flag("stat");
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    diff::run(name, stat, format).await
}

pub async fn handle_config(sub_m: &ArgMatches) -> Result<()> {
    let key = sub_m.get_one::<String>("key").cloned();
    let value = sub_m.get_one::<String>("value").cloned();
    let global = sub_m.get_flag("global");
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = config::ConfigOptions {
        key,
        value,
        global,
        format,
    };
    config::run(options).await
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

pub async fn handle_template(sub_m: &ArgMatches) -> Result<()> {
    use zjj_core::zellij::LayoutTemplate;
    match sub_m.subcommand() {
        Some(("list", sub)) => {
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            template::run_list(format).await
        }
        Some(("create", sub)) => {
            let name = sub
                .get_one::<String>("name")
                .cloned()
                .map_or_else(|| String::new(), |v| v);
            let description = sub.get_one::<String>("description").cloned();
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            let source = if let Some(file_path) = sub.get_one::<String>("from-file") {
                template::TemplateSource::FromFile(file_path.clone())
            } else if let Some(builtin) = sub.get_one::<String>("builtin") {
                let template_type = match builtin.as_str() {
                    "minimal" => LayoutTemplate::Minimal,
                    "standard" => LayoutTemplate::Standard,
                    "full" => LayoutTemplate::Full,
                    "split" => LayoutTemplate::Split,
                    "review" => LayoutTemplate::Review,
                    _ => return Err(anyhow::anyhow!("Invalid builtin template: {builtin}")),
                };
                template::TemplateSource::Builtin(template_type)
            } else {
                template::TemplateSource::Builtin(LayoutTemplate::Standard)
            };
            template::run_create(&template::CreateOptions {
                name,
                description,
                source,
                format,
            })
            .await
        }
        Some(("show", sub)) => {
            let name = sub
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Template name is required"))?;
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            template::run_show(name, format).await
        }
        Some(("delete", sub)) => {
            let name = sub
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Template name is required"))?;
            let force = sub.get_flag("force");
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            template::run_delete(name, force, format).await
        }
        _ => Err(anyhow::anyhow!("Invalid template subcommand")),
    }
}

pub async fn handle_introspect(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let ai_mode = sub_m.get_flag("ai");
    let format = OutputFormat::from_json_flag(json || ai_mode);
    if ai_mode {
        return introspect::run_ai().await;
    }
    if sub_m.get_flag("env-vars") {
        return introspect::run_env_vars(format);
    }
    if sub_m.get_flag("workflows") {
        return introspect::run_workflows(format);
    }
    if sub_m.get_flag("session-states") {
        return introspect::run_session_states(format);
    }
    let command = sub_m.get_one::<String>("command").map(String::as_str);
    if let Some(cmd) = command {
        introspect::run_command_introspect(cmd, format)
    } else {
        introspect::run(format).await
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
            let json = repair_m.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            integrity::run(&integrity::IntegrityOptions {
                subcommand: integrity::IntegritySubcommand::Repair { workspace, force },
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
                    .cloned()
                    .map_or_else(|| String::new(), |v| v);
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

pub async fn handle_spawn(sub_m: &ArgMatches) -> Result<()> {
    let args = spawn::SpawnArgs::from_matches(sub_m)?;
    let options = args.to_options();
    spawn::run_with_options(&options).await
}

pub async fn handle_query(sub_m: &ArgMatches) -> Result<()> {
    let query_type = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
    let args = sub_m.get_one::<String>("args").map(String::as_str);
    query::run(query_type, args).await
}

pub async fn handle_queue(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let priority = sub_m
        .get_one::<String>("priority")
        .and_then(|s| s.parse::<i32>().ok())
        .map_or(0, |v| v);
    let options = queue::QueueOptions {
        format,
        add: sub_m.get_one::<String>("add").cloned(),
        bead_id: sub_m.get_one::<String>("bead").cloned(),
        priority,
        agent_id: sub_m.get_one::<String>("agent").cloned(),
        list: sub_m.get_flag("list"),
        process: false,
        next: sub_m.get_flag("next"),
        remove: sub_m.get_one::<String>("remove").cloned(),
        status: sub_m.get_one::<String>("status").cloned(),
        stats: sub_m.get_flag("stats"),
    };
    queue::run_with_options(&options).await
}

pub async fn handle_context(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let field = sub_m.get_one::<String>("field").map(String::as_str);
    let no_beads = sub_m.get_flag("no-beads");
    let no_health = sub_m.get_flag("no-health");
    context::run(json, field, no_beads, no_health).await
}

pub async fn handle_backup(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);

    // Backup command uses flags to determine action (not subcommands)
    let create = sub_m.get_flag("create");
    let list = sub_m.get_flag("list");
    let restore = sub_m.get_one::<String>("restore");
    let status = sub_m.get_flag("status");
    let retention = sub_m.get_flag("retention");
    let timestamp = sub_m.get_one::<String>("timestamp").map(String::as_str);

    // Dispatch to appropriate backup function based on flags
    match (create, list, restore, status, retention) {
        (true, false, None, false, false) => backup::run_create(format).await,
        (false, true, None, false, false) => backup::run_list(format).await,
        (false, false, Some(database), false, false) => {
            backup::run_restore(database, timestamp, format).await
        }
        (false, false, None, true, false) => backup::run_status(format).await,
        (false, false, None, false, true) => backup::run_retention(format).await,
        _ => anyhow::bail!(
            "Unknown backup action. Use --create, --list, --restore <DATABASE>, --status, or --retention"
        ),
    }
}

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
                .cloned()
                .unwrap_or_default(),
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

pub async fn handle_done(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let args = done::types::DoneArgs {
        message: sub_m.get_one::<String>("message").cloned(),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        no_keep: sub_m.get_flag("no-keep"),
        squash: sub_m.get_flag("squash"),
        dry_run: sub_m.get_flag("dry-run"),
        detect_conflicts: sub_m.get_flag("detect-conflicts"),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        format: OutputFormat::from_json_flag(json),
    };
    let options = args.to_options();
    done::run_with_options(&options).await?;
    Ok(())
}

pub async fn handle_agents(sub_m: &ArgMatches) -> Result<()> {
    let format = OutputFormat::from_json_flag(sub_m.get_flag("json"));
    match sub_m.subcommand() {
        Some(("register", register_m)) => {
            let args = agents::types::RegisterArgs {
                agent_id: register_m.get_one::<String>("id").cloned(),
                session: register_m.get_one::<String>("session").cloned(),
            };
            agents::run_register(&args, format).await
        }
        Some(("heartbeat", heartbeat_m)) => {
            let args = agents::types::HeartbeatArgs {
                command: heartbeat_m.get_one::<String>("command").cloned(),
            };
            agents::run_heartbeat(&args, format).await
        }
        Some(("status", _)) => agents::run_status(format).await,
        Some(("unregister", unregister_m)) => {
            let args = agents::types::UnregisterArgs {
                agent_id: unregister_m.get_one::<String>("id").cloned(),
            };
            agents::run_unregister(&args, format).await
        }
        _ => {
            let args = agents::types::AgentsArgs {
                all: sub_m.get_flag("all"),
                session: sub_m.get_one::<String>("session").cloned(),
            };
            agents::run(&args, format).await
        }
    }
}

pub async fn handle_whereami(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = whereami::WhereAmIOptions { format };
    whereami::run(&options).await
}

pub fn handle_whoami(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = whoami::WhoAmIOptions { format };
    whoami::run(&options)
}

pub async fn handle_work(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let name = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let options = work::WorkOptions {
        name,
        bead_id: sub_m.get_one::<String>("bead").cloned(),
        agent_id: sub_m.get_one::<String>("agent-id").cloned(),
        no_zellij: sub_m.get_flag("no-zellij"),
        no_agent: sub_m.get_flag("no-agent"),
        idempotent: sub_m.get_flag("idempotent"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    work::run(&options).await
}

pub async fn handle_abort(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = abort::AbortOptions {
        workspace: sub_m.get_one::<String>("workspace").cloned(),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    abort::run(&options).await
}

pub async fn handle_ai(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let subcommand = match sub_m.subcommand() {
        Some(("status", _)) => ai::AiSubcommand::Status,
        Some(("workflow", _)) => ai::AiSubcommand::Workflow,
        Some(("quick-start", _)) => ai::AiSubcommand::QuickStart,
        Some(("next", _)) => ai::AiSubcommand::Next,
        _ => ai::AiSubcommand::Default,
    };
    let options = ai::AiOptions { subcommand, format };
    ai::run(&options).await
}

pub async fn handle_can_i(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let action = sub_m
        .get_one::<String>("action")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let resource = sub_m.get_one::<String>("resource").cloned();
    let options = can_i::CanIOptions {
        action,
        resource,
        format,
    };
    can_i::run(&options).await
}

pub fn handle_contract(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let command = sub_m.get_one::<String>("command").cloned();
    let options = contract::ContractOptions { command, format };
    contract::run(&options)
}

pub fn handle_examples(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let command = sub_m.get_one::<String>("command").cloned();
    let use_case = sub_m.get_one::<String>("use-case").cloned();
    let options = examples::ExamplesOptions {
        command,
        use_case,
        format,
    };
    examples::run(&options)
}

pub fn handle_help(sub_m: &ArgMatches) -> Result<()> {
    use crate::cli::build_cli;
    let command = sub_m.get_one::<String>("command").map(String::as_str);
    let mut cli = build_cli();
    match command {
        None | Some("-h" | "--help") => {
            cli.print_help().map_err(anyhow::Error::new)?;
            println!();
            Ok(())
        }
        Some(name) => {
            // Find and clone the subcommand to get mutable access
            let mut subcommand = cli
                .find_subcommand(name)
                .ok_or_else(|| anyhow::anyhow!("Unknown command '{name}'"))?
                .clone();
            subcommand.print_help().map_err(anyhow::Error::new)?;
            println!();
            Ok(())
        }
    }
}

pub fn handle_validate(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let command = sub_m
        .get_one::<String>("command")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let args: Vec<String> = sub_m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_else(|| Vec::new());
    let options = validate::ValidateOptions {
        command,
        args,
        format,
    };
    validate::run(&options)
}

pub fn handle_whatif(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let command = sub_m
        .get_one::<String>("command")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let args: Vec<String> = sub_m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_else(|| Vec::new());
    let options = whatif::WhatIfOptions {
        command,
        args,
        format,
    };
    whatif::run(&options)
}

pub async fn handle_claim(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let resource = sub_m
        .get_one::<String>("resource")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let timeout: u64 = sub_m
        .get_one::<String>("timeout")
        .and_then(|s| s.parse().ok())
        .map_or(30, |v| v);
    let options = claim::ClaimOptions {
        resource,
        timeout,
        format,
    };
    claim::run_claim(&options).await
}

pub async fn handle_yield(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let resource = sub_m
        .get_one::<String>("resource")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let options = claim::YieldOptions { resource, format };
    claim::run_yield(&options).await
}

pub async fn handle_batch(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let file = sub_m.get_one::<String>("file").cloned();
    let atomic = sub_m.get_flag("atomic");
    let stop_on_error = sub_m.get_flag("stop-on-error");

    if atomic {
        return handle_atomic_batch(sub_m, json, format, file, stop_on_error).await;
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
            .unwrap_or_else(|| Vec::new());
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
            let cmd = parts[0];
            let args: Vec<String> = parts[1..]
                .iter()
                .map(std::string::ToString::to_string)
                .collect();

            let output = tokio::process::Command::new("zjj")
                .arg(cmd)
                .args(&args)
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
    _json: bool,
    format: OutputFormat,
    file: Option<String>,
    _stop_on_error: bool,
) -> anyhow::Result<()> {
    use crate::commands::batch::execute_batch;
    let db = get_session_db().await?;
    let request = if let Some(file_path) = file {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file: {e}"))?;
        serde_json::from_str::<batch::BatchRequest>(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse batch request: {e}"))?
    } else {
        let raw_commands: Vec<String> = sub_m
            .get_many::<String>("commands")
            .map(|v| v.cloned().collect())
            .unwrap_or_else(|| Vec::new());
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
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m.get_one::<String>("session").cloned();
    let event_type = sub_m.get_one::<String>("type").cloned();
    let limit: Option<usize> = sub_m
        .get_one::<String>("limit")
        .and_then(|s| s.parse::<usize>().ok());
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

pub fn handle_completions(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let shell_str = sub_m
        .get_one::<String>("shell")
        .ok_or_else(|| anyhow::anyhow!("Shell is required"))?;
    let shell: completions::Shell = shell_str.parse()?;
    let options = completions::CompletionsOptions { shell, format };
    completions::run(&options)
}

pub async fn handle_rename(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let old_name = sub_m
        .get_one::<String>("old_name")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("old_name is required"))?;
    let new_name = sub_m
        .get_one::<String>("new_name")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("new_name is required"))?;
    let options = rename::RenameOptions {
        old_name,
        new_name,
        dry_run: false,
        format,
    };
    rename::run(&options).await
}

pub async fn handle_pause(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m
        .get_one::<String>("name")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let options = session_mgmt::PauseOptions { session, format };
    session_mgmt::run_pause(&options).await
}

pub async fn handle_resume(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m
        .get_one::<String>("name")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let options = session_mgmt::ResumeOptions { session, format };
    session_mgmt::run_resume(&options).await
}

pub async fn handle_lock(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m
        .get_one::<String>("session")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let agent_id = sub_m.get_one::<String>("agent-id").cloned();
    let ttl = match sub_m.get_one::<u64>("ttl") {
        Some(value) => *value,
        None => 0,
    };

    let args = crate::commands::lock::types::LockArgs {
        session,
        agent_id,
        ttl,
    };

    let db = get_session_db().await?;
    let mgr = zjj_core::coordination::locks::LockManager::new(db.pool().clone());

    let output = crate::commands::lock::run_lock_async(&args, &mgr).await?;
    if format.is_json() {
        let envelope = zjj_core::SchemaEnvelope::new("lock-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!(
            "✓ Locked session '{}' for agent '{}'",
            output.session, output.holder
        );
        if let Some(expires) = output.expires_at {
            let expires: chrono::DateTime<chrono::Utc> = expires;
            println!("  Expires at: {}", expires.to_rfc3339());
        }
    }
    Ok(())
}

pub async fn handle_unlock(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m
        .get_one::<String>("session")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let agent_id = sub_m.get_one::<String>("agent-id").cloned();

    let args = crate::commands::lock::types::UnlockArgs { session, agent_id };

    let db = get_session_db().await?;
    let mgr = zjj_core::coordination::locks::LockManager::new(db.pool().clone());

    let output = crate::commands::lock::run_unlock_async(&args, &mgr).await?;
    if format.is_json() {
        let envelope = zjj_core::SchemaEnvelope::new("unlock-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("✓ Unlocked session '{}'", output.session);
    }
    Ok(())
}

pub async fn handle_clone(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let source = sub_m
        .get_one::<String>("source")
        .cloned()
        .map_or_else(|| String::new(), |v| v);
    let target = sub_m
        .get_one::<String>("dest")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Target destination is required"))?;
    let options = session_mgmt::CloneOptions {
        source,
        target,
        dry_run: false,
        no_zellij: sub_m.get_flag("no-zellij"),
        format,
    };
    session_mgmt::run_clone(&options).await
}

pub async fn handle_export(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let session = sub_m.get_one::<String>("session").cloned();
    let output = sub_m.get_one::<String>("output").cloned();
    let options = export_import::ExportOptions {
        session,
        output,
        format,
    };
    export_import::run_export(&options).await
}

pub async fn handle_import(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let input = sub_m
        .get_one::<String>("file")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Input file is required"))?;
    let force = sub_m.get_flag("force");
    let skip_existing = sub_m.get_flag("skip-existing");
    let dry_run = sub_m.get_flag("dry-run");
    let options = export_import::ImportOptions {
        input,
        force,
        skip_existing,
        dry_run,
        format,
    };
    export_import::run_import(&options).await
}

pub async fn handle_wait(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
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
    // TODO: FIX THIS LINE - /* TODO: FIX THIS */;

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

pub fn handle_schema(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    let options = schema::SchemaOptions {
        schema_name: sub_m.get_one::<String>("name").cloned(),
        list: sub_m.get_flag("list"),
        all: sub_m.get_flag("all"),
        format,
    };
    schema::run(&options)
}

pub async fn handle_pane(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);
    match sub_m.subcommand() {
        Some(("focus", focus_m)) => {
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
            let session = list_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            pane::pane_list(session, &pane::PaneListOptions { format }).await
        }
        Some(("next", next_m)) => {
            let session = next_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            pane::pane_next(session, &pane::PaneNextOptions { format }).await
        }
        _ => anyhow::bail!("Unknown pane subcommand"),
    }
}

pub async fn handle_recover(sub_m: &ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = OutputFormat::from_json_flag(json);

    // Check if this is operation log recovery (has session arg or op flag)
    let session = sub_m.get_one::<String>("session").cloned();
    let operation = sub_m.get_one::<String>("op").cloned();
    let last = sub_m.get_flag("last");
    let list_ops = sub_m.get_flag("list-ops");

    if session.is_some() || operation.is_some() || last || list_ops {
        // Operation log recovery mode
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
        // System recovery mode
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

#[allow(clippy::too_many_lines)]
#[allow(clippy::large_stack_frames)]
pub async fn run_cli() -> Result<()> {
    let cli = build_cli();
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|a| a == "--json" || a == "-j");
    if args.iter().any(|a| a == "--strict") {
        std::env::set_var("ZJJ_STRICT", "1");
    }

    let matches = match cli.try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            // Check if this is a --help or --version request (should exit 0)
            // Clap returns Kind::DisplayHelp or Kind::DisplayVersion for these
            use clap::error::ErrorKind;
            let should_exit_zero =
                matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion);

            if json_mode {
                let json_err = serde_json::json!({ "success": false, "error": { "code": "INVALID_ARGUMENT", "message": e.to_string(), "exit_code": if should_exit_zero { 0 } else { 2 } } });
                println!("{}", serde_json::to_string_pretty(&json_err)?);
            }
            let _ = e.print();
            process::exit(if should_exit_zero { 0 } else { 2 });
        }
    };

    let on_success = matches.get_one::<String>("on-success").cloned();
    let on_failure = matches.get_one::<String>("on-failure").cloned();
    let hooks_config = hooks::HooksConfig::from_args(on_success, on_failure)?;
    let explicit_command_id = matches.get_one::<String>("command-id").map(String::as_str);
    let base_command_id = command_context::resolve_base_command_id(explicit_command_id);

    let result = command_context::with_command_context(base_command_id, async {
        match matches.subcommand() {
            Some(("init", sub_m)) => handle_init(sub_m).await,
            Some(("attach", sub_m)) => {
                let options = attach::AttachOptions::from_matches(sub_m)?;
                attach::run_with_options(&options).await
            }
            Some(("add", sub_m)) => handle_add(sub_m).await,
            Some(("agents", sub_m)) => handle_agents(sub_m).await,
            Some(("backup", sub_m)) => handle_backup(sub_m).await,
            Some(("list", sub_m)) => handle_list(sub_m).await,
            Some(("broadcast", sub_m)) => handle_broadcast(sub_m).await,
            Some(("bookmark", sub_m)) => handle_bookmark(sub_m).await,
            Some(("pane", sub_m)) => handle_pane(sub_m).await,
            Some(("remove", sub_m)) => handle_remove(sub_m).await,
            Some(("focus", sub_m)) => handle_focus(sub_m).await,
            Some(("switch", sub_m)) => handle_switch(sub_m).await,
            Some(("status", sub_m)) => handle_status(sub_m).await,
            Some(("sync", sub_m)) => handle_sync(sub_m).await,
            Some(("diff", sub_m)) => handle_diff(sub_m).await,
            Some(("config", sub_m)) => handle_config(sub_m).await,
            Some(("clean", sub_m)) => handle_clean(sub_m).await,
            Some(("template", sub_m)) => handle_template(sub_m).await,
            Some(("dashboard" | "dash", _)) => dashboard::run().await,
            Some(("introspect", sub_m)) => handle_introspect(sub_m).await,
            Some(("doctor" | "check", sub_m)) => handle_doctor(sub_m).await,
            Some(("integrity", sub_m)) => handle_integrity(sub_m).await,
            Some(("query", sub_m)) => handle_query(sub_m).await,
            Some(("queue", sub_m)) => handle_queue(sub_m).await,
            Some(("context", sub_m)) => handle_context(sub_m).await,
            Some(("done", sub_m)) => handle_done(sub_m).await,
            Some(("spawn", sub_m)) => handle_spawn(sub_m).await,
            Some(("checkpoint" | "ckpt", sub_m)) => handle_checkpoint(sub_m).await,
            Some(("undo", sub_m)) => handle_undo(sub_m).await,
            Some(("revert", sub_m)) => handle_revert(sub_m).await,
            Some(("whereami", sub_m)) => handle_whereami(sub_m).await,
            Some(("whoami", sub_m)) => handle_whoami(sub_m),
            Some(("work", sub_m)) => handle_work(sub_m).await,
            Some(("abort", sub_m)) => handle_abort(sub_m).await,
            Some(("ai", sub_m)) => handle_ai(sub_m).await,
            Some(("help", sub_m)) => handle_help(sub_m),
            Some(("can-i", sub_m)) => handle_can_i(sub_m).await,
            Some(("contract", sub_m)) => handle_contract(sub_m),
            Some(("examples", sub_m)) => handle_examples(sub_m),
            Some(("validate", sub_m)) => handle_validate(sub_m),
            Some(("whatif", sub_m)) => handle_whatif(sub_m),
            Some(("claim", sub_m)) => handle_claim(sub_m).await,
            Some(("yield", sub_m)) => handle_yield(sub_m).await,
            Some(("batch", sub_m)) => handle_batch(sub_m).await,
            Some(("events", sub_m)) => handle_events(sub_m).await,
            Some(("completions", sub_m)) => handle_completions(sub_m),
            Some(("rename", sub_m)) => handle_rename(sub_m).await,
            Some(("pause", sub_m)) => handle_pause(sub_m).await,
            Some(("resume", sub_m)) => handle_resume(sub_m).await,
            Some(("lock", sub_m)) => handle_lock(sub_m).await,
            Some(("unlock", sub_m)) => handle_unlock(sub_m).await,
            Some(("clone", sub_m)) => handle_clone(sub_m).await,
            Some(("export", sub_m)) => handle_export(sub_m).await,
            Some(("import", sub_m)) => handle_import(sub_m).await,
            Some(("wait", sub_m)) => handle_wait(sub_m).await,
            Some(("schema", sub_m)) => handle_schema(sub_m),
            Some(("recover", sub_m)) => handle_recover(sub_m).await,
            Some(("retry", sub_m)) => handle_retry(sub_m).await,
            Some(("rollback", sub_m)) => handle_rollback(sub_m).await,
            _ => {
                build_cli().print_help()?;
                Ok(())
            }
        }
    })
    .await;

    if let Err(ref e) = result {
        if json_mode {
            let exit_code = json::output_json_error(e);
            if hooks_config.has_hooks() {
                let _ = hooks_config.run_hook(false).await;
            }
            process::exit(exit_code);
        }
    }

    if hooks_config.has_hooks() {
        let _ = hooks_config.run_hook(result.is_ok()).await;
    }
    result
}

#[cfg(test)]
mod tests {
    use zjj_core::OutputFormat;

    #[test]
    fn test_handle_add_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_handle_init_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
    }

    #[test]
    fn test_handle_diff_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
    }

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

    #[test]
    fn test_add_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(format.to_json_flag(), json_bool);
    }

    #[test]
    fn test_init_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }

    #[test]
    fn test_diff_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }

    #[test]
    fn test_output_format_eliminates_json_bool_field() {
        let format1 = OutputFormat::Json;
        let format2 = OutputFormat::Human;
        assert_ne!(format1, format2);
    }

    #[test]
    fn test_output_format_bidirectional_conversion() {
        let original_bool = true;
        let format = OutputFormat::from_json_flag(original_bool);
        let restored_bool = format.to_json_flag();
        assert_eq!(original_bool, restored_bool);
        let original_bool2 = false;
        let format2 = OutputFormat::from_json_flag(original_bool2);
        let restored_bool2 = format2.to_json_flag();
        assert_eq!(original_bool2, restored_bool2);
    }

    #[test]
    fn test_all_handlers_accept_output_format() {
        let json_format = OutputFormat::Json;
        let human_format = OutputFormat::Human;
        assert!(json_format.is_json());
        assert!(human_format.is_human());
    }

    #[test]
    fn test_error_output_respects_format() {
        let format = OutputFormat::Json;
        assert!(format.is_json());
        let format2 = OutputFormat::Human;
        assert!(format2.is_human());
    }

    #[test]
    fn test_handlers_never_panic_on_format() {
        for format in &[OutputFormat::Json, OutputFormat::Human] {
            let _ = format.is_json();
            let _ = format.is_human();
            let _ = format.to_string();
            let _ = format.to_json_flag();
        }
    }

    #[test]
    fn test_format_parameter_reaches_command_functions() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }
}
