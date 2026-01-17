//! State management and orchestration for initialization

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::{dependencies, directory_setup, health, repo, workspace_operations};
use crate::database::SessionDb;

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
/// 6. Create .jjz directory and initialize database
pub async fn run_with_cwd_and_flags(
    cwd: Option<&Path>,
    repair: bool,
    force: bool,
) -> Result<()> {
    // Resolve working directory
    let cwd = resolve_cwd(cwd)?;

    // Check dependencies before any other work
    dependencies::check_dependencies()?;

    // Ensure we're in a JJ repository
    repo::ensure_jj_repo_with_cwd(&cwd)?;

    // Get repo root and database path
    let root = repo::jj_root_with_cwd(&cwd)?;
    let zjj_dir = root.join(".jjz");
    let db_path = zjj_dir.join("state.db");

    // Handle special flags
    if force {
        if zjj_dir.exists() {
            return workspace_operations::force_reinitialize(&zjj_dir, &db_path).await;
        }
        println!("No existing .jjz directory found. Proceeding with normal initialization.");
    }

    if repair {
        if !zjj_dir.exists() {
            bail!(
                "Cannot repair: .jjz directory does not exist.\n\
                 \n\
                 Run 'jjz init' first to initialize."
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

    // Create new .jjz directory
    create_new_initialization(&zjj_dir, &root, &db_path).await
}

/// Resolve the working directory with helpful error messages
fn resolve_cwd(cwd: Option<&Path>) -> Result<PathBuf> {
    match cwd {
        Some(p) => Ok(PathBuf::from(p)),
        None => std::env::current_dir()
            .context("Failed to get current directory")
            .context("Suggestions:")
            .context("  - Check if you have permission to access the current directory")
            .context("  - Try running from a different directory: cd /path/to/repo && jjz init"),
    }
}

/// Handle initialization of an existing .jjz directory
async fn handle_existing_directory(zjj_dir: &Path, db_path: &Path) -> Result<()> {
    match health::check_database_health(db_path).await {
        health::DatabaseHealth::Healthy => {
            println!("ZJZ already initialized in this repository.");
            print_initialization_hints();
            Ok(())
        }
        health::DatabaseHealth::Missing => {
            println!("Database file is missing but .jjz directory exists.");
            println!("Recreating database...");
            let _db = SessionDb::create_or_open(db_path).await?;
            Ok(())
        }
        health::DatabaseHealth::Empty => {
            println!("Database file is empty or corrupted.");
            print_repair_hints();
            bail!("Database is empty. Use --repair or --force to fix.");
        }
        health::DatabaseHealth::Corrupted(msg) => {
            println!("Database appears to be corrupted: {msg}");
            print_repair_hints();
            bail!("Database is corrupted. Use --repair or --force to fix.");
        }
        health::DatabaseHealth::WrongSchema => {
            println!("Database has wrong schema or is corrupted.");
            print_repair_hints();
            bail!("Database has wrong schema. Use --repair or --force to fix.");
        }
    }
}

/// Create a fresh .jjz directory with all necessary files
async fn create_new_initialization(
    zjj_dir: &Path,
    root: &Path,
    _db_path: &Path,
) -> Result<()> {
    // Create fresh directory structure (includes config setup and database creation)
    directory_setup::create_fresh_structure(zjj_dir).await?;

    // Print success message
    println!("Initialized ZJZ in {}", root.display());
    println!("  Data directory: .jjz/");
    println!("  Configuration: .jjz/config.toml");
    println!("  State database: .jjz/state.db");
    println!("  Layouts: .jjz/layouts/");

    Ok(())
}

/// Print hints for a healthy database
fn print_initialization_hints() {
    println!("\nSuggestions:");
    println!("  - View configuration: cat .jjz/config.toml");
    println!("  - Check status: jjz status");
    println!("  - List sessions: jjz list");
    println!("  - To repair database: jjz init --repair");
    println!("  - To force reinitialize: jjz init --force");
}

/// Print hints for database repair
fn print_repair_hints() {
    println!("\nTo fix this issue:");
    println!("  - Run 'jjz init --repair' to attempt repair");
    println!("  - Run 'jjz init --force' to reinitialize (creates backup)");
}
