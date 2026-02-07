//! Stress tests for recovery logging.
//!
//! These tests verify that recovery logging correctly handles:
//! - Concurrent logging from multiple threads/tasks
//! - Large log files (1000+ entries)
//! - Log file integrity under load
//! - Size constraints and readability

use std::path::PathBuf;
use std::sync::Mutex;

use futures::future::join_all;
use tokio::io::AsyncReadExt;
use tokio::time::{sleep, Duration};

use zjj_core::config::{RecoveryConfig, RecoveryPolicy};
use zjj_core::recovery::log_recovery;
use zjj_core::Error;

// Global mutex to serialize test execution (prevent directory conflicts)
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Maximum allowed log file size (1MB)
const MAX_LOG_SIZE_BYTES: u64 = 1024 * 1024;

/// Helper to create a temporary .zjj directory and return its path
async fn create_temp_zjj_dir() -> Result<PathBuf, Error> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    Ok(zjj_dir)
}

/// Helper to read all log entries from a recovery log file
async fn read_log_entries(zjj_dir: &PathBuf) -> Result<Vec<String>, Error> {
    let log_path = zjj_dir.join("recovery.log");

    if !tokio::fs::try_exists(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to check log: {e}")))?
    {
        return Ok(Vec::new());
    }

    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {e}")))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {e}")))?;

    // Parse log entries (format: [timestamp] message)
    let entries: Vec<String> = contents
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(entries)
}

/// Helper to get log file size in bytes
async fn get_log_size(zjj_dir: &PathBuf) -> Result<u64, Error> {
    let log_path = zjj_dir.join("recovery.log");

    if !tokio::fs::try_exists(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to check log: {e}")))?
    {
        return Ok(0);
    }

    let metadata = tokio::fs::metadata(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to get metadata: {e}")))?;
    Ok(metadata.len())
}

#[tokio::test]
async fn test_concurrent_recovery_logging() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test that 20 concurrent loggers all write successfully without data loss
    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;

    let parent = temp_dir.path().to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: true,
    };

    let num_loggers = 20;

    // Spawn 20 concurrent loggers
    let handles = (0..num_loggers).map(|i| {
        let config = config.clone();
        let parent = parent.clone();

        tokio::spawn(async move {
            let original = std::env::current_dir()
                .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
            std::env::set_current_dir(&parent)
                .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

            // Each logger writes 5 entries
            for j in 0..5 {
                let message = format!("Logger {} entry {}", i, j);
                log_recovery(&message, &config).await?;
                // Small delay to simulate real logging pattern
                sleep(Duration::from_millis(1)).await;
            }

            std::env::set_current_dir(original)
                .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

            Ok::<(), Error>(())
        })
    });

    // Wait for all loggers to complete
    let results = join_all(handles).await;

    // Verify all loggers succeeded
    for result in results {
        result.map_err(|e| Error::IoError(format!("Task join failed: {e}")))??;
    }

    // Read back the log file
    let entries = read_log_entries(&zjj_dir).await?;

    // Keep temp_dir alive until here
    let _ = temp_dir;

    // Verify all entries were written (20 loggers * 5 entries each = 100 total)
    assert_eq!(
        entries.len(),
        100,
        "Expected 100 log entries from 20 concurrent loggers"
    );

    // Verify all unique logger IDs are present
    let mut logger_ids = std::collections::HashSet::new();
    for entry in &entries {
        // Extract logger ID from entry format: "Logger {i} entry {j}"
        if let Some(start) = entry.find("Logger ") {
            let after_logger = &entry[start + 7..];
            if let Some(end) = after_logger.find(' ') {
                let id_str = &after_logger[..end];
                if let Ok(id) = id_str.parse::<usize>() {
                    logger_ids.insert(id);
                }
            }
        }
    }

    assert_eq!(
        logger_ids.len(),
        20,
        "All 20 logger IDs should be present in log"
    );

    // Verify log file size is reasonable (should be < 10KB for 100 entries)
    let log_size = get_log_size(&zjj_dir).await?;

    assert!(
        log_size < 10_000,
        "Log file size {} bytes is too large for 100 entries",
        log_size
    );

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())

#[tokio::test]
async fn test_large_recovery_log_handling() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test that 1000 log entries are handled correctly
    let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir).await.map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    let parent = zjj_dir.parent().ok_or_else(|| Error::IoError("Parent not found".to_string()))?.to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: true,
    };

    let num_entries = 1000;

    // Change to parent directory so .zjj is visible
    let original = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
    std::env::set_current_dir(&parent)
        .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

    // Write 1000 entries
    for i in 0..num_entries {
        let message = format!("Recovery action {}", i);
        log_recovery(&message, &config).await?;

        // Batch writes without delay for performance
        if i % 100 == 0 {
            // Small yield every 100 entries
            sleep(Duration::from_millis(1)).await;
        }
    }

    std::env::set_current_dir(original)
        .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

    // Read back the log file
    let entries = read_log_entries(&zjj_dir).await?;

    // Verify all 1000 entries were written
    assert_eq!(
        entries.len(),
        1000,
        "Expected 1000 log entries to be written"
    );

    // Verify log file is readable (all entries have timestamps)
    let all_have_timestamps = entries.iter().all(|entry| {
        // Format: [timestamp] message
        entry.starts_with('[') && entry.contains(']')
    });

    assert!(
        all_have_timestamps,
        "All log entries should have timestamps in [timestamp] format"
    );

    // Verify log file size is reasonable (< 1MB requirement)
    let log_size = get_log_size(&zjj_dir).await?;

    assert!(
        log_size < MAX_LOG_SIZE_BYTES,
        "Log file size {} bytes exceeds 1MB limit for 1000 entries",
        log_size
    );

    // Verify log file size is reasonable (should be < 200KB for 1000 entries)
    // Each entry is approximately: [2024-01-01T12:00:00Z] Recovery action 123\n
    // Which is about 55 bytes, so 1000 entries should be ~55KB
    assert!(
        log_size < 200_000,
        "Log file size {} bytes is unexpectedly large for 1000 entries",
        log_size
    );

    // Verify log entries are sequential (no gaps or corruption)
    let mut entry_numbers = Vec::new();
    for entry in &entries {
        // Extract entry number from "Recovery action {i}"
        if let Some(start) = entry.find("Recovery action ") {
            let num_str = &entry[start + 16..];
            if let Ok(num) = num_str.parse::<usize>() {
                entry_numbers.push(num);
            }
        }
    }

    assert_eq!(
        entry_numbers.len(),
        1000,
        "Should extract 1000 entry numbers"
    );

    // Verify sequential (0-999)
    entry_numbers.sort();
    for (i, &num) in entry_numbers.iter().enumerate() {
        assert_eq!(
            num, i,
            "Entry numbers should be sequential, expected {} got {}",
            i, num
        );
    }

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())
}

#[tokio::test]
async fn test_concurrent_logging_integrity() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test log integrity under heavy concurrent load
    let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir).await.map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    let parent = zjj_dir.parent().ok_or_else(|| Error::IoError("Parent not found".to_string()))?.to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: true,
    };

    // Spawn 50 concurrent writers, each writing 10 entries rapidly
    let handles = (0..50).map(|worker_id| {
        let config = config.clone();
        let parent = parent.clone();

        tokio::spawn(async move {
            let original = std::env::current_dir()
                .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
            std::env::set_current_dir(&parent)
                .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

            for entry_id in 0..10 {
                let message = format!("Worker {} entry {}", worker_id, entry_id);
                log_recovery(&message, &config).await?;
            }

            std::env::set_current_dir(original)
                .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

            Ok::<(), Error>(())
        })
    });

    // Wait for all writers
    let results = join_all(handles).await;
    for result in results {
        result.map_err(|e| Error::IoError(format!("Task join failed: {e}")))??;
    }

    // Verify log integrity
    let entries = read_log_entries(&zjj_dir).await?;

    // Should have exactly 500 entries (50 workers * 10 entries)
    assert_eq!(
        entries.len(),
        500,
        "Expected 500 entries from concurrent workers"
    );

    // Verify no corrupted entries (all should have proper format)
    let all_valid = entries.iter().all(|entry| {
        // Each entry should have: [timestamp] Worker X entry Y
        entry.starts_with('[') &&
            entry.contains(']') &&
            entry.contains("Worker ") &&
            entry.contains(" entry ")
    });

    assert!(all_valid, "All log entries should have valid format");

    // Count unique worker IDs
    let mut worker_ids = std::collections::HashSet::new();
    for entry in &entries {
        if let Some(start) = entry.find("Worker ") {
            let after_worker = &entry[start + 7..];
            if let Some(end) = after_worker.find(' ') {
                let id_str = &after_worker[..end];
                if let Ok(id) = id_str.parse::<usize>() {
                    worker_ids.insert(id);
                }
            }
        }
    }

    assert_eq!(
        worker_ids.len(),
        50,
        "All 50 worker IDs should be present"
    );

    // Verify log file is still under size limit
    let log_size = get_log_size(&zjj_dir).await?;

    assert!(
        log_size < MAX_LOG_SIZE_BYTES,
        "Log file {} bytes exceeds 1MB limit",
        log_size
    );

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())

#[tokio::test]
async fn test_recovery_log_disabled() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test that logging is properly disabled when configured
    let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir).await.map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    let parent = zjj_dir.parent().ok_or_else(|| Error::IoError("Parent not found".to_string()))?.to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: false, // Disabled
    };

    let original = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
    std::env::set_current_dir(&parent)
        .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

    // Try to write some entries
    for i in 0..10 {
        let message = format!("This should not be logged {}", i);
        log_recovery(&message, &config).await?;
    }

    std::env::set_current_dir(original)
        .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

    // Verify log file was not created (or is empty)
    let entries = read_log_entries(&zjj_dir).await?;

    assert_eq!(
        entries.len(),
        0,
        "No entries should be logged when logging is disabled"
    );

    // Verify log file doesn't exist
    let log_path = zjj_dir.join("recovery.log");
    let exists = tokio::fs::try_exists(&log_path).await?;
    assert!(
        !exists,
        "Log file should not exist when logging is disabled"
    );

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())

#[tokio::test]
async fn test_recovery_log_append_behavior() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test that log entries are appended (not overwritten)
    let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir).await.map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    let parent = zjj_dir.parent().ok_or_else(|| Error::IoError("Parent not found".to_string()))?.to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: true,
    };

    let original = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
    std::env::set_current_dir(&parent)
        .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

    // Write first batch
    for i in 0..10 {
        let message = format!("Batch 1 entry {}", i);
        log_recovery(&message, &config).await?;
    }

    std::env::set_current_dir(original)
        .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

    // Verify first batch
    let entries1 = read_log_entries(&zjj_dir).await?;
    assert_eq!(entries1.len(), 10, "First batch should have 10 entries");

    let original = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
    std::env::set_current_dir(&parent)
        .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

    // Write second batch
    for i in 0..10 {
        let message = format!("Batch 2 entry {}", i);
        log_recovery(&message, &config).await?;
    }

    std::env::set_current_dir(original)
        .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

    // Verify both batches are present
    let entries2 = read_log_entries(&zjj_dir).await?;

    assert_eq!(
        entries2.len(),
        20,
        "Log should contain both batches (20 total)"
    );

    // Verify first batch entries are still there
    let batch1_count = entries2
        .iter()
        .filter(|e| e.contains("Batch 1"))
        .count();
    assert_eq!(
        batch1_count, 10,
        "First batch entries should still be present"
    );

    // Verify second batch entries are appended
    let batch2_count = entries2
        .iter()
        .filter(|e| e.contains("Batch 2"))
        .count();
    assert_eq!(
        batch2_count, 10,
        "Second batch entries should be appended"
    );

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())

#[tokio::test]
async fn test_recovery_log_with_special_characters() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test logging messages with special characters, unicode, etc.
    let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir).await.map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    let parent = zjj_dir.parent().ok_or_else(|| Error::IoError("Parent not found".to_string()))?.to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: true,
    };

    let special_messages = vec![
        "Test with unicode: café, 日本語, 🚀",
        "Test with quotes: \"single\" and 'double'",
        "Test with newlines and\nline breaks",
        "Test with tabs\tand\tspaces",
        "Test with special chars: !@#$%^&*()[]{}|;:,.<>?",
        "Test with backslashes \\ and / forward slashes",
        "Test with emojis: ✅ ❌ ⚠️ 📝",
        "Test with long path: /very/long/path/to/some/file.txt",
    ];

    let original = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
    std::env::set_current_dir(&parent)
        .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

    for message in &special_messages {
        log_recovery(message, &config).await?;
    }

    std::env::set_current_dir(original)
        .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

    // Read back and verify all messages were logged
    let entries = read_log_entries(&zjj_dir).await?;

    assert_eq!(
        entries.len(),
        special_messages.len(),
        "All special character messages should be logged"
    );

    // Verify each special message is present
    for message in &special_messages {
        let found = entries.iter().any(|entry| entry.contains(message));
        assert!(
            found,
            "Special message '{}' should be in log",
            message
        );
    }

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())

#[tokio::test]
async fn test_recovery_log_empty_message_handling() -> Result<(), Error> {
    // Serialize test execution to prevent directory conflicts
    let _lock = TEST_MUTEX.lock().unwrap();

    // Test logging empty or whitespace-only messages
    let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir).await.map_err(|e| Error::IoError(format!("Failed to create .zjj: {e}")))?;
    let parent = zjj_dir.parent().ok_or_else(|| Error::IoError("Parent not found".to_string()))?.to_path_buf();

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Warn,
        log_recovered: true,
    };

    let original = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get cwd: {e}")))?;
    std::env::set_current_dir(&parent)
        .map_err(|e| Error::IoError(format!("Failed to change dir: {e}")))?;

    // These should still be logged (even if empty/whitespace)
    log_recovery("", &config).await?;
    log_recovery("   ", &config).await?;
    log_recovery("\t\n", &config).await?;
    log_recovery("Valid message", &config).await?;

    std::env::set_current_dir(original)
        .map_err(|e| Error::IoError(format!("Failed to restore dir: {e}")))?;

    // Verify all messages were logged
    let entries = read_log_entries(&zjj_dir).await?;

    assert_eq!(
        entries.len(),
        4,
        "All messages including empty ones should be logged"
    );

    // Verify valid message is present
    let valid_found = entries.iter().any(|e| e.contains("Valid message"));
    assert!(valid_found, "Valid message should be in log");

    // Keep temp_dir alive until here
    let _ = temp_dir;

    Ok(())
}
