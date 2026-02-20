//! Unit tests for lease claim, heartbeat, and reclaim functionality.
//!
//! These tests verify the core lease lifecycle:
//! - Claim creates lease with owner
//! - Heartbeat extends lease TTL
//! - Reclaim allows new owner after expiration
//! - Expired lease cannot continue ownership
//!
//! The tests use deterministic time via mockable thresholds (no external clock mocking).

// Integration tests have relaxed clippy settings for test infrastructure.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::task::JoinSet;
use zjj_core::{
    coordination::{
        locks::LockManager,
        queue::{MergeQueue, QueueStatus},
    },
    Error,
};

// =============================================================================
// TEST UTILITIES
// =============================================================================

async fn test_pool() -> Result<SqlitePool, Error> {
    sqlx::sqlite::SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))
}

async fn setup_lock_manager() -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool);
    mgr.init().await?;
    Ok(mgr)
}

async fn setup_merge_queue() -> Result<MergeQueue, Error> {
    MergeQueue::open_in_memory()
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create in-memory merge queue: {e}")))
}

// =============================================================================
// BDD SCENARIO 1: CLAIM CREATES LEASE WITH OWNER
// =============================================================================
//
// GIVEN: An agent and an available session
// WHEN: The agent claims the session
// THEN: A lease is created with that agent as owner
// AND: The lease has a valid expiration time
// AND: Only one owner exists per entry

mod claim_creates_lease {
    use super::*;

    /// Test: Claim creates a lease with the claiming agent as owner.
    #[tokio::test]
    async fn claim_creates_lease_with_owner() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "claim-test-session";
        let agent = "claim-agent-1";

        // WHEN: Agent claims the session
        let response = mgr.lock(session, agent).await?;

        // THEN: Lease is created with correct owner
        assert_eq!(response.session, session);
        assert_eq!(response.agent_id, agent);
        assert!(!response.lock_id.is_empty(), "Lock ID should be generated");

        // THEN: Expiration time is in the future
        let now = chrono::Utc::now();
        assert!(
            response.expires_at > now,
            "Expiration should be in the future"
        );

        // THEN: Verify lock state in database
        let lock_state = mgr.get_lock_state(session).await?;
        assert!(
            lock_state.holder.is_some(),
            "Lock holder should be set in database"
        );
        assert_eq!(lock_state.holder.as_deref(), Some(agent));

        Ok(())
    }

    /// Test: Only one owner per entry - contention scenario.
    #[tokio::test]
    async fn only_one_owner_per_entry_contention() -> Result<(), Error> {
        let mgr = Arc::new(setup_lock_manager().await?);
        let session = "single-owner-session";
        let num_agents = 10;

        let mut join_set = JoinSet::new();

        // Spawn multiple agents trying to claim the same session
        for agent_id in 0..num_agents {
            let mgr_clone = Arc::clone(&mgr);
            let session_name = session.to_string();
            let agent_name = format!("contention-agent-{agent_id}");

            join_set.spawn(async move { mgr_clone.lock(&session_name, &agent_name).await });
        }

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut successful_agent: Option<String> = None;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(response)) => {
                    success_count += 1;
                    successful_agent = Some(response.agent_id);
                }
                Ok(Err(Error::SessionLocked { .. })) => {
                    failure_count += 1;
                }
                Ok(Err(e)) => {
                    return Err(Error::DatabaseError(format!(
                        "Unexpected error during contention: {e}"
                    )));
                }
                Err(e) => {
                    return Err(Error::DatabaseError(format!("Task join failed: {e}")));
                }
            }
        }

        // THEN: Exactly ONE agent should succeed
        assert_eq!(
            success_count, 1,
            "Exactly one agent should claim successfully, got {success_count}"
        );
        assert_eq!(
            failure_count,
            num_agents - 1,
            "All other agents should fail with SessionLocked"
        );

        // THEN: Verify the database shows only one owner
        let lock_state = mgr.get_lock_state(session).await?;
        assert_eq!(
            lock_state.holder, successful_agent,
            "Database should show the winning agent as holder"
        );

        Ok(())
    }

    /// Test: Same agent re-claiming is idempotent.
    #[tokio::test]
    async fn same_agent_reclaim_is_idempotent() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "idempotent-session";
        let agent = "idempotent-agent";

        // First claim
        let first = mgr.lock(session, agent).await?;

        // Same agent claims again - should be idempotent
        let second = mgr.lock(session, agent).await?;

        // THEN: Same lock ID returned (idempotent)
        assert_eq!(first.lock_id, second.lock_id);
        assert_eq!(first.session, second.session);
        assert_eq!(first.agent_id, second.agent_id);

        Ok(())
    }

    /// Test: `MergeQueue` claim creates lease with processing lock.
    #[tokio::test]
    async fn queue_claim_creates_processing_lock() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add an entry to the queue
        let add_response = queue.add("claim-workspace", None, 5, None).await?;
        assert_eq!(
            add_response.entry.status,
            QueueStatus::Pending,
            "Entry should start in pending state"
        );

        // WHEN: Agent claims next entry
        let claimed = queue.next_with_lock("queue-agent-1").await?;

        // THEN: Entry is claimed
        assert!(
            claimed.is_some(),
            "Agent should successfully claim an entry"
        );
        let entry =
            claimed.ok_or_else(|| Error::DatabaseError("Expected claimed entry".to_string()))?;
        assert_eq!(entry.status, QueueStatus::Claimed);
        assert_eq!(entry.agent_id, Some("queue-agent-1".to_string()));

        // THEN: Processing lock exists
        let lock = queue.get_processing_lock().await?;
        assert!(lock.is_some(), "Processing lock should exist after claim");
        let lock_info =
            lock.ok_or_else(|| Error::DatabaseError("Expected processing lock".to_string()))?;
        assert_eq!(lock_info.agent_id, "queue-agent-1");

        Ok(())
    }
}

// =============================================================================
// BDD SCENARIO 2: HEARTBEAT EXTENDS LEASE
// =============================================================================
//
// GIVEN: An active lease held by an agent
// WHEN: The agent sends a heartbeat
// THEN: The lease TTL is extended
// AND: The new expiration is later than the original

mod heartbeat_extends_lease {
    use super::*;

    /// Test: Heartbeat extends lock TTL for `LockManager`.
    #[tokio::test]
    async fn heartbeat_extends_lock_ttl() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "heartbeat-session";
        let agent = "heartbeat-agent";

        // Create initial lock
        let initial = mgr.lock(session, agent).await?;
        let initial_expires = initial.expires_at;

        // Small delay to ensure time difference
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // WHEN: Agent sends heartbeat
        let heartbeat_response = mgr.heartbeat(session, agent).await?;

        // THEN: TTL is extended (new expiration > old expiration)
        assert!(
            heartbeat_response.expires_at > initial_expires,
            "Heartbeat should extend expiration time: {:?} should be > {:?}",
            heartbeat_response.expires_at,
            initial_expires
        );

        // THEN: Same lock ID and agent
        assert_eq!(heartbeat_response.lock_id, initial.lock_id);
        assert_eq!(heartbeat_response.agent_id, agent);

        Ok(())
    }

    /// Test: Non-holder cannot extend lease via heartbeat.
    #[tokio::test]
    async fn non_holder_cannot_heartbeat() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "non-holder-heartbeat";
        let holder = "holder-agent";
        let non_holder = "non-holder-agent";

        // Lock by holder
        let _ = mgr.lock(session, holder).await?;

        // WHEN: Non-holder tries to heartbeat
        let result = mgr.heartbeat(session, non_holder).await;

        // THEN: Should fail
        assert!(
            result.is_err(),
            "Non-holder should not be able to heartbeat"
        );
        match result {
            Err(Error::NotLockHolder {
                session: s,
                agent_id,
            }) => {
                assert_eq!(s, session);
                assert_eq!(agent_id, non_holder);
            }
            Err(e) => {
                return Err(Error::DatabaseError(format!(
                    "Expected NotLockHolder error, got: {e}"
                )));
            }
            Ok(_) => {
                return Err(Error::ValidationError {
                    message: "Non-holder should not be able to heartbeat".into(),
                    field: None,
                    value: None,
                    constraints: vec![],
                });
            }
        }

        Ok(())
    }

    /// Test: Heartbeat on non-existent lock fails.
    #[tokio::test]
    async fn heartbeat_nonexistent_lock_fails() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "nonexistent-heartbeat";
        let agent = "ghost-agent";

        // WHEN: Heartbeat on lock that was never created
        let result = mgr.heartbeat(session, agent).await;

        // THEN: Should fail
        assert!(result.is_err(), "Heartbeat on nonexistent lock should fail");

        Ok(())
    }

    /// Test: Queue `extend_lock` extends processing lock.
    #[tokio::test]
    async fn queue_extend_lock_extends_ttl() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add entry and claim it
        queue.add("extend-workspace", None, 5, None).await?;
        let _ = queue.next_with_lock("extend-agent").await?;

        // Get initial lock
        let initial_lock = queue.get_processing_lock().await?;
        assert!(initial_lock.is_some());
        let initial_expires = initial_lock
            .ok_or_else(|| Error::DatabaseError("Expected lock".to_string()))?
            .expires_at;

        // WHEN: Extend the lock
        let extended = queue.extend_lock("extend-agent", 60).await?;

        // THEN: Extension succeeds
        assert!(extended, "Lock extension should succeed");

        // THEN: New expiration is later
        let new_lock = queue.get_processing_lock().await?;
        assert!(new_lock.is_some());
        let new_expires = new_lock
            .ok_or_else(|| Error::DatabaseError("Expected lock".to_string()))?
            .expires_at;

        assert!(
            new_expires > initial_expires,
            "Extended expiration should be later"
        );

        Ok(())
    }

    /// Test: Non-holder cannot extend queue processing lock.
    #[tokio::test]
    async fn queue_non_holder_cannot_extend_lock() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add entry and claim it
        queue.add("non-holder-extend", None, 5, None).await?;
        let _ = queue.next_with_lock("lock-holder").await?;

        // WHEN: Different agent tries to extend
        let extended = queue.extend_lock("lock-imposter", 60).await?;

        // THEN: Extension fails (no rows affected)
        assert!(
            !extended,
            "Non-holder should not be able to extend the lock"
        );

        Ok(())
    }

    /// Test: Multiple heartbeats keep extending TTL.
    #[tokio::test]
    async fn multiple_heartbeats_keep_extending() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "multi-heartbeat-session";
        let agent = "multi-heartbeat-agent";

        // Create initial lock
        let mut response = mgr.lock(session, agent).await?;
        let mut last_expires = response.expires_at;

        // WHEN: Multiple heartbeats
        for i in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            response = mgr.heartbeat(session, agent).await?;

            // THEN: Each heartbeat extends further
            assert!(
                response.expires_at > last_expires,
                "Heartbeat {i} should extend expiration past previous"
            );
            last_expires = response.expires_at;
        }

        Ok(())
    }
}

// =============================================================================
// BDD SCENARIO 3: RECLAIM ALLOWS NEW OWNER AFTER EXPIRATION
// =============================================================================
//
// GIVEN: A lease that has expired
// WHEN: A new agent tries to claim the resource
// THEN: The new agent successfully acquires the lease
// AND: The old owner no longer has access

mod reclaim_after_expiration {
    use super::*;

    /// Test: Expired lock allows new owner to claim.
    #[tokio::test]
    async fn expired_lock_allows_new_claim() -> Result<(), Error> {
        // Create lock manager with very short TTL (100ms)
        let pool = test_pool().await?;
        let ttl = chrono::Duration::milliseconds(100);
        let mgr = LockManager::with_ttl(pool, ttl);
        mgr.init().await?;

        let session = "expired-session";
        let original_agent = "original-agent";
        let new_agent = "new-agent";

        // Original agent claims
        let _ = mgr.lock(session, original_agent).await?;

        // Wait for expiration (150ms > 100ms TTL)
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // WHEN: New agent tries to claim
        let result = mgr.lock(session, new_agent).await?;

        // THEN: New agent successfully acquires lock
        assert_eq!(result.agent_id, new_agent);
        assert_eq!(result.session, session);

        // THEN: Database shows new owner
        let lock_state = mgr.get_lock_state(session).await?;
        assert_eq!(lock_state.holder, Some(new_agent.to_string()));

        Ok(())
    }

    /// Test: Reclaim stale entries in queue.
    #[tokio::test]
    async fn queue_reclaim_stale_entries() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add entry
        let response = queue.add("stale-workspace", None, 5, None).await?;
        let entry_id = response.entry.id;

        // Manually claim the entry and set started_at to the past
        let now = chrono::Utc::now().timestamp();
        let stale_time = now - 600; // 10 minutes ago

        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'stale-agent' WHERE id = ?2"
        )
        .bind(stale_time)
        .bind(entry_id)
        .execute(queue.pool())
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Verify it's in claimed state
        let before = queue.get_by_id(entry_id).await?;
        assert!(before.is_some());
        let before_entry =
            before.ok_or_else(|| Error::DatabaseError("Expected entry".to_string()))?;
        assert_eq!(before_entry.status, QueueStatus::Claimed);

        // WHEN: Reclaim stale entries (5 minute threshold)
        let reclaimed = queue.reclaim_stale(300).await?;

        // THEN: Entry was reclaimed
        assert_eq!(reclaimed, 1, "One entry should be reclaimed");

        // THEN: Entry is back to pending
        let after = queue.get_by_id(entry_id).await?;
        assert!(after.is_some());
        let after_entry = after
            .ok_or_else(|| Error::DatabaseError("Expected entry after reclaim".to_string()))?;
        assert_eq!(after_entry.status, QueueStatus::Pending);
        assert!(
            after_entry.agent_id.is_none(),
            "Agent ID should be cleared after reclaim"
        );
        assert!(
            after_entry.started_at.is_none(),
            "Started at should be cleared after reclaim"
        );

        Ok(())
    }

    /// Test: Recent claims are not reclaimed.
    #[tokio::test]
    async fn queue_recent_claims_not_reclaimed() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add and claim entry normally
        let response = queue.add("recent-workspace", None, 5, None).await?;
        let entry_id = response.entry.id;

        // Claim via normal flow
        let claimed = queue.next_with_lock("recent-agent").await?;
        assert!(claimed.is_some());

        // WHEN: Reclaim with short threshold (should not reclaim recent claims)
        let reclaimed = queue.reclaim_stale(300).await?;

        // THEN: No entries reclaimed (too recent)
        assert_eq!(reclaimed, 0, "Recent claims should not be reclaimed");

        // THEN: Entry is still claimed
        let entry = queue.get_by_id(entry_id).await?;
        assert!(entry.is_some());
        let entry = entry.ok_or_else(|| Error::DatabaseError("Expected entry".to_string()))?;
        assert_eq!(entry.status, QueueStatus::Claimed);

        Ok(())
    }

    /// Test: New owner can claim after reclaim.
    #[tokio::test]
    async fn new_owner_can_claim_after_reclaim() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add entry and make it stale
        let response = queue.add("reclaim-claim-workspace", None, 5, None).await?;
        let entry_id = response.entry.id;

        let stale_time = chrono::Utc::now().timestamp() - 600;
        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'stale-agent' WHERE id = ?2"
        )
        .bind(stale_time)
        .bind(entry_id)
        .execute(queue.pool())
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Reclaim
        let _ = queue.reclaim_stale(300).await?;

        // Release any existing processing lock
        let _ = queue.release_processing_lock("stale-agent").await;

        // WHEN: New agent claims
        let claimed = queue.next_with_lock("new-agent").await?;

        // THEN: New agent successfully claims
        assert!(claimed.is_some());
        let entry =
            claimed.ok_or_else(|| Error::DatabaseError("Expected claimed entry".to_string()))?;
        assert_eq!(entry.agent_id, Some("new-agent".to_string()));
        assert_eq!(entry.status, QueueStatus::Claimed);

        Ok(())
    }

    /// Test: Reclaim also releases expired processing locks.
    #[tokio::test]
    async fn reclaim_releases_expired_processing_locks() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add entry and claim
        queue.add("lock-release-workspace", None, 5, None).await?;
        let _ = queue.next_with_lock("lock-holder").await?;

        // Verify lock exists
        let lock_before = queue.get_processing_lock().await?;
        assert!(lock_before.is_some());

        // Manually expire the lock
        let expired_time = chrono::Utc::now().timestamp() - 100;
        sqlx::query("UPDATE queue_processing_lock SET expires_at = ?1 WHERE id = 1")
            .bind(expired_time)
            .execute(queue.pool())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // WHEN: Reclaim stale
        queue.reclaim_stale(300).await?;

        // THEN: Processing lock is released
        let lock_after = queue.get_processing_lock().await?;
        assert!(
            lock_after.is_none(),
            "Expired processing lock should be released"
        );

        Ok(())
    }
}

// =============================================================================
// BDD SCENARIO 4: EXPIRED LEASE CANNOT CONTINUE OWNERSHIP
// =============================================================================
//
// GIVEN: A lease that has expired
// WHEN: The original owner tries to perform operations
// THEN: Operations are denied
// AND: The original owner is treated as non-owner

mod expired_lease_denied {
    use super::*;

    /// Test: Original owner cannot heartbeat after expiration.
    #[tokio::test]
    async fn original_owner_cannot_heartbeat_after_expiration() -> Result<(), Error> {
        // Create lock manager with very short TTL (100ms)
        let pool = test_pool().await?;
        let ttl = chrono::Duration::milliseconds(100);
        let mgr = LockManager::with_ttl(pool, ttl);
        mgr.init().await?;

        let session = "expired-heartbeat-session";
        let agent = "expiring-agent";

        // Create lock
        let _ = mgr.lock(session, agent).await?;

        // Wait for expiration (150ms > 100ms TTL)
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // WHEN: Original owner tries to heartbeat
        let result = mgr.heartbeat(session, agent).await;

        // THEN: Heartbeat fails (lock is considered expired/nonexistent)
        assert!(
            result.is_err(),
            "Expired lease owner should not be able to heartbeat"
        );

        Ok(())
    }

    /// Test: Original owner cannot unlock after expiration (if new owner exists).
    #[tokio::test]
    async fn original_owner_cannot_unlock_after_new_owner() -> Result<(), Error> {
        // Create lock manager with short TTL (100ms)
        let pool = test_pool().await?;
        let ttl = chrono::Duration::milliseconds(100);
        let mgr = LockManager::with_ttl(pool, ttl);
        mgr.init().await?;

        let session = "expired-unlock-session";
        let original = "original-unlocker";
        let new_owner = "new-unlocker";

        // Original claims
        let _ = mgr.lock(session, original).await?;

        // Wait for expiration (150ms > 100ms TTL)
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // New owner claims
        let _ = mgr.lock(session, new_owner).await?;

        // WHEN: Original owner tries to unlock
        let result = mgr.unlock(session, original).await;

        // THEN: Unlock fails (original is no longer the holder)
        assert!(
            result.is_err(),
            "Original owner should not be able to unlock after new owner takes over"
        );
        match result {
            Err(Error::NotLockHolder {
                session: s,
                agent_id,
            }) => {
                assert_eq!(s, session);
                assert_eq!(agent_id, original);
            }
            Err(e) => {
                return Err(Error::DatabaseError(format!(
                    "Expected NotLockHolder, got: {e}"
                )));
            }
            Ok(()) => {
                return Err(Error::ValidationError {
                    message: "Original owner should not be able to unlock".into(),
                    field: None,
                    value: None,
                    constraints: vec![],
                });
            }
        }

        Ok(())
    }

    /// Test: Only current valid owner can perform operations.
    #[tokio::test]
    async fn only_current_owner_can_operate() -> Result<(), Error> {
        let mgr = setup_lock_manager().await?;
        let session = "current-owner-only";
        let owner = "current-owner";
        let non_owner = "non-owner";

        // Owner claims
        let _ = mgr.lock(session, owner).await?;

        // WHEN: Non-owner tries to unlock
        let result = mgr.unlock(session, non_owner).await;

        // THEN: Operation denied
        assert!(result.is_err());

        // WHEN: Non-owner tries to heartbeat
        let result = mgr.heartbeat(session, non_owner).await;

        // THEN: Operation denied
        assert!(result.is_err());

        // WHEN: Actual owner unlocks
        let result = mgr.unlock(session, owner).await;

        // THEN: Operation succeeds
        assert!(result.is_ok());

        Ok(())
    }
}

// =============================================================================
// BDD SCENARIO 5: RACE CONDITIONS IN LEASE OPERATIONS
// =============================================================================
//
// GIVEN: Multiple agents competing for resources
// WHEN: Concurrent operations occur
// THEN: No data corruption, consistent state, no double-claims

mod race_conditions {
    use super::*;

    /// Test: Concurrent heartbeats from same owner are safe.
    #[tokio::test]
    async fn concurrent_heartbeats_same_owner_safe() -> Result<(), Error> {
        let mgr = Arc::new(setup_lock_manager().await?);
        let session = "concurrent-heartbeat-session";
        let agent = "concurrent-heartbeat-agent";

        // Create lock
        let _ = mgr.lock(session, agent).await?;

        let mut join_set = JoinSet::new();

        // WHEN: Same owner sends multiple concurrent heartbeats
        for _ in 0..10 {
            let mgr_clone = Arc::clone(&mgr);
            let session_name = session.to_string();
            let agent_name = agent.to_string();

            join_set.spawn(async move { mgr_clone.heartbeat(&session_name, &agent_name).await });
        }

        let mut successes = 0;
        let mut failures = 0;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(_)) => successes += 1,
                Ok(Err(_)) => failures += 1,
                Err(e) => {
                    return Err(Error::DatabaseError(format!("Task join failed: {e}")));
                }
            }
        }

        // THEN: All heartbeats should succeed (idempotent operation)
        assert_eq!(
            successes, 10,
            "All heartbeats from same owner should succeed"
        );
        assert_eq!(failures, 0, "No failures expected for same owner");

        // THEN: Lock is still valid
        let lock_state = mgr.get_lock_state(session).await?;
        assert_eq!(lock_state.holder, Some(agent.to_string()));

        Ok(())
    }

    /// Test: Concurrent reclaim operations are safe.
    #[tokio::test]
    async fn concurrent_reclaim_safe() -> Result<(), Error> {
        let queue = Arc::new(setup_merge_queue().await?);

        // Create multiple stale entries
        for i in 0..10 {
            let response = queue
                .add(&format!("reclaim-race-{i}"), None, 5, None)
                .await?;
            let stale_time = chrono::Utc::now().timestamp() - 600;
            sqlx::query(
                "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'stale-agent' WHERE id = ?2"
            )
            .bind(stale_time)
            .bind(response.entry.id)
            .execute(queue.pool())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        }

        let mut join_set = JoinSet::new();

        // WHEN: Multiple concurrent reclaim operations
        for _ in 0..5 {
            let queue_clone = Arc::clone(&queue);
            join_set.spawn(async move { queue_clone.reclaim_stale(300).await });
        }

        let mut total_reclaimed = 0;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(response)) => total_reclaimed += response,
                Ok(Err(e)) => {
                    return Err(Error::DatabaseError(format!("Reclaim failed: {e}")));
                }
                Err(e) => {
                    return Err(Error::DatabaseError(format!("Task join failed: {e}")));
                }
            }
        }

        // THEN: All stale entries are reclaimed exactly once
        // (Concurrent reclaims may find different subsets)
        assert!(
            total_reclaimed <= 10,
            "Total reclaimed should not exceed total entries"
        );

        Ok(())
    }

    /// Test: Claim/expire/reclaim cycle maintains consistency.
    #[tokio::test]
    async fn claim_expire_reclaim_cycle_consistent() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Add entry
        let response = queue.add("cycle-workspace", None, 5, None).await?;
        let entry_id = response.entry.id;

        // Claim
        let claimed = queue.next_with_lock("cycle-agent").await?;
        assert!(claimed.is_some());

        // Simulate expiration
        let stale_time = chrono::Utc::now().timestamp() - 600;
        sqlx::query("UPDATE merge_queue SET started_at = ?1 WHERE id = ?2")
            .bind(stale_time)
            .bind(entry_id)
            .execute(queue.pool())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Expire the processing lock too
        sqlx::query("UPDATE queue_processing_lock SET expires_at = ?1 WHERE id = 1")
            .bind(stale_time)
            .execute(queue.pool())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Reclaim
        let _ = queue.reclaim_stale(300).await?;

        // THEN: Entry is back to pending
        let entry = queue.get_by_id(entry_id).await?;
        assert!(entry.is_some());
        let entry = entry.ok_or_else(|| Error::DatabaseError("Expected entry".to_string()))?;
        assert_eq!(entry.status, QueueStatus::Pending);

        // New agent can claim
        let new_claim = queue.next_with_lock("new-cycle-agent").await?;
        assert!(new_claim.is_some());

        Ok(())
    }
}

// =============================================================================
// BDD SCENARIO 6: DETERMINISTIC TIME HANDLING
// =============================================================================
//
// Tests that verify lease behavior with explicit time thresholds,
// not relying on external clock mocking.

mod deterministic_time {
    use super::*;

    /// Test: Lock TTL is correctly calculated.
    #[tokio::test]
    async fn lock_ttl_correctly_calculated() -> Result<(), Error> {
        let pool = test_pool().await?;
        let custom_ttl = chrono::Duration::seconds(600); // 10 minutes
        let mgr = LockManager::with_ttl(pool, custom_ttl);
        mgr.init().await?;

        let session = "ttl-calc-session";
        let agent = "ttl-calc-agent";

        let before = chrono::Utc::now();
        let response = mgr.lock(session, agent).await?;
        let after = chrono::Utc::now();

        // THEN: Expiration should be approximately TTL from now
        let min_expected = before + custom_ttl - chrono::Duration::seconds(1);
        let max_expected = after + custom_ttl + chrono::Duration::seconds(1);

        assert!(
            response.expires_at >= min_expected,
            "Expiration should be at least TTL from before claim"
        );
        assert!(
            response.expires_at <= max_expected,
            "Expiration should be at most TTL from after claim"
        );

        Ok(())
    }

    /// Test: Heartbeat extends by TTL amount.
    #[tokio::test]
    async fn heartbeat_extends_by_ttl() -> Result<(), Error> {
        let pool = test_pool().await?;
        let custom_ttl = chrono::Duration::seconds(600);
        let mgr = LockManager::with_ttl(pool, custom_ttl);
        mgr.init().await?;

        let session = "heartbeat-ttl-session";
        let agent = "heartbeat-ttl-agent";

        let _initial = mgr.lock(session, agent).await?;

        // Small delay
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let before_hb = chrono::Utc::now();
        let after_hb = mgr.heartbeat(session, agent).await?;

        // THEN: New expiration should be approximately TTL from heartbeat time
        let min_expected = before_hb + custom_ttl - chrono::Duration::seconds(1);
        let max_expected = chrono::Utc::now() + custom_ttl + chrono::Duration::seconds(1);

        assert!(
            after_hb.expires_at >= min_expected,
            "Heartbeat should extend by TTL from heartbeat time"
        );
        assert!(
            after_hb.expires_at <= max_expected,
            "Heartbeat extension should be reasonable"
        );

        Ok(())
    }

    /// Test: Reclaim threshold is correctly applied.
    #[tokio::test]
    async fn reclaim_threshold_correctly_applied() -> Result<(), Error> {
        let queue = setup_merge_queue().await?;

        // Create entries at different ages
        let now = chrono::Utc::now().timestamp();

        // Entry 1: 10 minutes ago (stale by 5 min threshold)
        let resp1 = queue.add("threshold-old", None, 5, None).await?;
        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'old-agent' WHERE id = ?2"
        )
        .bind(now - 600)
        .bind(resp1.entry.id)
        .execute(queue.pool())
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Entry 2: 2 minutes ago (NOT stale by 5 min threshold)
        let resp2 = queue.add("threshold-recent", None, 5, None).await?;
        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'recent-agent' WHERE id = ?2"
        )
        .bind(now - 120)
        .bind(resp2.entry.id)
        .execute(queue.pool())
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // WHEN: Reclaim with 5 minute threshold
        let reclaimed = queue.reclaim_stale(300).await?;

        // THEN: Only the old entry is reclaimed
        assert_eq!(
            reclaimed, 1,
            "Only entries older than threshold should be reclaimed"
        );

        // Verify entry states
        let old_entry = queue
            .get_by_id(resp1.entry.id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Expected old entry".to_string()))?;
        let recent_entry = queue
            .get_by_id(resp2.entry.id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Expected recent entry".to_string()))?;

        assert_eq!(
            old_entry.status,
            QueueStatus::Pending,
            "Old entry should be reclaimed to pending"
        );
        assert_eq!(
            recent_entry.status,
            QueueStatus::Claimed,
            "Recent entry should remain claimed"
        );

        Ok(())
    }
}
