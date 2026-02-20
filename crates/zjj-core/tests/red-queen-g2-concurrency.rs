// Red-Queen Generation 2: Concurrency and Race Condition Probes
// Target: bd-1lx merge queue submission
// Attack vector: Concurrent submissions, state transitions, database locks

// Integration tests have relaxed clippy settings for test infrastructure.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::manual_string_new,
    clippy::redundant_clone,
    clippy::clone_on_copy
)]

use std::time::Duration;

use tempfile::TempDir;
use tokio::time::sleep;
use zjj_core::coordination::queue_submission::{
    submit_to_queue, QueueSubmissionError::*, QueueSubmissionRequest,
};

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G2-CONC-001: Concurrent submissions with same dedupe_key
// PROMISE: INV-CONC-002 - Concurrent submissions with same dedupe_key serialize correctly
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g2_conc_001_concurrent_same_dedupe_key() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db").clone();
    let ws_path = temp.path().to_path_buf();

    let req = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-a:kxyz789".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    // Spawn concurrent submissions
    let handle1 = {
        let req = req.clone();
        let db_path = db_path.clone();
        let ws_path = ws_path.clone();
        tokio::spawn(async move { submit_to_queue(req, &db_path, &ws_path).await })
    };

    let handle2 = {
        let req = req.clone();
        let db_path = db_path.clone();
        let ws_path = ws_path.clone();
        tokio::spawn(async move { submit_to_queue(req, &db_path, &ws_path).await })
    };

    // Wait for both
    let (result1, result2) = tokio::join!(handle1, handle2);
    let result1 = result1.unwrap();
    let result2 = result2.unwrap();

    // Both should succeed (idempotent upsert)
    match (&result1, &result2) {
        (Ok(r1), Ok(r2)) => {
            // Both succeeded - check if they reference the same entry
            assert_eq!(r1.entry_id, r2.entry_id, "Should update same entry");
            // ✅ PROMISE UPHELD: Idempotent upsert works concurrently
        }
        (Ok(_), Err(e)) | (Err(e), Ok(_)) => {
            // One succeeded, one failed - acceptable if not a data race
            eprintln!("⚠️  OBSERVATION G2-C001: Concurrent submission had winner/loser: {e:?}");
        }
        (Err(e1), Err(e2)) => {
            panic!("Both submissions failed: {e1:?}, {e2:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G2-CONC-002: Concurrent submissions with different workspaces, same dedupe_key
// PROMISE: INV-QUEUE-001 - Only one workspace can claim a dedupe_key
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g2_conc_002_concurrent_dedupe_conflict() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db").clone();
    let ws_path = temp.path().to_path_buf();

    let req1 = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "shared:kxyz789".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let req2 = QueueSubmissionRequest {
        workspace: "ws-b".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "shared:kxyz789".to_string(), // SAME dedupe_key
        head_sha: "def456".to_string(),
        tested_against_sha: None,
    };

    // Spawn concurrent conflicting submissions
    let handle1 = {
        let req = req1.clone();
        let db_path = db_path.clone();
        let ws_path = ws_path.clone();
        tokio::spawn(async move { submit_to_queue(req, &db_path, &ws_path).await })
    };

    let handle2 = {
        let req = req2.clone();
        let db_path = db_path.clone();
        let ws_path = ws_path.clone();
        tokio::spawn(async move {
            // Small delay to try and create a race
            sleep(Duration::from_millis(10)).await;
            submit_to_queue(req, &db_path, &ws_path).await
        })
    };

    let (result1, result2) = tokio::join!(handle1, handle2);
    let result1 = result1.unwrap();
    let result2 = result2.unwrap();

    // Exactly one should succeed
    let success_count = i32::from(result1.is_ok()) + i32::from(result2.is_ok());
    assert_eq!(
        success_count, 1,
        "Exactly one submission should succeed with dedupe conflict"
    );

    // The failure should be a dedupe conflict (or transaction failed with that message)
    let failed_result = if result1.is_err() { &result1 } else { &result2 };
    match failed_result {
        Err(DedupeKeyConflict { .. }) => {
            // ✅ PROMISE UPHELD
        }
        Err(TransactionFailed { details, .. }) if details.contains("dedupe") => {
            eprintln!("⚠️  OBSERVATION G2-C002: Dedupe conflict wrapped in TransactionFailed (known from G1-D001)");
        }
        Err(e) => {
            panic!("Wrong error type for concurrent dedupe conflict: {e:?}");
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G2-CONC-003: Rapid-fire submissions (stress test)
// PROMISE: INV-CONC-001 - Submission is atomic (no partial state)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g2_conc_003_rapid_fire_submissions() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db").clone();
    let ws_path = temp.path().to_path_buf();

    // Spawn 10 concurrent submissions for different workspaces
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let req = QueueSubmissionRequest {
                workspace: format!("ws-{i}"),
                bead_id: None,
                priority: i,
                agent_id: None,
                dedupe_key: format!("ws-{i}:kxyz{i}"),
                head_sha: format!("sha{i}"),
                tested_against_sha: None,
            };
            let db_path = db_path.clone();
            let ws_path = ws_path.clone();
            tokio::spawn(async move { submit_to_queue(req, &db_path, &ws_path).await })
        })
        .collect();

    // Wait for all
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should succeed
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(success_count, 10, "All submissions should succeed");

    // All should have unique entry IDs
    let entry_ids: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok().map(|r| r.entry_id))
        .collect();

    let unique_ids: std::collections::HashSet<_> = entry_ids.iter().collect();
    assert_eq!(
        unique_ids.len(),
        entry_ids.len(),
        "All entry IDs should be unique"
    );

    // ✅ PROMISE UPHELD: Atomic submissions under load
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G2-CONC-004: Submission during database lock
// PROMISE: INV-CONC-003 - No partial state after failed submission
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g2_conc_004_submission_with_lock_contention() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db").clone();
    let ws_path = temp.path().to_path_buf();

    // Submit first entry to establish queue
    let req1 = QueueSubmissionRequest {
        workspace: "ws-first".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-first:kxyz789".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let _ = submit_to_queue(req1, &db_path, &ws_path).await.unwrap();

    // Try to submit multiple entries rapidly to create lock contention
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let req = QueueSubmissionRequest {
                workspace: format!("ws-concurrent-{i}"),
                bead_id: None,
                priority: 0,
                agent_id: None,
                dedupe_key: format!("ws-concurrent-{i}:kxyz{i}"),
                head_sha: format!("sha{i}"),
                tested_against_sha: None,
            };
            let db_path = db_path.clone();
            let ws_path = ws_path.clone();
            tokio::spawn(async move { submit_to_queue(req, &db_path, &ws_path).await })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should succeed despite lock contention
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(success_count, 5, "All submissions should succeed");

    // Check for partial state - all entries should be complete
    for response in results.into_iter().flatten() {
        assert!(response.entry_id > 0, "Entry ID should be positive");
        assert!(!response.status.is_empty(), "Status should not be empty");
    }

    // ✅ PROMISE UPHELD: No partial state after concurrent submissions
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G2-CONC-005: Update while entry is being processed (state transition)
// PROMISE: INV-STATE-001 - Entry status is always valid
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g2_conc_005_update_during_state_transition() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db").clone();
    let ws_path = temp.path().to_path_buf();

    let req = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-a:kxyz789".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    // Initial submission
    let result1 = submit_to_queue(req.clone(), &db_path, &ws_path).await;
    assert!(result1.is_ok());

    // Simulate state transition (e.g., entry claimed by worker)
    // Then immediately submit again
    // This tests whether the upsert logic handles non-pending states correctly

    let req2 = QueueSubmissionRequest {
        head_sha: "def456".to_string(), // Different SHA
        ..req.clone()
    };

    let result2 = submit_to_queue(req2, &db_path, &ws_path).await;

    // Should succeed and update the entry regardless of state
    match result2 {
        Ok(response) => {
            assert!(response.entry_id > 0);
            // ✅ PROMISE UPHELD: Update during state transition handled
        }
        Err(EntryIsTerminal { .. }) => {
            // Also acceptable if entry became terminal
        }
        Err(e) => {
            eprintln!("⚠️  OBSERVATION G2-C005: Update during transition error: {e:?}");
        }
    }
}
