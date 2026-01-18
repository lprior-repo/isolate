//! Database backup and restore commands

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{commands::get_session_db, database::SessionDb};

/// JSON output for backup command
#[derive(Debug, Serialize)]
pub struct BackupOutput {
    pub backup_path: PathBuf,
    pub session_count: usize,
    pub message: String,
}

/// JSON output for verify-backup command
#[derive(Debug, Serialize)]
pub struct VerifyBackupOutput {
    pub backup_path: PathBuf,
    pub valid: bool,
    pub session_count: usize,
    pub message: String,
}

/// Run the backup command
///
/// # Errors
///
/// Returns error if:
/// - Database cannot be accessed
/// - Backup file cannot be written
/// - Parent directory doesn't exist
pub async fn run_backup(backup_path: Option<&str>, json: bool) -> Result<()> {
    let db = get_session_db().await?;

    // Determine backup path
    let path = if let Some(p) = backup_path {
        PathBuf::from(p)
    } else {
        // Default: .zjj/backups/jjz-backup-<timestamp>.json
        let data_dir = crate::commands::zjj_data_dir()?;
        let backup_dir = data_dir.join("backups");

        // Create backups directory if it doesn't exist
        std::fs::create_dir_all(&backup_dir).context("Failed to create backups directory")?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("System time error")?
            .as_secs();

        backup_dir.join(format!("jjz-backup-{timestamp}.json"))
    };

    // Get session count before backup
    let sessions = db.list(None).await?;
    let count = sessions.len();

    // Perform backup
    db.backup(&path)
        .await
        .with_context(|| format!("Failed to create backup at '{}'", path.display()))?;

    if json {
        let output = BackupOutput {
            backup_path: path.clone(),
            session_count: count,
            message: format!("Successfully backed up {count} sessions"),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✓ Backup created successfully");
        println!("  Path: {}", path.display());
        println!("  Sessions: {count}");
    }

    Ok(())
}

/// Run the restore command
///
/// # Errors
///
/// Returns error if:
/// - Backup file doesn't exist or is invalid
/// - Database cannot be accessed
/// - Restore operation fails
pub async fn run_restore(backup_path: &str, force: bool, json: bool) -> Result<()> {
    let path = Path::new(backup_path);

    // Verify backup file exists
    anyhow::ensure!(path.exists(), "Backup file not found: {}", path.display());

    // Verify backup before restore
    let session_count = SessionDb::verify_backup(path).with_context(|| {
        format!(
            "Invalid backup file: {}\n\
             The backup file is corrupted or has an incompatible format.",
            path.display()
        )
    })?;

    // Warn user about destructive operation
    if !force {
        eprintln!("⚠️  WARNING: This will replace ALL existing session data!");
        eprintln!("  Backup contains {session_count} sessions");
        eprintln!();
        eprint!("Continue? [y/N] ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let confirmed =
            input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes");

        if !confirmed {
            eprintln!("Restore cancelled.");
            return Ok(());
        }
    }

    // Perform restore
    let db = get_session_db().await?;
    db.restore(path)
        .await
        .context("Failed to restore database from backup")?;

    if json {
        let output = serde_json::json!({
            "backup_path": path,
            "session_count": session_count,
            "message": format!("Successfully restored {session_count} sessions"),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✓ Database restored successfully");
        println!("  Restored {session_count} sessions from backup");
    }

    Ok(())
}

/// Run the verify-backup command
///
/// # Errors
///
/// Returns error if file cannot be read or parsing fails
pub async fn run_verify_backup(backup_path: &str, json: bool) -> Result<()> {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    let path = Path::new(backup_path);

    // Verify backup file
    let result = SessionDb::verify_backup(path);

    match result {
        Ok(count) => {
            if json {
                let output = VerifyBackupOutput {
                    backup_path: path.to_path_buf(),
                    valid: true,
                    session_count: count,
                    message: format!("Backup is valid ({count} sessions)"),
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("✓ Backup file is valid");
                println!("  Path: {}", path.display());
                println!("  Sessions: {count}");
            }
            Ok(())
        }
        Err(err) => {
            if json {
                let output = VerifyBackupOutput {
                    backup_path: path.to_path_buf(),
                    valid: false,
                    session_count: 0,
                    message: format!("Invalid backup: {err}"),
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("✗ Backup file is invalid");
                println!("  Path: {}", path.display());
                println!("  Error: {err}");
            }
            Err(err.into())
        }
    }
}
