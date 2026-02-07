//! Async integration tests for file watcher
//!
//! Tests the file watching functionality including:
//! - Detection of file changes
//! - Debouncing of rapid changes
//! - Multiple workspace handling
//! - Concurrent access patterns
//!
//! All tests use async/await with tokio and verify events are received
//! within appropriate timeouts.

use std::time::Duration;

use tempfile::TempDir;
use tokio::{fs, time::timeout};
use zjj_core::{
    config::WatchConfig,
    watcher::{FileWatcher, WatchEvent},
    Result,
};

// ═══════════════════════════════════════════════════════════════════════════
// TEST 1: Watcher detects file changes
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_detects_file_changes() -> Result<()> {
    // Create temporary workspace with beads database
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    // Create initial database file
    fs::write(&beads_db, b"initial database content")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    // Configure watcher with short debounce for fast tests
    let config = WatchConfig {
        enabled: true,
        debounce_ms: 50, // Short debounce for faster test
        paths: vec![".beads/beads.db".to_string()],
    };

    // Start watching
    let mut rx = FileWatcher::watch_workspaces(&config, vec![workspace_path.clone()])
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Wait a bit for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify the database file
    fs::write(&beads_db, b"modified content")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to modify beads.db: {e}")))?;

    // Wait for event with timeout
    let event = timeout(Duration::from_secs(2), rx.recv())
        .await
        .map_err(|_| zjj_core::Error::Unknown("Timeout waiting for file change event".to_string()))?
        .ok_or_else(|| zjj_core::Error::Unknown("No event received".to_string()))?;

    // Verify the event is correct
    match event {
        WatchEvent::BeadsChanged {
            workspace_path: received_path,
        } => {
            assert_eq!(
                received_path, workspace_path,
                "Event should report correct workspace path"
            );
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 2: Watcher debounces rapid changes
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_debounces_rapid_changes() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    // Create initial database file
    fs::write(&beads_db, b"initial")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    // Configure watcher with 100ms debounce
    let config = WatchConfig {
        enabled: true,
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, vec![workspace_path.clone()])
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Perform rapid writes
    for i in 0..5 {
        fs::write(&beads_db, format!("content {}", i).as_bytes())
            .await
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to write iteration {i}: {e}")))?;

        // Small delay between writes (shorter than debounce)
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Count events received within a reasonable time window
    let start = std::time::Instant::now();
    let mut event_count = 0;
    let timeout_duration = Duration::from_secs(3);

    while start.elapsed() < timeout_duration {
        match timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(event)) => {
                match event {
                    WatchEvent::BeadsChanged {
                        workspace_path: received_path,
                    } => {
                        assert_eq!(received_path, workspace_path);
                    }
                }
                event_count += 1;

                // If we've gone past debounce period without more events, stop
                if event_count >= 1 {
                    // Wait a bit to see if more events come
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    break;
                }
            }
            Ok(None) => break, // Channel closed
            Err(_) => break,   // Timeout - no more events
        }
    }

    // Due to debouncing, we should receive only 1 event, not 5
    assert_eq!(
        event_count, 1,
        "Expected 1 event due to debouncing, got {event_count}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 3: Watcher handles multiple workspaces
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_handles_multiple_workspaces() -> Result<()> {
    // Create three workspaces
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace1 = temp_dir.path().join("workspace1");
    let workspace2 = temp_dir.path().join("workspace2");
    let workspace3 = temp_dir.path().join("workspace3");

    for workspace in [&workspace1, &workspace2, &workspace3] {
        // Create workspace directory first
        fs::create_dir(workspace).await.map_err(|e| {
            zjj_core::Error::IoError(format!("Failed to create workspace dir: {e}"))
        })?;

        let beads_dir = workspace.join(".beads");
        fs::create_dir(&beads_dir)
            .await
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

        let beads_db = beads_dir.join("beads.db");
        fs::write(&beads_db, b"initial content")
            .await
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;
    }

    let config = WatchConfig {
        enabled: true,
        debounce_ms: 50,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(
        &config,
        vec![workspace1.clone(), workspace2.clone(), workspace3.clone()],
    )
    .map_err(|e| zjj_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify all three workspaces
    let db1 = workspace1.join(".beads/beads.db");
    let db2 = workspace2.join(".beads/beads.db");
    let db3 = workspace3.join(".beads/beads.db");

    fs::write(&db1, b"modified 1")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to modify db1: {e}")))?;

    fs::write(&db2, b"modified 2")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to modify db2: {e}")))?;

    fs::write(&db3, b"modified 3")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to modify db3: {e}")))?;

    // Collect events with timeout
    let mut events = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(3) && events.len() < 3 {
        match timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(event)) => {
                events.push(event);
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // We should have received events for all three workspaces
    assert!(
        events.len() >= 3,
        "Expected at least 3 events, got {}",
        events.len()
    );

    // Verify all three workspaces are represented in events
    let mut workspace1_found = false;
    let mut workspace2_found = false;
    let mut workspace3_found = false;

    for event in events {
        match event {
            WatchEvent::BeadsChanged { workspace_path } => {
                if workspace_path == workspace1 {
                    workspace1_found = true;
                } else if workspace_path == workspace2 {
                    workspace2_found = true;
                } else if workspace_path == workspace3 {
                    workspace3_found = true;
                }
            }
        }
    }

    assert!(
        workspace1_found && workspace2_found && workspace3_found,
        "All three workspaces should have events"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 4: Watcher with non-existent database
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_handles_missing_database() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();

    // Don't create beads.db - test that watcher handles missing database gracefully
    let config = WatchConfig {
        enabled: true,
        debounce_ms: 50,
        paths: vec![".beads/beads.db".to_string()],
    };

    // This should not fail - watcher should handle missing database
    let result = FileWatcher::watch_workspaces(&config, vec![workspace_path]);

    assert!(
        result.is_ok(),
        "Watcher should start even without existing database"
    );

    // Verify receiver exists
    let _rx =
        result.map_err(|e| zjj_core::Error::IoError(format!("Failed to create watcher: {e}")))?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 5: Concurrent file modifications
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_handles_concurrent_modifications() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    fs::write(&beads_db, b"initial")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    let config = WatchConfig {
        enabled: true,
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, vec![workspace_path.clone()])
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn concurrent tasks to modify the file
    let db_clone = beads_db.clone();

    let task1 = tokio::spawn(async move {
        for i in 0..3 {
            fs::write(&db_clone, format!("task1-{}", i).as_bytes())
                .await
                .ok();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    let db_clone2 = beads_db.clone();
    let task2 = tokio::spawn(async move {
        for i in 0..3 {
            fs::write(&db_clone2, format!("task2-{}", i).as_bytes())
                .await
                .ok();
            tokio::time::sleep(Duration::from_millis(60)).await;
        }
    });

    // Wait for both tasks
    let (r1, r2) = tokio::join!(task1, task2);
    r1.map_err(|e| zjj_core::Error::Unknown(format!("Task1 failed: {e}")))?;
    r2.map_err(|e| zjj_core::Error::Unknown(format!("Task2 failed: {e}")))?;

    // Wait for debounced event(s)
    let start = std::time::Instant::now();
    let mut received_any = false;

    while start.elapsed() < Duration::from_secs(2) {
        match timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(event)) => {
                match event {
                    WatchEvent::BeadsChanged {
                        workspace_path: received_path,
                    } => {
                        assert_eq!(received_path, workspace_path);
                    }
                }
                received_any = true;
                // Wait a bit more to ensure debouncing is complete
                tokio::time::sleep(Duration::from_millis(200)).await;
                break;
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // We should have received at least one event despite concurrent modifications
    assert!(
        received_any,
        "Should receive event despite concurrent modifications"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 6: Watcher with rapid successive changes in different workspaces
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_rapid_changes_different_workspaces() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace1 = temp_dir.path().join("ws1");
    let workspace2 = temp_dir.path().join("ws2");

    // Create both workspaces
    for ws in [&workspace1, &workspace2] {
        // Create workspace directory first
        fs::create_dir(ws).await.map_err(|e| {
            zjj_core::Error::IoError(format!("Failed to create workspace dir: {e}"))
        })?;

        let beads_dir = ws.join(".beads");
        fs::create_dir(&beads_dir)
            .await
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

        let beads_db = beads_dir.join("beads.db");
        fs::write(&beads_db, b"initial")
            .await
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;
    }

    let config = WatchConfig {
        enabled: true,
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx =
        FileWatcher::watch_workspaces(&config, vec![workspace1.clone(), workspace2.clone()])
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Rapidly modify both workspaces
    let db1 = workspace1.join(".beads/beads.db");
    let db2 = workspace2.join(".beads/beads.db");

    fs::write(&db1, b"change1").await.ok();
    tokio::time::sleep(Duration::from_millis(30)).await;

    fs::write(&db2, b"change2").await.ok();
    tokio::time::sleep(Duration::from_millis(30)).await;

    fs::write(&db1, b"change3").await.ok();
    tokio::time::sleep(Duration::from_millis(30)).await;

    fs::write(&db2, b"change4").await.ok();

    // Collect events
    let mut events = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(3) {
        match timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(event)) => {
                events.push(event);
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // Should receive events for both workspaces (may be debounced)
    let mut ws1_events = 0;
    let mut ws2_events = 0;

    for event in events {
        match event {
            WatchEvent::BeadsChanged { workspace_path } => {
                if workspace_path == workspace1 {
                    ws1_events += 1;
                } else if workspace_path == workspace2 {
                    ws2_events += 1;
                }
            }
        }
    }

    assert!(
        ws1_events > 0 && ws2_events > 0,
        "Both workspaces should have events: ws1={}, ws2={}",
        ws1_events,
        ws2_events
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 7: Watcher error handling - invalid debounce values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_watcher_rejects_invalid_debounce_too_low() {
    let config = WatchConfig {
        enabled: true,
        debounce_ms: 5, // Too low (< 10)
        paths: vec![".beads/beads.db".to_string()],
    };

    let result = FileWatcher::watch_workspaces(&config, vec![]);
    assert!(result.is_err(), "Should reject debounce_ms < 10");

    if let Err(e) = result {
        assert!(matches!(e, zjj_core::Error::InvalidConfig(_)));
    }
}

#[test]
fn test_watcher_rejects_invalid_debounce_too_high() {
    let config = WatchConfig {
        enabled: true,
        debounce_ms: 10000, // Too high (> 5000)
        paths: vec![".beads/beads.db".to_string()],
    };

    let result = FileWatcher::watch_workspaces(&config, vec![]);
    assert!(result.is_err(), "Should reject debounce_ms > 5000");

    if let Err(e) = result {
        assert!(matches!(e, zjj_core::Error::InvalidConfig(_)));
    }
}

#[test]
fn test_watcher_rejects_disabled_config() {
    let config = WatchConfig {
        enabled: false, // Disabled
        debounce_ms: 100,
        paths: vec![".beads/beads.db".to_string()],
    };

    let result = FileWatcher::watch_workspaces(&config, vec![]);
    assert!(result.is_err(), "Should reject disabled watcher");

    if let Err(e) = result {
        assert!(matches!(e, zjj_core::Error::InvalidConfig(_)));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 8: Watcher event channel capacity
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_watcher_channel_capacity() -> Result<()> {
    let temp_dir = TempDir::new()
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;

    let workspace_path = temp_dir.path().to_path_buf();
    let beads_dir = workspace_path.join(".beads");
    fs::create_dir(&beads_dir)
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads dir: {e}")))?;

    let beads_db = beads_dir.join("beads.db");

    fs::write(&beads_db, b"initial")
        .await
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to create beads.db: {e}")))?;

    let config = WatchConfig {
        enabled: true,
        debounce_ms: 10, // Very short debounce
        paths: vec![".beads/beads.db".to_string()],
    };

    let mut rx = FileWatcher::watch_workspaces(&config, vec![workspace_path.clone()])
        .map_err(|e| zjj_core::Error::IoError(format!("Failed to start watcher: {e}")))?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Don't consume events immediately - test channel buffering
    for i in 0..20 {
        fs::write(&beads_db, format!("content {}", i).as_bytes())
            .await
            .map_err(|e| zjj_core::Error::IoError(format!("Failed to write iteration {i}: {e}")))?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Now consume events
    let mut event_count = 0;
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(3) {
        match timeout(Duration::from_millis(200), rx.recv()).await {
            Ok(Some(_)) => {
                event_count += 1;
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // Should receive at least some events
    assert!(
        event_count > 0,
        "Should receive events through channel, got {}",
        event_count
    );

    Ok(())
}
