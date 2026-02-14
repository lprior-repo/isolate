//! Initialize ZJJ - sets up everything needed

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fs4::fs_std::FileExt;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::db::SessionDb;

mod deps;
mod setup;
mod types;

use deps::{check_dependencies, ensure_jj_repo_with_cwd, is_jj_repo_with_cwd, jj_root_with_cwd};
use setup::{
    create_agents_md, create_claude_md, create_docs, create_jj_hooks, create_jjignore,
    create_moon_pipeline, create_repo_ai_instructions, DEFAULT_CONFIG,
};
use types::{build_init_response, InitPaths, InitResponse};

/// Run init command with options
#[derive(Debug, Clone, Copy, Default)]
pub struct InitOptions {
    pub format: OutputFormat,
    pub dry_run: bool,
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
pub async fn run() -> Result<()> {
    run_with_options(InitOptions::default()).await
}

/// Run init command with options
pub async fn run_with_options(options: InitOptions) -> Result<()> {
    run_with_cwd_format_and_options(None, options).await
}

/// Run init command with cwd and format
pub async fn run_with_cwd_and_format(cwd: Option<&Path>, format: OutputFormat) -> Result<()> {
    run_with_cwd_format_and_options(
        cwd,
        InitOptions {
            format,
            dry_run: false,
        },
    )
    .await
}

async fn run_with_cwd_format_and_options(cwd: Option<&Path>, options: InitOptions) -> Result<()> {
    let cwd = match cwd {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    // Check required dependencies
    check_dependencies().await?;

    if options.dry_run {
        return preview_init(&cwd, options.format).await;
    }

    // Initialize JJ repo if needed
    ensure_jj_repo_with_cwd(&cwd, options.format.is_json()).await?;

    // Get the repo root using the provided cwd
    let root = jj_root_with_cwd(&cwd).await?;
    let zjj_dir = root.join(".zjj");

    // Define paths for all essential files
    let config_path = zjj_dir.join("config.toml");
    let layouts_dir = zjj_dir.join("layouts");
    let db_path = zjj_dir.join("state.db");

    // Check if already fully initialized
    let is_fully_initialized = tokio::fs::try_exists(&zjj_dir).await.unwrap_or(false)
        && tokio::fs::try_exists(&config_path).await.unwrap_or(false)
        && tokio::fs::try_exists(&layouts_dir).await.unwrap_or(false)
        && tokio::fs::try_exists(&db_path).await.unwrap_or(false);

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

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("init-response", "single", response);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("zjj already initialized in this repository.");
        }

        return Ok(());
    }

    // Create .zjj directory if missing
    if !tokio::fs::try_exists(&zjj_dir).await.unwrap_or(false) {
        tokio::fs::create_dir_all(&zjj_dir)
            .await
            .context("Failed to create .zjj directory")?;
    }

    // Acquire exclusive lock to prevent concurrent initialization
    let lock_path = zjj_dir.join(".init.lock");

    // Use spawn_blocking to avoid blocking Tokio executor on lock acquisition
    let lock_path_clone = lock_path.clone();
    let lock_file = tokio::task::spawn_blocking(move || -> Result<std::fs::File> {
        let is_symlink = match std::fs::symlink_metadata(&lock_path_clone) {
            Ok(metadata) => metadata.is_symlink(),
            Err(_) => false,
        };
        if is_symlink {
            bail!("Security: .zjj/.init.lock is a symlink. Remove it and re-run zjj init.");
        }

        let mut open_options = std::fs::OpenOptions::new();
        open_options
            .create(true)
            .read(true)
            .write(true)
            .truncate(false);

        #[cfg(unix)]
        {
            open_options.custom_flags(libc::O_NOFOLLOW);
        }

        // Protect against symlink attacks and avoid truncating lock targets.
        let file = open_options
            .open(&lock_path_clone)
            .with_context(|| format!("Failed to open lock file at {}", lock_path_clone.display()))
            .or_else(|error| {
                let symlink_detected = match std::fs::symlink_metadata(&lock_path_clone) {
                    Ok(metadata) => metadata.is_symlink(),
                    Err(_) => false,
                };

                if symlink_detected {
                    bail!("Security: .zjj/.init.lock is a symlink. Remove it and re-run zjj init.");
                }

                Err(error)
            })?;

        // Acquire exclusive lock (blocking - waits for other init to complete)
        file.lock_exclusive()
            .context("Failed to acquire init lock")?;

        Ok(file)
    })
    .await
    .context("Lock task panicked")??;

    // Double-check initialization status after acquiring lock
    let is_now_initialized = tokio::fs::try_exists(&config_path).await.unwrap_or(false)
        && tokio::fs::try_exists(&layouts_dir).await.unwrap_or(false)
        && tokio::fs::try_exists(&db_path).await.unwrap_or(false);

    if is_now_initialized {
        // Another process completed initialization while we waited
        // Release lock but keep file on disk (file locks are inode-based)
        let _ = lock_file.unlock();

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

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("init-response", "single", response);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("zjj already initialized in this repository.");
        }

        return Ok(());
    }

    // Create config.toml if missing
    if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        tokio::fs::write(&config_path, DEFAULT_CONFIG)
            .await
            .context("Failed to create config.toml")?;
    }

    // Create layouts directory if missing
    if !tokio::fs::try_exists(&layouts_dir).await.unwrap_or(false) {
        tokio::fs::create_dir_all(&layouts_dir)
            .await
            .context("Failed to create layouts directory")?;
    }

    // Create .jjignore to prevent .zjj tracking (avoids nested .jj conflicts)
    create_jjignore(&root).await?;

    // Create JJ hooks to enforce zjj workflow (agents can't bypass with --no-verify)
    create_jj_hooks(&root).await?;

    // Create repo-level AI discoverability file
    create_repo_ai_instructions(&root).await?;

    // Create Moon build pipeline configuration (.moon/)
    create_moon_pipeline(&root).await?;

    // Create unified AI instructions (AGENTS.md and CLAUDE.md)
    create_agents_md(&root)
        .await
        .context("Failed to create AGENTS.md")?;
    create_claude_md(&root)
        .await
        .context("Failed to create CLAUDE.md")?;

    // Create documentation files from templates
    create_docs(&root)
        .await
        .context("Failed to create documentation files")?;

    // Initialize the database (create if it doesn't exist)
    // db_path already defined above
    let _db = SessionDb::create_or_open(&db_path).await?;

    // Release lock but keep file on disk (file locks are inode-based)
    let _ = lock_file.unlock();

    if options.format.is_json() {
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

async fn preview_init(cwd: &Path, format: OutputFormat) -> Result<()> {
    let repo_exists = is_jj_repo_with_cwd(cwd).await?;
    let root = if repo_exists {
        jj_root_with_cwd(cwd).await?
    } else {
        cwd.to_path_buf()
    };

    let zjj_dir = root.join(".zjj");
    let config_path = zjj_dir.join("config.toml");
    let layouts_dir = zjj_dir.join("layouts");
    let db_path = zjj_dir.join("state.db");

    let has_zjj_dir = tokio::fs::try_exists(&zjj_dir).await.unwrap_or(false);
    let has_config = tokio::fs::try_exists(&config_path).await.unwrap_or(false);
    let has_layouts = tokio::fs::try_exists(&layouts_dir).await.unwrap_or(false);
    let has_db = tokio::fs::try_exists(&db_path).await.unwrap_or(false);

    let is_fully_initialized = has_zjj_dir && has_config && has_layouts && has_db;
    let mut response = build_init_response(&root, is_fully_initialized);
    response.jj_initialized = repo_exists;
    response.message = if is_fully_initialized {
        "Dry run: zjj is already initialized; no changes required.".to_string()
    } else {
        format!("Dry run: would initialize zjj in {}", root.display())
    };

    let mut actions = Vec::new();
    if !repo_exists {
        actions.push("Would initialize JJ repository".to_string());
    }
    if !has_zjj_dir {
        actions.push("Would create .zjj directory".to_string());
    }
    if !has_config {
        actions.push("Would create .zjj/config.toml".to_string());
    }
    if !has_layouts {
        actions.push("Would create .zjj/layouts".to_string());
    }
    if !has_db {
        actions.push("Would create .zjj/state.db".to_string());
    }
    if !is_fully_initialized {
        actions.push("Would create hooks, docs, and helper files".to_string());
    }

    if format.is_json() {
        let mut envelope =
            serde_json::to_value(SchemaEnvelope::new("init-response", "single", response))?;
        if let Some(obj) = envelope.as_object_mut() {
            obj.insert("dry_run".to_string(), serde_json::Value::Bool(true));
            obj.insert(
                "planned_actions".to_string(),
                serde_json::to_value(actions)?,
            );
        }
        println!("{}", serde_json::to_string(&envelope)?);
    } else {
        println!("{}", response.message);
        if actions.is_empty() {
            println!("  No changes would be made.");
        } else {
            actions.iter().for_each(|action| println!("  - {action}"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
