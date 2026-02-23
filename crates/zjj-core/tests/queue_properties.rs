//! Property-based tests for queue invariants using proptest.
//!
//! These tests define the contract that the Queue object must satisfy.
//! RED PHASE: These tests MUST FAIL initially until the implementation is complete.
//!
//! Properties tested:
//! 1. Single worker at a time (exclusive processing lock)
//! 2. Priority ordering preserved (FIFO within same priority)
//! 3. State machine transitions valid (only allowed transitions)
//! 4. Terminal states immutable (no transitions from merged/failed_terminal/cancelled)

// Integration tests have relaxed clippy settings for test ergonomics.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
    clippy::needless_continue
)]

use proptest::prelude::*;
use zjj_core::coordination::{
    pure_queue::{PureQueue, PureQueueError},
    queue_status::{QueueStatus, TransitionError},
};

// =============================================================================
// STRATEGIES
// =============================================================================

/// Strategy for generating valid agent IDs
fn agent_id_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,19}"
}

/// Strategy for generating valid workspace names
fn workspace_strategy() -> impl Strategy<Value = String> {
    "ws-[a-zA-Z0-9_-]{1,20}"
}

/// Strategy for generating valid priorities
fn priority_strategy() -> impl Strategy<Value = i32> {
    0..=10i32
}

/// Strategy for generating valid QueueStatus values
fn queue_status_strategy() -> impl Strategy<Value = QueueStatus> {
    prop_oneof![
        Just(QueueStatus::Pending),
        Just(QueueStatus::Claimed),
        Just(QueueStatus::Rebasing),
        Just(QueueStatus::Testing),
        Just(QueueStatus::ReadyToMerge),
        Just(QueueStatus::Merging),
        Just(QueueStatus::Merged),
        Just(QueueStatus::FailedRetryable),
        Just(QueueStatus::FailedTerminal),
        Just(QueueStatus::Cancelled),
    ]
}

/// Strategy for generating non-terminal states
fn non_terminal_status_strategy() -> impl Strategy<Value = QueueStatus> {
    prop_oneof![
        Just(QueueStatus::Pending),
        Just(QueueStatus::Claimed),
        Just(QueueStatus::Rebasing),
        Just(QueueStatus::Testing),
        Just(QueueStatus::ReadyToMerge),
        Just(QueueStatus::Merging),
        Just(QueueStatus::FailedRetryable),
    ]
}

/// Strategy for generating terminal states
fn terminal_status_strategy() -> impl Strategy<Value = QueueStatus> {
    prop_oneof![
        Just(QueueStatus::Merged),
        Just(QueueStatus::FailedTerminal),
        Just(QueueStatus::Cancelled),
    ]
}

// =============================================================================
// PROPERTY 1: SINGLE WORKER AT A TIME
// =============================================================================

proptest! {
    /// Property: Only one agent can hold the processing lock at any time.
    /// This is the "single worker invariant" - concurrent claim attempts must
    /// result in exactly one holder.
    #[test]
    fn prop_single_worker_at_a_time(
        agent1 in agent_id_strategy(),
        agent2 in agent_id_strategy(),
    ) {
        // Setup: Create a queue with pending entries
        let queue = PureQueue::new();
        let queue = queue.add("ws-test", 5, None);
        prop_assert!(queue.is_ok());
        let queue = queue.expect("queue should be created successfully");

        // Agent1 claims successfully
        let claim1 = queue.claim_next(&agent1);
        prop_assert!(claim1.is_ok(), "First agent should be able to claim");
        let (queue_after_claim1, claimed_ws) = claim1.expect("claim should succeed");

        // Verify lock is held by agent1
        prop_assert!(queue_after_claim1.is_locked());
        prop_assert_eq!(queue_after_claim1.lock_holder(), Some(&agent1));

        // If agent2 is different from agent1, claim must fail
        if agent1 != agent2 {
            let claim2 = queue_after_claim1.claim_next(&agent2);
            prop_assert!(claim2.is_err(), "Second agent should not be able to claim while first holds lock");
        }

        // Agent1 releases the lock by transitioning to terminal
        let queue_released = queue_after_claim1.release(&claimed_ws);
        prop_assert!(queue_released.is_ok());
        let queue_released = queue_released.expect("release should succeed");

        // Now agent2 can claim (if different from agent1)
        if agent1 != agent2 {
            let claim2 = queue_released.claim_next(&agent2);
            prop_assert!(claim2.is_ok(), "Second agent should be able to claim after release");
        }
    }

    /// Property: Concurrent claim attempts result in exactly one winner.
    #[test]
    fn prop_concurrent_claims_single_winner(
        agents in proptest::collection::vec(agent_id_strategy(), 2..10),
    ) {
        // Setup: Create a queue with one pending entry
        let queue = PureQueue::new();
        let queue = queue.add("ws-test", 5, None);
        prop_assert!(queue.is_ok());
        let queue = queue.expect("queue should be created successfully");

        // First agent to claim wins
        let first_agent = &agents[0];
        let claim_result = queue.claim_next(first_agent);
        prop_assert!(claim_result.is_ok(), "First agent should claim successfully");

        let (queue_after_claim, _) = claim_result.expect("claim result should be valid");

        // Count how many agents can claim now (should be 0 - only one winner)
        let successful_claims: Vec<_> = agents
            .iter()
            .filter(|agent| {
                queue_after_claim.claim_next(agent).is_ok()
            })
            .collect();

        prop_assert!(
            successful_claims.is_empty(),
            "No other agents should be able to claim after first agent holds lock"
        );
    }
}

// =============================================================================
// PROPERTY 2: PRIORITY ORDERING PRESERVED
// =============================================================================

proptest! {
    /// Property: Entries are dequeued in priority order (lowest first).
    /// Within the same priority, FIFO order is preserved.
    #[test]
    fn prop_priority_ordering_preserved(
        entries in proptest::collection::vec(
            (workspace_strategy(), priority_strategy()),
            1..20,
        ),
    ) {
        // Build queue from entries
        let mut queue = PureQueue::new();
        for (workspace, priority) in &entries {
            // Skip duplicate workspaces
            if queue.get(workspace).is_some() {
                continue;
            }
            queue = queue.add(workspace, *priority, None).expect("queue operation should succeed");
        }

        // Claim entries one by one and verify ordering
        let mut claimed_order: Vec<(String, i32)> = Vec::new();
        let mut current_queue = queue.clone();
        let agent = "test-agent";

        // Claim all entries
        loop {
            match current_queue.claim_next(agent) {
                Ok((new_queue, workspace)) => {
                    if let Some(entry) = new_queue.get(&workspace) {
                        claimed_order.push((workspace.clone(), entry.priority));
                    }
                    // Release and transition to terminal to allow next claim
                    let released = new_queue.release(&workspace).expect("queue operation should succeed");
                    current_queue = released.transition_status(&workspace, QueueStatus::Merged).expect("queue operation should succeed");
                }
                Err(PureQueueError::NoPendingEntries) => break,
                Err(_) => break,
            }
        }

        // Verify priority ordering: each claimed entry should have priority >= previous
        let mut prev_priority: Option<i32> = None;
        for (workspace, priority) in &claimed_order {
            if let Some(prev) = prev_priority {
                prop_assert!(
                    *priority >= prev,
                    "Priority ordering violated: {:?} has priority {} but previous had {}",
                    workspace,
                    priority,
                    prev
                );
            }
            prev_priority = Some(*priority);
        }
    }

    /// Property: Adding a higher priority entry doesn't affect already-claimed entries.
    #[test]
    fn prop_priority_respects_claimed_entries(
        claimed_workspace in workspace_strategy(),
        higher_priority in priority_strategy(),
        lower_priority in priority_strategy(),
    ) {
        // Skip if higher_priority >= lower_priority
        if higher_priority >= lower_priority {
            return Ok(());
        }

        // Create queue with claimed entry
        let queue = PureQueue::new();
        let queue = queue.add(&claimed_workspace, lower_priority, None);
        prop_assert!(queue.is_ok());
        let queue = queue.expect("queue operation should succeed");

        // Claim the entry
        let claim_result = queue.claim_next("agent1");
        prop_assert!(claim_result.is_ok());
        let (queue_after_claim, ws_claimed) = claim_result.expect("queue operation should succeed");
        prop_assert_eq!(ws_claimed, claimed_workspace.clone());

        // Add a higher priority entry (different workspace)
        let higher_ws = format!("{}-higher", claimed_workspace);
        let queue_with_higher = queue_after_claim.add(&higher_ws, higher_priority, None);
        prop_assert!(queue_with_higher.is_ok());
        let queue_with_higher = queue_with_higher.expect("queue operation should succeed");

        // Verify claimed entry is still claimed
        let claimed_entry = queue_with_higher.get(&claimed_workspace);
        prop_assert!(claimed_entry.is_some());
        let claimed_entry = claimed_entry.expect("queue operation should succeed");
        prop_assert!(claimed_entry.is_claimed());

        // The higher priority entry should be pending, not affecting the claimed one
        let higher_entry = queue_with_higher.get(&higher_ws);
        prop_assert!(higher_entry.is_some());
        let higher_entry = higher_entry.expect("queue operation should succeed");
        prop_assert!(higher_entry.is_claimable());
    }

    /// Property: Priority ordering is stable across multiple operations.
    #[test]
    fn prop_priority_ordering_stable(
        batch1 in proptest::collection::vec((workspace_strategy(), priority_strategy()), 1..5),
        batch2 in proptest::collection::vec((workspace_strategy(), priority_strategy()), 1..5),
    ) {
        // Build queue with batch1
        let mut queue = PureQueue::new();
        for (workspace, priority) in &batch1 {
            if queue.get(workspace).is_some() {
                continue;
            }
            queue = queue.add(workspace, *priority, None).expect("queue operation should succeed");
        }

        // Add batch2
        for (workspace, priority) in &batch2 {
            if queue.get(workspace).is_some() {
                continue;
            }
            queue = queue.add(workspace, *priority, None).expect("queue operation should succeed");
        }

        // Verify queue is consistent
        prop_assert!(queue.is_consistent());

        // Get pending entries in order
        let pending = queue.pending_in_order();

        // Verify ordering by (priority, added_at)
        for pair in pending.windows(2) {
            let first = &pair[0];
            let second = &pair[1];
            prop_assert!(
                (first.priority, first.added_at) <= (second.priority, second.added_at),
                "Priority ordering violated: {:?} should come before {:?}",
                first.workspace,
                second.workspace
            );
        }
    }
}

// =============================================================================
// PROPERTY 3: STATE MACHINE TRANSITIONS VALID
// =============================================================================

proptest! {
    /// Property: All transitions from non-terminal states must be validated.
    /// Invalid transitions must return an error.
    ///
    /// This tests the state machine contract defined in queue_status.rs.
    /// The test validates that the state machine behaves correctly for all
    /// combinations of from/to states.
    #[test]
    fn prop_valid_transitions_succeed(from in non_terminal_status_strategy(), to in queue_status_strategy()) {
        // The state machine validates transitions synchronously
        let result = from.validate_transition(to);

        // Check if this is a valid transition according to the state machine rules
        let is_valid = from.can_transition_to(to);

        // The result should match what can_transition_to reports
        if is_valid {
            prop_assert!(result.is_ok(), "Valid transition {:?} -> {:?} should succeed", from, to);
        } else {
            prop_assert!(result.is_err(), "Invalid transition {:?} -> {:?} should fail", from, to);
        }
    }

    /// Property: Terminal states cannot transition to any other state (except themselves).
    #[test]
    fn prop_terminal_states_no_exit(terminal in terminal_status_strategy(), target in queue_status_strategy()) {
        let result = terminal.validate_transition(target);

        if terminal == target {
            // Same state is always valid (idempotent)
            prop_assert!(result.is_ok(), "Same-state transition for {:?} should succeed", terminal);
        } else {
            // All other transitions from terminal states must fail
            prop_assert!(result.is_err(), "Terminal state {:?} cannot transition to {:?}", terminal, target);
        }
    }

    /// Property: TransitionError contains both from and to states.
    #[test]
    fn prop_transition_error_contains_states(from in terminal_status_strategy(), to in non_terminal_status_strategy()) {
        let result = from.validate_transition(to);

        if let Err(TransitionError { from: err_from, to: err_to }) = result {
            prop_assert_eq!(err_from, from);
            prop_assert_eq!(err_to, to);
        } else {
            prop_assert!(false, "Transition from {:?} to {:?} should fail with TransitionError", from, to);
        }
    }

    /// Property: State machine is deterministic - same input always produces same output.
    #[test]
    fn prop_state_machine_deterministic(from in queue_status_strategy(), to in queue_status_strategy()) {
        let result1 = from.validate_transition(to);
        let result2 = from.validate_transition(to);

        match (result1, result2) {
            (Ok(()), Ok(())) => {}
            (Err(e1), Err(e2)) => {
                prop_assert_eq!(e1.from, e2.from);
                prop_assert_eq!(e1.to, e2.to);
            }
            _ => prop_assert!(false, "State machine should be deterministic"),
        }
    }

    /// Property: All valid transitions can be reversed or lead to terminal state.
    /// This ensures no "deadlock" states where you can't proceed or fail.
    #[test]
    fn prop_no_deadlock_states(status in non_terminal_status_strategy()) {
        // Every non-terminal state must have at least one valid outgoing transition
        // to either: next state, failed state, or cancelled

        let all_statuses = QueueStatus::all();
        let valid_transitions: Vec<_> = all_statuses
            .iter()
            .filter(|&&target| status.validate_transition(target).is_ok())
            .copied()
            .collect();

        // Must have at least the identity transition
        prop_assert!(!valid_transitions.is_empty(), "State {:?} has no valid transitions", status);

        // Non-terminal states (except FailedRetryable to Pending) should have path to terminal
        let has_path_to_terminal = valid_transitions.iter().any(|&t| t.is_terminal());
        let is_failed_retryable = status == QueueStatus::FailedRetryable;

        prop_assert!(
            has_path_to_terminal || is_failed_retryable,
            "State {:?} must have path to terminal state",
            status
        );
    }
}

// =============================================================================
// PROPERTY 4: TERMINAL STATES IMMUTABLE
// =============================================================================

proptest! {
    /// Property: Merged entries cannot be modified or transitioned.
    #[test]
    fn prop_merged_immutable(target in queue_status_strategy()) {
        let result = QueueStatus::Merged.validate_transition(target);

        if target == QueueStatus::Merged {
            prop_assert!(result.is_ok(), "Same-state transition for Merged should succeed");
        } else {
            prop_assert!(result.is_err(), "Merged state cannot transition to {:?}", target);
        }
    }

    /// Property: FailedTerminal entries cannot be modified or transitioned.
    #[test]
    fn prop_failed_terminal_immutable(target in queue_status_strategy()) {
        let result = QueueStatus::FailedTerminal.validate_transition(target);

        if target == QueueStatus::FailedTerminal {
            prop_assert!(result.is_ok(), "Same-state transition for FailedTerminal should succeed");
        } else {
            prop_assert!(result.is_err(), "FailedTerminal state cannot transition to {:?}", target);
        }
    }

    /// Property: Cancelled entries cannot be modified or transitioned.
    #[test]
    fn prop_cancelled_immutable(target in queue_status_strategy()) {
        let result = QueueStatus::Cancelled.validate_transition(target);

        if target == QueueStatus::Cancelled {
            prop_assert!(result.is_ok(), "Same-state transition for Cancelled should succeed");
        } else {
            prop_assert!(result.is_err(), "Cancelled state cannot transition to {:?}", target);
        }
    }

    /// Property: is_terminal() matches the actual immutability behavior.
    #[test]
    fn prop_is_terminal_matches_behavior(status in queue_status_strategy()) {
        let is_terminal = status.is_terminal();
        let all_statuses = QueueStatus::all();

        let can_transition_out = all_statuses
            .iter()
            .filter(|&&target| target != status)
            .any(|&target| status.validate_transition(target).is_ok());

        prop_assert_eq!(
            is_terminal,
            !can_transition_out,
            "is_terminal()={:?} for {:?} but can_transition_out={:?}",
            is_terminal,
            status,
            can_transition_out
        );
    }
}

// =============================================================================
// ADDITIONAL INVARIANT PROPERTIES
// =============================================================================

proptest! {
    /// Property: All status values have valid string representations.
    #[test]
    fn prop_status_roundtrip(status in queue_status_strategy()) {
        let s = status.to_string();
        let parsed: Result<QueueStatus, _> = s.parse::<QueueStatus>();

        prop_assert!(parsed.is_ok(), "Status {:?} string '{}' should parse", status, s);
        if let Ok(parsed_status) = parsed {
            prop_assert_eq!(parsed_status, status);
        }
    }

    /// Property: Status comparison is consistent.
    #[test]
    fn prop_status_equality_consistent(s1 in queue_status_strategy(), s2 in queue_status_strategy()) {
        // Equality is reflexive
        prop_assert_eq!(s1, s1);

        // Equality is symmetric
        prop_assert_eq!(s1 == s2, s2 == s1);

        // Same status should have same is_terminal
        if s1 == s2 {
            prop_assert_eq!(s1.is_terminal(), s2.is_terminal());
        }
    }

    /// Property: Transition from any state to itself is always valid.
    #[test]
    fn prop_idempotent_transitions(status in queue_status_strategy()) {
        let result = status.validate_transition(status);
        prop_assert!(result.is_ok(), "Self-transition for {:?} should always be valid", status);
    }
}

/// Property: The queue status domain is finite and known.
#[test]
fn prop_all_statuses_are_known() {
    let all = QueueStatus::all();
    let unique: Vec<_> = all.iter().collect();

    // All statuses should be unique
    assert_eq!(all.len(), unique.len(), "All statuses should be unique");

    // We expect exactly 10 statuses
    assert_eq!(all.len(), 10, "Expected exactly 10 queue statuses");

    // Terminal statuses should be exactly 3
    let terminal_count = all.iter().filter(|s| s.is_terminal()).count();
    assert_eq!(terminal_count, 3, "Expected exactly 3 terminal statuses");
}

// =============================================================================
// GREEN PHASE TESTS
// These tests verify the PureQueue implementation satisfies all invariants.
// =============================================================================

proptest! {
    /// Property: Queue operations are atomic - partial failures leave queue unchanged.
    #[test]
    fn prop_queue_operations_atomic(
        initial_entries in proptest::collection::vec((workspace_strategy(), priority_strategy()), 1..10),
        operation_type in 0u8..10,
    ) {
        // Build initial queue
        let mut queue = PureQueue::new();
        for (workspace, priority) in &initial_entries {
            if queue.get(workspace).is_some() {
                continue;
            }
            queue = queue.add(workspace, *priority, None).expect("queue operation should succeed");
        }

        // Save original state for later verification
        let original_len = queue.len();
        let original_consistent = queue.is_consistent();
        prop_assert!(original_consistent, "Original queue should always be consistent");

        // Try various operations - they should either succeed or leave queue unchanged
        // We use clone() where needed since PureQueue is immutable by design
        let queue_ref = queue.clone();
        let result = match operation_type {
            0 => {
                // Try to add a duplicate workspace (should fail atomically)
                if let Some((ws, _)) = initial_entries.first() {
                    queue_ref.clone().add(ws, 5, None)
                } else {
                    Ok(queue_ref)
                }
            }
            1 => {
                // Try to claim with an agent
                queue_ref.clone().claim_next("test-agent").map(|(q, _)| q)
            }
            2..=9 => {
                // No-op: queue stays the same
                Ok(queue_ref)
            }
            _ => Ok(queue_ref),
        };

        match result {
            Ok(new_queue) => {
                // Successful operation - queue should be consistent
                prop_assert!(new_queue.is_consistent(), "Queue must be consistent after successful operation");
            }
            Err(_) => {
                // Failed operation - original queue should be unchanged
                // Since PureQueue is immutable, the original queue is unchanged by design
                prop_assert_eq!(queue.len(), original_len, "Queue length should be unchanged after failed operation");
                prop_assert!(queue.is_consistent(), "Queue must remain consistent after failed operation");
            }
        }
    }

    /// Property: Queue state is always consistent after any sequence of operations.
    #[test]
    fn prop_queue_state_consistent(
        operations in proptest::collection::vec(0u8..10, 1..20),
    ) {
        let mut queue = PureQueue::new();
        let mut workspace_counter = 0u32;
        let mut claimed_workspaces: Vec<String> = Vec::new();

        for op in operations {
            // Clone queue for operations that might fail
            let current_queue = queue.clone();
            let result: Result<PureQueue, _> = match op {
                0..=2 => {
                    // Add a new entry
                    workspace_counter += 1;
                    let workspace = format!("ws-{}", workspace_counter);
                    current_queue.add(&workspace, (op as i32) % 11, None)
                }
                3 => {
                    // Claim next pending entry
                    match current_queue.claim_next("agent") {
                        Ok((new_queue, ws)) => {
                            claimed_workspaces.push(ws);
                            Ok(new_queue)
                        }
                        Err(e) => Err(e),
                    }
                }
                4 => {
                    // Release a claimed entry
                    if let Some(ws) = claimed_workspaces.pop() {
                        current_queue.release(&ws).map_err(|e| match e {
                            PureQueueError::NotClaimed(_) => PureQueueError::NotFound(ws),
                            other => other,
                        })
                    } else {
                        Ok(current_queue)
                    }
                }
                5 => {
                    // Transition to terminal
                    if let Some(ws) = claimed_workspaces.last() {
                        current_queue.transition_status(ws, QueueStatus::Merged)
                    } else {
                        Ok(current_queue)
                    }
                }
                _ => Ok(current_queue),
            };

            match result {
                Ok(new_queue) => {
                    queue = new_queue;
                    prop_assert!(queue.is_consistent(), "Queue must be consistent after operation {:?}", op);
                }
                Err(_) => {
                    // Operation failed - original queue is unchanged (immutability)
                    prop_assert!(queue.is_consistent(), "Queue must be consistent even after failed operation");
                }
            }
        }

        // Final state must be consistent
        prop_assert!(queue.is_consistent(), "Final queue state must be consistent");
    }

    /// Property: Deduplication key prevents duplicate work.
    #[test]
    fn prop_dedupe_key_prevents_duplicates(
        workspace1 in workspace_strategy(),
        workspace2 in workspace_strategy(),
        dedupe_key in "[a-zA-Z0-9_-]{5,20}",
    ) {
        // Skip if workspaces are the same
        if workspace1 == workspace2 {
            return Ok(());
        }

        // Create queue with first entry using dedupe key
        let queue = PureQueue::new();
        let queue = queue.add(&workspace1, 5, Some(&dedupe_key));
        prop_assert!(queue.is_ok());
        let queue = queue.expect("queue operation should succeed");

        // Try to add second entry with same dedupe key - should fail
        let result = queue.add(&workspace2, 5, Some(&dedupe_key));
        prop_assert!(
            matches!(result, Err(PureQueueError::DuplicateDedupeKey(_))),
            "Adding entry with duplicate dedupe key should fail"
        );

        // Original queue is unchanged
        prop_assert!(queue.is_consistent());
        prop_assert!(queue.get(&workspace1).is_some());

        // Add with different dedupe key should succeed
        let result = queue.add(&workspace2, 5, Some("different-key"));
        prop_assert!(result.is_ok(), "Adding entry with different dedupe key should succeed");
    }
}
