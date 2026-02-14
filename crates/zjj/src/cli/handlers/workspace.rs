//! Workspace session management handlers

use anyhow::Result;
use clap::ArgMatches;
use zjj_core::json::SchemaEnvelope;

use super::json_format::get_format;
use crate::{
    commands::{
        add, attach, focus, init, list, remove, rename, session_mgmt, spawn, status, switch, work,
    },
    json,
};

fn extract_json_payload(doc: &'static str) -> &'static str {
    doc.find('{').map_or(doc, |index| &doc[index..])
}

pub async fn handle_init(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    init::run_with_options(init::InitOptions { format }).await
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
    let idempotent = sub_m.get_flag("idempotent");
    let dry_run = sub_m.get_flag("dry-run");

    let options = add::AddOptions {
        name: name.clone(),
        bead_id,
        no_hooks,
        template,
        no_open,
        no_zellij,
        format: get_format(sub_m),
        idempotent,
        dry_run,
    };

    add::run_with_options(&options).await
}

pub async fn handle_list(sub_m: &ArgMatches) -> Result<()> {
    let all = sub_m.get_flag("all");
    let verbose = sub_m.get_flag("verbose");
    let format = get_format(sub_m);
    let bead = sub_m.get_one::<String>("bead").cloned();
    let agent = sub_m.get_one::<String>("agent").map(String::as_str);
    let state = sub_m.get_one::<String>("state").map(String::as_str);
    list::run(all, verbose, format, bead.as_deref(), agent, state).await
}

pub async fn handle_remove(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let format = get_format(sub_m);
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
    let no_zellij = sub_m.get_flag("no-zellij");
    let format = get_format(sub_m);
    let options = focus::FocusOptions { format, no_zellij };
    focus::run_with_options(name, &options).await
}

pub async fn handle_status(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!(
            "{}",
            extract_json_payload(crate::cli::json_docs::ai_contracts::status())
        );
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        if get_format(sub_m).is_json() {
            println!(
                "{}",
                extract_json_payload(crate::cli::json_docs::ai_contracts::command_flow())
            );
        } else {
            println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        }
        return Ok(());
    }

    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let format = get_format(sub_m);
    let watch = sub_m.get_flag("watch");
    status::run(name, format, watch).await
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
        no_zellij: sub_m.get_flag("no-zellij"),
        no_agent: sub_m.get_flag("no-agent"),
        idempotent: sub_m.get_flag("idempotent"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };
    work::run(&options).await
}

pub async fn handle_attach(sub_m: &ArgMatches) -> Result<()> {
    let options = attach::AttachOptions::from_matches(sub_m)?;
    attach::run_with_options(&options).await
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
    let no_zellij = sub_m.get_flag("no-zellij");
    let options = rename::RenameOptions {
        old_name,
        new_name,
        dry_run: false,
        no_zellij,
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
        no_zellij: sub_m.get_flag("no-zellij"),
        format,
    };
    session_mgmt::run_clone(&options).await
}

pub async fn handle_pause(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m.get_one::<String>("name").cloned().unwrap_or_default();
    let options = session_mgmt::PauseOptions { session, format };
    session_mgmt::run_pause(&options).await
}

pub async fn handle_resume(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m.get_one::<String>("name").cloned().unwrap_or_default();
    let options = session_mgmt::ResumeOptions { session, format };
    session_mgmt::run_resume(&options).await
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
}
