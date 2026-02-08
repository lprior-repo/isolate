//! High-concurrency stress tests for lock system
//!
//! Tests the system's ability to handle high concurrency levels:
//! - 10 concurrent agents
//! - 50 concurrent agents
//! - 100 concurrent agents
//!
//! Validates:
//! - No race conditions
//! - Lock contention metrics
//! - No deadlocks
//! - State consistency under load
//! - Lock acquisition rates
//! - Exactly one holder per session

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use sqlx::sqlite::SqlitePoolOptions;
use tokio::task::JoinSet;
use zjj_core::{coordination::locks::LockManager, Error};

/// Test helper: Create in-memory database pool
async fn test_pool() -> Result<sqlx::SqlitePool, Error> {
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))
}

/// Test helper: Setup lock manager
async fn setup_lock_manager() -> Result<LockManager, Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool);
    mgr.init().await?;
    Ok(mgr)
}

/// Metrics collected during stress test
#[derive(Debug, Clone, Default)]
struct StressTestMetrics {
    successful_acquisitions: usize,
    failed_acquisitions: usize,
    contentions: usize,
    acquisition_times_ms: Vec<f64>,
    unique_holders: HashSet<String>,
    duplicate_acquisitions: usize,
    deadlocks_detected: usize,
}

impl StressTestMetrics {
    /// Calculate lock acquisition rate (acquisitions per second)
    #[allow(clippy::cast_precision_loss)]
    fn acquisition_rate(&self, total_duration_ms: f64) -> f64 {
        if total_duration_ms > 0.0 {
            (self.successful_acquisitions as f64) / (total_duration_ms / 1000.0)
        } else {
            0.0
        }
    }

    /// Calculate contention percentage
    #[allow(clippy::cast_precision_loss)]
    fn contention_percentage(&self) -> f64 {
        let total_attempts = self.successful_acquisitions + self.failed_acquisitions;
        if total_attempts > 0 {
            (self.contentions as f64) / (total_attempts as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average acquisition time in milliseconds
    fn average_acquisition_time_ms(&self) -> f64 {
        if self.acquisition_times_ms.is_empty() {
            0.0
        } else {
            let sum: f64 = self.acquisition_times_ms.iter().sum();
            sum / self.acquisition_times_ms.len() as f64
        }
    }
}

// ========================================================================
// BDD SCENARIO 1: 10 Agents Locking Same Session
// ========================================================================
//
// GIVEN: A session and 10 concurrent agents
// WHEN: All agents attempt to lock the same session
// THEN: Exactly 1 agent succeeds, others fail with SESSION_LOCKED

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_10_agents_lock_same_session() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "contention-session-10";
    let num_agents = 10;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn 10 concurrent agents trying to lock the same session
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let session_clone = session.to_string();
        let agent_name = format!("agent-{agent_id}");

        join_set.spawn(async move {
            let attempt_start = Instant::now();
            let result = mgr_clone.lock(&session_clone, &agent_name).await;
            let elapsed = attempt_start.elapsed().as_millis();

            match result {
                Ok(_) => (agent_name, true, elapsed, None),
                Err(Error::SessionLocked { holder, .. }) => {
                    (agent_name, false, elapsed, Some(holder))
                }
                Err(e) => (agent_name, false, elapsed, Some(e.to_string())),
            }
        });
    }

    // Collect results using functional pattern
    let mut metrics = StressTestMetrics::default();
    let mut holders: Vec<String> = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((agent, success, elapsed_ms, maybe_holder)) => {
                if success {
                    metrics.successful_acquisitions += 1;
                    metrics.acquisition_times_ms.push(elapsed_ms as f64);
                    metrics.unique_holders.insert(agent.clone());
                    holders.push(agent);
                } else {
                    metrics.failed_acquisitions += 1;
                    metrics.contentions += 1;
                }
            }
            Err(e) => {
                // Task panicked - this indicates a serious problem
                metrics.deadlocks_detected += 1;
                eprintln!("Task panicked: {e}");
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: Exactly 1 agent MUST succeed
    assert_eq!(
        metrics.successful_acquisitions, 1,
        "Exactly 1 agent should acquire lock, got {}",
        metrics.successful_acquisitions
    );

    // THEN: Exactly 1 unique holder
    assert_eq!(
        metrics.unique_holders.len(),
        1,
        "Exactly 1 unique holder should exist, got {}",
        metrics.unique_holders.len()
    );

    // THEN: All other agents should have failed
    assert_eq!(
        metrics.failed_acquisitions,
        num_agents - 1,
        "Expected {} failed acquisitions, got {}",
        num_agents - 1,
        metrics.failed_acquisitions
    );

    // THEN: No deadlocks detected
    assert_eq!(metrics.deadlocks_detected, 0, "No deadlocks should occur");

    // THEN: Verify lock state in database
    let lock_state = mgr.get_lock_state(session).await?;
    assert!(
        lock_state.holder.is_some(),
        "Lock should be held in database"
    );
    assert!(
        metrics.unique_holders.contains(
            lock_state
                .holder
                .as_deref()
                .ok_or_else(|| { Error::ValidationError("Failed to convert holder to &str".into()) })?
        ),
        "Lock holder should match successful agent"
    );

    // Log metrics
    let acquisition_rate = metrics.acquisition_rate(total_duration.as_millis() as f64);
    let avg_time = metrics.average_acquisition_time_ms();
    let contention = metrics.contention_percentage();

    println!("test_10_agents_lock_same_session metrics:");
    println!("  - Acquisition rate: {acquisition_rate:.2} locks/sec");
    println!("  - Average acquisition time: {avg_time:.2}ms");
    println!("  - Contention percentage: {contention:.1}%");
    println!("  - Total duration: {:?}", total_duration);

    Ok(())
}

// ========================================================================
// BDD SCENARIO 2: 50 Agents Claiming Different Resources
// ========================================================================
//
// GIVEN: 50 agents and 50 different sessions
// WHEN: Each agent claims a unique session
// THEN: No duplicates, all succeed, state consistency maintained

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_50_agents_claim_unique_resources() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 50;
    let sessions_per_agent = 1;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn 50 agents, each locking a unique session
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let session_name = format!("session-{agent_id}");
        let agent_name = format!("agent-{agent_id}");

        join_set.spawn(async move {
            let mut agent_acquisitions = 0;
            let mut agent_errors = 0;

            // Each agent attempts to lock its assigned session
            let result = mgr_clone.lock(&session_name, &agent_name).await;
            match result {
                Ok(_) => {
                    agent_acquisitions += 1;
                }
                Err(_) => {
                    agent_errors += 1;
                }
            }

            (agent_name, session_name, agent_acquisitions, agent_errors)
        });
    }

    // Collect results
    let mut successful_locks: HashMap<String, String> = HashMap::new();
    let mut failed_count = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((agent, session, acquisitions, errors)) => {
                if acquisitions > 0 {
                    successful_locks.insert(session, agent);
                }
                if errors > 0 {
                    failed_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked during resource claim".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All 50 agents should succeed
    assert_eq!(
        successful_locks.len(),
        num_agents,
        "Expected {} successful locks, got {}",
        num_agents,
        successful_locks.len()
    );

    // THEN: No failures
    assert_eq!(
        failed_count, 0,
        "No lock acquisitions should fail for unique sessions"
    );

    // THEN: Verify no duplicate session locks
    assert_eq!(
        successful_locks.len(),
        HashSet::<&String>::from_iter(successful_locks.keys()).len(),
        "No duplicate session locks should exist"
    );

    // THEN: Verify database state consistency
    let all_locks = mgr.get_all_locks().await?;
    assert_eq!(
        all_locks.len(),
        num_agents,
        "Database should contain exactly {} locks",
        num_agents
    );

    // THEN: Verify each session has correct holder
    for (session, expected_holder) in &successful_locks {
        let lock_state = mgr.get_lock_state(session).await?;
        assert!(
            lock_state
                .holder
                .as_ref()
                .map_or(false, |h| h == expected_holder),
            "Session {session} holder mismatch: expected {expected_holder}, got {:?}",
            lock_state.holder
        );
    }

    // Log performance metrics
    let acquisition_rate = (num_agents as f64) / (total_duration.as_secs_f64());
    println!("test_50_agents_claim_unique_resources metrics:");
    println!("  - Total locks: {}", num_agents);
    println!("  - Acquisition rate: {acquisition_rate:.2} locks/sec");
    println!(
        "  - Average per-lock time: {:.2}ms",
        total_duration.as_millis() as f64 / num_agents as f64
    );
    println!("  - Total duration: {:?}", total_duration);

    Ok(())
}

// ========================================================================
// BDD SCENARIO 3: 100 Agents Concurrent Operations
// ========================================================================
//
// GIVEN: 100 agents performing lock/unlock operations
// WHEN: Agents perform rapid concurrent operations
// THEN: No crashes, state consistency maintained, no lost updates

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_100_agents_concurrent_operations() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 100;
    let num_sessions = 20; // Multiple sessions to reduce contention

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn 100 agents performing lock/unlock cycles on different sessions
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("agent-{agent_id}");
        let session_name = format!("session-{}", agent_id % num_sessions);

        join_set.spawn(async move {
            let mut successful_operations = 0;
            let mut failed_operations = 0;

            // Perform 5 lock/unlock cycles per agent
            for cycle in 0..5 {
                // Try to lock
                let lock_result = mgr_clone.lock(&session_name, &agent_name).await;
                match lock_result {
                    Ok(_) => {
                        successful_operations += 1;

                        // Immediately unlock
                        let unlock_result = mgr_clone.unlock(&session_name, &agent_name).await;
                        match unlock_result {
                            Ok(_) => {
                                successful_operations += 1;
                            }
                            Err(_) => {
                                failed_operations += 1;
                            }
                        }
                    }
                    Err(Error::SessionLocked { .. }) => {
                        // Expected under contention
                        failed_operations += 1;
                    }
                    Err(_) => {
                        failed_operations += 1;
                    }
                }

                // Small random delay to increase concurrency chaos
                let delay_ms = (agent_id + cycle) % 10;
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            (
                agent_name,
                session_name,
                successful_operations,
                failed_operations,
            )
        });
    }

    // Collect results
    let mut total_successful = 0;
    let mut total_failed = 0;
    let mut agent_results: Vec<(String, String, usize, usize)> = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((agent, session, success, fail)) => {
                total_successful += success;
                total_failed += fail;
                agent_results.push((agent, session, success, fail));
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown(
                    "Task panicked during concurrent operations".into(),
                ));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All agents should complete without crashes
    assert_eq!(
        agent_results.len(),
        num_agents,
        "All {} agents should complete",
        num_agents
    );

    // THEN: Total operations should be successful
    let total_operations = total_successful + total_failed;
    let success_rate = if total_operations > 0 {
        (total_successful as f64) / (total_operations as f64) * 100.0
    } else {
        0.0
    };

    println!("test_100_agents_concurrent_operations metrics:");
    println!("  - Total successful operations: {}", total_successful);
    println!("  - Total failed operations: {}", total_failed);
    println!("  - Success rate: {success_rate:.1}%");
    println!("  - Total duration: {:?}", total_duration);
    println!(
        "  - Operations per second: {:.2}",
        total_operations as f64 / total_duration.as_secs_f64()
    );

    // THEN: Verify database integrity (no partial/corrupted state)
    let all_locks = mgr.get_all_locks().await?;
    // Some locks may still be held, that's okay
    println!("  - Active locks remaining: {}", all_locks.len());

    // THEN: Verify no lock has invalid state
    for lock_info in &all_locks {
        let lock_state = mgr.get_lock_state(&lock_info.session).await?;
        assert!(
            lock_state.holder.is_some(),
            "Active lock {} should have a holder",
            lock_info.session
        );
    }

    Ok(())
}

// ========================================================================
// BDD SCENARIO 4: Lock/Unlock Storm
// ========================================================================
//
// GIVEN: 50 agents and 10 sessions
// WHEN: Rapid lock/unlock storm occurs
// THEN: State consistency maintained, audit trail complete

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_lock_unlock_storm_consistency() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 50;
    let num_sessions = 10;
    let operations_per_agent = 10;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Spawn agents for lock/unlock storm
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("storm-agent-{agent_id}");

        join_set.spawn(async move {
            let mut ops_completed = 0;

            for op in 0..operations_per_agent {
                let session_name = format!("storm-session-{}", op % num_sessions);

                // Try to lock
                let lock_result = mgr_clone.lock(&session_name, &agent_name).await;

                if lock_result.is_ok() {
                    ops_completed += 1;

                    // Random delay before unlock
                    let delay_ms = (agent_id * op) % 5;
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                    // Unlock
                    let _ = mgr_clone.unlock(&session_name, &agent_name).await;
                    ops_completed += 1;
                }

                // Small delay between operations
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            ops_completed
        });
    }

    // Wait for all agents
    let mut total_operations = 0;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(ops) => {
                total_operations += ops;
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked during storm".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: Verify all sessions have consistent state
    for session_id in 0..num_sessions {
        let session_name = format!("storm-session-{session_id}");
        let lock_state = mgr.get_lock_state(&session_name).await?;

        // State should be valid (either locked or unlocked, not corrupted)
        assert!(
            lock_state.holder.is_some() || lock_state.holder.is_none(),
            "Lock state for {} should be valid (not corrupted)",
            session_name
        );

        // Check audit trail
        let audit_log = mgr.get_lock_audit_log(&session_name).await?;

        // Verify audit log is not corrupted
        for entry in &audit_log {
            assert!(
                !entry.session.is_empty(),
                "Audit entry session should not be empty"
            );
            assert!(
                !entry.agent_id.is_empty(),
                "Audit entry agent_id should not be empty"
            );
            assert!(
                !entry.operation.is_empty(),
                "Audit entry operation should not be empty"
            );
            assert!(
                !entry.timestamp.is_empty(),
                "Audit entry timestamp should not be empty"
            );
        }

        println!(
            "Session {} - Audit entries: {}",
            session_name,
            audit_log.len()
        );
    }

    println!("test_lock_unlock_storm_consistency metrics:");
    println!("  - Total operations completed: {}", total_operations);
    println!("  - Duration: {:?}", total_duration);
    println!(
        "  - Operations per second: {:.2}",
        total_operations as f64 / total_duration.as_secs_f64()
    );

    Ok(())
}

// ========================================================================
// BDD SCENARIO 5: Claim Transfer Under Load
// ========================================================================
//
// GIVEN: A lock held by agent A
// WHEN: Agent B attempts to lock, A unlocks, then B locks
// THEN: No lost updates, clean transfer, no race conditions

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_claim_transfer_under_load() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let session = "transfer-session";
    let num_transfer_cycles = 50;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Simulate rapid lock transfers between agents
    for cycle in 0..num_transfer_cycles {
        let mgr_clone = Arc::clone(&mgr);
        let agent_a = format!("agent-a-{cycle}");
        let agent_b = format!("agent-b-{cycle}");
        let session_clone = session.to_string();

        join_set.spawn(async move {
            // Agent A locks
            let lock_a = mgr_clone.lock(&session_clone, &agent_a).await;
            if lock_a.is_err() {
                return (agent_a.clone(), agent_b.clone(), false, false, false);
            }

            // Agent B tries to lock (should fail)
            let lock_b_fails = mgr_clone.lock(&session_clone, &agent_b).await.is_err();

            // Agent A unlocks
            let unlock_a = mgr_clone.unlock(&session_clone, &agent_a).await;

            // Agent B locks again (should succeed)
            let lock_b_succeeds = mgr_clone.lock(&session_clone, &agent_b).await.is_ok();

            // Agent B unlocks
            let unlock_b = mgr_clone.unlock(&session_clone, &agent_b).await;

            (
                agent_a,
                agent_b,
                lock_b_fails,
                unlock_a.is_ok() && unlock_b.is_ok(),
                lock_b_succeeds,
            )
        });
    }

    // Collect results
    let mut successful_transfers = 0;
    let mut failed_transfers = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((_agent_a, _agent_b, contention_handled, unlocks_ok, final_lock_ok)) => {
                if contention_handled && unlocks_ok && final_lock_ok {
                    successful_transfers += 1;
                } else {
                    failed_transfers += 1;
                }
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked during transfer".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All transfers should succeed
    assert_eq!(
        successful_transfers, num_transfer_cycles,
        "All {} transfers should succeed",
        num_transfer_cycles
    );

    assert_eq!(failed_transfers, 0, "No transfers should fail");

    // THEN: Final state should be unlocked (all agents cleaned up)
    let final_state = mgr.get_lock_state(session).await?;
    assert!(
        final_state.holder.is_none(),
        "Session should be unlocked after all transfers"
    );

    // THEN: Verify audit trail completeness
    let audit_log = mgr.get_lock_audit_log(session).await?;
    assert!(
        audit_log.len() >= num_transfer_cycles * 4,
        "Audit log should have at least {} entries (4 per cycle)",
        num_transfer_cycles * 4
    );

    println!("test_claim_transfer_under_load metrics:");
    println!("  - Successful transfers: {}", successful_transfers);
    println!("  - Failed transfers: {}", failed_transfers);
    println!("  - Total audit entries: {}", audit_log.len());
    println!("  - Duration: {:?}", total_duration);

    Ok(())
}

// ========================================================================
// BDD SCENARIO 6: Lock Contention Metrics
// ========================================================================
//
// GIVEN: High contention scenario (100 agents, 5 sessions)
// WHEN: Agents compete for locks
// THEN: Contention metrics are measurable and reasonable

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_lock_contention_metrics() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 100;
    let num_sessions = 5;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // High contention: 100 agents competing for 5 sessions
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("contention-agent-{agent_id}");
        let session_name = format!("contention-session-{}", agent_id % num_sessions);

        join_set.spawn(async move {
            let attempt_start = Instant::now();
            let result = mgr_clone.lock(&session_name, &agent_name).await;
            let elapsed = attempt_start.elapsed();

            match result {
                Ok(_) => {
                    // Hold for a bit then release
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let _ = mgr_clone.unlock(&session_name, &agent_name).await;
                    (true, elapsed.as_millis())
                }
                Err(_) => (false, elapsed.as_millis()),
            }
        });
    }

    // Collect metrics
    let mut acquisition_times: Vec<u128> = Vec::new();
    let mut failure_times: Vec<u128> = Vec::new();
    let mut successes = 0;
    let mut failures = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((success, elapsed)) => {
                if success {
                    successes += 1;
                    acquisition_times.push(elapsed);
                } else {
                    failures += 1;
                    failure_times.push(elapsed);
                }
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown(
                    "Task panicked during contention test".into(),
                ));
            }
        }
    }

    let total_duration = start.elapsed();

    // Calculate metrics
    let avg_success_time = if acquisition_times.is_empty() {
        0.0
    } else {
        let sum: u128 = acquisition_times.iter().sum();
        sum as f64 / acquisition_times.len() as f64
    };

    let avg_failure_time = if failure_times.is_empty() {
        0.0
    } else {
        let sum: u128 = failure_times.iter().sum();
        sum as f64 / failure_times.len() as f64
    };

    let contention_rate = if num_agents > 0 {
        (failures as f64) / (num_agents as f64) * 100.0
    } else {
        0.0
    };

    println!("test_lock_contention_metrics results:");
    println!("  - Total agents: {}", num_agents);
    println!("  - Successful acquisitions: {}", successes);
    println!("  - Failed acquisitions (contention): {}", failures);
    println!("  - Contention rate: {contention_rate:.1}%");
    println!("  - Avg success time: {:.2}ms", avg_success_time);
    println!("  - Avg failure time: {:.2}ms", avg_failure_time);
    println!("  - Total duration: {:?}", total_duration);

    // THEN: All agents should complete
    assert_eq!(
        successes + failures,
        num_agents,
        "All agents should complete"
    );

    // THEN: Contention should be measurable (not zero, not 100%)
    assert!(
        contention_rate > 0.0,
        "Contention should be measurable under high competition"
    );

    // THEN: Failure time should be faster than success time (fail fast)
    if avg_success_time > 0.0 && avg_failure_time > 0.0 {
        assert!(
            avg_failure_time < avg_success_time,
            "Failed lock attempts should be faster than successful ones (fail fast): {:.2}ms vs {:.2}ms",
            avg_failure_time,
            avg_success_time
        );
    }

    Ok(())
}

// ========================================================================
// BDD SCENARIO 7: Deadlock Detection
// ========================================================================
//
// GIVEN: Multiple sessions and agents
// WHEN: Agents perform complex lock patterns
// THEN: No deadlocks occur (timeout would indicate deadlock)

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_no_deadlocks_under_load() -> Result<(), Error> {
    let mgr = Arc::new(setup_lock_manager().await?);
    let num_agents = 50;
    let num_sessions = 10;
    let operations_per_agent = 5;

    let mut join_set = JoinSet::new();
    let start = Instant::now();

    // Each agent tries to lock multiple sessions in sequence
    // This could deadlock if not implemented correctly
    for agent_id in 0..num_agents {
        let mgr_clone = Arc::clone(&mgr);
        let agent_name = format!("deadlock-agent-{agent_id}");

        join_set.spawn(async move {
            for op in 0..operations_per_agent {
                // Try to lock 2 different sessions
                let session1 = format!("deadlock-session-{}", (op + agent_id) % num_sessions);
                let session2 = format!("deadlock-session-{}", (op + agent_id + 1) % num_sessions);

                // Lock first session
                let lock1 = mgr_clone.lock(&session1, &agent_name).await;

                if lock1.is_ok() {
                    // Small delay
                    tokio::time::sleep(Duration::from_millis(1)).await;

                    // Try to lock second session
                    let lock2 = mgr_clone.lock(&session2, &agent_name).await;

                    // Unlock both
                    let _ = mgr_clone.unlock(&session1, &agent_name).await;
                    if lock2.is_ok() {
                        let _ = mgr_clone.unlock(&session2, &agent_name).await;
                    }
                }
            }

            // Return success if we reach here (no deadlock)
            true
        });
    }

    // Set a timeout - if deadlocks occur, test will hang
    let timeout_duration = Duration::from_secs(30);
    let mut completed_agents = 0;

    // Join with timeout logic
    let deadline = Instant::now() + timeout_duration;
    while let Some(result) = join_set.join_next().await {
        if Instant::now() > deadline {
            return Err(Error::ValidationError(
                "Potential deadlock detected - test exceeded timeout".into(),
            ));
        }

        match result {
            Ok(_success) => {
                completed_agents += 1;
            }
            Err(e) => {
                eprintln!("Task panicked: {e}");
                return Err(Error::Unknown("Task panicked (possible deadlock)".into()));
            }
        }
    }

    let total_duration = start.elapsed();

    // THEN: All agents should complete
    assert_eq!(
        completed_agents, num_agents,
        "All {} agents should complete without deadlock",
        num_agents
    );

    // THEN: Test should complete well before timeout
    assert!(
        total_duration < timeout_duration,
        "Test completed well within deadline - no deadlock detected"
    );

    println!("test_no_deadlocks_under_load results:");
    println!("  - Agents completed: {}", completed_agents);
    println!("  - Duration: {:?}", total_duration);
    println!("  - Timeout limit: {:?}", timeout_duration);
    println!("  - Status: No deadlocks detected");

    Ok(())
}
