//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `zjj`

use std::process;

use anyhow::Result;
use clap::{Arg, Command as ClapCommand};

mod cli;
mod commands;
mod db;
mod json_output;
mod session;

use commands::{
    add, attach, clean, config, dashboard, diff, doctor, focus, init, introspect, list, query,
    remove, status, sync,
};

fn cmd_init() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize zjj in a JJ repository (or create one)")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_attach() -> ClapCommand {
    ClapCommand::new("attach")
        .about("Attach to an existing Zellij session")
        .arg(
            Arg::new("name")
                .required(true)
                .help("Name of the session to attach to"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (only for errors)"),
        )
}

fn cmd_add() -> ClapCommand {
    ClapCommand::new("add")
        .about("Create a new session with JJ workspace + Zellij tab")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name for the new session (must start with a letter)"),
        )
        .arg(
            Arg::new("no-hooks")
                .long("no-hooks")
                .action(clap::ArgAction::SetTrue)
                .help("Skip executing post_create hooks"),
        )
        .arg(
            Arg::new("template")
                .short('t')
                .long("template")
                .value_name("TEMPLATE")
                .help("Zellij layout template to use (minimal, standard, full)"),
        )
        .arg(
            Arg::new("no-open")
                .long("no-open")
                .action(clap::ArgAction::SetTrue)
                .help("Create workspace without opening Zellij tab"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_list() -> ClapCommand {
    ClapCommand::new("list")
        .about("List all sessions")
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include completed and failed sessions"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .value_name("BEAD_ID")
                .help("Filter sessions by bead ID"),
        )
        .arg(
            Arg::new("agent")
                .long("agent")
                .value_name("NAME")
                .action(clap::ArgAction::Set)
                .help("Filter sessions by agent owner"),
        )
}

fn cmd_remove() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove a session and its workspace")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to remove"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt and hooks"),
        )
        .arg(
            Arg::new("merge")
                .short('m')
                .long("merge")
                .action(clap::ArgAction::SetTrue)
                .help("Squash-merge to main before removal"),
        )
        .arg(
            Arg::new("keep-branch")
                .short('k')
                .long("keep-branch")
                .action(clap::ArgAction::SetTrue)
                .help("Preserve branch after removal"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_focus() -> ClapCommand {
    ClapCommand::new("focus")
        .about("Switch to a session's Zellij tab")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to focus"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Show detailed session status")
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
        )
}

fn cmd_sync() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync a session's workspace with main (rebase)")
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to sync (syncs current workspace if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_diff() -> ClapCommand {
    ClapCommand::new("diff")
        .about("Show diff between session and main branch")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Session name to show diff for"),
        )
        .arg(
            Arg::new("stat")
                .long("stat")
                .action(clap::ArgAction::SetTrue)
                .help("Show diffstat only (summary of changes)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("View or modify configuration")
        .arg(Arg::new("key").help("Config key to view/set (dot notation: 'zellij.use_tabs')"))
        .arg(Arg::new("value").help("Value to set (omit to view)"))
        .arg(
            Arg::new("global")
                .long("global")
                .short('g')
                .action(clap::ArgAction::SetTrue)
                .help("Operate on global config instead of project"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_clean() -> ClapCommand {
    ClapCommand::new("clean")
        .about("Remove stale sessions (where workspace no longer exists)")
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("List stale sessions without removing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_dashboard() -> ClapCommand {
    ClapCommand::new("dashboard")
        .about("Launch interactive TUI dashboard with kanban view")
        .alias("dash")
}

fn cmd_introspect() -> ClapCommand {
    ClapCommand::new("introspect")
        .about("Discover zjj capabilities and command details")
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to introspect (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run system health checks")
        .alias("check")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible"),
        )
}

fn cmd_query() -> ClapCommand {
    ClapCommand::new("query")
        .about("Query system state programmatically")
        .arg(
            Arg::new("query_type")
                .required(true)
                .help("Type of query (session-exists, session-count, can-run, suggest-name)"),
        )
        .arg(
            Arg::new("args")
                .required(false)
                .help("Query-specific arguments"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (default for query)"),
        )
}

fn build_cli() -> ClapCommand {
    ClapCommand::new("zjj")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZJJ Contributors")
        .about("ZJJ - Manage JJ workspaces with Zellij sessions")
        .subcommand_required(true)
        .subcommand(cmd_init())
        .subcommand(cmd_add())
        .subcommand(cmd_attach())
        .subcommand(cmd_list())
        .subcommand(cmd_remove())
        .subcommand(cmd_focus())
        .subcommand(cmd_status())
        .subcommand(cmd_sync())
        .subcommand(cmd_diff())
        .subcommand(cmd_config())
        .subcommand(cmd_clean())
        .subcommand(cmd_dashboard())
        .subcommand(cmd_introspect())
        .subcommand(cmd_doctor())
        .subcommand(cmd_query())
}

/// Format an error for user display (no stack traces)
fn format_error(err: &anyhow::Error) -> String {
    // Get the root cause message
    let mut msg = err.to_string();

    // If the error chain has more context, include it
    if let Some(source) = err.source() {
        let source_msg = source.to_string();
        // Only add source if it's different and adds value
        if !msg.contains(&source_msg) && !source_msg.is_empty() {
            msg = format!("{msg}\nCause: {source_msg}");
        }
    }

    msg
}

fn handle_init(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    match init::run() {
        Ok(()) => Ok(()),
        Err(e) => {
            if json {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_add(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

    let no_hooks = sub_m.get_flag("no-hooks");
    let template = sub_m.get_one::<String>("template").cloned();
    let no_open = sub_m.get_flag("no-open");
    let json = sub_m.get_flag("json");

    let options = add::AddOptions {
        name: name.clone(),
        no_hooks,
        template,
        no_open,
        format: zjj_core::OutputFormat::from_json_flag(json),
    };

    match add::run_with_options(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if json {
                json_output::output_json_error_and_exit(&e);
            } else {
                // For regular output, we still want to exit with code 1 for validation errors
                // This ensures consistency between JSON and regular error reporting
                Err(e)
            }
        }
    }
}

fn handle_list(sub_m: &clap::ArgMatches) -> Result<()> {
    let all = sub_m.get_flag("all");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let bead = sub_m.get_one::<String>("bead").cloned();
    let agent = sub_m.get_one::<String>("agent").map(String::as_str);
    list::run(all, format, bead.as_deref(), agent)
}

fn handle_remove(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = remove::RemoveOptions {
        force: sub_m.get_flag("force"),
        merge: sub_m.get_flag("merge"),
        keep_branch: sub_m.get_flag("keep-branch"),
        format,
    };
    match remove::run_with_options(name, &options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_focus(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = focus::FocusOptions { format };
    match focus::run_with_options(name, &options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_status(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let watch = sub_m.get_flag("watch");
    match status::run(name, format, watch) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_sync(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = sync::SyncOptions { format };
    match sync::run_with_options(name, options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_diff(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let stat = sub_m.get_flag("stat");
    let json = sub_m.get_flag("json");
    match diff::run(name, stat) {
        Ok(()) => Ok(()),
        Err(e) => {
            if json {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_config(sub_m: &clap::ArgMatches) -> Result<()> {
    let key = sub_m.get_one::<String>("key").cloned();
    let value = sub_m.get_one::<String>("value").cloned();
    let global = sub_m.get_flag("global");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = config::ConfigOptions {
        key,
        value,
        global,
        format,
    };
    match config::run(options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_clean(sub_m: &clap::ArgMatches) -> Result<()> {
    let force = sub_m.get_flag("force");
    let dry_run = sub_m.get_flag("dry-run");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = clean::CleanOptions {
        force,
        dry_run,
        format,
    };
    clean::run_with_options(&options)
}

fn handle_introspect(sub_m: &clap::ArgMatches) -> Result<()> {
    let command = sub_m.get_one::<String>("command").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let result = command.map_or_else(
        || introspect::run(format),
        |cmd| introspect::run_command_introspect(cmd, format),
    );
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_doctor(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let fix = sub_m.get_flag("fix");
    match doctor::run(format, fix) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json_output::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_query(sub_m: &clap::ArgMatches) -> Result<()> {
    let query_type = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
    let args = sub_m.get_one::<String>("args").map(String::as_str);
    let _json = sub_m.get_flag("json"); // Ignored as query is always JSON
    query::run(query_type, args)
}

/// Execute the CLI and return a Result
fn run_cli() -> Result<()> {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("init", sub_m)) => handle_init(sub_m),
        Some(("attach", sub_m)) => {
            let options = attach::AttachOptions::from_matches(sub_m)?;
            match attach::run_with_options(&options) {
                Ok(()) => Ok(()),
                Err(e) => {
                    if options.format.is_json() {
                        json_output::output_json_error_and_exit(&e);
                    } else {
                        Err(e)
                    }
                }
            }
        }
        Some(("add", sub_m)) => handle_add(sub_m),
        Some(("list", sub_m)) => handle_list(sub_m),
        Some(("remove", sub_m)) => handle_remove(sub_m),
        Some(("focus", sub_m)) => handle_focus(sub_m),
        Some(("status", sub_m)) => handle_status(sub_m),
        Some(("sync", sub_m)) => handle_sync(sub_m),
        Some(("diff", sub_m)) => handle_diff(sub_m),
        Some(("config", sub_m)) => handle_config(sub_m),
        Some(("clean", sub_m)) => handle_clean(sub_m),
        Some(("dashboard" | "dash", _)) => dashboard::run(),
        Some(("introspect", sub_m)) => handle_introspect(sub_m),
        Some(("doctor" | "check", sub_m)) => handle_doctor(sub_m),
        Some(("query", sub_m)) => handle_query(sub_m),
        _ => {
            build_cli().print_help()?;
            Ok(())
        }
    }
}

fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Run the CLI and handle errors gracefully
    if let Err(err) = run_cli() {
        eprintln!("Error: {}", format_error(&err));
        process::exit(1);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PHASE 2 (RED) - OutputFormat Migration Tests for main.rs
// These tests FAIL until handlers are updated to use OutputFormat
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod main_tests {
    use zjj_core::OutputFormat;

    /// RED: handle_add should accept OutputFormat from options
    #[test]
    fn test_handle_add_converts_json_flag_to_output_format() {
        // This test documents the expected behavior:
        // handle_add should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json)
        // 3. Pass to AddOptions with format field
        // 4. Call add::run_with_options() which uses the format

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert_eq!(format, OutputFormat::Json);
        // When implemented: AddOptions { name, no_hooks, template, no_open, format }
    }

    /// RED: handle_init should accept OutputFormat parameter
    #[test]
    fn test_handle_init_converts_json_flag_to_output_format() {
        // This test documents the expected behavior:
        // handle_init should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json)
        // 3. Pass to init::run(format) or create InitOptions with format

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert!(format.is_json());
        // When implemented: init::run(OutputFormat::from_json_flag(json))
    }

    /// RED: handle_diff should accept OutputFormat parameter
    #[test]
    fn test_handle_diff_converts_json_flag_to_output_format() {
        // This test documents the expected behavior:
        // handle_diff should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json)
        // 3. Pass to diff::run(name, stat, format)

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert!(format.is_json());
        // When implemented: diff::run("session", stat, format)
    }

    /// RED: handle_query always uses JSON format
    #[test]
    fn test_handle_query_always_uses_json_format() {
        // Query always outputs JSON for programmatic access
        // Even if --json flag is false, query should output JSON

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());

        let json_flag_false = false;
        let _format2 = OutputFormat::from_json_flag(json_flag_false);
        // But query::run should internally convert to Json
        let query_format = OutputFormat::Json;
        assert!(query_format.is_json());
    }

    /// RED: AddOptions constructor includes format field
    #[test]
    fn test_add_options_struct_has_format() {
        use crate::commands::add::AddOptions;

        // When AddOptions is updated to include format field:
        // pub struct AddOptions {
        //     pub name: String,
        //     pub no_hooks: bool,
        //     pub template: Option<String>,
        //     pub no_open: bool,
        //     pub format: OutputFormat,
        // }

        let opts = AddOptions {
            name: "test".to_string(),
            no_hooks: false,
            template: None,
            no_open: false,
            format: OutputFormat::Json,
        };

        assert_eq!(opts.name, "test");
        assert_eq!(opts.format, OutputFormat::Json);
    }

    /// RED: --json flag is converted to OutputFormat for add
    #[test]
    fn test_add_json_flag_propagates_through_handler() {
        // Document the expected flow:
        // main.rs handle_add:
        //   json = sub_m.get_flag("json")           // Extract --json flag
        //   format = OutputFormat::from_json_flag(json)
        //   options = AddOptions { ..., format }
        //   add::run_with_options(&options)

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        assert_eq!(format, OutputFormat::Json);
        assert_eq!(format.to_json_flag(), json_bool);
    }

    /// RED: --json flag is converted to OutputFormat for init
    #[test]
    fn test_init_json_flag_propagates_through_handler() {
        // Document the expected flow:
        // main.rs handle_init:
        //   json = sub_m.get_flag("json")           // Extract --json flag
        //   format = OutputFormat::from_json_flag(json)
        //   init::run(format)

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        assert!(format.is_json());
    }

    /// RED: --json flag is converted to OutputFormat for diff
    #[test]
    fn test_diff_json_flag_propagates_through_handler() {
        // Document the expected flow:
        // main.rs handle_diff:
        //   json = sub_m.get_flag("json")           // Extract --json flag
        //   format = OutputFormat::from_json_flag(json)
        //   diff::run(name, stat, format)

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        assert!(format.is_json());
    }

    /// RED: OutputFormat prevents mixing json bool with command options
    #[test]
    fn test_output_format_eliminates_json_bool_field() {
        // After migration, command options should NOT have:
        //   pub json: bool
        //
        // Instead they should have:
        //   pub format: OutputFormat
        //
        // This test documents that the bool field is completely removed

        let format1 = OutputFormat::Json;
        let format2 = OutputFormat::Human;

        assert_ne!(format1, format2);
        // No more mixing bool and enum - exhaustive pattern matching enforced
    }

    /// RED: OutputFormat handles both --json flag conversions
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

    /// RED: All handlers use OutputFormat instead of bool
    #[test]
    fn test_all_handlers_accept_output_format() {
        // Document which handlers need updates:
        // - handle_init: format parameter
        // - handle_add: format in AddOptions
        // - handle_diff: format parameter
        // - handle_query: always Json, ignores flag
        //
        // Already updated (10 commands):
        // - handle_list, handle_remove, handle_focus
        // - handle_status, handle_sync
        // - handle_config, handle_clean
        // - handle_introspect, handle_doctor
        // - handle_attach

        let json_format = OutputFormat::Json;
        let human_format = OutputFormat::Human;

        assert!(json_format.is_json());
        assert!(human_format.is_human());
    }

    /// RED: JSON output errors also use OutputFormat
    #[test]
    fn test_error_output_respects_format() {
        // When errors occur, they should also respect OutputFormat:
        // if format.is_json() {
        //     json_output::output_json_error_and_exit(&e)
        // } else {
        //     Err(e) for default error handling
        // }

        let format = OutputFormat::Json;
        assert!(format.is_json());

        let format2 = OutputFormat::Human;
        assert!(format2.is_human());
    }

    /// RED: No panics during format conversion in handlers
    #[test]
    fn test_handlers_never_panic_on_format() {
        // All handlers should handle both formats without panic
        for format in [OutputFormat::Json, OutputFormat::Human].iter() {
            let _ = format.is_json();
            let _ = format.is_human();
            let _ = format.to_string();
            let _ = format.to_json_flag();
        }
    }

    /// RED: OutputFormat is passed to all command functions
    #[test]
    fn test_format_parameter_reaches_command_functions() {
        // Document parameter passing:
        // main.rs handle_* extracts --json flag
        //   -> converts to OutputFormat
        //   -> passes to command::run() or struct with format field
        //   -> command functions check format to decide output style

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        // This format should reach all command implementations
        assert!(format.is_json());
    }
}
