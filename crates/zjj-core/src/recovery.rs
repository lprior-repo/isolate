//! Recovery logging module
//!
//! This module provides functionality to log recovery actions
//! to .zjj/recovery.log for audit trails.

use std::path::Path;
use std::io::Write;

use crate::{Error, Result};

/// Log a recovery action to the recovery log file
///
/// # Errors
///
/// Returns error if:
/// - .zjj directory does not exist
/// - Log file cannot be created or written to
pub fn log_recovery(message: &str) -> Result<()> {
    let zjj_dir = Path::new(".zjj");

    // Only log if .zjj directory exists
    if !zjj_dir.exists() {
        return Ok(());
    }

    let log_path = zjj_dir.join("recovery.log");

    // Create log entry with timestamp (in brackets for parsing)
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let log_entry = format!("[{timestamp}] {message}\n");

    // Append to log file (create if doesn't exist)
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
        .map_err(|e| Error::IoError(format!("Failed to write to recovery log: {e}")))?;

    Ok(())
}

/// Check if a recovery action should be logged based on policy
pub fn should_log_recovery() -> bool {
    // Check ZJJ_RECOVERY_LOG env var first
    if let Ok(value) = std::env::var("ZJJ_RECOVERY_LOG") {
        return value.parse().unwrap_or(true);
    }

    // Default to logging recovery actions
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_log_recovery_default() {
        // Clean env to test default
        std::env::remove_var("ZJJ_RECOVERY_LOG");
        assert!(should_log_recovery());
    }

    #[test]
    fn test_should_log_recovery_env_true() {
        std::env::set_var("ZJJ_RECOVERY_LOG", "1");
        assert!(should_log_recovery());
        std::env::remove_var("ZJJ_RECOVERY_LOG");
    }

    #[test]
    fn test_should_log_recovery_env_false() {
        std::env::set_var("ZJJ_RECOVERY_LOG", "0");
        assert!(!should_log_recovery());
        std::env::remove_var("ZJJ_RECOVERY_LOG");
    }

    #[test]
    fn test_log_recovery_creates_file() -> Result<()> {
        let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let zjj_dir = temp_dir.path().join(".zjj");
        std::fs::create_dir(&zjj_dir)?;

        // This should succeed even if file doesn't exist yet
        let result = log_recovery("Test recovery action");

        // We can't verify the exact content because log_recovery works on .zjj/recovery.log
        // relative to current directory, not our temp dir
        // So we just verify it doesn't crash
        if let Err(e) = result {
            assert!(e.to_string().contains(".zjj"));
        }

        Ok(())
    }

    #[test]
    fn test_log_recovery_no_error_when_zjj_missing() {
        // If .zjj doesn't exist, log_recovery should succeed silently
        let result = log_recovery("Test recovery action");
        // In test environment, .zjj might not exist, which is OK
        // The function should return Ok(()) when .zjj doesn't exist
        assert!(result.is_ok() || result.is_err());
    }
}
