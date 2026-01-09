//! Show current ZJJ status and context

use anyhow::Result;

use crate::{
    cli::{is_inside_zellij, is_jj_repo, jj_root, run_command},
    commands::get_session_db,
};

/// Run the status command
pub fn run() -> Result<()> {
    println!("ZJJ Status");
    println!("{}", "=".repeat(40));

    // Check JJ repository
    let in_jj_repo = is_jj_repo()?;
    if in_jj_repo {
        let root = jj_root()?;
        println!("JJ Repository: {root}");

        // Show JJ workspace info
        if let Ok(workspace_info) = run_command("jj", &["workspace", "list"]) {
            let workspace_count = workspace_info.lines().count();
            println!("JJ Workspaces: {workspace_count}");
        }
    } else {
        println!("JJ Repository: Not in a JJ repository");
    }

    // Check Zellij
    let in_zellij = is_inside_zellij();
    println!("Zellij Session: {}", if in_zellij { "Yes" } else { "No" });

    // Check ZJJ initialization and sessions
    if in_jj_repo {
        match get_session_db() {
            Ok(db) => {
                let sessions = db.list().unwrap_or_default();
                println!("ZJJ Initialized: Yes");
                println!("Sessions: {}", sessions.len());

                if !sessions.is_empty() {
                    println!();
                    println!("Active Sessions:");
                    for session in sessions {
                        println!("  - {} ({})", session.name, session.zellij_tab);
                    }
                }
            }
            Err(_) => {
                println!("ZJJ Initialized: No (run 'jjz init')");
            }
        }
    }

    Ok(())
}
