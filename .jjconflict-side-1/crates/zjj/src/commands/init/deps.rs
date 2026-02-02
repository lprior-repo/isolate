//! Dependency checking and JJ repository management

use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::cli::{is_jj_installed, is_zellij_installed};

/// Check that required dependencies are installed
pub(super) fn check_dependencies() -> Result<()> {
    let mut missing = Vec::new();

    if !is_jj_installed() {
        missing.push("jj (Jujutsu)");
    }

    if !is_zellij_installed() {
        missing.push("zellij");
    }

    if missing.is_empty() {
        return Ok(());
    }

    let mut msg = String::from("Missing required dependencies:\n\n");

    for dep in &missing {
        let _ = writeln!(msg, "  - {dep}");
    }

    msg.push_str("\nInstallation instructions:\n");

    if missing.contains(&"jj (Jujutsu)") {
        msg.push_str("\n  jj (Jujutsu):\n");
        msg.push_str("    cargo install jj-cli\n");
        msg.push_str("    # or: brew install jj\n");
        msg.push_str("    # or: https://martinvonz.github.io/jj/latest/install-and-setup/\n");
    }

    if missing.contains(&"zellij") {
        msg.push_str("\n  zellij:\n");
        msg.push_str("    cargo install zellij\n");
        msg.push_str("    # or: brew install zellij\n");
        msg.push_str("    # or: https://zellij.dev/documentation/installation\n");
    }

    bail!("{msg}")
}

/// Ensure we're in a JJ repository, initializing one if needed with a specific cwd
pub(super) fn ensure_jj_repo_with_cwd(cwd: &Path) -> Result<()> {
    if is_jj_repo_with_cwd(cwd)? {
        return Ok(());
    }

    println!("No JJ repository found. Initializing one...");
    init_jj_repo_with_cwd(cwd)?;
    println!("Initialized JJ repository.");

    Ok(())
}

/// Get the JJ root using a specific working directory
pub(super) fn jj_root_with_cwd(cwd: &Path) -> Result<PathBuf> {
    let output = std::process::Command::new("jj")
        .args(["root"])
        .current_dir(cwd)
        .output()
        .context("Failed to run jj root")?;

    if !output.status.success() {
        bail!("jj failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

/// Check if we're in a JJ repo using a specific cwd
fn is_jj_repo_with_cwd(cwd: &Path) -> Result<bool> {
    let output = std::process::Command::new("jj")
        .args(["status"])
        .current_dir(cwd)
        .output()?;

    Ok(output.status.success())
}

/// Initialize a JJ repo using a specific cwd
fn init_jj_repo_with_cwd(cwd: &Path) -> Result<()> {
    let output = std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(cwd)
        .output()
        .context("Failed to run jj git init")?;

    if !output.status.success() {
        bail!(
            "jj git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
