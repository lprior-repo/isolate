//! Switch to a session's Zellij tab

use anyhow::Result;

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, run_command},
    commands::get_session_db,
};

/// Run the focus command
pub fn run(name: &str) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    if is_inside_zellij() {
        // Inside Zellij: Switch to the tab
        run_command("zellij", &["action", "go-to-tab-name", &session.zellij_tab])?;
        println!("Switched to session '{name}'");
    } else {
        // Outside Zellij: Attach to the Zellij session
        // User will land in session and can navigate to desired tab
        println!("Session '{name}' is in tab '{}'", session.zellij_tab);
        println!("Attaching to Zellij session...");
        attach_to_zellij_session(None)?;
        // Note: This never returns - we exec into Zellij
    }

    Ok(())
}
