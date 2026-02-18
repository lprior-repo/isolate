// Red-Queen Generation 1: Deduplication Edge Case Probes
// Target: bd-1lx merge queue submission
// Attack vector: Deduplication key enforcement

use std::path::PathBuf;

use tempfile::TempDir;
use zjj_core::coordination::queue_submission::{
    submit_to_queue,
    QueueSubmissionError::{self, *},
    QueueSubmissionRequest,
};

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G1-DEDUP-001: Same dedupe_key, different workspaces (sequential)
// PROMISE: INV-QUEUE-001 - No two active entries share same dedupe_key
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g1_dedup_001_same_dedupe_different_workspaces() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db");
    let ws_path = temp.path();

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

    // Submit first - should succeed
    let result1 = submit_to_queue(req1, &db_path, ws_path).await;
    assert!(result1.is_ok(), "First submission should succeed");

    // Submit second with same dedupe_key - should fail
    let result2 = submit_to_queue(req2, &db_path, ws_path).await;
    match result2 {
        Err(DedupeKeyConflict {
            dedupe_key,
            existing_workspace,
            provided_workspace,
        }) => {
            assert_eq!(dedupe_key, "shared:kxyz789");
            assert_eq!(existing_workspace, "ws-a");
            assert_eq!(provided_workspace, "ws-b");
            // ✅ PROMISE UPHELD
        }
        Ok(_) => {
            panic!("❌ SURVIVOR G1-D001: Second submission with same dedupe_key should have failed with DedupeKeyConflict");
        }
        Err(e) => {
            panic!("Wrong error type: {e:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G1-DEDUP-002: Empty dedupe_key
// PROMISE: PRE-Q-001 - Dedupe_key must be valid
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g1_dedup_002_empty_dedupe_key() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db");
    let ws_path = temp.path();

    let req = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "".to_string(), // EMPTY
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let result = submit_to_queue(req, &db_path, ws_path).await;
    match result {
        Err(InvalidDedupeKey { dedupe_key, reason }) => {
            assert_eq!(dedupe_key, "");
            // ✅ PROMISE UPHELD
        }
        Ok(_) => {
            panic!(
                "❌ SURVIVOR G1-D002: Empty dedupe_key should be rejected with InvalidDedupeKey"
            );
        }
        Err(e) => {
            panic!("Wrong error type for empty dedupe_key: {e:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G1-DEDUP-003: Dedupe key without colon
// PROMISE: INV-ID-003 - dedupe_key format is "workspace:change_id"
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g1_dedup_003_dedupe_key_without_colon() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db");
    let ws_path = temp.path();

    let req = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "invalid-format-no-colon".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let result = submit_to_queue(req, &db_path, ws_path).await;
    match result {
        Err(InvalidDedupeKey { dedupe_key, reason }) => {
            assert_eq!(dedupe_key, "invalid-format-no-colon");
            assert!(reason.contains("colon") || reason.contains("separator"));
            // ✅ PROMISE UPHELD
        }
        Ok(_) => {
            panic!("❌ SURVIVOR G1-D003: Malformed dedupe_key (no colon) should be rejected");
        }
        Err(e) => {
            eprintln!("Wrong error type: {e:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G1-DEDUP-004: Dedupe key with multiple colons
// PROMISE: INV-ID-003 - dedupe_key format validation consistency
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g1_dedup_004_dedupe_key_multiple_colons() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db");
    let ws_path = temp.path();

    let req = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws:change:extra:parts".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let result = submit_to_queue(req, &db_path, ws_path).await;
    // Multiple colons might be valid - check behavior
    match result {
        Ok(_) => {
            // If accepted, that's a behavior to document
            eprintln!("⚠️  OBSERVATION G1-D004: Multiple colons accepted in dedupe_key");
        }
        Err(InvalidDedupeKey { .. }) => {
            // Also valid - strict enforcement
        }
        Err(e) => {
            eprintln!("Unexpected error: {e:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G1-DEDUP-005: Same workspace, different dedupe_key
// PROMISE: PRE-Q-002 - If entry exists, workspace must match
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g1_dedup_005_same_workspace_different_dedupe_key() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db");
    let ws_path = temp.path();

    let req1 = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-a:kxyz789".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let req2 = QueueSubmissionRequest {
        workspace: "ws-a".to_string(), // SAME workspace
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-a:different123".to_string(), // DIFFERENT dedupe_key
        head_sha: "def456".to_string(),
        tested_against_sha: None,
    };

    // Submit first
    let result1 = submit_to_queue(req1, &db_path, ws_path).await;
    assert!(result1.is_ok());

    // Submit same workspace with different dedupe_key
    let result2 = submit_to_queue(req2, &db_path, ws_path).await;
    match result2 {
        Err(AlreadyInQueue {
            session,
            existing_dedupe_key,
            provided_dedupe_key,
        }) => {
            assert_eq!(session, "ws-a");
            assert_eq!(existing_dedupe_key, "ws-a:kxyz789");
            assert_eq!(provided_dedupe_key, "ws-a:different123");
            // ✅ PROMISE UPHELD
        }
        Ok(_) => {
            panic!(
                "❌ SURVIVOR G1-D005: Same workspace with different dedupe_key should be rejected"
            );
        }
        Err(e) => {
            panic!("Wrong error type: {e:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RQ-G1-DEDUP-006: Idempotent submission (same workspace, same dedupe_key)
// PROMISE: Graphite-style idempotent upsert
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn rq_g1_dedup_006_idempotent_submission() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("queue.db");
    let ws_path = temp.path();

    let req1 = QueueSubmissionRequest {
        workspace: "ws-a".to_string(),
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-a:kxyz789".to_string(),
        head_sha: "abc123".to_string(),
        tested_against_sha: None,
    };

    let req2 = QueueSubmissionRequest {
        workspace: "ws-a".to_string(), // SAME workspace
        bead_id: None,
        priority: 0,
        agent_id: None,
        dedupe_key: "ws-a:kxyz789".to_string(), // SAME dedupe_key
        head_sha: "def456".to_string(),         // Different SHA
        tested_against_sha: None,
    };

    // Submit first
    let result1 = submit_to_queue(req1, &db_path, ws_path).await;
    assert!(result1.is_ok());
    let entry_id_1 = result1.unwrap().entry_id;

    // Submit same workspace with same dedupe_key (idempotent upsert)
    let result2 = submit_to_queue(req2, &db_path, ws_path).await;
    match result2 {
        Ok(response) => {
            // Should succeed and update the existing entry
            assert_eq!(response.entry_id, entry_id_1, "Should update same entry");
            assert_eq!(response.submission_type.as_str(), "updated");
            // ✅ PROMISE UPHELD
        }
        Err(e) => {
            panic!("Idempotent submission should succeed: {e:?}");
        }
    }
}
