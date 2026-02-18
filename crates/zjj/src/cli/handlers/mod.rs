//! CLI command handlers that bridge between `clap` and internal logic
//!
//! This module is organized into logical submodules:
//! - `workspace`: Session/workspace management (init, add, remove, focus, etc.)
//! - `sync`: Sync, diff, submit, done, abort
//! - `bookmark`: Bookmark operations
//! - `template`: Template operations
//! - `integrity`: Integrity, doctor, clean, prune
//! - `queue`: Queue operations
//! - `checkpoint`: Checkpoint, undo, revert, recover, retry, rollback
//! - `coordination`: Agents, broadcast, claim, yield, lock, unlock
//! - `introspection`: AI, introspect, context, whereami, whoami, etc.
//! - `batch`: Batch and events operations
//! - `backup`: Backup, export, import
//! - `utility`: Config, query, schema, completions, wait, pane
//! - `json_format`: Shared JSON format extraction helper

use std::process;

use anyhow::Result;
use serde_json::json;
use zjj_core::{json::schemas, SchemaEnvelope};

use crate::{cli::build_cli, command_context, hooks, json};

pub mod backup;
pub mod batch;
pub mod bookmark;
pub mod checkpoint;
pub mod coordination;
pub mod integrity;
pub mod introspection;
pub mod json_format;
pub mod queue;
pub mod sync;
pub mod template;
pub mod utility;
pub mod workspace;

pub use self::{
    backup::{handle_backup, handle_export, handle_import},
    batch::{handle_batch, handle_events},
    bookmark::handle_bookmark,
    checkpoint::{
        handle_checkpoint, handle_recover, handle_retry, handle_revert, handle_rollback,
        handle_undo,
    },
    coordination::{
        handle_agents, handle_broadcast, handle_claim, handle_lock, handle_unlock, handle_yield,
    },
    integrity::{handle_clean, handle_doctor, handle_integrity, handle_prune_invalid},
    introspection::{
        handle_ai, handle_can_i, handle_context, handle_contract, handle_examples, handle_help,
        handle_introspect, handle_validate, handle_whatif, handle_whereami, handle_whoami,
    },
    queue::handle_queue,
    sync::{handle_abort, handle_diff, handle_done, handle_submit, handle_sync},
    template::handle_template,
    utility::{
        handle_completions, handle_config, handle_pane, handle_query, handle_schema, handle_wait,
    },
    workspace::{
        handle_add, handle_attach, handle_clone, handle_focus, handle_init, handle_list,
        handle_pause, handle_remove, handle_rename, handle_resume, handle_spawn, handle_status,
        handle_switch, handle_work,
    },
};

#[derive(Debug)]
pub struct CommandExit {
    exit_code: i32,
}

impl CommandExit {
    pub const fn new(exit_code: i32) -> Self {
        Self { exit_code }
    }

    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl std::fmt::Display for CommandExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command failed with exit code {}", self.exit_code)
    }
}

impl std::error::Error for CommandExit {}

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

fn output_json_display(display_type: &str, content: &str) {
    let payload = json!({
        "display_type": display_type,
        "content": content,
    });
    let envelope = SchemaEnvelope::new(schemas::CLI_DISPLAY_RESPONSE, "single", payload);

    if let Ok(json_output) = serde_json::to_string_pretty(&envelope) {
        println!("{json_output}");
    } else {
        let fallback = json!({
            "$schema": "zjj://cli-display-response/v1",
            "_schema_version": "1.0",
            "schema_type": "single",
            "success": true,
            "display_type": display_type,
            "content": content,
        });
        if let Ok(serialized) = serde_json::to_string_pretty(&fallback) {
            println!("{serialized}");
        }
    }
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::large_stack_frames)]
pub async fn run_cli() -> Result<()> {
    let cli = build_cli();
    // SECURITY: std::env::args() is acceptable here - only used for CLI flag parsing
    // (--json, --strict), not for security-critical operations or path validation.
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|a| a == "--json" || a == "-j");
    if args.iter().any(|a| a == "--strict") {
        std::env::set_var("ZJJ_STRICT", "1");
    }

    let matches = match cli.try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            use clap::error::ErrorKind;
            let should_exit_zero =
                matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion);

            if json_mode {
                if should_exit_zero {
                    let display_type = match e.kind() {
                        ErrorKind::DisplayHelp => "help",
                        ErrorKind::DisplayVersion => "version",
                        _ => "display",
                    };
                    output_json_display(display_type, &e.to_string());
                    process::exit(0);
                }

                let exit_code = json::output_json_parse_error(e.to_string());
                process::exit(exit_code);
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
            Some(("attach", sub_m)) => handle_attach(sub_m).await,
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
            Some(("prune-invalid", sub_m)) => handle_prune_invalid(sub_m).await,
            Some(("template", sub_m)) => handle_template(sub_m).await,
            Some(("introspect", sub_m)) => handle_introspect(sub_m).await,
            Some(("doctor" | "check", sub_m)) => handle_doctor(sub_m).await,
            Some(("integrity", sub_m)) => handle_integrity(sub_m).await,
            Some(("query", sub_m)) => handle_query(sub_m).await,
            Some(("queue", sub_m)) => handle_queue(sub_m).await,
            Some(("context", sub_m)) => handle_context(sub_m).await,
            Some(("done", sub_m)) => handle_done(sub_m).await,
            Some(("submit", sub_m)) => handle_submit(sub_m).await,
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
        if let Some(command_exit) = e.downcast_ref::<CommandExit>() {
            if hooks_config.has_hooks() {
                let _ = hooks_config.run_hook(false).await;
            }
            process::exit(command_exit.exit_code());
        }

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
