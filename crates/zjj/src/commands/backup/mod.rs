//! Database backup command handler

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod backup_internal;

pub mod create;
pub mod list;
pub mod restore;
pub mod retention;

// Re-export backup types
pub use backup_internal::{BackupConfig, BackupMetadata};

use anyhow::{Context, Result};
use zjj_core::OutputFormat;

use crate::cli::jj_root;

/// Create backup
pub async fn run_create(format: OutputFormat) -> Result<()> {
    let root = jj_root().await.context("Failed to get JJ root")?;
    let root_path = std::path::PathBuf::from(&root);

    let config = BackupConfig::default();

    let backup_paths = create::backup_all_databases(&root_path, &config).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": true,
                "backups": backup_paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>(),
                "message": format!("Created {} backup(s)", backup_paths.len())
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            if backup_paths.is_empty() {
                println!("No databases found to backup");
            } else {
                println!("Created {} backup(s):", backup_paths.len());
                for path in &backup_paths {
                    println!("  - {}", path.display());
                }
            }
        }
    }

    Ok(())
}

/// List backups
pub async fn run_list(format: OutputFormat) -> Result<()> {
    let root = jj_root().await.context("Failed to get JJ root")?;
    let root_path = std::path::PathBuf::from(&root);

    let config = BackupConfig::default();
    let all_backups = list::list_all_backups(&root_path, &config).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": true,
                "databases": all_backups
                    .iter()
                    .map(|(name, backups)| {
                        serde_json::json!({
                            "database": name,
                            "backup_count": backups.len(),
                            "backups": backups.iter().map(|b| serde_json::json!({
                                "path": b.path.display().to_string(),
                                "timestamp": b.timestamp.to_rfc3339(),
                                "size_bytes": b.size_bytes
                            })).collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            if all_backups.is_empty() {
                println!("No backups found");
            } else {
                for (db_name, backups) in &all_backups {
                    println!("Database: {} ({} backup(s))", db_name, backups.len());
                    for backup in backups {
                        println!(
                            "  - {} ({}, {} bytes)",
                            backup.path.display(),
                            backup.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            backup.size_bytes
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Restore from backup
pub async fn run_restore(
    database: &str,
    timestamp: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let root = jj_root().await.context("Failed to get JJ root")?;
    let root_path = std::path::PathBuf::from(&root);

    let config = BackupConfig::default();

    // If timestamp provided, find specific backup
    // Otherwise use latest
    let backup_path = if let Some(ts) = timestamp {
        // Find backup with matching timestamp
        let backups = list::list_database_backups(&root_path, database, &config).await?;
        backups
            .iter()
            .find(|b| b.timestamp.format("%Y%m%d-%H%M%S").to_string() == ts)
            .map(|b| b.path.clone())
            .ok_or_else(|| anyhow::anyhow!("No backup found with timestamp: {ts}"))?
    } else {
        restore::find_latest_backup(&root_path, database, &config).await?
    };

    // Determine target path
    let target_path = match database {
        "state.db" | "queue.db" => root_path.join(".zjj").join(database),
        "beads.db" => root_path.join(".beads").join(database),
        _ => anyhow::bail!("Unknown database: {database}"),
    };

    // Verify checksum by default
    restore::restore_backup(&backup_path, &target_path, true).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": true,
                "database": database,
                "restored_from": backup_path.display().to_string(),
                "restored_to": target_path.display().to_string()
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            println!("Restored {} from: {}", database, backup_path.display());
            println!("To: {}", target_path.display());
        }
    }

    Ok(())
}

/// Apply retention policy
pub async fn run_retention(format: OutputFormat) -> Result<()> {
    let root = jj_root().await.context("Failed to get JJ root")?;
    let root_path = std::path::PathBuf::from(&root);

    let config = BackupConfig::default();
    let removed = retention::apply_retention_policy_all(&root_path, &config).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": true,
                "removed_count": removed.len(),
                "removed_backups": removed
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            if removed.is_empty() {
                println!("No backups to remove (all within retention limit)");
            } else {
                println!("Removed {} old backup(s):", removed.len());
                for path in &removed {
                    println!("  - {}", path);
                }
            }
        }
    }

    Ok(())
}

/// Show backup status
pub async fn run_status(format: OutputFormat) -> Result<()> {
    let root = jj_root().await.context("Failed to get JJ root")?;
    let root_path = std::path::PathBuf::from(&root);

    let config = BackupConfig::default();
    let statuses = retention::get_retention_status(&root_path, &config).await?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": true,
                "retention_policy": {
                    "max_backups_per_database": config.retention_count
                },
                "databases": statuses
                    .iter()
                    .map(|s| serde_json::json!({
                        "database": s.database_name,
                        "backup_count": s.backup_count,
                        "retention_limit": s.retention_limit,
                        "total_size_bytes": s.total_size_bytes,
                        "would_free_bytes": s.would_free_bytes,
                        "within_limit": s.within_limit,
                        "total_size_human": retention::RetentionStatus::format_size(s.total_size_bytes),
                        "would_free_human": retention::RetentionStatus::format_size(s.would_free_bytes)
                    }))
                    .collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            println!(
                "Backup Status (Retaining last {} backups per database):",
                config.retention_count
            );
            println!();

            for status in &statuses {
                println!("Database: {}", status.database_name);
                println!(
                    "  Backups: {} / {}",
                    status.backup_count, status.retention_limit
                );
                println!(
                    "  Total size: {}",
                    retention::RetentionStatus::format_size(status.total_size_bytes)
                );

                if !status.within_limit {
                    println!(
                        "  Would free: {}",
                        retention::RetentionStatus::format_size(status.would_free_bytes)
                    );
                    println!("  ⚠️  Over retention limit");
                }
                println!();
            }
        }
    }

    Ok(())
}
