//! Switch to a different workspace

use anyhow::Result;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{
    cli::{is_inside_zellij, is_terminal, run_command},
    commands::get_session_db,
    json::FocusOutput,
};

/// Options for the switch command
#[derive(Debug, Clone, Default)]
pub struct SwitchOptions {
    /// Output format
    pub format: OutputFormat,
    /// Show context after switching
    pub show_context: bool,
    /// Skip Zellij integration entirely (for non-TTY environments)
    pub no_zellij: bool,
}

/// Run the switch command with options
pub async fn run_with_options(name: Option<&str>, options: &SwitchOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Resolve the name (either provided or selected interactively)
    let resolved_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Interactive selection
        let sessions = db.list(None).await?;

        if sessions.is_empty() {
            if options.format.is_json() {
                return Err(anyhow::anyhow!("No sessions found"));
            }
            println!("No sessions found. Create one with 'zjj add <name>'.");
            return Ok(());
        }

        if let Some(session) = crate::selector::select_session(&sessions).await? {
            session.name
        } else {
            return Ok(()); // User cancelled
        }
    };

    // Get the session
    let session = db.get(&resolved_name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{resolved_name}' not found"
        )))
    })?;

    let zellij_tab = session.zellij_tab;

    // Check if we should skip Zellij integration
    let no_zellij = options.no_zellij || !is_terminal() || !crate::cli::is_zellij_installed().await;

    if no_zellij {
        // Skip Zellij integration - just print info
        if options.format.is_json() {
            let output = FocusOutput {
                name: resolved_name.clone(),
                zellij_tab: zellij_tab.clone(),
                message: format!(
                    "Session '{resolved_name}' is in tab '{zellij_tab}' (Zellij disabled)"
                ),
            };
            let envelope = SchemaEnvelope::new("switch-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("Session '{resolved_name}' is in tab '{zellij_tab}'");
            println!("Workspace path: {}", session.workspace_path);
        }
        return Ok(());
    }

    // Only switch if inside Zellij
    if !is_inside_zellij() {
        let zellij_installed = crate::cli::is_zellij_installed().await;
        if !is_terminal() && !options.no_zellij && !options.format.is_json() {
            eprintln!("Not in a terminal. Use --json for scripted access.");
        } else if !zellij_installed && !options.no_zellij && !options.format.is_json() {
            eprintln!("Zellij not found. Install it or use --no-zellij flag.");
        }
        if options.format.is_json() {
            return Err(anyhow::anyhow!(
                "Cannot switch tabs outside Zellij. Use 'zjj attach' instead."
            ));
        }
        println!("Not inside Zellij session.");
        println!(
            "Use 'zjj attach' to enter Zellij, then use 'zjj switch' to navigate between tabs."
        );
        return Ok(());
    }

    // Switch to the tab
    run_command("zellij", &["action", "go-to-tab-name", &zellij_tab]).await?;

    if options.format.is_json() {
        let output = FocusOutput {
            name: resolved_name.clone(),
            zellij_tab,
            message: format!("Switched to session '{resolved_name}'"),
        };
        let envelope = SchemaEnvelope::new("switch-response", "single", output);
        println!("{}", serde_json::to_string(&envelope)?);
    } else {
        println!("✓ Switched to: {resolved_name}");

        // Show context if requested
        if options.show_context {
            println!();
            println!("📍 Session Details");
            println!("  Workspace: {}", session.workspace_path);

            // Try to get bead info from metadata
            if let Some(ref metadata) = session.metadata {
                if let Some(bead_id) = metadata.get("bead_id").and_then(|v| v.as_str()) {
                    print!("  Bead:      {bead_id}");
                    if let Some(title) = metadata.get("bead_title").and_then(|v| v.as_str()) {
                        print!(" ({title})");
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_options_default() {
        let options = SwitchOptions::default();
        assert_eq!(options.format, OutputFormat::Human);
        assert!(!options.show_context);
        assert!(!options.no_zellij);
    }

    #[test]
    fn test_switch_options_no_zellij() {
        let options = SwitchOptions {
            format: OutputFormat::Json,
            show_context: true,
            no_zellij: true,
        };
        assert_eq!(options.format, OutputFormat::Json);
        assert!(options.show_context);
        assert!(options.no_zellij);
    }
}
