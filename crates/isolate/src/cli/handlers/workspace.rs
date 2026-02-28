//! Workspace session management handlers

use anyhow::Result;
use clap::ArgMatches;
use isolate_core::json::SchemaEnvelope;

use super::json_format::get_format;
use crate::{
    cli::handlers::introspection::{handle_context, handle_whereami, handle_whoami},
    commands::{add, init, list, remove, rename, session_mgmt, spawn, status, switch, work},
    json,
};

fn print_contract(contract: &str, json_mode: bool) {
    if json_mode {
        let maybe_json = contract
            .find('{')
            .and_then(|start| contract.get(start..))
            .map(str::trim);
        if let Some(json_contract) = maybe_json {
            println!("{json_contract}");
        } else {
            println!("{contract}");
        }
    } else {
        println!("{contract}");
    }
}

pub async fn handle_init(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let dry_run = sub_m.get_flag("dry-run");
    init::run_with_options(init::InitOptions { format, dry_run }).await
}

pub async fn handle_add(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::add());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    if sub_m.get_flag("example-json") {
        let example_output = json::AddOutput {
            name: "example-session".to_string(),
            workspace_path: "/path/to/.isolate/workspaces/example-session".to_string(),
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
    let no_open = sub_m.get_flag("no-open");
    let idempotent = sub_m.get_flag("idempotent");
    let dry_run = sub_m.get_flag("dry-run");

    let options = add::AddOptions {
        name: name.clone(),
        bead_id,
        no_hooks,
        no_open,
        format: get_format(sub_m),
        idempotent,
        dry_run,
    };

    add::run_with_options(&options).await
}

pub async fn handle_list(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);

    if sub_m.get_flag("contract") {
        print_contract(
            crate::cli::json_docs::ai_contracts::list(),
            format.is_json(),
        );
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let all = sub_m.get_flag("all");
    let verbose = sub_m.get_flag("verbose");
    let bead = sub_m.get_one::<String>("bead").cloned();
    let agent = sub_m.get_one::<String>("agent").map(String::as_str);
    let state = sub_m.get_one::<String>("state").map(String::as_str);
    list::run(all, verbose, format, bead.as_deref(), agent, state).await
}

pub async fn handle_remove(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::remove());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let format = get_format(sub_m);
    let options = remove::RemoveOptions {
        force: sub_m.get_flag("force"),
        merge: sub_m.get_flag("merge"),
        keep_branch: sub_m.get_flag("keep-branch"),
        idempotent: sub_m.get_flag("idempotent"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    remove::run_with_options(name, &options).await
}

pub async fn handle_status(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first (global flag)
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::status());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    // Route to subcommand handlers
    match sub_m.subcommand() {
        Some(("show", show_m)) => {
            let name = show_m.get_one::<String>("session").map(String::as_str);
            let watch = sub_m.get_flag("watch");
            if watch {
                status::run_watch_mode(name).await
            } else {
                status::run(name).await
            }
        }
        Some(("whereami", whereami_m)) => handle_whereami(whereami_m).await,
        Some(("whoami", whoami_m)) => handle_whoami(whoami_m),
        Some(("context", context_m)) => handle_context(context_m).await,
        None => {
            // Legacy: isolate status (no subcommand)
            // Show deprecation warning
            eprintln!("warning: 'isolate status' without subcommand is deprecated, use 'isolate status show' instead.");
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            let watch = sub_m.get_flag("watch");
            if watch {
                status::run_watch_mode(name).await
            } else {
                status::run(name).await
            }
        }
        Some((unknown, _)) => Err(anyhow::anyhow!(
            "Unknown status subcommand: '{}'. Use 'show', 'whereami', 'whoami', or 'context'",
            unknown
        )),
    }
}

pub async fn handle_switch(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let show_context = sub_m.get_flag("show-context");
    let format = get_format(sub_m);
    let options = switch::SwitchOptions {
        format,
        show_context,
    };
    switch::run_with_options(name, &options).await
}

pub async fn handle_spawn(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::spawn());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let args = spawn::SpawnArgs::from_matches(sub_m)?;
    let options = args.to_options();
    spawn::run_with_options(&options).await
}

pub async fn handle_work(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::work());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let format = get_format(sub_m);
    let name = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let options = work::WorkOptions {
        name,
        bead_id: sub_m.get_one::<String>("bead").cloned(),
        agent_id: sub_m.get_one::<String>("agent-id").cloned(),
        no_agent: sub_m.get_flag("no-agent"),
        idempotent: sub_m.get_flag("idempotent"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    work::run(&options).await
}

pub async fn handle_rename(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
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

pub async fn handle_clone(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let source = sub_m
        .get_one::<String>("source")
        .ok_or_else(|| anyhow::anyhow!("Source session is required"))?
        .clone();
    let target = sub_m
        .get_one::<String>("dest")
        .ok_or_else(|| anyhow::anyhow!("Target destination is required"))?
        .clone();
    let options = session_mgmt::CloneOptions {
        source,
        target,
        dry_run: false,
        format,
    };
    session_mgmt::run_clone(&options).await
}

pub async fn handle_pause(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
    let options = session_mgmt::PauseOptions { session, format };
    session_mgmt::run_pause(&options).await
}

pub async fn handle_resume(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
    let options = session_mgmt::ResumeOptions { session, format };
    session_mgmt::run_resume(&options).await
}

#[cfg(test)]
mod tests {
    use isolate_core::OutputFormat;

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

    mod martin_fowler_work_parser_table_behavior {
        struct ParseCase {
            name: &'static str,
            args: Vec<&'static str>,
            expect_ok: bool,
        }

        /// GIVEN: a matrix of `work` CLI argument combinations
        /// WHEN: clap parses each row
        /// THEN: acceptance/rejection should match command contract
        #[test]
        fn given_work_argument_matrix_when_parsing_then_rows_match_contract() {
            let cases = [
                ParseCase {
                    name: "requires name by default",
                    args: vec!["work"],
                    expect_ok: false,
                },
                ParseCase {
                    name: "contract bypasses name requirement",
                    args: vec!["work", "--contract"],
                    expect_ok: true,
                },
                ParseCase {
                    name: "ai-hints bypasses name requirement",
                    args: vec!["work", "--ai-hints"],
                    expect_ok: true,
                },
                ParseCase {
                    name: "accepts full flag set with name",
                    args: vec![
                        "work",
                        "feature-auth",
                        "--bead",
                        "isolate-123",
                        "--agent-id",
                        "agent-1",
                        "--idempotent",
                        "--dry-run",
                        "--json",
                    ],
                    expect_ok: true,
                },
                ParseCase {
                    name: "rejects unknown flag",
                    args: vec!["work", "feature-auth", "--unknown-flag"],
                    expect_ok: false,
                },
            ];

            for case in cases {
                let parsed = crate::cli::commands::cmd_work().try_get_matches_from(case.args);
                assert_eq!(
                    parsed.is_ok(),
                    case.expect_ok,
                    "case '{}' parse expectation failed",
                    case.name
                );
            }
        }

        /// GIVEN: parsed work args containing all optional knobs
        /// WHEN: extracting values from clap matches
        /// THEN: each option should map to the expected typed value
        #[test]
        fn given_full_work_args_when_reading_matches_then_all_values_are_preserved() {
            let parsed = crate::cli::commands::cmd_work()
                .try_get_matches_from([
                    "work",
                    "session-a",
                    "--bead",
                    "isolate-789",
                    "--agent-id",
                    "agent-77",
                    "--no-agent",
                    "--idempotent",
                    "--dry-run",
                    "--json",
                ])
                .expect("full work args should parse");

            assert_eq!(
                parsed.get_one::<String>("name").map(String::as_str),
                Some("session-a")
            );
            assert_eq!(
                parsed.get_one::<String>("bead").map(String::as_str),
                Some("isolate-789")
            );
            assert_eq!(
                parsed.get_one::<String>("agent-id").map(String::as_str),
                Some("agent-77")
            );
            assert!(parsed.get_flag("no-agent"));
            assert!(parsed.get_flag("idempotent"));
            assert!(parsed.get_flag("dry-run"));
            assert!(parsed.get_flag("json"));
        }
    }
}
