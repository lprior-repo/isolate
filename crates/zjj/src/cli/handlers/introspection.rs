//! AI and introspection handlers

use anyhow::Result;
use clap::ArgMatches;
use zjj_core::OutputFormat;

use super::json_format::get_format;
use crate::{
    cli::build_cli,
    commands::{
        ai, can_i, context, contract, examples, introspect, validate, whatif, whereami, whoami,
    },
};

pub async fn handle_ai(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
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

pub async fn handle_introspect(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::introspect());
        return Ok(());
    }
    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }
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

pub async fn handle_context(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let field = sub_m.get_one::<String>("field").map(String::as_str);
    let no_beads = sub_m.get_flag("no-beads");
    let no_health = sub_m.get_flag("no-health");
    context::run(format, field, no_beads, no_health).await
}

pub async fn handle_whereami(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::whereami());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let format = get_format(sub_m);
    let options = whereami::WhereAmIOptions { format };
    whereami::run(&options).await
}

pub fn handle_whoami(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::whoami());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let format = get_format(sub_m);
    let options = whoami::WhoAmIOptions { format };
    whoami::run(&options)
}

pub async fn handle_can_i(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::can_i());
        return Ok(());
    }
    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }
    let format = get_format(sub_m);
    let action = sub_m
        .get_one::<String>("action")
        .ok_or_else(|| anyhow::anyhow!("Action is required"))?
        .clone();
    let resource = sub_m.get_one::<String>("resource").cloned();
    let options = can_i::CanIOptions {
        action,
        resource,
        format,
    };
    can_i::run(&options).await
}

pub fn handle_contract(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let command = sub_m.get_one::<String>("command").cloned();
    let options = contract::ContractOptions { command, format };
    contract::run(&options)
}

pub fn handle_examples(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
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
    let command = sub_m.get_one::<String>("command").map(String::as_str);
    let mut cli = build_cli();
    match command {
        None | Some("-h" | "--help") => {
            cli.print_help().map_err(anyhow::Error::new)?;
            println!();
            Ok(())
        }
        Some(name) => {
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
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::validate());
        return Ok(());
    }
    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }
    let format = get_format(sub_m);
    let command = sub_m
        .get_one::<String>("command")
        .ok_or_else(|| anyhow::anyhow!("Command is required"))?
        .clone();
    let args: Vec<String> = sub_m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let options = validate::ValidateOptions {
        command,
        args,
        format,
    };
    validate::run(&options)
}

pub fn handle_whatif(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let command = sub_m
        .get_one::<String>("command")
        .ok_or_else(|| anyhow::anyhow!("Command is required"))?
        .clone();
    let args: Vec<String> = sub_m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let options = whatif::WhatIfOptions {
        command,
        args,
        format,
    };
    let result = whatif::run(&options)?;

    if format.is_json() {
        let envelope = zjj_core::json::SchemaEnvelope::new("whatif-response", "single", result);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        println!("{json_str}");
    } else {
        println!("What-if preview for '{}' command:", options.command);
        println!();

        for step in &result.steps {
            println!("  {}. {}", step.order, step.description);
            println!("     > {}", step.action);
            if step.can_fail {
                if let Some(failure) = &step.on_failure {
                    println!("     (can fail: {failure})");
                } else {
                    println!("     (can fail)");
                }
            }
            println!();
        }

        if !result.creates.is_empty() {
            println!("  Creates:");
            for create in &result.creates {
                println!("    - {} ({})", create.resource, create.resource_type);
                println!("      {}", create.description);
            }
            println!();
        }

        if !result.modifies.is_empty() {
            println!("  Modifies:");
            for modify in &result.modifies {
                println!("    - {} ({})", modify.resource, modify.resource_type);
                println!("      {}", modify.description);
            }
            println!();
        }

        if !result.deletes.is_empty() {
            println!("  Deletes:");
            for delete in &result.deletes {
                println!("    - {} ({})", delete.resource, delete.resource_type);
                println!("      {}", delete.description);
            }
            println!();
        }

        if !result.side_effects.is_empty() {
            println!("  Side effects:");
            for effect in &result.side_effects {
                println!("    - {}", effect);
            }
            println!();
        }

        if result.reversible {
            println!("  Reversible: Yes");
            if let Some(undo) = &result.undo_command {
                println!("  Undo command: {}", undo);
            }
            println!();
        }

        if !result.warnings.is_empty() {
            println!("  Warnings:");
            for warning in &result.warnings {
                println!("    - {}", warning);
            }
            println!();
        }

        if !result.prerequisites.is_empty() {
            println!("  Prerequisites:");
            for prereq in &result.prerequisites {
                let status = match prereq.status {
                    whatif::PrerequisiteStatus::Met => "✓ Met",
                    whatif::PrerequisiteStatus::NotMet => "✗ Not met",
                    whatif::PrerequisiteStatus::Unknown => "? Unknown",
                };
                println!("    {} {}", status, prereq.check);
                println!("      {}", prereq.description);
            }
            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use zjj_core::OutputFormat;

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
