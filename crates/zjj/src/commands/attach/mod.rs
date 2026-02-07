use anyhow::{Context, Result};
use clap::ArgMatches;
use tokio::process::Command;

use crate::commands::get_session_db;

/// Options for the attach command
#[derive(Debug)]
pub struct AttachOptions {
    /// Name of the session to attach to
    pub name: String,
}

impl AttachOptions {
    /// Create options from clap matches
    pub fn from_matches(matches: &ArgMatches) -> Result<Self> {
        let name = matches
            .get_one::<String>("name")
            .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

        Ok(Self { name: name.clone() })
    }
}

/// Run the attach command with given options
pub async fn run_with_options(opts: &AttachOptions) -> Result<()> {
    let db = get_session_db().await?;

    // 1. Validate session exists
    let _session = db
        .get(&opts.name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", opts.name))?;

    // 2. Check if Zellij is installed
    Command::new("zellij")
        .arg("--version")
        .output()
        .await
        .map_err(|_| anyhow::anyhow!("Zellij is not installed or not in PATH"))?;

    // 3. Try to go to existing tab first (if zellij is running)
    let status = Command::new("zellij")
        .args(["action", "go-to-tab-name", &format!("zjj:{}", opts.name)])
        .status()
        .await
        .ok();

    // 4. If that fails, try to start Zellij with the session
    if status.is_none_or(|s| !s.success()) {
        // 'attach -c' connects to an existing session or creates a new one
        let status2 = Command::new("zellij")
            .args(["attach", "-c", &opts.name])
            .status()
            .await
            .context("Failed to execute zellij attach")?;

        if !status2.success() {
            anyhow::bail!("Failed to attach to session '{}'", opts.name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attach_options_creation() {
        let opts = AttachOptions {
            name: "test-session".to_string(),
        };
        assert_eq!(opts.name, "test-session");
    }
}
