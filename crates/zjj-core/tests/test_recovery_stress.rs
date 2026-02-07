// Recovery Logging Stress Tests
//
// Comprehensive stress tests for recovery logging functionality under
// high concurrency and large data volumes.

use std::sync::Mutex;

use futures::future::join_all;
use tokio::{
    io::AsyncReadExt,
    time::{sleep, Duration},
};
use zjj_core::{
    config::{RecoveryConfig, RecoveryPolicy},
    recovery::log_recovery,
    Error,
};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Test concurrent recovery logging with multiple writers
#[tokio::test]
async fn test_concurrent_recovery_logging() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true,
    };

    // Change to temp directory so log_recovery creates the file there
    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    // Spawn 20 concurrent loggers
    let handles: Vec<_> = (0..20)
        .map(|i| {
            tokio::spawn(async move {
                let result = log_recovery(&format!("Concurrent message {}", i), &config).await;
                (i, result)
            })
        })
        .collect();

    // Wait for all to complete
    let results = join_all(handles).await;

    // Restore original directory
    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Verify all succeeded
    for result in results {
        let (i, result) = result.map_err(|e| Error::IoError(format!("Task join failed: {}", e)))?;
        result.map_err(|e| Error::IoError(format!("Task {} failed: {}", i, e)))?;
    }

    // Verify log file contains all messages
    let log_path = zjj_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {}", e)))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {}", e)))?;

    for i in 0..20 {
        assert!(
            content.contains(&format!("Concurrent message {}", i)),
            "Missing message {} in log",
            i
        );
    }

    Ok(())
}

/// Test handling of large recovery logs
#[tokio::test]
async fn test_large_recovery_log_handling() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true,
    };

    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    // Log 1000 entries
    for i in 0..1000 {
        let message = format!(
            "Entry {}: Some relatively long message to increase log size",
            i
        );
        log_recovery(&message, &config).await?;
    }

    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Verify log file exists and is reasonable size (< 1MB for 1000 entries)
    let log_path = zjj_dir.join("recovery.log");
    let metadata = tokio::fs::metadata(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to get metadata: {}", e)))?;
    let file_size = metadata.len();

    assert!(
        file_size < 1_000_000,
        "Log file too large: {} bytes (expected < 1MB for 1000 entries)",
        file_size
    );

    assert!(file_size > 0, "Log file is empty");

    // Verify we can read the entire file
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {}", e)))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {}", e)))?;

    // Spot check some entries
    assert!(content.contains("Entry 0:"));
    assert!(content.contains("Entry 500:"));
    assert!(content.contains("Entry 999:"));

    Ok(())
}

/// Test concurrent logging with integrity checks
#[tokio::test]
async fn test_concurrent_logging_integrity() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true,
    };

    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    const NUM_WORKERS: usize = 50;
    const ENTRIES_PER_WORKER: usize = 10;

    // Spawn 50 workers, each logging 10 entries
    let handles: Vec<_> = (0..NUM_WORKERS)
        .map(|worker_id| {
            tokio::spawn(async move {
                for entry_id in 0..ENTRIES_PER_WORKER {
                    let message = format!("Worker {} entry {}", worker_id, entry_id);
                    if let Err(e) = log_recovery(&message, &config).await {
                        return Err(e);
                    }
                }
                Ok::<(), Error>(())
            })
        })
        .collect();

    // Wait for all workers
    let results = join_all(handles).await;
    for result in results {
        let result = result.map_err(|e| Error::IoError(format!("Worker join failed: {}", e)))?;
        result.map_err(|e| Error::IoError(format!("Worker failed: {}", e)))?;
    }

    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Verify integrity: all messages present
    let log_path = zjj_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {}", e)))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {}", e)))?;

    let mut found = vec![0; NUM_WORKERS];
    for worker_id in 0..NUM_WORKERS {
        for entry_id in 0..ENTRIES_PER_WORKER {
            let message = format!("Worker {} entry {}", worker_id, entry_id);
            if content.contains(&message) {
                found[worker_id] += 1;
            }
        }
    }

    // Each worker should have all entries present
    for (worker_id, count) in found.iter().enumerate() {
        assert_eq!(
            *count, ENTRIES_PER_WORKER,
            "Worker {}: expected {} entries, found {}",
            worker_id, ENTRIES_PER_WORKER, count
        );
    }

    Ok(())
}

/// Test recovery logging when disabled
#[tokio::test]
async fn test_recovery_log_disabled() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: false, // Disabled
    };

    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    // Logging should succeed even if disabled
    log_recovery("This should not be logged", &config).await?;

    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Wait a bit to ensure any async operations complete
    sleep(Duration::from_millis(100)).await;

    // Verify log file was NOT created
    let log_path = zjj_dir.join("recovery.log");
    let log_exists = tokio::fs::try_exists(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to check existence: {}", e)))?;
    assert!(
        !log_exists,
        "Log file should not exist when recovery logging is disabled"
    );

    Ok(())
}

/// Test that recovery logging appends rather than overwrites
#[tokio::test]
async fn test_recovery_log_append_behavior() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true,
    };

    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    // Log first message
    log_recovery("First message", &config).await?;

    // Give filesystem time to flush
    sleep(Duration::from_millis(50)).await;

    // Log second message
    log_recovery("Second message", &config).await?;

    // Give filesystem time to flush
    sleep(Duration::from_millis(50)).await;

    // Log third message
    log_recovery("Third message", &config).await?;

    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Verify all messages are present
    let log_path = zjj_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {}", e)))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {}", e)))?;

    assert!(content.contains("First message"), "Missing first message");
    assert!(content.contains("Second message"), "Missing second message");
    assert!(content.contains("Third message"), "Missing third message");

    // Verify messages appear in order (first before second before third)
    let first_pos = content.find("First message")
        .expect("First message should be in log");
    let second_pos = content.find("Second message")
        .expect("Second message should be in log");
    let third_pos = content.find("Third message")
        .expect("Third message should be in log");

    assert!(
        first_pos < second_pos && second_pos < third_pos,
        "Messages not in correct order"
    );

    Ok(())
}

/// Test recovery logging with special characters and unicode
#[tokio::test]
async fn test_recovery_log_with_special_characters() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true,
    };

    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    // Test various special characters
    let test_messages = vec![
        "Unicode: 你好世界 🎉",
        "Special chars: \\n \\t \\r \\\\ '",
        "Quotes: double and single",
        "Path separators: /home/user/test\\path",
        "Emoji: 🚀 🔥 💻 🦀",
        "Math: ∑ ∫ ∞ √ ≠ ≤ ≥",
        "Currency: $ £ € ¥ ₹",
        "Arrows: ← → ↑ ↓ ↔",
        "Box drawing: ┌─┐ │ └─┘",
        "Mixed: Test with emoji 🦀 and unicode 你好 and $pecials!",
    ];

    for message in test_messages {
        log_recovery(message, &config).await?;
        sleep(Duration::from_millis(10)).await;
    }

    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Verify all messages are preserved correctly
    let log_path = zjj_dir.join("recovery.log");
    let mut content = String::new();
    let mut file = tokio::fs::File::open(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to open log: {}", e)))?;
    file.read_to_string(&mut content)
        .await
        .map_err(|e| Error::IoError(format!("Failed to read log: {}", e)))?;

    assert!(content.contains("你好世界"));
    assert!(content.contains("🎉"));
    assert!(content.contains("\\n"));
    assert!(content.contains("double"));
    assert!(content.contains("🚀"));
    assert!(content.contains("∑ ∫"));
    assert!(content.contains("£ € ¥"));
    assert!(content.contains("← →"));
    assert!(content.contains("┌─┐"));
    assert!(content.contains("🦀"));

    Ok(())
}

/// Test recovery logging with empty and whitespace messages
#[tokio::test]
async fn test_recovery_log_empty_message_handling() -> Result<(), Error> {
    let _guard = TEST_MUTEX.lock()
        .expect("TEST_MUTEX lock should be available");

    let temp_dir = tempfile::tempdir()
        .map_err(|e| Error::IoError(format!("Failed to create temp dir: {}", e)))?;
    let zjj_dir = temp_dir.path().join(".zjj");
    tokio::fs::create_dir(&zjj_dir)
        .await
        .map_err(|e| Error::IoError(format!("Failed to create .zjj: {}", e)))?;

    let config = RecoveryConfig {
        policy: RecoveryPolicy::Silent,
        log_recovered: true,
    };

    let original_dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current dir: {}", e)))?;
    std::env::set_current_dir(temp_dir.path())
        .map_err(|e| Error::IoError(format!("Failed to set current dir: {}", e)))?;

    // Test edge cases
    let test_cases = vec!["", " ", "  ", "\t", "\n", "\r\n", "   \t\t   "];

    for message in test_cases {
        let result = log_recovery(message, &config).await;

        // Empty/whitespace messages should either succeed or fail gracefully
        // We don't prescribe the exact behavior, just ensure no panics
        match result {
            Ok(()) => {}
            Err(Error::ValidationError(_)) => {
                // Acceptable: empty messages rejected
            }
            Err(e) => {
                return Err(e);
            }
        }

        sleep(Duration::from_millis(10)).await;
    }

    std::env::set_current_dir(&original_dir)
        .map_err(|e| Error::IoError(format!("Failed to restore current dir: {}", e)))?;

    // Verify log file is readable (even if empty)
    let log_path = zjj_dir.join("recovery.log");
    let log_exists = tokio::fs::try_exists(&log_path)
        .await
        .map_err(|e| Error::IoError(format!("Failed to check existence: {}", e)))?;
    if log_exists {
        let mut content = String::new();
        let mut file = tokio::fs::File::open(&log_path)
            .await
            .map_err(|e| Error::IoError(format!("Failed to open log: {}", e)))?;
        file.read_to_string(&mut content)
            .await
            .map_err(|e| Error::IoError(format!("Failed to read log: {}", e)))?;

        // File should be readable without errors
        assert!(content.len() < 10_000_000, "Log file unexpectedly large");
    }

    Ok(())
}
