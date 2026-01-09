//! Create a new session with JJ workspace + Zellij tab

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use zjj_core::jj;

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, jj_root, run_command},
    commands::get_session_db,
};

/// Run the add command
pub fn run(name: &str) -> Result<()> {
    let db = get_session_db()?;

    // Check if session already exists
    if db.get(name)?.is_some() {
        bail!("Session '{name}' already exists");
    }

    let root = jj_root()?;
    let workspace_path = format!("{root}/.jjz/workspaces/{name}");

    // Create the JJ workspace (works outside Zellij too)
    create_jj_workspace(&root, name, &workspace_path)?;

    // Insert into database after workspace is created successfully
    let session = db.create(name, &workspace_path)?;

    if is_inside_zellij() {
        // Inside Zellij: Create tab and switch to it
        create_zellij_tab(&session.zellij_tab, &workspace_path)?;
        println!(
            "Created session '{name}' with Zellij tab '{}'",
            session.zellij_tab
        );
    } else {
        // Outside Zellij: Create layout and exec into Zellij
        println!("Created session '{name}'");
        println!("Launching Zellij with new tab...");

        let layout = create_session_layout(&session.zellij_tab, &workspace_path);
        attach_to_zellij_session(Some(&layout))?;
        // Note: This never returns - we exec into Zellij
    }

    Ok(())
}

/// Create a JJ workspace for the session
fn create_jj_workspace(_repo_root: &str, name: &str, workspace_path: &str) -> Result<()> {
    // Use the JJ workspace manager from core
    let path = PathBuf::from(workspace_path);
    jj::workspace_create(name, &path)
        .map_err(|e| anyhow::anyhow!("Failed to create JJ workspace: {e}"))?;

    Ok(())
}

/// Create a Zellij tab for the session
fn create_zellij_tab(tab_name: &str, workspace_path: &str) -> Result<()> {
    // Create new tab with the session name
    run_command("zellij", &["action", "new-tab", "--name", tab_name])
        .context("Failed to create Zellij tab")?;

    // Change to the workspace directory in the new tab
    // We use write-chars to send the cd command
    let cd_command = format!("cd {workspace_path}\n");
    run_command("zellij", &["action", "write-chars", &cd_command])
        .context("Failed to change directory in Zellij tab")?;

    Ok(())
}

/// Create a Zellij layout for the session
/// This layout creates a tab with the session name and cwd set to workspace
fn create_session_layout(tab_name: &str, workspace_path: &str) -> String {
    format!(
        r#"
layout {{
    tab name="{tab_name}" {{
        pane {{
            cwd "{workspace_path}"
        }}
    }}
}}
"#
    )
}
