//! Initialize ZJJ - sets up everything needed

#![allow(clippy::too_many_lines)]
#![allow(clippy::suspicious_open_options)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::db::SessionDb;

mod deps;
mod setup;
mod types;

// ============================================================================
// InitLock - RAII guard for init lock file
// ============================================================================

/// Lock file timeout - after this duration, a stale lock is considered abandoned
const STALE_LOCK_TIMEOUT_SECS: u64 = 60;

/// RAII guard that ensures lock file cleanup on drop (including error paths)
struct InitLock {
    file: std::fs::File,
    path: PathBuf,
    released: bool,
}

impl InitLock {
    /// Acquire exclusive lock, handling stale locks and symlink attacks
    fn acquire(lock_path: PathBuf) -> Result<Self> {
        // ADVERSARIAL FIX: Use O_NOFOLLOW on Unix to prevent symlink race (TOCTOU)
        // This ensures the open call fails if the path is a symlink.
        let mut options = std::fs::OpenOptions::new();
        options.create(true).write(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }

        // Open lock file
        let file = options.open(&lock_path).with_context(|| {
            let mut msg = format!("Failed to create lock file at {}", lock_path.display());
            #[cfg(unix)]
            {
                if let Ok(meta) = std::fs::symlink_metadata(&lock_path) {
                    if meta.is_symlink() {
                        msg = format!(
                            "Security: {} is a symlink. This is not allowed. Remove it and retry.",
                            lock_path.display()
                        );
                    }
                }
            }
            msg
        })?;

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
            self.file.unlock().context("Failed to release lock")?;
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
#[allow(dead_code)] // Convenience wrapper, currently unused but part of public API
pub async fn run() -> Result<()> {
    run_with_options(InitOptions::default()).await
}

/// Run init command with options
pub async fn run_with_options(options: InitOptions) -> Result<()> {
    run_with_cwd_and_options(None, options).await
}

/// Run init command with cwd and options
#[allow(clippy::used_underscore_binding)]
pub async fn run_with_cwd_and_options(cwd: Option<&Path>, options: InitOptions) -> Result<()> {
    let cwd = match cwd {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    // Check required dependencies
    check_dependencies().await?;

    if options.dry_run {
        println!("Would initialize ZJJ in {}", cwd.display());
        println!("Would create .zjj directory structure:");
        println!("  - .zjj/");
        println!("  - .zjj/config.toml");
        println!("  - .zjj/state.db");
        println!("  - .zjj/layouts/");
        println!("Would check/initialize JJ repository");
        println!("Would create hook integration");
        return Ok(());
    }

    // Initialize JJ repo if needed
    ensure_jj_repo_with_cwd(&cwd, options.format.is_json()).await?;

    // Get the repo root using the provided cwd
    let root = jj_root_with_cwd(&cwd).await?;
    let zjj_dir = root.join(".zjj");

    // Create .zjj directory if missing before acquiring lock
    if !tokio::fs::try_exists(&zjj_dir).await.unwrap_or(false) {
        tokio::fs::create_dir_all(&zjj_dir)
            .await
            .context("Failed to create .zjj directory")?;
    }

    // Acquire init lock to prevent concurrent initialization
    let lock_path = zjj_dir.join(".init.lock");
    let _lock = InitLock::acquire(lock_path)?;

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

    // Release the lock before final output
    _lock.release()?;

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

#[cfg(test)]
mod tests;
