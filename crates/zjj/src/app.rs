use anyhow::Result;

use crate::{
    cli::{args::build_cli, output_help_json, setup},
    commands::{config, init, list, prime, version},
};

/// Execute the CLI and return a Result
///
/// This is the main entry point for all CLI command execution.
/// It parses arguments and dispatches to the appropriate command handler.
pub async fn run_cli() -> Result<()> {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("init", sub_m)) => {
            init::run_with_flags(sub_m.get_flag("repair"), sub_m.get_flag("force")).await
        }
        Some(("add" | "add-batch" | "list" | "remove" | "focus" | "status", sub_m)) => {
            let cmd = matches
                .subcommand_name()
                .ok_or_else(|| anyhow::anyhow!("No command"))?;
            crate::cli::dispatch::handle_session_cmd(cmd, sub_m).await
        }
        Some(("sync", sub_m)) => crate::cli::dispatch::handle_sync_cmd(sub_m).await,
        Some(("diff", sub_m)) => crate::cli::dispatch::handle_diff_cmd(sub_m).await,
        Some(("agent", sub_m)) => crate::cli::dispatch::handle_agent_cmd(sub_m).await,
        Some(("config", sub_m)) => {
            config::run(config::ConfigOptions {
                key: sub_m.get_one::<String>("key").cloned(),
                value: sub_m.get_one::<String>("value").cloned(),
                global: sub_m.get_flag("global"),
                json: sub_m.get_flag("json"),
                validate: sub_m.get_flag("validate"),
            })
            .await
        }
        Some(("context" | "ctx", sub_m)) => {
            let cmd = matches
                .subcommand_name()
                .ok_or_else(|| anyhow::anyhow!("No command"))?;
            crate::cli::dispatch::handle_introspection_cmd(cmd, sub_m).await
        }
        Some(("prime", sub_m)) => {
            prime::run_with_quiet(sub_m.get_flag("json"), sub_m.get_flag("quiet")).await
        }
        Some(("dashboard" | "dash", sub_m)) => {
            let cmd = matches
                .subcommand_name()
                .ok_or_else(|| anyhow::anyhow!("No command"))?;
            crate::cli::dispatch::handle_introspection_cmd(cmd, sub_m).await
        }
        Some(("introspect", sub_m)) => {
            crate::cli::dispatch::handle_introspection_cmd("introspect", sub_m).await
        }
        Some(("doctor" | "check", sub_m)) => {
            let cmd = matches
                .subcommand_name()
                .ok_or_else(|| anyhow::anyhow!("No command"))?;
            crate::cli::dispatch::handle_introspection_cmd(cmd, sub_m).await
        }
        Some(("backup" | "restore" | "verify-backup" | "completions" | "query", sub_m)) => {
            let cmd = matches
                .subcommand_name()
                .ok_or_else(|| anyhow::anyhow!("No command"))?;
            crate::cli::dispatch::handle_utility_cmd(cmd, sub_m).await
        }
        Some(("version", sub_m)) => version::run(sub_m.get_flag("json")).await,
        // TODO: Re-enable when hooks module is created
        // Some(("hooks", sub_m)) => match sub_m.subcommand() {
        //     Some(("install", install_m)) => {
        //         hooks::install_hooks(install_m.get_flag("dry-run"), install_m.get_flag("json"))
        //     }
        //     _ => {
        //         anyhow::bail!("Unknown hooks subcommand. Try 'jjz hooks install'")
        //     }
        // },
        None => {
            // No subcommand provided - show helpful overview
            println!("jjz - JJ workspace manager with Zellij sessions\n");
            println!("📋 Your sessions:\n");

            // Try to show session list, fall back to help if not initialized
            if matches!(
                list::run(false, false, false, list::ListFilter::default()).await,
                Ok(())
            ) {
                println!("\n💡 Quick commands:");
                println!("   jjz add <name>      Create new session");
                println!("   jjz list            Show all sessions");
                println!("   jjz dashboard       Interactive dashboard");
                println!("   jjz --help          Full help");
            } else {
                // Not initialized yet
                println!("⚠️  jjz not initialized in this repository\n");
                println!("Get started:");
                println!("   jjz init            Initialize jjz (first time setup)");
                println!("   jjz --help          See all commands");
                println!("   jjz --help-json     Machine-readable docs (for AI)");
            }
            Ok(())
        }
        _ => {
            build_cli().print_help()?;
            Ok(())
        }
    }
}

/// Main application runner
///
/// This function orchestrates the entire CLI lifecycle:
/// 1. Parse early flags (--help-json, --json)
/// 2. Initialize logging
/// 3. Create async runtime
/// 4. Execute CLI command
/// 5. Handle errors and exit codes
///
/// # Arguments
/// * `config` - Setup configuration from early flag parsing
///
/// # Returns
/// Never returns - exits the process with appropriate exit code
pub fn run(config: &setup::SetupConfig) -> ! {
    use std::process;

    // Handle --help-json early exit
    if config.help_json_requested {
        output_help_json();
        process::exit(0);
    }

    // Initialize tracing subscriber for logging
    if let Err(e) = setup::init_tracing() {
        setup::output_error(
            config.json_mode,
            "TRACING_ERROR",
            &format!("Failed to initialize tracing: {e}"),
        );
        process::exit(1);
    }

    // Create tokio runtime
    let runtime = match setup::create_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            setup::output_error(config.json_mode, "RUNTIME_ERROR", &e.to_string());
            process::exit(1);
        }
    };

    // Run the CLI and handle errors gracefully
    runtime.block_on(async {
        if let Err(err) = run_cli().await {
            let error_msg = crate::cli::format_error(&err);
            let exit_code = crate::cli::get_exit_code(&err);
            if config.json_mode {
                crate::cli::output_json_error("ERROR", &error_msg, None);
            } else {
                eprintln!("Error: {error_msg}");
            }
            process::exit(exit_code);
        }
    });

    // Successful exit
    process::exit(0);
}
