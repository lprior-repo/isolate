//! Backup retention policy management

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use std::path::Path;

use super::{list::BackupInfo, BackupConfig};
use tokio::fs;

/// Apply retention policy to backups for a specific database
///
/// Keeps the most recent N backups as configured in BackupConfig.
/// Removes older backups beyond the retention count.
///
/// # Errors
///
/// Returns error if:
/// - Backup directory cannot be read
/// - Old backups cannot be removed
pub async fn apply_retention_policy(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<Vec<String>> {
    let backups = super::list::list_database_backups(root, database_name, config).await?;

    if backups.len() <= config.retention_count {
        return Ok(Vec::new());
    }

    // Remove backups beyond retention count (oldest ones)
    let backups_to_remove = &backups[config.retention_count..];

    let mut removed_paths = Vec::new();

    for backup in backups_to_remove {
        // Remove backup file
        fs::remove_file(&backup.path)
            .await
            .with_context(|| format!("Failed to remove backup: {}", backup.path.display()))?;

        // Remove metadata file if exists
        let metadata_path = backup.path.with_extension("json");
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to remove backup metadata: {}",
                        metadata_path.display()
                    )
                })?;
        }

        removed_paths.push(backup.path.display().to_string());
    }

    Ok(removed_paths)
}

/// Apply retention policy to all databases
///
/// # Errors
///
/// Returns error if any retention operation fails
pub async fn apply_retention_policy_all(
    root: &Path,
    config: &BackupConfig,
) -> Result<Vec<String>> {
    let databases = vec!["state.db", "queue.db", "beads.db"];

    let mut all_removed = Vec::new();

    for db_name in databases {
        match apply_retention_policy(root, db_name, config).await {
            Ok(removed) => all_removed.extend(removed),
            Err(e) => {
                tracing::warn!("Failed to apply retention policy to {}: {}", db_name, e);
                // Continue with other databases
            }
        }
    }

    Ok(all_removed)
}

/// Calculate total disk space used by backups
///
/// # Errors
///
/// Returns error if backup metadata cannot be accessed
pub async fn calculate_backup_size(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<u64> {
    let backups = super::list::list_database_backups(root, database_name, config).await?;

    let total_size = backups.iter().map(|b| b.size_bytes).sum();

    Ok(total_size)
}

/// Calculate total disk space that would be freed by applying retention policy
///
/// # Errors
///
/// Returns error if backup metadata cannot be accessed
pub async fn calculate_freed_space(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<u64> {
    let backups = super::list::list_database_backups(root, database_name, config).await?;

    if backups.len() <= config.retention_count {
        return Ok(0);
    }

    let backups_to_remove = &backups[config.retention_count..];
    let freed_space = backups_to_remove.iter().map(|b| b.size_bytes).sum();

    Ok(freed_space)
}

/// Get retention policy status for all databases
///
/// Returns a summary of current backup counts and space usage
///
/// # Errors
///
/// Returns error if backup information cannot be retrieved
pub async fn get_retention_status(
    root: &Path,
    config: &BackupConfig,
) -> Result<Vec<RetentionStatus>> {
    let databases = vec!["state.db", "queue.db", "beads.db"];

    let mut statuses = Vec::new();

    for db_name in databases {
        let backups = super::list::list_database_backups(root, db_name, config).await?;
        let total_size = calculate_backup_size(root, db_name, config).await?;
        let freed_space = calculate_freed_space(root, db_name, config).await?;

        statuses.push(RetentionStatus {
            database_name: db_name.to_string(),
            backup_count: backups.len(),
            retention_limit: config.retention_count,
            total_size_bytes: total_size,
            would_free_bytes: freed_space,
            within_limit: backups.len() <= config.retention_count,
        });
    }

    Ok(statuses)
}

/// Retention policy status for a database
#[derive(Debug, Clone)]
pub struct RetentionStatus {
    /// Database name
    pub database_name: String,
    /// Current number of backups
    pub backup_count: usize,
    /// Maximum backups to retain
    pub retention_limit: usize,
    /// Total disk space used by backups
    pub total_size_bytes: u64,
    /// Disk space that would be freed by applying retention
    pub would_free_bytes: u64,
    /// Whether backup count is within retention limit
    pub within_limit: bool,
}

impl RetentionStatus {
    /// Format size as human-readable string
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{bytes} B")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_apply_retention_policy_removes_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            retention_count: 2,
        };

        let backup_dir = root.join("backups").join("test.db");
        fs::create_dir_all(&backup_dir).await.unwrap();

        // Create 3 backups
        for i in 1..=3 {
            let backup_path = backup_dir.join(format!("backup-2025010{i}-120000.db"));
            fs::write(&backup_path, b"data").await.unwrap();
        }

        // Apply retention (keep 2, remove 1)
        let removed = apply_retention_policy(root, "test.db", &config)
            .await
            .unwrap();

        assert_eq!(removed.len(), 1);

        // Verify 2 backups remain
        let backups = super::list::list_database_backups(root, "test.db", &config)
            .await
            .unwrap();

        assert_eq!(backups.len(), 2);
    }

    #[tokio::test]
    async fn test_calculate_backup_size() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            ..Default::default()
        };

        let backup_dir = root.join("backups").join("test.db");
        fs::create_dir_all(&backup_dir).await.unwrap();

        // Create backups with known sizes
        let backup1 = backup_dir.join("backup-20250101-100000.db");
        let backup2 = backup_dir.join("backup-20250101-120000.db");

        fs::write(&backup1, vec![0u8; 1000]).await.unwrap();
        fs::write(&backup2, vec![0u8; 2000]).await.unwrap();

        let total_size = calculate_backup_size(root, "test.db", &config)
            .await
            .unwrap();

        assert_eq!(total_size, 3000);
    }

    #[tokio::test]
    async fn test_retention_status_within_limit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            retention_count: 5,
        };

        let backup_dir = root.join("backups").join("test.db");
        fs::create_dir_all(&backup_dir).await.unwrap();

        // Create 3 backups (under limit of 5)
        for i in 1..=3 {
            let backup_path = backup_dir.join(format!("backup-2025010{i}-120000.db"));
            fs::write(&backup_path, vec![0u8; 1000]).await.unwrap();
        }

        let statuses = get_retention_status(root, &config).await.unwrap();

        let test_status = statuses
            .iter()
            .find(|s| s.database_name == "test.db")
            .expect("test.db status not found");

        assert_eq!(test_status.backup_count, 3);
        assert!(test_status.within_limit);
        assert_eq!(test_status.would_free_bytes, 0);
    }
}
