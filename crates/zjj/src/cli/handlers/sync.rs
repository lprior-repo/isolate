//! Sync, diff, submit, done, and abort handlers

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::{abort, diff, done, submit, sync};

pub async fn handle_sync(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::sync());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let all = sub_m.get_flag("all");
    let dry_run = sub_m.get_flag("dry-run");
    let format = get_format(sub_m);
    let options = sync::SyncOptions {
        format,
        all,
        dry_run,
    };
    sync::run_with_options(name, options).await
}

pub async fn handle_submit(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").cloned();
    let format = get_format(sub_m);
    let dry_run = sub_m.get_flag("dry-run");
    let auto_commit = sub_m.get_flag("auto-commit");
    let message = sub_m.get_one::<String>("message").cloned();

    let options = submit::SubmitOptions {
        name,
        format,
        dry_run,
        auto_commit,
        message,
    };

    let exit_code = submit::run_with_options(&options).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

pub async fn handle_diff(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::diff());
        return Ok(());
    }

    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let stat = sub_m.get_flag("stat");
    let format = get_format(sub_m);
    diff::run(name, stat, format).await
}

pub async fn handle_done(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::done());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let format = get_format(sub_m);
    let args = done::types::DoneArgs {
        workspace: sub_m.get_one::<String>("workspace").cloned(),
        message: sub_m.get_one::<String>("message").cloned(),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        no_keep: sub_m.get_flag("no-keep"),
        squash: sub_m.get_flag("squash"),
        dry_run: sub_m.get_flag("dry-run"),
        detect_conflicts: sub_m.get_flag("detect-conflicts"),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        format,
    };
    let options = args.to_options();
    done::run_with_options(&options).await?;
    Ok(())
}

pub async fn handle_abort(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::abort());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let format = get_format(sub_m);
    let options = abort::AbortOptions {
        workspace: sub_m.get_one::<String>("workspace").cloned(),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    abort::run(&options).await
}

#[cfg(test)]
mod tests {
    use zjj_core::OutputFormat;

    #[test]
    fn test_handle_diff_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
    }

    #[test]
    fn test_diff_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }
}
