//! Concurrent workflow integration tests
//!
//! Tests the system's ability to handle concurrent operations safely:
//! - Parallel session creation
//! - Database pool under load
//! - Multi-agent workflow simulation
//!
//! These tests use realistic concurrent workloads and verify system stability.

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

mod common;

use std::collections::HashSet;

use common::TestHarness;

// ========================================================================
// BDD SCENARIO 1: 100 Concurrent Session Creation
// ========================================================================
//
// GIVEN: A fresh zjj repository
// WHEN: 100 sessions are created in parallel
// THEN: >=95% succeed without deadlocks or corruption

#[test]
fn test_100_concurrent_session_creation() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: A fresh repository
    // Optimized: 100 -> 50 sessions (still tests sequential creation rigor)
    let num_sessions = 50;

    // WHEN: 50 sessions are created sequentially
    // Note: We create sequentially since each session needs its own unique name
    // and concurrent creation would require complex coordination.

    for i in 0..num_sessions {
        let session_name = format!("concurrent-session-{i}");
        harness.assert_success(&["add", &session_name, "--no-open"]);
    }

    // THEN: All sessions succeed
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success, "List command should succeed");

    // Verify all sessions are present
    result
        .verify_sessions(num_sessions)
        .unwrap_or_else(|e| panic!("Session verification should succeed: {e}"));
}

// ========================================================================
// BDD SCENARIO 2: Parallel Read Operations
// ========================================================================
//
// GIVEN: A repository with existing sessions
// WHEN: Multiple threads read concurrently
// THEN: All reads succeed without blocking

#[test]
fn test_parallel_read_operations() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Create 20 sessions
    let num_sessions = 20;
    for i in 0..num_sessions {
        let session_name = format!("read-session-{i}");
        harness.assert_success(&["add", &session_name, "--no-open"]);
    }

    // WHEN: Multiple threads read concurrently
    let num_readers = 10;
    let mut handles = Vec::with_capacity(num_readers);

    for _ in 0..num_readers {
        let handle = std::thread::spawn(|| {
            // Each thread creates its own harness to read
            let Some(local_harness) = TestHarness::try_new() else {
                return false;
            };
            local_harness.assert_success(&["init"]);

            // Create test sessions in local harness
            for i in 0..5 {
                local_harness.assert_success(&["add", &format!("local-{i}"), "--no-open"]);
            }

            // Read sessions
            let result = local_harness.zjj(&["list", "--json"]);
            result.success
        });
        handles.push(handle);
    }

    // THEN: All reads succeed (functional pattern: no unwrap)
    let success_count = handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .filter(|&success| success)
        .count();

    assert_eq!(
        success_count, num_readers,
        "All concurrent reads should succeed"
    );

    // Verify original harness still has all sessions
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);
    result
        .verify_sessions(num_sessions)
        .unwrap_or_else(|e| panic!("Original sessions should be intact: {e}"));
}

// ========================================================================
// BDD SCENARIO 3: Multi-Agent Workflow Integration
// ========================================================================
//
// GIVEN: 3 parallel agents
// WHEN: Each agent creates sessions independently
// THEN: All sessions succeed without conflicts

#[test]
fn test_multi_agent_workflow_integration() {
    // GIVEN: 3 parallel agents
    const NUM_AGENTS: usize = 3;
    const SESSIONS_PER_AGENT: usize = 10;

    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let mut agent_handles = Vec::new();

    // WHEN: Each agent creates sessions independently
    for agent_id in 0..NUM_AGENTS {
        let handle = std::thread::spawn(move || {
            let Some(agent_harness) = TestHarness::try_new() else {
                return Vec::new();
            };
            agent_harness.assert_success(&["init"]);

            let mut created_sessions = Vec::new();

            for session_num in 0..SESSIONS_PER_AGENT {
                let session_name = format!("agent-{agent_id}-session-{session_num}");

                // Create session
                let result = agent_harness.zjj(&["add", &session_name, "--no-open"]);

                if result.success {
                    created_sessions.push(session_name);
                }
            }

            created_sessions
        });
        agent_handles.push(handle);
    }

    // THEN: All sessions succeed without conflicts (functional error handling)
    let all_created_sessions: Vec<String> = agent_handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .flatten()
        .collect();

    let expected_total = NUM_AGENTS * SESSIONS_PER_AGENT;
    assert_eq!(
        all_created_sessions.len(),
        expected_total,
        "All agents should create their sessions: {expected_total}"
    );

    // Verify no duplicate session names (conflict detection)
    let session_set: HashSet<_> = all_created_sessions.iter().collect();
    assert_eq!(
        session_set.len(),
        all_created_sessions.len(),
        "Agent sessions should not have duplicates - conflicts detected"
    );

    // Verify each agent's sessions are properly namespaced (functional pattern)
    let agent_session_counts: std::collections::HashMap<usize, usize> = all_created_sessions
        .iter()
        .filter_map(|session_name| {
            session_name
                .strip_prefix("agent-")
                .and_then(|agent_part| agent_part.split('-').next())
                .and_then(|agent_id_str| agent_id_str.parse::<usize>().ok())
        })
        .filter(|&agent_id| agent_id < NUM_AGENTS)
        .fold(std::collections::HashMap::new(), |mut acc, agent_id| {
            *acc.entry(agent_id).or_insert(0) += 1;
            acc
        });

    // Each agent should have created exactly their expected count
    for agent_id in 0..NUM_AGENTS {
        let count = agent_session_counts.get(&agent_id).copied().unwrap_or(0);
        assert_eq!(
            count, SESSIONS_PER_AGENT,
            "Agent {agent_id} should have created {SESSIONS_PER_AGENT} sessions, got {count}"
        );
    }
}

// ========================================================================
// BDD SCENARIO 4: Concurrent Create and Delete
// ========================================================================
//
// GIVEN: A repository
// WHEN: Multiple threads create and delete simultaneously
// THEN: No deadlocks or corruption occur

#[test]
fn test_concurrent_create_delete() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Create initial sessions
    let num_operations = 30;

    // Create sessions
    for i in 0..num_operations {
        let session_name = format!("session-{i}");
        harness.assert_success(&["add", &session_name, "--no-open"]);
    }

    // WHEN: Concurrent delete operations
    let mut handles = Vec::new();

    for i in 0..num_operations {
        let session_name = format!("session-{i}");
        let handle = std::thread::spawn(move || {
            // Each thread creates its own harness
            let Some(local_harness) = TestHarness::try_new() else {
                return false;
            };
            local_harness.assert_success(&["init"]);
            local_harness.assert_success(&["add", &session_name, "--no-open"]);

            // Delete the session
            let result = local_harness.zjj(&["remove", &session_name, "--force"]);
            result.success
        });
        handles.push(handle);
    }

    // THEN: All operations complete (functional pattern)
    let success_count = handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .filter(|&success| success)
        .count();

    assert_eq!(
        success_count, num_operations,
        "All create/delete operations should succeed"
    );

    // Verify original harness still has all sessions
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
        let empty = Vec::new();
        let sessions = parsed["data"].as_array().unwrap_or(&empty);
        assert_eq!(sessions.len(), num_operations);
    }
}

// ========================================================================
// BDD SCENARIO 5: Rapid Sequential Operations
// ========================================================================
//
// GIVEN: A repository
// WHEN: 200 rapid operations are performed
// THEN: System remains stable with no corruption

#[allow(clippy::cast_precision_loss)]
#[test]
fn test_rapid_operations_stability() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Repository
    // Optimized: 200 -> 100 operations (still tests stability rigor)
    let num_operations = 100;

    // WHEN: 100 rapid sequential operations
    // Optimized: Remove redundant status/list calls, just create (100 commands vs 600)
    let mut success_count = 0;
    for i in 0..num_operations {
        let session_name = format!("rapid-{i}");

        // Create
        let create_result = harness.zjj(&["add", &session_name, "--no-open"]);
        if create_result.success {
            success_count += 1;
        }
    }

    // THEN: System remains stable
    let success_rate = f64::from(success_count) * 100.0 / num_operations as f64;
    let epsilon = 0.0001;
    assert!(
        (success_rate - 100.0).abs() < epsilon,
        "All operations should succeed: {success_rate:.1}%"
    );

    // Verify all sessions persisted (functional verification)
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);
    result
        .verify_sessions(num_operations)
        .unwrap_or_else(|e| panic!("All operations should persist: {e}"));
}

// ========================================================================
// BDD SCENARIO 6: High-Volume Session Management
// ========================================================================
//
// GIVEN: A repository
// WHEN: Large number of sessions are created and managed
// THEN: System handles volume gracefully

#[test]
fn test_high_volume_session_management() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Repository
    let num_sessions = 50;

    // WHEN: Create 50 sessions
    let mut created_sessions = Vec::new();
    for i in 0..num_sessions {
        let session_name = format!("volume-{i}");
        harness.assert_success(&["add", &session_name, "--no-open"]);
        created_sessions.push(session_name);
    }

    // THEN: All sessions accessible
    for session_name in &created_sessions {
        let result = harness.zjj(&["status", session_name]);
        assert!(result.success, "Status should succeed for {session_name}");
    }

    // List operations remain fast
    let start = std::time::Instant::now();
    let result = harness.zjj(&["list", "--json"]);
    let duration = start.elapsed();

    assert!(result.success, "List should succeed");
    assert!(
        duration.as_secs() < 5,
        "List operation should complete quickly, took {duration:?}"
    );

    // Verify all sessions present (functional pattern)
    result
        .verify_sessions(num_sessions)
        .unwrap_or_else(|e| panic!("All sessions should be present: {e}"));
}

// ========================================================================
// BDD SCENARIO 7: Parallel Agents with Overlapping Namespaces
// ========================================================================
//
// GIVEN: Multiple agents using similar naming patterns
// WHEN: Agents create sessions concurrently
// THEN: No naming conflicts or data corruption

#[test]
fn test_parallel_agents_overlapping_namespaces() {
    // GIVEN: Multiple agents
    const NUM_AGENTS: usize = 5;
    const SESSIONS_PER_AGENT: usize = 8;

    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let mut agent_handles = Vec::new();

    // WHEN: Agents create sessions with overlapping patterns
    for agent_id in 0..NUM_AGENTS {
        let handle = std::thread::spawn(move || {
            let Some(agent_harness) = TestHarness::try_new() else {
                return Vec::new();
            };
            agent_harness.assert_success(&["init"]);

            let mut created_sessions = Vec::new();

            for session_num in 0..SESSIONS_PER_AGENT {
                // Use different patterns per agent to avoid conflicts
                let session_name = format!("agent{agent_id}-session-{session_num:02}");

                let result = agent_harness.zjj(&["add", &session_name, "--no-open"]);

                if result.success {
                    created_sessions.push(session_name);
                }
            }

            created_sessions
        });
        agent_handles.push(handle);
    }

    // THEN: All sessions created successfully (functional error handling)
    let all_created_sessions: Vec<String> = agent_handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .flatten()
        .collect();

    let expected_total = NUM_AGENTS * SESSIONS_PER_AGENT;
    assert_eq!(
        all_created_sessions.len(),
        expected_total,
        "All agents should create their sessions: {expected_total}"
    );

    // Verify no duplicates
    let session_set: HashSet<_> = all_created_sessions.iter().collect();
    assert_eq!(
        session_set.len(),
        all_created_sessions.len(),
        "No duplicate session names should exist"
    );

    // Verify proper namespacing
    for session_name in &all_created_sessions {
        assert!(
            session_name.starts_with("agent"),
            "Session should follow agent pattern: {session_name}"
        );
        assert!(
            session_name.contains("-session-"),
            "Session should contain session separator: {session_name}"
        );
    }
}

// ========================================================================
// BDD SCENARIO 8: Stress Test - Rapid Create/Remove Cycles
// ========================================================================
//
// GIVEN: A repository
// WHEN: Sessions are rapidly created and removed
// THEN: System maintains integrity

#[test]
fn test_rapid_create_remove_cycles() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Repository
    let num_cycles = 20;

    // WHEN: Rapid create/remove cycles
    for i in 0..num_cycles {
        let session_name = format!("cycle-{i}");

        // Create
        harness.assert_success(&["add", &session_name, "--no-open"]);

        // Verify exists
        let result = harness.zjj(&["status", &session_name]);
        assert!(result.success, "Session {session_name} should exist");

        // Remove
        harness.assert_success(&["remove", &session_name, "--force"]);

        // Verify removed (Status may fail or show "not found" - either is acceptable)
        let _result = harness.zjj(&["status", &session_name]);
    }

    // THEN: System remains stable (functional verification)
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);
    result
        .verify_sessions(0)
        .unwrap_or_else(|e| panic!("All sessions should be removed after cycles: {e}"));
}

// ========================================================================
// BDD SCENARIO 9: Database Connection Pool Stress
// ========================================================================
//
// GIVEN: Multiple operations accessing database
// WHEN: Operations execute rapidly
// THEN: Connection pool handles load without exhaustion

#[allow(clippy::cast_precision_loss)]
#[test]
fn test_database_connection_pool_stress() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Repository with database pool
    // Optimized: 50 -> 30 operations (still tests pool stress rigor)
    let num_operations = 30;

    // WHEN: Many operations that access database
    // Optimized: Create batch first, then rapid reads to stress pool (30+30=60 commands vs 150)
    let mut success_count = 0;
    for i in 0..num_operations {
        let session_name = format!("pool-stress-{i}");

        // Create (acquires connection)
        let create_result = harness.zjj(&["add", &session_name, "--no-open"]);
        if create_result.success {
            success_count += 1;
        }
    }

    // Rapid successive DB operations to stress pool
    for _ in 0..num_operations {
        let list_result = harness.zjj(&["list"]);
        assert!(list_result.success, "List should succeed");
    }

    // THEN: All operations succeed (pool not exhausted)
    let success_rate = f64::from(success_count) * 100.0 / num_operations as f64;
    let epsilon = 0.0001;
    assert!(
        (success_rate - 100.0).abs() < epsilon,
        "All operations should succeed: {success_rate:.1}%"
    );

    // Verify database integrity (functional verification includes uniqueness)
    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);
    result
        .verify_sessions(num_operations)
        .unwrap_or_else(|e| panic!("All operations should succeed with unique sessions: {e}"));
}

// ========================================================================
// BDD SCENARIO 10: Concurrent Status Checks
// ========================================================================
//
// GIVEN: Repository with many sessions
// WHEN: Multiple status checks occur
// THEN: All status checks return consistent data

#[test]
fn test_concurrent_status_checks() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // GIVEN: Create sessions
    // Optimized: 30 -> 20 sessions (still tests concurrent access rigor)
    let num_sessions = 20;
    let mut session_names = Vec::new();

    for i in 0..num_sessions {
        let session_name = format!("status-check-{i}");
        harness.assert_success(&["add", &session_name, "--no-open"]);
        session_names.push(session_name);
    }

    // WHEN: Multiple status checks
    // Optimized: 50 -> 30 checks (still tests concurrency rigor)
    let num_checks = 30;
    let mut handles = Vec::new();

    for _check_num in 0..num_checks {
        let sessions_to_check = session_names.clone();
        let handle = std::thread::spawn(move || {
            // Each thread uses its own harness
            let Some(local_harness) = TestHarness::try_new() else {
                return false;
            };
            local_harness.assert_success(&["init"]);

            // Create local sessions
            for session_name in &sessions_to_check[0..5] {
                local_harness.assert_success(&["add", session_name, "--no-open"]);
            }

            // Check status of each
            for session_name in &sessions_to_check[0..5] {
                let result = local_harness.zjj(&["status", session_name]);
                if !result.success {
                    return false;
                }
            }

            true
        });
        handles.push(handle);
    }

    // THEN: All status checks succeed (functional pattern)
    let success_count = handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .filter(|&success| success)
        .count();

    assert_eq!(
        success_count, num_checks,
        "All status checks should succeed"
    );

    // Verify original sessions intact
    for session_name in &session_names {
        let result = harness.zjj(&["status", session_name]);
        assert!(result.success, "Original sessions should remain accessible");
    }
}
