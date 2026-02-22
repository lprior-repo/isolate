//! Custom assertions for merge train E2E tests
//!
//! This module provides specialized assertion helpers for validating:
//! - State machine transitions
//! - Train event emission
//! - Position ordering
//! - Lock state
//! - JSONL output structure

// Allow test code ergonomics
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::future_not_send,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::redundant_closure_for_method_calls
)]

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use sqlx::Row;
use zjj_core::coordination::{QueueEntry, QueueStatus};

use super::merge_train_fixtures::JsonlCapturer;

/// Assert that a state transition is valid
pub fn assert_state_transition(from: QueueStatus, to: QueueStatus) {
    assert!(
        from.can_transition_to(to),
        "Invalid state transition: {:?} -> {:?}",
        from,
        to
    );
}

/// Assert that a state transition is invalid
pub fn assert_invalid_state_transition(from: QueueStatus, to: QueueStatus) {
    assert!(
        !from.can_transition_to(to),
        "Expected invalid transition, but {:?} -> {:?} is valid",
        from,
        to
    );
}

/// Assert that a status is terminal
pub fn assert_terminal_status(status: QueueStatus) {
    assert!(
        status.is_terminal(),
        "Expected terminal status, but {:?} is not terminal",
        status
    );
}

/// Assert that a status is not terminal
pub fn assert_non_terminal_status(status: QueueStatus) {
    assert!(
        !status.is_terminal(),
        "Expected non-terminal status, but {:?} is terminal",
        status
    );
}

/// Assert that TrainStep event was emitted
pub async fn assert_train_step(capturer: &JsonlCapturer, entry_id: i64, action: &str) {
    let steps = capturer.find_train_steps(entry_id).await;
    let found = steps
        .iter()
        .any(|s| s.get("action").and_then(|a| a.as_str()) == Some(action));

    assert!(
        found,
        "Expected TrainStep with action '{action}' for entry {entry_id}, but not found. Steps: {steps:?}",
        steps = steps
    );
}

/// Assert that TrainStep was NOT emitted
pub async fn assert_no_train_step(capturer: &JsonlCapturer, entry_id: i64, action: &str) {
    let steps = capturer.find_train_steps(entry_id).await;
    let found = steps
        .iter()
        .any(|s| s.get("action").and_then(|a| a.as_str()) == Some(action));

    assert!(
        !found,
        "Expected no TrainStep with action '{action}' for entry {entry_id}, but found one"
    );
}

/// Assert that TrainResult event was emitted with correct counts
pub async fn assert_train_result(
    capturer: &JsonlCapturer,
    merged: usize,
    failed: usize,
    kicked: usize,
) {
    let result = capturer
        .train_result()
        .await
        .expect("Expected TrainResult event, but none found");

    let total_merged = result
        .get("merged")
        .and_then(|m: &JsonValue| m.as_array())
        .map(Vec::len)
        .unwrap_or(0);

    let total_failed = result
        .get("failed")
        .and_then(|f: &JsonValue| f.as_array())
        .map(Vec::len)
        .unwrap_or(0);

    let total_kicked = result
        .get("kicked")
        .and_then(|k: &JsonValue| k.as_array())
        .map(Vec::len)
        .unwrap_or(0);

    assert_eq!(
        total_merged, merged,
        "Expected {merged} merged entries, got {total_merged}"
    );
    assert_eq!(
        total_failed, failed,
        "Expected {failed} failed entries, got {total_failed}"
    );
    assert_eq!(
        total_kicked, kicked,
        "Expected {kicked} kicked entries, got {total_kicked}"
    );

    // Verify total_processed
    let total_processed = usize::try_from(
        result
            .get("total_processed")
            .and_then(|t: &JsonValue| t.as_u64())
            .expect("TrainResult should have total_processed"),
    )
    .expect("total_processed should fit in usize");

    assert_eq!(
        total_processed,
        merged + failed + kicked,
        "total_processed should equal merged + failed + kicked"
    );
}

/// Assert that processing lock is held
pub async fn assert_lock_held(pool: &sqlx::sqlite::SqlitePool, agent_id: &str) {
    let lock = sqlx::query("SELECT * FROM queue_processing_lock WHERE id = 1")
        .fetch_optional(pool)
        .await
        .expect("Failed to query lock table");

    assert!(
        lock.is_some(),
        "Expected processing lock to be held, but lock table is empty"
    );

    let lock = lock.unwrap();
    let lock_agent_id: String = lock.get("agent_id");
    assert_eq!(
        lock_agent_id, agent_id,
        "Expected lock held by {agent_id}, got {lock_agent_id}"
    );
}

/// Assert that processing lock is NOT held
pub async fn assert_no_lock(pool: &sqlx::sqlite::SqlitePool) {
    let lock = sqlx::query("SELECT * FROM queue_processing_lock WHERE id = 1")
        .fetch_optional(pool)
        .await
        .expect("Failed to query lock table");

    assert!(
        lock.is_none(),
        "Expected no processing lock, but lock is held by {}",
        lock.unwrap().get::<String, _>("agent_id")
    );
}

/// Assert that lock has expired
pub async fn assert_lock_expired(pool: &sqlx::sqlite::SqlitePool) {
    let lock = sqlx::query("SELECT expires_at FROM queue_processing_lock WHERE id = 1")
        .fetch_one(pool)
        .await
        .expect("Failed to query lock table");

    let expires_at: i64 = lock.get("expires_at");
    let now = chrono::Utc::now().timestamp();

    assert!(
        expires_at < now,
        "Expected lock to be expired, but expires_at ({expires_at}) is in the future"
    );
}

/// Assert that priorities are unique
pub fn assert_unique_priorities(entries: &[QueueEntry]) {
    let mut seen = std::collections::HashSet::new();

    for entry in entries {
        assert!(
            seen.insert(entry.priority),
            "Duplicate priority {} found for entry {} ({})",
            entry.priority,
            entry.id,
            entry.workspace
        );
    }
}

/// Assert that dedupe keys are unique
pub fn assert_unique_dedupe_keys(entries: &[QueueEntry]) {
    let mut seen: HashMap<String, &QueueEntry> = std::collections::HashMap::new();

    for entry in entries {
        if let Some(key) = &entry.dedupe_key {
            if let Some(prev) = seen.get(key) {
                panic!(
                    "Duplicate dedupe_key '{key}' for entries {} ({}) and {} ({})",
                    entry.id, entry.workspace, prev.id, prev.workspace
                );
            }
            seen.insert(key.clone(), entry);
        }
    }
}

/// Assert that entry has specific status
pub fn assert_entry_status(entry: &QueueEntry, expected: QueueStatus) {
    assert_eq!(
        entry.status, expected,
        "Expected entry {} ({}) to have status {expected:?}, got {:?}",
        entry.id, entry.workspace, entry.status
    );
}

/// Assert that entry has error message
pub fn assert_entry_has_error(entry: &QueueEntry) {
    assert!(
        entry.error_message.is_some(),
        "Expected entry {} ({}) to have error message, but none found",
        entry.id,
        entry.workspace
    );
}

/// Assert that entry has no error message
pub fn assert_entry_no_error(entry: &QueueEntry) {
    assert!(
        entry.error_message.is_none(),
        "Expected entry {} ({}) to have no error, but found: {:?}",
        entry.id,
        entry.workspace,
        entry.error_message
    );
}

/// Assert that attempt count is as expected
pub fn assert_attempt_count(entry: &QueueEntry, expected: i32) {
    assert_eq!(
        entry.attempt_count, expected,
        "Expected entry {} ({}) to have attempt_count {expected}, got {}",
        entry.id, entry.workspace, entry.attempt_count
    );
}

/// Assert that head_sha is set
pub fn assert_head_sha_set(entry: &QueueEntry) {
    assert!(
        entry.head_sha.is_some(),
        "Expected entry {} ({}) to have head_sha, but none found",
        entry.id,
        entry.workspace
    );
}

/// Assert that head_sha changed
pub fn assert_head_sha_changed(old: &QueueEntry, new: &QueueEntry) {
    assert!(
        old.head_sha != new.head_sha,
        "Expected head_sha to change for entry {} ({}), but both are {:?}",
        old.id,
        old.workspace,
        old.head_sha
    );
}

/// Assert that tested_against_sha is set
pub fn assert_tested_against_sha_set(entry: &QueueEntry) {
    assert!(
        entry.tested_against_sha.is_some(),
        "Expected entry {} ({}) to have tested_against_sha, but none found",
        entry.id,
        entry.workspace
    );
}

/// Assert event was emitted with specific type
pub async fn assert_event_type(capturer: &JsonlCapturer, event_type: &str) {
    let events = capturer.find_by_type(event_type).await;
    assert!(
        !events.is_empty(),
        "Expected at least one event of type '{event_type}', but found none"
    );
}

/// Assert event was NOT emitted with specific type
pub async fn assert_no_event_type(capturer: &JsonlCapturer, event_type: &str) {
    let events = capturer.find_by_type(event_type).await;
    assert!(
        events.is_empty(),
        "Expected no events of type '{event_type}', but found {}",
        events.len()
    );
}

/// Assert event count matches expected
pub async fn assert_event_count(capturer: &JsonlCapturer, expected: usize) {
    let count = capturer.count().await;
    assert_eq!(count, expected, "Expected {expected} events, got {count}");
}

/// Assert JSONL output is valid (each line is valid JSON)
pub fn assert_valid_jsonl(output: &str) {
    for (i, line) in output.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: Result<JsonValue, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "Line {i} is not valid JSON: {line}\nError: {:?}",
            parsed.err()
        );
    }
}

/// Assert all events have required timestamp field
pub async fn assert_timestamps_present(capturer: &JsonlCapturer) {
    let events = capturer.events().await;

    for (i, event) in events.iter().enumerate() {
        assert!(
            event.get("timestamp").is_some(),
            "Event {i} missing timestamp field: {event}",
            event = serde_json::to_string(event).unwrap()
        );
    }
}

/// Assert all events have required type field
pub async fn assert_event_types_present(capturer: &JsonlCapturer) {
    let events = capturer.events().await;

    for (i, event) in events.iter().enumerate() {
        assert!(
            event.get("type").is_some(),
            "Event {i} missing type field: {event}",
            event = serde_json::to_string(event).unwrap()
        );
    }
}

/// Assert entries are processed in position order
pub fn assert_sequential_processing(entry_ids: &[i64]) {
    let mut sorted = entry_ids.to_vec();
    sorted.sort_unstable();

    assert_eq!(
        entry_ids, &sorted,
        "Entries were not processed in sequential order. Got {entry_ids:?}, expected {sorted:?}"
    );
}

/// Assert train duration is reasonable
pub fn assert_reasonable_duration(duration_secs: i64, max_expected_secs: i64) {
    assert!(
        duration_secs > 0,
        "Duration should be positive, got {duration_secs}s"
    );
    assert!(
        duration_secs < max_expected_secs,
        "Duration {duration_secs}s exceeds expected maximum {max_expected_secs}s"
    );
}

/// Assert that exactly one entry is active at a time
pub fn assert_single_active_entry(entries: &[QueueEntry]) {
    let active_count = entries
        .iter()
        .filter(|e| {
            matches!(
                e.status,
                QueueStatus::Claimed
                    | QueueStatus::Rebasing
                    | QueueStatus::Testing
                    | QueueStatus::ReadyToMerge
                    | QueueStatus::Merging
            )
        })
        .count();

    assert_eq!(
        active_count, 1,
        "Expected exactly 1 active entry, found {active_count}"
    );
}

/// Assert that only entries with passing tests are merged
pub fn assert_only_passing_tests_merged(
    entries: &[QueueEntry],
    test_results: &HashMap<String, bool>,
) {
    for entry in entries {
        if entry.status == QueueStatus::Merged {
            let passed = test_results.get(&entry.workspace).unwrap_or(&false);
            assert!(
                passed,
                "Entry {} ({}) was merged but tests did not pass",
                entry.id, entry.workspace
            );
        }
    }
}

/// Assert that entries without conflicts are merged
pub fn assert_only_conflict_free_merged(
    entries: &[QueueEntry],
    conflict_results: &HashMap<String, bool>,
) {
    for entry in entries {
        if entry.status == QueueStatus::Merged {
            let has_conflicts = conflict_results.get(&entry.workspace).unwrap_or(&false);
            assert!(
                !has_conflicts,
                "Entry {} ({}) was merged but has conflicts",
                entry.id, entry.workspace
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use zjj_core::coordination::QueueStatus;

    use super::*;

    #[test]
    fn test_assert_state_transition_valid() {
        // These should not panic
        assert_state_transition(QueueStatus::Pending, QueueStatus::Claimed);
        assert_state_transition(QueueStatus::Claimed, QueueStatus::Rebasing);
        assert_state_transition(QueueStatus::Testing, QueueStatus::ReadyToMerge);
        assert_state_transition(QueueStatus::Merging, QueueStatus::Merged);
    }

    #[test]
    #[should_panic(expected = "Invalid state transition")]
    fn test_assert_state_transition_invalid() {
        assert_state_transition(QueueStatus::Pending, QueueStatus::Merged);
    }

    #[test]
    fn test_assert_invalid_state_transition() {
        // These should not panic
        assert_invalid_state_transition(QueueStatus::Pending, QueueStatus::Merged);
        assert_invalid_state_transition(QueueStatus::Merged, QueueStatus::Pending);
    }

    #[test]
    fn test_assert_terminal_status() {
        // These should not panic
        assert_terminal_status(QueueStatus::Merged);
        assert_terminal_status(QueueStatus::FailedTerminal);
        assert_terminal_status(QueueStatus::Cancelled);
    }

    #[test]
    #[should_panic(expected = "Expected terminal status")]
    fn test_assert_terminal_status_fails_for_non_terminal() {
        assert_terminal_status(QueueStatus::Pending);
    }

    #[test]
    fn test_assert_valid_jsonl() {
        let valid_jsonl = r#"{"type":"TrainStep"}
{"type":"TrainResult"}
{"type":"Train"}"#;
        assert_valid_jsonl(valid_jsonl);
    }

    #[test]
    #[should_panic(expected = "is not valid JSON")]
    fn test_assert_valid_jsonl_invalid() {
        let invalid_jsonl = r#"{"type":"TrainStep"}
not json
{"type":"TrainResult"}"#;
        assert_valid_jsonl(invalid_jsonl);
    }

    #[test]
    fn test_assert_sequential_processing() {
        // Should not panic
        assert_sequential_processing(&[1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic(expected = "not processed in sequential order")]
    fn test_assert_sequential_processing_out_of_order() {
        assert_sequential_processing(&[1, 3, 2, 4]);
    }

    #[test]
    fn test_assert_reasonable_duration() {
        // Should not panic
        assert_reasonable_duration(10, 100);
        assert_reasonable_duration(1, 1000);
    }

    #[test]
    #[should_panic(expected = "Duration should be positive")]
    fn test_assert_reasonable_duration_zero() {
        assert_reasonable_duration(0, 100);
    }

    #[test]
    #[should_panic(expected = "exceeds expected maximum")]
    fn test_assert_reasonable_duration_too_long() {
        assert_reasonable_duration(200, 100);
    }
}
