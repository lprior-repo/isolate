//! Initialize ZJJ - sets up everything needed

use std::{fmt::Write, fs};

use anyhow::{bail, Context, Result};

use crate::{
    cli::{init_jj_repo, is_jj_installed, is_jj_repo, is_zellij_installed, jj_root},
    db::SessionDb,
};

/// Run the init command
///
/// This command:
/// 1. Checks that required dependencies (jj, zellij) are installed
/// 2. Initializes a JJ repository if not already present
/// 3. Creates the .jjz directory and sessions database
pub fn run() -> Result<()> {
    // Check required dependencies
    check_dependencies()?;

    // Initialize JJ repo if needed
    ensure_jj_repo()?;

    // Get the repo root
    let root = jj_root()?;
    let zjj_dir = format!("{root}/.jjz");

    // Check if already initialized
    if fs::metadata(&zjj_dir).is_ok() {
        println!("ZJJ already initialized in this repository.");
        return Ok(());
    }

    // Create .jjz directory
    fs::create_dir_all(&zjj_dir).context("Failed to create .jjz directory")?;

    // Initialize the database
    let db_path = format!("{zjj_dir}/sessions.db");
    let _db = SessionDb::open(std::path::Path::new(&db_path))?;

    println!("Initialized ZJJ in {root}");
    println!("  Data directory: .jjz/");
    println!("  Sessions database: .jjz/sessions.db");

    Ok(())
}

/// Check that required dependencies are installed
fn check_dependencies() -> Result<()> {
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

/// Ensure we're in a JJ repository, initializing one if needed
fn ensure_jj_repo() -> Result<()> {
    if is_jj_repo()? {
        return Ok(());
    }

    println!("No JJ repository found. Initializing one...");
    init_jj_repo()?;
    println!("Initialized JJ repository.");

    Ok(())
}
