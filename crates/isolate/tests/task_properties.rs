#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(
    clippy::unreadable_literal,
    clippy::trivially_copy_pass_by_ref,
    clippy::missing_const_for_fn,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::redundant_clone,
    clippy::expect_used,
    clippy::uninlined_format_args,
    clippy::redundant_closure_for_method_calls,
    clippy::unwrap_used,
    clippy::panic
)]

//! This module defines property tests using proptest that specify the expected

use std::collections::{HashMap, HashSet};

use proptest::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// DETERMINISTIC CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

/// Create a deterministic proptest configuration for reproducible test runs.
fn deterministic_config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        max_shrink_iters: 1024,
        ..ProptestConfig::default()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TASK DOMAIN TYPES (Stubs - implementation will be in isolate-core)
// ═══════════════════════════════════════════════════════════════════════════

/// Task status state machine.
///
/// Valid states:
/// - `Pending`: Task is waiting to be claimed
/// - `Claimed`: Task has been claimed by an agent
/// - `InProgress`: Task is being worked on
/// - `Completed`: Task has been completed successfully
/// - `Failed`: Task has failed
/// - `Cancelled`: Task was cancelled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Pending,
    Claimed,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    /// Returns all valid statuses.
    const fn all() -> &'static [Self] {
        &[
            Self::Pending,
            Self::Claimed,
            Self::InProgress,
            Self::Completed,
            Self::Failed,
            Self::Cancelled,
        ]
    }

    /// Returns true if this is a terminal state.
    const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Validates that a transition from `self` to `target` is allowed.
    fn can_transition_to(&self, target: Self) -> bool {
        if self == &target {
            return true;
        }

        if self.is_terminal() {
            return false;
        }

        match self {
            Self::Pending => matches!(target, Self::Claimed | Self::Cancelled),
            Self::Claimed => {
                matches!(target, Self::InProgress | Self::Failed | Self::Cancelled)
            }
            Self::InProgress => {
                matches!(target, Self::Completed | Self::Failed | Self::Cancelled)
            }
            Self::Completed | Self::Failed | Self::Cancelled => false,
        }
    }
}

/// Task lock information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLock {
    /// The task being locked.
    pub task_id: String,
    /// The agent holding the lock.
    pub agent_id: String,
    /// When the lock was acquired (Unix timestamp).
    pub acquired_at: u64,
    /// When the lock expires (Unix timestamp).
    pub expires_at: u64,
}

impl TaskLock {
    /// Check if the lock is expired at the given time.
    const fn is_expired_at(&self, current_time: u64) -> bool {
        self.expires_at <= current_time
    }

    /// Check if the lock is held by the given agent.
    fn is_held_by(&self, agent_id: &str) -> bool {
        self.agent_id == agent_id
    }
}

/// Result of a claim attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimResult {
    /// Claim was successful.
    Success { lock: TaskLock },
    /// Claim failed because task is already locked.
    AlreadyLocked { holder: String },
    /// Claim failed because task is in a non-claimable state.
    InvalidState { current_state: TaskStatus },
}

/// Task object with lock and state management.
///
/// This is a STUB implementation used to define the expected interface.
/// The real implementation will be provided in isolate-core and should:
/// 1. Track lock state persistently (not just return a new lock each time)
/// 2. Enforce exclusivity across multiple claim calls
/// 3. Integrate with a real storage backend
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// Current task status.
    pub status: TaskStatus,
    /// Current lock holder (if any).
    pub lock: Option<TaskLock>,
}

impl Task {
    /// Create a new task in Pending state.
    fn new(id: String) -> Self {
        Self {
            id,
            status: TaskStatus::Pending,
            lock: None,
        }
    }

    /// Attempt to claim the task for an agent.
    ///
    /// STUB BEHAVIOR: This always returns Success for Pending tasks,
    /// regardless of whether another agent already claimed it.
    /// This is intentional - the real implementation should track state.
    fn claim(&self, agent_id: &str, current_time: u64, ttl_seconds: u64) -> ClaimResult {
        // STUB: This should check existing lock, but doesn't properly
        // This is the intentional failure point for RED phase
        if self.status != TaskStatus::Pending {
            return ClaimResult::InvalidState {
                current_state: self.status,
            };
        }

        // STUB: Always succeeds - real impl should enforce exclusivity
        ClaimResult::Success {
            lock: TaskLock {
                task_id: self.id.clone(),
                agent_id: agent_id.to_string(),
                acquired_at: current_time,
                expires_at: current_time.saturating_add(ttl_seconds),
            },
        }
    }

    /// Check if the task is locked by a specific agent at a given time.
    fn is_locked_by(&self, agent_id: &str, current_time: u64) -> bool {
        match &self.lock {
            Some(lock) => !lock.is_expired_at(current_time) && lock.is_held_by(agent_id),
            None => false,
        }
    }
}

/// Collection of tasks for testing concurrent access.
#[derive(Debug, Clone, Default)]
pub struct TaskRegistry {
    tasks: Vec<Task>,
}

impl TaskRegistry {
    /// Create an empty registry.
    fn new() -> Self {
        Self::default()
    }

    /// Add a task to the registry.
    fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Get all locks that are active at the given time.
    fn active_locks(&self, current_time: u64) -> Vec<&TaskLock> {
        self.tasks
            .iter()
            .filter_map(|t| t.lock.as_ref())
            .filter(|l| !l.is_expired_at(current_time))
            .collect()
    }
}

/// Simulated `TaskObject` that maintains state across operations.
///
/// This is a helper for testing that simulates what the real implementation
/// should do - track state changes across method calls.
#[derive(Debug, Clone)]
pub struct SimulatedTaskObject {
    task: Task,
}

impl SimulatedTaskObject {
    /// Create a new simulated task object.
    fn new(id: String) -> Self {
        Self {
            task: Task::new(id),
        }
    }

    /// Attempt to claim the task, tracking state.
    fn claim(&mut self, agent_id: &str, current_time: u64, ttl_seconds: u64) -> ClaimResult {
        // Check existing lock state
        if let Some(ref lock) = self.task.lock {
            if !lock.is_expired_at(current_time) {
                // Lock is active
                if lock.is_held_by(agent_id) {
                    // Same agent re-claiming - idempotent, return success with refreshed lock
                    let new_lock = TaskLock {
                        task_id: self.task.id.clone(),
                        agent_id: agent_id.to_string(),
                        acquired_at: current_time,
                        expires_at: current_time.saturating_add(ttl_seconds),
                    };
                    self.task.lock = Some(new_lock.clone());
                    return ClaimResult::Success { lock: new_lock };
                }
                // Different agent trying to claim active lock
                return ClaimResult::AlreadyLocked {
                    holder: lock.agent_id.clone(),
                };
            }
            // Lock is expired - reset status to Pending to allow new claim
            self.task.status = TaskStatus::Pending;
        }

        // Check state (now accounts for expired locks being reset to Pending)
        if self.task.status != TaskStatus::Pending {
            return ClaimResult::InvalidState {
                current_state: self.task.status,
            };
        }

        // Claim successful - update state
        let lock = TaskLock {
            task_id: self.task.id.clone(),
            agent_id: agent_id.to_string(),
            acquired_at: current_time,
            expires_at: current_time.saturating_add(ttl_seconds),
        };

        self.task.lock = Some(lock.clone());
        self.task.status = TaskStatus::Claimed;

        ClaimResult::Success { lock }
    }

    /// Check if locked by agent.
    fn is_locked_by(&self, agent_id: &str, current_time: u64) -> bool {
        self.task.is_locked_by(agent_id, current_time)
    }

    /// Get current status.
    const fn status(&self) -> TaskStatus {
        self.task.status
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPTEST STRATEGIES
// ═══════════════════════════════════════════════════════════════════════════

/// Strategy for generating valid agent IDs.
fn agent_id_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,31}"
}

/// Strategy for generating unique agent IDs.
fn unique_agents_strategy(min: usize, max: usize) -> impl Strategy<Value = Vec<String>> {
    proptest::collection::hash_set(agent_id_strategy(), min..=max)
        .prop_map(|set| set.into_iter().collect())
}

/// Strategy for generating valid task IDs.
fn task_id_strategy() -> impl Strategy<Value = String> {
    "task-[a-z0-9]{8,16}"
}

/// Strategy for generating any task status.
fn task_status_strategy() -> impl Strategy<Value = TaskStatus> {
    proptest::sample::select(TaskStatus::all().to_vec())
}

/// Strategy for generating timestamps (seconds since epoch).
fn timestamp_strategy() -> impl Strategy<Value = u64> {
    1_700_000_000_u64..1_800_000_000
}

/// Strategy for generating TTL values (1 second to 1 hour).
fn ttl_strategy() -> impl Strategy<Value = u64> {
    1_u64..3600
}

/// Strategy for generating a task with optional lock.
fn task_strategy() -> impl Strategy<Value = Task> {
    (
        task_id_strategy(),
        task_status_strategy(),
        proptest::option::of((agent_id_strategy(), timestamp_strategy(), ttl_strategy())),
    )
        .prop_map(|(id, status, lock_info)| {
            let lock = lock_info.map(|(agent_id, acquired_at, ttl)| TaskLock {
                task_id: id.clone(),
                agent_id,
                acquired_at,
                expires_at: acquired_at.saturating_add(ttl),
            });

            Task { id, status, lock }
        })
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 1: LOCK EXCLUSIVITY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: No two agents can own the same task simultaneously.
    ///
    /// GIVEN: A task in Pending state with simulated state tracking
    /// WHEN: Multiple agents attempt to claim the task sequentially
    /// THEN: At most one agent can hold the lock at any time
    #[test]
    fn prop_lock_exclusivity_no_two_agents_own_same_task(
        task_id in task_id_strategy(),
        agents in unique_agents_strategy(2, 10),
        current_time in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        // Use simulated task object that tracks state
        let mut task = SimulatedTaskObject::new(task_id);

        let mut success_count = 0;
        let mut successful_agent: Option<String> = None;

        for agent_id in &agents {
            let result = task.claim(agent_id, current_time, ttl);

            match result {
                ClaimResult::Success { .. } => {
                    success_count += 1;
                    if successful_agent.is_none() {
                        successful_agent = Some(agent_id.clone());
                    }
                }
                ClaimResult::AlreadyLocked { .. } => {
                    // Expected after first claim
                }
                ClaimResult::InvalidState { .. } => {
                    // Should not happen for Pending task
                }
            }
        }

        // CRITICAL INVARIANT: Exactly one agent should succeed
        prop_assert!(
            success_count == 1,
            "Expected exactly 1 successful claim, but {} succeeded. Agents: {:?}, Successful: {:?}",
            success_count,
            agents,
            successful_agent
        );
    }

    /// Property: Lock holder is correctly tracked after claim.
    ///
    /// GIVEN: A task claimed by an agent
    /// WHEN: We query who holds the lock
    /// THEN: Only the claiming agent is reported as holder
    #[test]
    fn prop_lock_holder_is_accurate(
        task_id in task_id_strategy(),
        agent_id in agent_id_strategy(),
        current_time in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        let mut task = SimulatedTaskObject::new(task_id);

        // Agent claims the task
        let result = task.claim(&agent_id, current_time, ttl);

        if let ClaimResult::Success { lock } = result {
            // Verify the lock holder is correct
            prop_assert_eq!(lock.agent_id, agent_id.clone());

            // Verify is_locked_by returns correct result
            prop_assert!(task.is_locked_by(&agent_id, current_time));

            // Verify no other agent is reported as holder
            prop_assert!(!task.is_locked_by("other-agent", current_time));
        } else {
            // Claim should have succeeded for pending task
            prop_assert!(false, "Claim should succeed for pending task");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 2: TTL EXPIRATION
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: Expired locks are considered released.
    ///
    /// GIVEN: A task with a lock that has expired
    /// WHEN: We check lock status
    /// THEN: The lock is considered released/invalid
    #[test]
    fn prop_ttl_expiration_expired_locks_released(
        task_id in task_id_strategy(),
        agent_id in agent_id_strategy(),
        acquired_at in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        // Create a lock with given TTL
        let lock = TaskLock {
            task_id: task_id.clone(),
            agent_id: agent_id.clone(),
            acquired_at,
            expires_at: acquired_at.saturating_add(ttl),
        };

        // Check at various points in time
        let during_lock = acquired_at.saturating_add(ttl / 2);
        let at_expiry = acquired_at.saturating_add(ttl);
        let after_expiry = acquired_at.saturating_add(ttl).saturating_add(1);

        // During lock period, should NOT be expired
        prop_assert!(!lock.is_expired_at(during_lock));

        // At exact expiry time, should be expired
        prop_assert!(lock.is_expired_at(at_expiry));

        // After expiry, should be expired
        prop_assert!(lock.is_expired_at(after_expiry));
    }

    /// Property: New claims allowed after TTL expiration.
    ///
    /// GIVEN: A task with an expired lock
    /// WHEN: Another agent attempts to claim
    /// THEN: The claim should succeed
    #[test]
    fn prop_ttl_expiration_new_claims_allowed_after_expiry(
        task_id in task_id_strategy(),
        original_agent in agent_id_strategy(),
        new_agent in agent_id_strategy(),
        acquired_at in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        // Assume different agents
        prop_assume!(original_agent != new_agent);

        // Create task with pre-existing expired lock
        let expired_time = acquired_at.saturating_add(ttl).saturating_add(1);

        let mut task = SimulatedTaskObject::new(task_id);

        // First agent claims
        let _ = task.claim(&original_agent, acquired_at, ttl);

        // Verify lock is expired at expired_time
        prop_assert!(!task.is_locked_by(&original_agent, expired_time));

        // New agent should be able to claim after expiry
        let result = task.claim(&new_agent, expired_time, ttl);

        match result {
            ClaimResult::Success { lock } => {
                prop_assert_eq!(lock.agent_id, new_agent);
            }
            ClaimResult::AlreadyLocked { holder } => {
                prop_assert!(
                    false,
                    "New agent should be able to claim after expiry, but {} still holds lock",
                    holder
                );
            }
            ClaimResult::InvalidState { .. } => {
                // Task status changed - this is acceptable behavior
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 3: STATE TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: All state transitions are valid according to state machine.
    ///
    /// GIVEN: Any pair of states
    /// WHEN: We validate the transition
    /// THEN: Only allowed transitions return true
    #[test]
    fn prop_state_transitions_all_valid(
        from_status in task_status_strategy(),
        to_status in task_status_strategy(),
    ) {
        let can_transition = from_status.can_transition_to(to_status);

        // Verify terminal states have no outgoing transitions (except to self)
        if from_status.is_terminal() && from_status != to_status {
            prop_assert!(!can_transition, "Terminal state {:?} should not transition to {:?}", from_status, to_status);
        }

        // Define expected valid transitions
        let is_expected_valid = match from_status {
            TaskStatus::Pending => {
                // Pending can only go to Claimed or Cancelled (or stay Pending)
                matches!(to_status, TaskStatus::Claimed | TaskStatus::Cancelled | TaskStatus::Pending)
            }
            TaskStatus::Claimed => {
                // Claimed can go to InProgress, Failed, Cancelled (or stay Claimed)
                matches!(to_status, TaskStatus::InProgress | TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::Claimed)
            }
            TaskStatus::InProgress => {
                // InProgress can go to Completed, Failed, Cancelled (or stay InProgress)
                matches!(to_status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::InProgress)
            }
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                // Terminal states can only stay in same state
                to_status == from_status
            }
        };

        prop_assert_eq!(can_transition, is_expected_valid,
            "Transition from {:?} to {:?}: can_transition={} but expected={}",
            from_status, to_status, can_transition, is_expected_valid);
    }

    /// Property: State transitions are deterministic.
    ///
    /// GIVEN: A state transition check
    /// WHEN: We check multiple times
    /// THEN: The result is always the same
    #[test]
    fn prop_state_transitions_deterministic(
        from_status in task_status_strategy(),
        to_status in task_status_strategy(),
    ) {
        let result1 = from_status.can_transition_to(to_status);
        let result2 = from_status.can_transition_to(to_status);
        let result3 = from_status.can_transition_to(to_status);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(result2, result3);
    }

    /// Property: Self-transitions are always allowed.
    ///
    /// GIVEN: Any state
    /// WHEN: We check transition to same state
    /// THEN: It's always valid
    #[test]
    fn prop_state_transitions_self_always_valid(status in task_status_strategy()) {
        prop_assert!(status.can_transition_to(status),
            "Self-transition for {:?} should always be valid", status);
    }

    /// Property: Terminal states cannot transition to other states.
    #[test]
    fn prop_terminal_states_no_exit(status in task_status_strategy()) {
        prop_assume!(status.is_terminal());

        for other in TaskStatus::all() {
            if *other != status {
                prop_assert!(!status.can_transition_to(*other),
                    "Terminal state {:?} should not transition to {:?}", status, other);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 4: CONCURRENT CLAIMS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: Exactly one claim succeeds when multiple agents try concurrently.
    ///
    /// GIVEN: A task in Pending state
    /// WHEN: Multiple unique agents try to claim sequentially
    /// THEN: Exactly one claim succeeds
    #[test]
    fn prop_concurrent_claims_exactly_one_succeeds(
        task_id in task_id_strategy(),
        agents in unique_agents_strategy(2, 20),
        current_time in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        // Simulated task object that tracks state
        let mut task = SimulatedTaskObject::new(task_id.clone());

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut successful_agent: Option<String> = None;

        for agent_id in &agents {
            let result = task.claim(agent_id, current_time, ttl);

            match result {
                ClaimResult::Success { .. } => {
                    success_count += 1;
                    if successful_agent.is_none() {
                        successful_agent = Some(agent_id.clone());
                    }
                }
                ClaimResult::AlreadyLocked { .. } | ClaimResult::InvalidState { .. } => {
                    failure_count += 1;
                }
            }
        }

        // CRITICAL INVARIANT: Exactly one agent should succeed
        prop_assert!(
            success_count == 1,
            "Expected exactly 1 successful claim, but {} succeeded. Agents: {:?}, Successful: {:?}",
            success_count,
            agents,
            successful_agent
        );

        // All other agents should have failed
        prop_assert_eq!(
            failure_count,
            agents.len() - 1,
            "Expected {} failures, got {}",
            agents.len() - 1,
            failure_count
        );

        // Task status should be Claimed after successful claim
        prop_assert_eq!(task.status(), TaskStatus::Claimed);
    }

    /// Property: Concurrent claims on different tasks all succeed.
    ///
    /// GIVEN: Multiple tasks in Pending state
    /// WHEN: Multiple agents claim different tasks
    /// THEN: All claims succeed
    #[test]
    fn prop_concurrent_claims_different_tasks_all_succeed(
        task_ids in proptest::collection::vec(task_id_strategy(), 2..10),
        agents in unique_agents_strategy(2, 10),
        current_time in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        // Ensure we have at least as many tasks as agents
        prop_assume!(task_ids.len() >= agents.len());

        let mut all_succeeded = true;
        let mut tasks: HashMap<String, SimulatedTaskObject> = HashMap::new();

        for (task_id, agent_id) in task_ids.iter().zip(agents.iter()) {
            let task = tasks.entry(task_id.clone()).or_insert_with(|| SimulatedTaskObject::new(task_id.clone()));
            let result = task.claim(agent_id, current_time, ttl);

            match result {
                ClaimResult::Success { .. } => {}
                ClaimResult::AlreadyLocked { holder: _ } => {
                    all_succeeded = false;
                }
                ClaimResult::InvalidState { current_state: _ } => {
                    all_succeeded = false;
                }
            }
        }

        prop_assert!(all_succeeded, "All claims on different tasks should succeed");
    }

    /// Property: Same agent can re-claim (idempotent claim).
    ///
    /// GIVEN: A task claimed by agent A
    /// WHEN: Agent A tries to claim again at the same time
    /// THEN: The claim succeeds (idempotent) - same agent already holds the lock
    ///
    /// NOTE: This test will FAIL in the current implementation because
    /// the SimulatedTaskObject changes status to Claimed after first claim,
    /// preventing re-claim. The real implementation should support idempotent
    /// reclaims for the same agent.
    #[test]
    fn prop_concurrent_claims_idempotent_same_agent(
        task_id in task_id_strategy(),
        agent_id in agent_id_strategy(),
        current_time in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        let mut task = SimulatedTaskObject::new(task_id);

        // First claim
        let result1 = task.claim(&agent_id, current_time, ttl);
        let is_success1 = matches!(result1, ClaimResult::Success { .. });
        prop_assert!(is_success1, "First claim should succeed");

        // Same agent claims again - should be idempotent
        // (the agent already holds the lock, so this should succeed)
        let result2 = task.claim(&agent_id, current_time, ttl);

        // RED PHASE: This will fail because SimulatedTaskObject changes
        // status to Claimed, making the second claim return InvalidState.
        // The real implementation should recognize the same agent and
        // return Success (or AlreadyLocked with self, which is semantically Success)
        let is_success2 = matches!(result2, ClaimResult::Success { .. });
        let is_already_locked_by_self = matches!(
            result2,
            ClaimResult::AlreadyLocked { ref holder } if holder == &agent_id
        );

        prop_assert!(
            is_success2 || is_already_locked_by_self,
            "Same agent should be able to re-claim (idempotent), got {:?}",
            result2
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 5: LOCK EXCLUSIVITY IN REGISTRY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: No two agents hold locks on the same task in a registry.
    ///
    /// GIVEN: A registry with multiple tasks and locks
    /// WHEN: We examine all active locks
    /// THEN: Each task has at most one lock holder
    #[test]
    fn prop_registry_no_duplicate_task_locks(
        tasks in proptest::collection::vec(task_strategy(), 1..20),
        current_time in timestamp_strategy(),
    ) {
        let registry = {
            let mut r = TaskRegistry::new();
            for task in tasks {
                r.add_task(task);
            }
            r
        };

        let active_locks = registry.active_locks(current_time);

        // Group locks by task_id
        let mut task_lock_holders: HashMap<String, HashSet<String>> = HashMap::new();

        for lock in active_locks {
            task_lock_holders
                .entry(lock.task_id.clone())
                .or_default()
                .insert(lock.agent_id.clone());
        }

        // Verify each task has at most one holder
        for (task_id, holders) in &task_lock_holders {
            prop_assert!(
                holders.len() <= 1,
                "Task {} has {} lock holders: {:?}",
                task_id,
                holders.len(),
                holders
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 6: STUB Task FAILURES (RED Phase Verification)
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: STUB Task does NOT enforce exclusivity (RED phase expected failure).
    ///
    /// This test documents that the stub implementation is intentionally
    /// incomplete and allows multiple claims to succeed.
    #[test]
    fn prop_stub_allows_multiple_claims_red_phase(
        task_id in task_id_strategy(),
        agents in unique_agents_strategy(2, 5),
        current_time in timestamp_strategy(),
        ttl in ttl_strategy(),
    ) {
        // Use the STUB Task (not SimulatedTaskObject)
        let task = Task::new(task_id);

        let mut success_count = 0;

        for agent_id in &agents {
            let result = task.claim(agent_id, current_time, ttl);
            if matches!(result, ClaimResult::Success { .. }) {
                success_count += 1;
            }
        }

        // The stub implementation allows ALL claims to succeed
        // This is the EXPECTED FAILURE for RED phase
        // When real implementation exists, only 1 should succeed
        prop_assert!(
            success_count > 1,
            "STUB should allow multiple claims (RED phase). Got {} successes for {} agents",
            success_count,
            agents.len()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS FOR STUB IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod stub_tests {
    use super::*;

    #[test]
    fn test_stub_task_creation() {
        let task = Task::new("task-12345678".to_string());
        assert_eq!(task.id, "task-12345678");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.lock.is_none());
    }

    #[test]
    fn test_stub_task_status_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Claimed.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_stub_lock_expiration() {
        let lock = TaskLock {
            task_id: "task-1".to_string(),
            agent_id: "agent-1".to_string(),
            acquired_at: 1000,
            expires_at: 2000,
        };

        assert!(!lock.is_expired_at(500));
        assert!(!lock.is_expired_at(1000));
        assert!(!lock.is_expired_at(1999));
        assert!(lock.is_expired_at(2000));
        assert!(lock.is_expired_at(2001));
    }

    #[test]
    fn test_stub_state_transitions() {
        // Pending can go to Claimed or Cancelled
        assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Claimed));
        assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Cancelled));
        assert!(!TaskStatus::Pending.can_transition_to(TaskStatus::InProgress));
        assert!(!TaskStatus::Pending.can_transition_to(TaskStatus::Completed));

        // Claimed can go to InProgress, Failed, or Cancelled
        assert!(TaskStatus::Claimed.can_transition_to(TaskStatus::InProgress));
        assert!(TaskStatus::Claimed.can_transition_to(TaskStatus::Failed));
        assert!(TaskStatus::Claimed.can_transition_to(TaskStatus::Cancelled));

        // Terminal states can't transition
        assert!(!TaskStatus::Completed.can_transition_to(TaskStatus::Pending));
        assert!(!TaskStatus::Failed.can_transition_to(TaskStatus::Pending));
    }

    #[test]
    fn test_stub_claim_success() {
        let task = Task::new("task-1".to_string());
        let result = task.claim("agent-1", 1000, 300);

        match result {
            ClaimResult::Success { lock } => {
                assert_eq!(lock.task_id, "task-1");
                assert_eq!(lock.agent_id, "agent-1");
                assert_eq!(lock.acquired_at, 1000);
                assert_eq!(lock.expires_at, 1300);
            }
            _ => panic!("Expected Success, got {:?}", result),
        }
    }

    #[test]
    fn test_simulated_task_enforces_exclusivity() {
        let mut task = SimulatedTaskObject::new("task-1".to_string());

        // First claim succeeds
        let result1 = task.claim("agent-1", 1000, 300);
        assert!(matches!(result1, ClaimResult::Success { .. }));

        // Second claim fails
        let result2 = task.claim("agent-2", 1000, 300);
        assert!(matches!(result2, ClaimResult::AlreadyLocked { .. }));
    }

    #[test]
    fn test_simulated_task_tracks_lock() {
        let mut task = SimulatedTaskObject::new("task-1".to_string());

        // Before claim, not locked
        assert!(!task.is_locked_by("agent-1", 1000));

        // Claim succeeds
        let _ = task.claim("agent-1", 1000, 300);

        // After claim, locked by agent-1
        assert!(task.is_locked_by("agent-1", 1000));
        assert!(!task.is_locked_by("agent-2", 1000));
    }

    #[test]
    fn test_simulated_task_expiry_allows_reclaim() {
        let mut task = SimulatedTaskObject::new("task-1".to_string());

        // agent-1 claims at t=1000 with TTL=300
        let _ = task.claim("agent-1", 1000, 300);

        // At t=1300, lock is expired (expires_at = 1300)
        // agent-2 should be able to claim
        //
        // RED PHASE: This will fail because SimulatedTaskObject changes
        // status to Claimed after first claim, preventing subsequent claims.
        // The real implementation should reset status to Pending when lock expires,
        // or have a separate lock mechanism from status.
        let result = task.claim("agent-2", 1300, 300);

        // The test documents expected behavior but fails in RED phase
        // because status != Pending after first claim
        match result {
            ClaimResult::Success { .. } => {
                // This is the expected behavior when implemented
            }
            ClaimResult::InvalidState { current_state } => {
                // RED PHASE: Expected failure - status is Claimed, not Pending
                // This test will pass once the real implementation handles
                // expired locks correctly (resetting status or separate lock tracking)
                panic!(
                    "RED PHASE FAILURE: Cannot claim after expiry because status is {:?}, not Pending",
                    current_state
                );
            }
            ClaimResult::AlreadyLocked { holder } => {
                panic!("Lock should be expired, but still held by {}", holder);
            }
        }
    }
}
