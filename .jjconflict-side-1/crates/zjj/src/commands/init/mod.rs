//! Initialize ZJJ - sets up everything needed

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::db::SessionDb;

mod deps;
mod setup;
mod types;

use deps::{check_dependencies, ensure_jj_repo_with_cwd, jj_root_with_cwd};
use setup::{
    create_jj_hooks, create_jjignore, create_moon_pipeline, create_repo_ai_instructions,
    DEFAULT_CONFIG,
};
use types::{build_init_response, InitPaths, InitResponse};

/// Run init command with options
#[derive(Debug, Clone, Copy, Default)]
pub struct InitOptions {
    pub format: OutputFormat,
}

/// Run the init command
///
/// This command:
/// 1. Checks that required dependencies (jj, zellij) are installed
/// 2. Initializes a JJ repository if not already present
/// 3. Creates the .zjj directory structure:
///    - .zjj/config.toml (default configuration)
///    - .zjj/state.db (sessions database)
///    - .zjj/layouts/ (Zellij layouts directory)
#[expect(dead_code)] // Convenience wrapper, currently unused but part of public API
pub fn run() -> Result<()> {
    run_with_options(InitOptions::default())
}

/// Run init command with options
pub fn run_with_options(options: InitOptions) -> Result<()> {
    run_with_cwd_and_format(None, options.format)
}

/// Run init command with cwd and format
pub fn run_with_cwd_and_format(cwd: Option<&Path>, format: OutputFormat) -> Result<()> {
    let cwd = match cwd {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    // Check required dependencies
    check_dependencies()?;

    // Initialize JJ repo if needed
    ensure_jj_repo_with_cwd(&cwd)?;

    // Get the repo root using the provided cwd
    let root = jj_root_with_cwd(&cwd)?;
    let zjj_dir = root.join(".zjj");

    // Define paths for all essential files
    let config_path = zjj_dir.join("config.toml");
    let layouts_dir = zjj_dir.join("layouts");
    let db_path = zjj_dir.join("state.db");

    // Check if already fully initialized
    let is_fully_initialized =
        zjj_dir.exists() && config_path.exists() && layouts_dir.exists() && db_path.exists();

    if is_fully_initialized {
        let response = InitResponse {
            message: "zjj already initialized in this repository.".to_string(),
            root: root.display().to_string(),
            paths: InitPaths {
                data_directory: ".zjj/".to_string(),
                config: ".zjj/config.toml".to_string(),
                state_db: ".zjj/state.db".to_string(),
                layouts: ".zjj/layouts/".to_string(),
            },
            jj_initialized: true,
            already_initialized: true,
        };

        if format.is_json() {
            let envelope = SchemaEnvelope::new("init-response", "single", response);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("zjj already initialized in this repository.");
        }

        return Ok(());
    }

    // Create .zjj directory if missing
    if !zjj_dir.exists() {
        fs::create_dir_all(&zjj_dir).context("Failed to create .zjj directory")?;
    }

    // Create config.toml if missing
    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_CONFIG).context("Failed to create config.toml")?;
    }

    // Create layouts directory if missing
    if !layouts_dir.exists() {
        fs::create_dir_all(&layouts_dir).context("Failed to create layouts directory")?;
    }

    // Create .jjignore to prevent .zjj tracking (avoids nested .jj conflicts)
    create_jjignore(&root)?;

    // Create JJ hooks to enforce zjj workflow (agents can't bypass with --no-verify)
    create_jj_hooks(&root)?;

    // Create repo-level AI discoverability file
    create_repo_ai_instructions(&root)?;

    // Create Moon build pipeline configuration (.moon/)
    create_moon_pipeline(&root)?;

    // Initialize the database (create if it doesn't exist)
    // db_path already defined above
    let _db = SessionDb::create_or_open_blocking(&db_path)?;

    if format.is_json() {
        let response = build_init_response(&root, false);
        let envelope = SchemaEnvelope::new("init-response", "single", response);
        println!("{}", serde_json::to_string(&envelope)?);
    } else {
        println!("Initialized zjj in {}", root.display());
        println!("  Data directory: .zjj/");
        println!("  Configuration: .zjj/config.toml");
        println!("  State database: .zjj/state.db");
        println!("  Layouts: .zjj/layouts/");
    }

    Ok(())
}

#[cfg(test)]
mod tests;
