//! State management and orchestration for initialization

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use super::{dependencies, directory_setup, health, repo, workspace_operations};
use crate::database::SessionDb;

/// JSON output for init command
#[derive(Debug, Serialize)]
pub struct InitOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zjj_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub already_initialized: Option<bool>,
}

/// Run the init command with an optional working directory and flags
///
/// # Arguments
///
/// * `cwd` - Optional working directory (defaults to current directory)
/// * `repair` - Attempt database repair if corrupted
/// * `force` - Force reinitialization with backup
///
/// # Flow
///
/// 1. Resolve working directory
/// 2. Check dependencies
/// 3. Ensure JJ repository exists
/// 4. Handle --force or --repair flags if specified
/// 5. Check if already initialized
/// 6. Create .zjj directory and initialize database
pub async fn run_with_cwd_and_flags(cwd: Option<&Path>, repair: bool, force: bool) -> Result<()> {
    // Resolve working directory
    let cwd = resolve_cwd(cwd)?;

    // Check dependencies before any other work
    dependencies::check_dependencies()?;

    // Ensure we're in a JJ repository
    repo::ensure_jj_repo_with_cwd(&cwd)?;

    // Get repo root and database path
    let root = repo::jj_root_with_cwd(&cwd)?;
    let zjj_dir = root.join(".zjj");
    let db_path = zjj_dir.join("state.db");

    // Handle special flags
    if force {
        if zjj_dir.exists() {
            return workspace_operations::force_reinitialize(&zjj_dir, &db_path).await;
        }
        println!("No existing .zjj directory found. Proceeding with normal initialization.");
    }

    if repair {
        if !zjj_dir.exists() {
            bail!(
                "Cannot repair: .zjj directory does not exist.\n\
                 \n\
                 Run 'zjj init' first to initialize."
            );
        }
        return health::repair_database(&db_path).await;
    }

    // Handle normal initialization
    if zjj_dir.exists() {
        // Directory exists - check database state
        handle_existing_directory(&zjj_dir, &db_path).await?;
        return Ok(());
    }

    // Create new .zjj directory
    create_new_initialization(&zjj_dir, &root, &db_path, false).await
}

/// Run the init command with an optional working directory and all flags including JSON
pub async fn run_with_cwd_and_all_flags(
    cwd: Option<&Path>,
    repair: bool,
    force: bool,
    json: bool,
) -> Result<()> {
    // Resolve working directory
    let cwd = resolve_cwd(cwd)?;

    // Check dependencies before any other work
    dependencies::check_dependencies()?;

    // Ensure we're in a JJ repository
    repo::ensure_jj_repo_with_cwd(&cwd)?;

    // Get repo root and database path
    let root = repo::jj_root_with_cwd(&cwd)?;
    let zjj_dir = root.join(".zjj");
    let db_path = zjj_dir.join("state.db");

    // Handle special flags
    if force {
        if zjj_dir.exists() {
            return workspace_operations::force_reinitialize(&zjj_dir, &db_path).await;
        }
        if json {
            output_json(&InitOutput {
                success: true,
                message: Some(
                    "No existing .zjj directory found. Proceeding with normal initialization."
                        .to_string(),
                ),
                zjj_dir: None,
                already_initialized: None,
            });
        } else {
            println!("No existing .zjj directory found. Proceeding with normal initialization.");
        }
    }

    if repair {
        if !zjj_dir.exists() {
            bail!(
                "Cannot repair: .zjj directory does not exist.\n\
                 \n\
                 Run 'zjj init' first to initialize."
            );
        }
        return health::repair_database(&db_path).await;
    }

    // Handle normal initialization
    if zjj_dir.exists() {
        // Directory exists - check database state
        handle_existing_directory_json(&zjj_dir, &db_path, json).await?;
        return Ok(());
    }

    // Create new .zjj directory
    create_new_initialization(&zjj_dir, &root, &db_path, json).await
}

/// Output JSON to stdout
fn output_json(output: &InitOutput) {
    if let Ok(json_str) = serde_json::to_string(output) {
        println!("{json_str}");
    }
}

/// Resolve the working directory with helpful error messages
fn resolve_cwd(cwd: Option<&Path>) -> Result<PathBuf> {
    cwd.map_or_else(
        || {
            std::env::current_dir()
                .context("Failed to get current directory")
                .context("Suggestions:")
                .context("  - Check if you have permission to access the current directory")
                .context("  - Try running from a different directory: cd /path/to/repo && zjj init")
        },
        |p| Ok(PathBuf::from(p)),
    )
}

/// Result of checking and potentially restoring config.toml
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigStatus {
    /// Config exists and is valid
    Present,
    /// Config was missing and has been restored
    Restored,
}

/// Ensure config.toml exists, restoring it if missing.
///
/// # Railway Pattern
/// Returns `Ok(ConfigStatus)` indicating whether config was present or restored.
/// Returns `Err` only if restoration fails.
fn ensure_config_exists(zjj_dir: &Path) -> Result<ConfigStatus> {
    let config_path = zjj_dir.join("config.toml");

    if config_path.exists() {
        Ok(ConfigStatus::Present)
    } else {
        super::config_setup::setup_config(zjj_dir)
            .map(|()| ConfigStatus::Restored)
            .context("Failed to restore missing config.toml")
    }
}

/// Handle initialization of an existing .zjj directory
async fn handle_existing_directory(zjj_dir: &Path, db_path: &Path) -> Result<()> {
    match health::check_database_health(db_path).await {
        health::DatabaseHealth::Healthy => {
            // Ensure config.toml exists using Railway pattern
            let config_status = ensure_config_exists(zjj_dir)?;

            // Report status based on what happened
            match config_status {
                ConfigStatus::Restored => {
                    println!("Config file was missing. Restored config.toml.");
                }
                ConfigStatus::Present => {
                    println!("ZJJ already initialized in this repository.");
                }
            }

            print_initialization_hints();
            Ok(())
        }
        health::DatabaseHealth::Missing => {
            println!("Database file is missing but .zjj directory exists.");
            println!("Recreating database...");
            SessionDb::create_or_open(db_path)
                .await
                .map(|_db| ())
                .context("Failed to recreate database")
        }
        health::DatabaseHealth::Empty => {
            print_repair_hints();
            bail!("Database is empty. Use --repair or --force to fix.");
        }
        health::DatabaseHealth::Corrupted(msg) => {
            println!("Database appears to be corrupted: {msg}");
            print_repair_hints();
            bail!("Database is corrupted. Use --repair or --force to fix.");
        }
        health::DatabaseHealth::WrongSchema => {
            print_repair_hints();
            bail!("Database has wrong schema. Use --repair or --force to fix.");
        }
    }
}

/// Handle initialization of an existing .zjj directory with JSON output support
async fn handle_existing_directory_json(zjj_dir: &Path, db_path: &Path, json: bool) -> Result<()> {
    match health::check_database_health(db_path).await {
        health::DatabaseHealth::Healthy => {
            // Ensure config.toml exists using Railway pattern
            let config_status = ensure_config_exists(zjj_dir)?;

            if json {
                let message = match config_status {
                    ConfigStatus::Restored => "Config file was missing. Restored config.toml.",
                    ConfigStatus::Present => "ZJJ already initialized in this repository.",
                };
                output_json(&InitOutput {
                    success: true,
                    message: Some(message.to_string()),
                    zjj_dir: Some(zjj_dir.to_string_lossy().to_string()),
                    already_initialized: Some(true),
                });
            } else {
                // Report status based on what happened
                match config_status {
                    ConfigStatus::Restored => {
                        println!("Config file was missing. Restored config.toml.");
                    }
                    ConfigStatus::Present => {
                        println!("ZJJ already initialized in this repository.");
                    }
                }
                print_initialization_hints();
            }
            Ok(())
        }
        health::DatabaseHealth::Missing => {
            if json {
                output_json(&InitOutput {
                    success: true,
                    message: Some("Database file was missing. Recreated database.".to_string()),
                    zjj_dir: Some(zjj_dir.to_string_lossy().to_string()),
                    already_initialized: Some(true),
                });
            } else {
                println!("Database file is missing but .zjj directory exists.");
                println!("Recreating database...");
            }
            SessionDb::create_or_open(db_path)
                .await
                .map(|_db| ())
                .context("Failed to recreate database")
        }
        health::DatabaseHealth::Empty => {
            if !json {
                print_repair_hints();
            }
            bail!("Database is empty. Use --repair or --force to fix.");
        }
        health::DatabaseHealth::Corrupted(msg) => {
            if !json {
                println!("Database appears to be corrupted: {msg}");
                print_repair_hints();
            }
            bail!("Database is corrupted. Use --repair or --force to fix.");
        }
        health::DatabaseHealth::WrongSchema => {
            if !json {
                print_repair_hints();
            }
            bail!("Database has wrong schema. Use --repair or --force to fix.");
        }
    }
}

/// Create a fresh .zjj directory with all necessary files
async fn create_new_initialization(
    zjj_dir: &Path,
    root: &Path,
    _db_path: &Path,
    json: bool,
) -> Result<()> {
    // Create fresh directory structure (includes config setup and database creation)
    directory_setup::create_fresh_structure(zjj_dir).await?;

    // Output result
    if json {
        output_json(&InitOutput {
            success: true,
            message: Some(format!("Initialized ZJJ in {}", root.display())),
            zjj_dir: Some(zjj_dir.to_string_lossy().to_string()),
            already_initialized: Some(false),
        });
    } else {
        println!("Initialized ZJJ in {}", root.display());
        println!("  Data directory: .zjj/");
        println!("  Configuration: .zjj/config.toml");
        println!("  State database: .zjj/state.db");
        println!("  Layouts: .zjj/layouts/");
    }

    Ok(())
}

/// Print hints for a healthy database
fn print_initialization_hints() {
    println!("\nSuggestions:");
    println!("  - View configuration: cat .zjj/config.toml");
    println!("  - Check status: zjj status");
    println!("  - List sessions: zjj list");
    println!("  - To repair database: zjj init --repair");
    println!("  - To force reinitialize: zjj init --force");
}

/// Print hints for database repair
fn print_repair_hints() {
    println!("\nTo fix this issue:");
    println!("  - Run 'zjj init --repair' to attempt repair");
    println!("  - Run 'zjj init --force' to reinitialize (creates backup)");
}
