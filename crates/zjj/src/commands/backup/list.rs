//! Backup listing functionality

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::backup_internal::{
    get_database_backup_dir, parse_backup_filename, BackupConfig, BackupMetadata,
};

/// Information about a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Path to backup file
    pub path: PathBuf,
    /// Timestamp from filename
    pub timestamp: DateTime<Utc>,
    /// Metadata (if available)
    pub metadata: Option<BackupMetadata>,
    /// Size in bytes
    pub size_bytes: u64,
}

impl BackupInfo {
    /// Create backup info from path and metadata
    pub fn new(
        path: PathBuf,
        timestamp: DateTime<Utc>,
        metadata: Option<BackupMetadata>,
        size_bytes: u64,
    ) -> Self {
        Self {
            path,
            timestamp,
            metadata,
            size_bytes,
        }
    }
}

/// List all backups for a specific database
///
/// # Errors
///
/// Returns error if:
/// - Backup directory cannot be read
/// - File metadata cannot be accessed
pub async fn list_database_backups(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<Vec<BackupInfo>> {
    let backup_dir = get_database_backup_dir(&config.backup_dir, database_name);

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&backup_dir)
        .await
        .context("Failed to read backup directory")?;

    let mut backups = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Only process .db files (not metadata .json files)
        if path.extension().and_then(|s| s.to_str()) != Some("db") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid backup filename"))?;

        // Parse timestamp from filename
        let timestamp = match parse_backup_filename(filename) {
            Ok(ts) => ts,
            Err(_) => continue, // Skip invalid filenames
        };

        // Get file size
        let size_bytes = fs::metadata(&path).await.map(|m| m.len()).unwrap_or(0);

        // Try to load metadata
        let metadata_path = path.with_extension("json");
        let metadata = if metadata_path.exists() {
            fs::read_to_string(&metadata_path)
                .await
                .ok()
                .and_then(|json| serde_json::from_str(&json).ok())
        } else {
            None
        };

        backups.push(BackupInfo::new(path, timestamp, metadata, size_bytes));
    }

    // Sort by timestamp (newest first)
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(backups)
}

/// List all backups across all databases
///
/// # Errors
///
/// Returns error if any backup directory cannot be read
pub async fn list_all_backups(
    root: &Path,
    config: &BackupConfig,
) -> Result<Vec<(String, Vec<BackupInfo>)>> {
    let databases = vec!["state.db", "queue.db", "beads.db"];

    let mut all_backups = Vec::new();

    for db_name in databases {
        let backups = list_database_backups(root, db_name, config).await?;
        if !backups.is_empty() {
            all_backups.push((db_name.to_string(), backups));
        }
    }

    Ok(all_backups)
}

use tokio::fs;

#[cfg(test)]
mod tests {
    use chrono::Timelike;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_list_backups_empty() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            ..Default::default()
        };

        let backups = list_database_backups(root, "test.db", &config)
            .await
            .unwrap();

        assert!(backups.is_empty());
    }

    #[tokio::test]
    async fn test_list_backups_sorted() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            ..Default::default()
        };

        let backup_dir = root.join("backups").join("test.db");
        fs::create_dir_all(&backup_dir).await.unwrap();

        // Create multiple backups with different timestamps
        let backup1 = backup_dir.join("backup-20250101-100000.db");
        let backup2 = backup_dir.join("backup-20250101-120000.db");
        let backup3 = backup_dir.join("backup-20250101-110000.db");

        fs::write(&backup1, b"data1").await.unwrap();
        fs::write(&backup2, b"data2").await.unwrap();
        fs::write(&backup3, b"data3").await.unwrap();

        let backups = list_database_backups(root, "test.db", &config)
            .await
            .unwrap();

        assert_eq!(backups.len(), 3);
        // Should be sorted newest first
        assert_eq!(backups[0].timestamp.hour(), 12);
        assert_eq!(backups[1].timestamp.hour(), 11);
        assert_eq!(backups[2].timestamp.hour(), 10);
    }
}
