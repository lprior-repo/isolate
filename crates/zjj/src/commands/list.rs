//! List all sessions

use anyhow::Result;

use crate::commands::get_session_db;

/// Run the list command
pub fn run() -> Result<()> {
    let db = get_session_db()?;
    let sessions = db.list()?;

    if sessions.is_empty() {
        println!("No sessions found.");
        println!("Use 'jjz add <name>' to create a session.");
        return Ok(());
    }

    println!("Sessions:");
    println!("{:<20} {:<30} WORKSPACE", "NAME", "ZELLIJ TAB");
    println!("{}", "-".repeat(70));

    for session in sessions {
        println!(
            "{:<20} {:<30} {}",
            session.name, session.zellij_tab, session.workspace_path
        );
    }

    Ok(())
}
