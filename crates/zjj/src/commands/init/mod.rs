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

// ============================================================================
// InitLock - RAII guard for init lock file
// ============================================================================

/// RAII guard that ensures lock file cleanup on drop (including error paths)
struct InitLock {
    file: std::fs::File,
    path: PathBuf,
    released: bool,
}

impl InitLock {
    /// Acquire exclusive lock, handling symlink attacks
    fn acquire(lock_path: PathBuf) -> Result<Self> {
        // Protect against symlink attacks.
        // On Unix, O_NOFOLLOW closes the TOCTOU gap between checking and opening.
        #[cfg(not(unix))]
        if let Ok(metadata) = std::fs::symlink_metadata(&lock_path) {
            if metadata.is_symlink() {
                bail!(
                    "Security: {} is a symlink. Remove it and re-run zjj init.",
                    lock_path.display()
                );
            }
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

        // Open lock file without truncate to avoid clobbering lock targets.
        let file = match open_options.open(&lock_path) {
            Ok(file) => file,
            Err(open_error) => {
                if let Ok(metadata) = std::fs::symlink_metadata(&lock_path) {
                    if metadata.is_symlink() {
                        bail!(
                            "Security: {} is a symlink. Remove it and re-run zjj init.",
                            lock_path.display()
                        );
                    }
                }

                return Err(open_error).with_context(|| {
                    format!("Failed to open lock file at {}", lock_path.display())
                });
            }
        };

        // Try to acquire exclusive lock (blocking)
        file.lock_exclusive().with_context(|| {
            format!(
                "Another zjj init is in progress. If this is incorrect, remove {} and retry.",
                lock_path.display()
            )
        })?;

        Ok(Self {
            file,
            path: lock_path,
            released: false,
        })
    }

    /// Explicitly release the lock (keeps file on disk to prevent inode races)
    fn release(mut self) -> Result<()> {
        if !self.released {
            self.released = true;
            // Release lock but keep file on disk (file locks are inode-based)
            self.file
                .unlock()
                .with_context(|| format!("Failed to release lock at {}", self.path.display()))?;
        }
        Ok(())
    }
}

impl Drop for InitLock {
    fn drop(&mut self) {
        if !self.released {
            // Best-effort lock release on drop (e.g., due to error)
            // Keep file on disk to prevent inode-based race conditions
            let _ = self.file.unlock();
        }
    }
}

use deps::{check_dependencies, ensure_jj_repo_with_cwd, jj_root_with_cwd};
use setup::{
    create_agents_md, create_claude_md, create_docs, create_jj_hooks, create_jjignore,
    create_moon_pipeline, create_repo_ai_instructions, DEFAULT_CONFIG,
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
pub async fn run() -> Result<()> {
    run_with_options(InitOptions::default()).await
}

/// Run init command with options
pub async fn run_with_options(options: InitOptions) -> Result<()> {
    run_with_cwd_and_format(None, options.format).await
}

/// Run init command with cwd and format
pub async fn run_with_cwd_and_format(cwd: Option<&Path>, format: OutputFormat) -> Result<()> {
    let cwd = match cwd {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    // Check required dependencies
    check_dependencies().await?;

    // Initialize JJ repo if needed
    ensure_jj_repo_with_cwd(&cwd, format.is_json()).await?;

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

        if format.is_json() {
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
    // Uses spawn_blocking to avoid blocking Tokio executor
    let lock_path = zjj_dir.join(".init.lock");
    let lock_path_clone = lock_path.clone();

    let init_lock = tokio::task::spawn_blocking(move || InitLock::acquire(lock_path_clone))
        .await
        .context("Lock acquisition task panicked")??;

    // Double-check initialization status after acquiring lock
    let is_now_initialized = tokio::fs::try_exists(&config_path).await.unwrap_or(false)
        && tokio::fs::try_exists(&layouts_dir).await.unwrap_or(false)
        && tokio::fs::try_exists(&db_path).await.unwrap_or(false);

    if is_now_initialized {
        // Another process completed initialization while we waited
        // RAII guard handles cleanup on drop
        drop(init_lock);

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

    // Release lock explicitly (RAII guard also handles cleanup on drop)
    init_lock.release().context("Failed to release init lock")?;

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
