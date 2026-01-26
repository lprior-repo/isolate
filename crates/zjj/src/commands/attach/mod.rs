use std::process::Command;

use anyhow::{Context, Result};
use clap::ArgMatches;
use zjj_core::OutputFormat;

use crate::commands::get_session_db;

/// Options for the attach command
#[derive(Debug)]
pub struct AttachOptions {
    /// Name of the session to attach to
    pub name: String,
    /// Output format
    pub format: OutputFormat,
}

impl AttachOptions {
    /// Create options from clap matches
    pub fn from_matches(matches: &ArgMatches) -> Result<Self> {
        let name = matches
            .get_one::<String>("name")
            .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
        let json = matches.get_flag("json");
        let format = OutputFormat::from_json_flag(json);

        Ok(Self {
            name: name.clone(),
            format,
        })
    }
}

/// Run the attach command with given options
pub fn run_with_options(opts: &AttachOptions) -> Result<()> {
    let db = get_session_db()?;

    // 1. Validate session exists
    let _session = db
        .get(&opts.name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", opts.name))?;

    // 2. Check if Zellij is installed
    if Command::new("zellij").arg("--version").output().is_err() {
        return Err(anyhow::anyhow!("Zellij is not installed or not in PATH"));
    }

    // 3. Try to go to existing tab first (if zellij is running)
    let mut cmd = Command::new("zellij");
    cmd.args(["action", "go-to-tab-name", &format!("zjj:{}", opts.name)]);

    // We ignore the error here because if we're not inside Zellij or it's not running,
    // this will fail, and we'll fall back to attach
    let status = cmd.status().ok();

    if status.is_none_or(|s| !s.success()) {
        // If that fails, try to start Zellij with the session
        // Note: 'attach -c' connects to an existing session or creates a new one
        // We use the session name as the zellij session name
        let mut cmd2 = Command::new("zellij");
        cmd2.args(["attach", "-c", &opts.name]);

        let status2 = cmd2.status().context("Failed to execute zellij attach")?;

        if !status2.success() {
            return Err(anyhow::anyhow!(
                "Failed to attach to session '{}'",
                opts.name
            ));
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
            format: OutputFormat::Human,
        };
        assert_eq!(opts.name, "test-session");
    }
}
