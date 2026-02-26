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
    clippy::expect_used
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
// AGENT DOMAIN TYPES (Stubs - implementation will be in isolate-core)
// ═══════════════════════════════════════════════════════════════════════════

/// Agent status state machine.
///
/// Valid states:
/// - `Pending`: Agent is registered but not active
/// - `Active`: Agent is actively working
/// - `Idle`: Agent is registered but not currently working
/// - `Stopped`: Agent has been stopped
/// - `Failed`: Agent encountered an error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentStatus {
    Pending,
    Active,
    Idle,
    Stopped,
    Failed,
}

impl AgentStatus {
    /// Returns all valid statuses.
    const fn all() -> &'static [Self] {
        &[
            Self::Pending,
            Self::Active,
            Self::Idle,
            Self::Stopped,
            Self::Failed,
        ]
    }

    /// Returns true if this is a terminal state.
    const fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
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
            Self::Pending => matches!(target, Self::Active | Self::Stopped),
            Self::Active => matches!(target, Self::Idle | Self::Stopped | Self::Failed),
            Self::Idle => matches!(target, Self::Active | Self::Stopped),
            Self::Stopped | Self::Failed => false,
        }
    }
}

/// Agent session binding information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBinding {
    /// The session the agent is bound to.
    pub session_id: String,
    /// When the binding was created (Unix timestamp).
    pub bound_at: u64,
    /// When the binding expires (Unix timestamp).
    pub expires_at: u64,
}

impl SessionBinding {
    /// Check if the binding is expired at the given time.
    const fn is_expired_at(&self, current_time: u64) -> bool {
        self.expires_at <= current_time
    }

    /// Check if the binding is for the given session.
    fn is_for_session(&self, session_id: &str) -> bool {
        self.session_id == session_id
    }
}

/// Agent object with heartbeat and session management.
///
/// This is a STUB implementation used to define the expected interface.
/// The real implementation will be provided in isolate-core and should:
/// 1. Track agent state persistently
/// 2. Enforce ID uniqueness across all agents
/// 3. Properly detect stale agents based on heartbeat timing
/// 4. Enforce session binding exclusivity
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent identifier.
    pub id: String,
    /// Current agent status.
    pub status: AgentStatus,
    /// Last heartbeat timestamp (Unix timestamp).
    pub last_seen: u64,
    /// When the agent was registered (Unix timestamp).
    pub registered_at: u64,
    /// Current session binding (if any).
    pub session_binding: Option<SessionBinding>,
    /// Number of heartbeats sent.
    pub heartbeat_count: u64,
}

impl Agent {
    /// Create a new agent in Pending state.
    fn new(id: String, registered_at: u64) -> Self {
        Self {
            id,
            status: AgentStatus::Pending,
            last_seen: registered_at,
            registered_at,
            session_binding: None,
            heartbeat_count: 0,
        }
    }

    /// Send a heartbeat, updating `last_seen`.
    fn heartbeat(&mut self, current_time: u64) {
        self.last_seen = current_time;
        self.heartbeat_count = self.heartbeat_count.saturating_add(1);
    }

    /// Check if the agent is stale at the given time.
    fn is_stale_at(&self, current_time: u64, timeout_secs: u64) -> bool {
        self.last_seen.saturating_add(timeout_secs) <= current_time
    }

    /// Bind the agent to a session.
    fn bind_to_session(&mut self, session_id: &str, current_time: u64, ttl_secs: u64) {
        self.session_binding = Some(SessionBinding {
            session_id: session_id.to_string(),
            bound_at: current_time,
            expires_at: current_time.saturating_add(ttl_secs),
        });
    }

    /// Release the session binding.
    fn release_session(&mut self) {
        self.session_binding = None;
    }

    /// Check if the agent is bound to a specific session.
    fn is_bound_to(&self, session_id: &str, current_time: u64) -> bool {
        match &self.session_binding {
            Some(binding) => {
                !binding.is_expired_at(current_time) && binding.is_for_session(session_id)
            }
            None => false,
        }
    }
}

/// Collection of agents for testing.
#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Agent>,
}

impl AgentRegistry {
    /// Create an empty registry.
    fn new() -> Self {
        Self::default()
    }

    /// Register an agent (enforces uniqueness).
    fn register(&mut self, agent: Agent) -> Result<(), AgentError> {
        if self.agents.contains_key(&agent.id) {
            return Err(AgentError::DuplicateId(agent.id.clone()));
        }
        self.agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    /// Check ID uniqueness.
    #[allow(dead_code)]
    fn check_id_uniqueness(&self, id: &str) -> bool {
        !self.agents.contains_key(id)
    }

    /// Get all active agents (not stale) at the given time.
    fn active_agents(&self, current_time: u64, timeout_secs: u64) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| !a.is_stale_at(current_time, timeout_secs))
            .collect()
    }

    /// Get all stale agents at the given time.
    fn stale_agents(&self, current_time: u64, timeout_secs: u64) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| a.is_stale_at(current_time, timeout_secs))
            .collect()
    }

    /// Find agents bound to a specific session.
    /// Note: Kept for potential future use in additional property tests.
    #[allow(dead_code)]
    fn agents_bound_to_session(&self, session_id: &str, current_time: u64) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| a.is_bound_to(session_id, current_time))
            .collect()
    }

    /// Bind an agent to a session with exclusivity enforcement.
    /// Returns error if session already has an active binding.
    #[allow(dead_code)]
    fn bind_agent_to_session(
        &mut self,
        agent_id: &str,
        session_id: &str,
        current_time: u64,
        ttl: u64,
    ) -> Result<(), AgentError> {
        // Check if session is already bound to another agent
        let session_already_bound = self
            .agents
            .values()
            .any(|a| a.id != agent_id && a.is_bound_to(session_id, current_time));

        if session_already_bound {
            return Err(AgentError::SessionAlreadyBound(session_id.to_string()));
        }

        // Bind the agent
        self.agents.get_mut(agent_id).map_or_else(
            || Err(AgentError::NotFound(agent_id.to_string())),
            |agent| {
                agent.bind_to_session(session_id, current_time, ttl);
                Ok(())
            },
        )
    }
}

/// Error type for agent operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// Agent ID already exists.
    DuplicateId(String),
    /// Agent not found.
    NotFound(String),
    /// Session already bound to another agent.
    SessionAlreadyBound(String),
    /// Invalid state transition.
    InvalidTransition { from: AgentStatus, to: AgentStatus },
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPTEST STRATEGIES
// ═══════════════════════════════════════════════════════════════════════════

/// Strategy for generating valid agent IDs.
fn agent_id_strategy() -> impl Strategy<Value = String> {
    "agent-[a-z0-9]{8,16}"
}

/// Strategy for generating unique agent IDs.
fn unique_agent_ids_strategy(min: usize, max: usize) -> impl Strategy<Value = Vec<String>> {
    proptest::collection::hash_set(agent_id_strategy(), min..=max)
        .prop_map(|set| set.into_iter().collect())
}

/// Strategy for generating valid session IDs.
fn session_id_strategy() -> impl Strategy<Value = String> {
    "session-[a-z0-9]{8,16}"
}

/// Strategy for generating any agent status.
fn agent_status_strategy() -> impl Strategy<Value = AgentStatus> {
    proptest::sample::select(AgentStatus::all().to_vec())
}

/// Strategy for generating timestamps (seconds since epoch).
fn timestamp_strategy() -> impl Strategy<Value = u64> {
    1_700_000_000_u64..1_800_000_000
}

/// Strategy for generating timeout values (1 second to 1 hour).
fn timeout_strategy() -> impl Strategy<Value = u64> {
    1_u64..3600
}

/// Strategy for generating an agent.
/// Note: Kept for potential future use in additional property tests.
#[allow(dead_code)]
fn agent_strategy() -> impl Strategy<Value = Agent> {
    (agent_id_strategy(), timestamp_strategy())
        .prop_map(|(id, registered_at)| Agent::new(id, registered_at))
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 1: ID UNIQUENESS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: All agent IDs in a registry must be unique.
    ///
    /// GIVEN: A registry with multiple agents
    /// WHEN: We check all agent IDs
    /// THEN: All IDs are unique (no duplicates)
    #[test]
    fn prop_agent_ids_are_unique(
        agent_ids in unique_agent_ids_strategy(2, 20),
        registered_at in timestamp_strategy(),
    ) {
        let mut registry = AgentRegistry::new();

        // Register all agents
        for id in agent_ids {
            let agent = Agent::new(id, registered_at);
            let result = registry.register(agent);
            prop_assert!(result.is_ok(), "Registration should succeed");
        }

        // Check that all IDs are unique
        let all_ids: Vec<&String> = registry.agents.keys().collect();
        let unique_ids: HashSet<&String> = all_ids.iter().copied().collect();

        prop_assert_eq!(
            all_ids.len(),
            unique_ids.len(),
            "All agent IDs should be unique"
        );
    }

    /// Property: Registering a duplicate ID should fail.
    ///
    /// GIVEN: An agent with ID "agent-test" is registered
    /// WHEN: We attempt to register another agent with ID "agent-test"
    /// THEN: The operation should fail with DuplicateId error
    #[test]
    fn prop_duplicate_registration_fails(
        agent_id in agent_id_strategy(),
        registered_at in timestamp_strategy(),
    ) {
        let mut registry = AgentRegistry::new();

        // Register first agent
        let agent1 = Agent::new(agent_id.clone(), registered_at);
        let result1 = registry.register(agent1);
        prop_assert!(result1.is_ok(), "First registration should succeed");

        // Attempt to register duplicate
        let agent2 = Agent::new(agent_id.clone(), registered_at);
        let result2 = registry.register(agent2);

        prop_assert!(
            result2.is_err(),
            "Duplicate registration for ID '{}' should fail",
            agent_id
        );

        // Verify the error type
        if let Err(e) = result2 {
            prop_assert!(
                matches!(e, AgentError::DuplicateId(_)),
                "Error should be DuplicateId, got {:?}",
                e
            );
        }
    }

    /// Property: ID uniqueness is maintained after multiple operations.
    ///
    /// GIVEN: A registry with unique agents
    /// WHEN: We perform various operations (heartbeats, status changes)
    /// THEN: ID uniqueness is preserved
    #[test]
    fn prop_id_uniqueness_preserved_after_operations(
        agent_ids in unique_agent_ids_strategy(3, 10),
        registered_at in timestamp_strategy(),
        current_time in timestamp_strategy(),
    ) {
        // Ensure current_time is after registered_at
        prop_assume!(current_time >= registered_at);

        let mut registry = AgentRegistry::new();

        // Register all agents
        for id in &agent_ids {
            let agent = Agent::new(id.clone(), registered_at);
            let _ = registry.register(agent);
        }

        // Perform heartbeats on all agents
        for id in &agent_ids {
            if let Some(agent) = registry.agents.get_mut(id) {
                agent.heartbeat(current_time);
            }
        }

        // Verify uniqueness is still maintained
        let all_ids: HashSet<&String> = registry.agents.keys().collect();
        prop_assert_eq!(
            all_ids.len(),
            agent_ids.len(),
            "ID uniqueness should be preserved after heartbeats"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 2: HEARTBEAT TIMING
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: Heartbeat updates last_seen timestamp.
    ///
    /// GIVEN: An agent registered at time T1
    /// WHEN: The agent sends a heartbeat at time T2 > T1
    /// THEN: The agent's last_seen is updated to T2
    #[test]
    fn prop_heartbeat_updates_last_seen(
        agent_id in agent_id_strategy(),
        registered_at in timestamp_strategy(),
        heartbeat_time in timestamp_strategy(),
    ) {
        // Ensure heartbeat_time is after registered_at
        prop_assume!(heartbeat_time > registered_at);

        let mut agent = Agent::new(agent_id, registered_at);

        // Verify initial last_seen
        prop_assert_eq!(agent.last_seen, registered_at);

        // Send heartbeat
        agent.heartbeat(heartbeat_time);

        // Verify last_seen updated
        prop_assert_eq!(
            agent.last_seen, heartbeat_time,
            "Heartbeat should update last_seen to {}",
            heartbeat_time
        );
    }

    /// Property: Heartbeat count increments with each heartbeat.
    ///
    /// GIVEN: An agent with heartbeat_count N
    /// WHEN: The agent sends a heartbeat
    /// THEN: The heartbeat_count becomes N + 1
    #[test]
    fn prop_heartbeat_increments_count(
        agent_id in agent_id_strategy(),
        registered_at in timestamp_strategy(),
        heartbeat_times in proptest::collection::vec(timestamp_strategy(), 1..10),
    ) {
        let mut agent = Agent::new(agent_id, registered_at);
        let initial_count = agent.heartbeat_count;

        let mut expected_count = initial_count;
        for time in &heartbeat_times {
            agent.heartbeat(*time);
            expected_count = expected_count.saturating_add(1);
        }

        prop_assert_eq!(
            agent.heartbeat_count, expected_count,
            "Heartbeat count should increment with each heartbeat"
        );
    }

    /// Property: Heartbeats are monotonic in time effect.
    ///
    /// GIVEN: An agent sends heartbeats at times T1, T2, T3
    /// WHERE: T1 < T2 < T3
    /// WHEN: We check last_seen after each heartbeat
    /// THEN: last_seen is always the most recent heartbeat time
    #[test]
    fn prop_heartbeat_monotonic_effect(
        agent_id in agent_id_strategy(),
        t1 in timestamp_strategy(),
        t2_delta in 1_u64..1000,
        t3_delta in 1_u64..1000,
    ) {
        let t2 = t1.saturating_add(t2_delta);
        let t3 = t2.saturating_add(t3_delta);

        let mut agent = Agent::new(agent_id, t1);

        // First heartbeat
        agent.heartbeat(t2);
        prop_assert_eq!(agent.last_seen, t2);

        // Second heartbeat
        agent.heartbeat(t3);
        prop_assert_eq!(agent.last_seen, t3);
    }

    /// Property: Agent with recent heartbeat is not stale.
    ///
    /// GIVEN: An agent sends heartbeat at time T
    /// AND: Timeout is X seconds
    /// WHEN: We check staleness at time T + (X/2)
    /// THEN: The agent is NOT stale
    #[test]
    fn prop_recent_heartbeat_not_stale(
        agent_id in agent_id_strategy(),
        heartbeat_time in timestamp_strategy(),
        timeout in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, heartbeat_time);
        agent.heartbeat(heartbeat_time);

        // Check at time halfway through timeout
        let check_time = heartbeat_time.saturating_add(timeout / 2);
        prop_assert!(
            !agent.is_stale_at(check_time, timeout),
            "Agent with recent heartbeat should not be stale"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 3: STALE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: Agent becomes stale after timeout.
    ///
    /// GIVEN: An agent sends heartbeat at time T
    /// AND: Timeout is X seconds
    /// WHEN: We check staleness at time T + X + 1
    /// THEN: The agent IS stale
    #[test]
    fn prop_agent_stale_after_timeout(
        agent_id in agent_id_strategy(),
        heartbeat_time in timestamp_strategy(),
        timeout in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, heartbeat_time);
        agent.heartbeat(heartbeat_time);

        // Check at time just after timeout
        let stale_time = heartbeat_time.saturating_add(timeout).saturating_add(1);
        prop_assert!(
            agent.is_stale_at(stale_time, timeout),
            "Agent should be stale after timeout period"
        );
    }

    /// Property: Agent is stale exactly at timeout boundary.
    ///
    /// GIVEN: An agent sends heartbeat at time T
    /// AND: Timeout is X seconds
    /// WHEN: We check staleness at time T + X
    /// THEN: The agent IS stale (boundary condition)
    ///
    /// GREEN PHASE: The implementation correctly uses <= for staleness.
    #[test]
    fn prop_agent_stale_at_boundary(
        agent_id in agent_id_strategy(),
        heartbeat_time in timestamp_strategy(),
        timeout in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, heartbeat_time);
        agent.heartbeat(heartbeat_time);

        // Check exactly at timeout boundary
        let boundary_time = heartbeat_time.saturating_add(timeout);

        // GREEN PHASE: The implementation uses <= (expired at boundary).
        prop_assert!(
            agent.is_stale_at(boundary_time, timeout),
            "Agent should be stale at exact timeout boundary"
        );
    }

    /// Property: Stale agent can be revived by heartbeat.
    ///
    /// GIVEN: An agent is stale at time T
    /// WHEN: The agent sends a heartbeat at time T
    /// THEN: The agent is no longer stale
    #[test]
    fn prop_stale_agent_revived_by_heartbeat(
        agent_id in agent_id_strategy(),
        registered_at in timestamp_strategy(),
        timeout in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, registered_at);

        // Make agent stale by advancing time
        let stale_time = registered_at.saturating_add(timeout).saturating_add(1);
        prop_assert!(agent.is_stale_at(stale_time, timeout));

        // Send heartbeat to revive
        agent.heartbeat(stale_time);
        prop_assert!(
            !agent.is_stale_at(stale_time, timeout),
            "Agent should not be stale immediately after heartbeat"
        );
    }

    /// Property: Registry correctly filters stale agents.
    ///
    /// GIVEN: A registry with some stale and some active agents
    /// WHEN: We query active agents
    /// THEN: Only non-stale agents are returned
    #[test]
    fn prop_registry_filters_stale_agents(
        agent_ids in unique_agent_ids_strategy(1, 10),
        base_time in timestamp_strategy(),
        offsets in proptest::collection::vec(0_u64..1000, 1..10),
        timeout in timeout_strategy(),
        current_time_offset in 0_u64..1000,
    ) {
        let mut registry = AgentRegistry::new();
        let current_time = base_time.saturating_add(current_time_offset);

        for (id, &offset) in agent_ids.iter().zip(offsets.iter()) {
            let registered_at = base_time;
            let last_heartbeat = base_time.saturating_add(offset);
            let mut agent = Agent::new(id.clone(), registered_at);
            agent.heartbeat(last_heartbeat);
            let _ = registry.register(agent);
        }

        // Get active agents
        let active = registry.active_agents(current_time, timeout);

        // Verify all returned agents are indeed active
        for agent in &active {
            prop_assert!(
                !agent.is_stale_at(current_time, timeout),
                "Active agent {} should not be stale",
                agent.id
            );
        }

        // Get stale agents
        let stale = registry.stale_agents(current_time, timeout);

        // Verify all returned agents are indeed stale
        for agent in &stale {
            prop_assert!(
                agent.is_stale_at(current_time, timeout),
                "Stale agent {} should be stale",
                agent.id
            );
        }

        // Verify total = active + stale
        let total_count = registry.agents.len();
        prop_assert_eq!(
            active.len() + stale.len(),
            total_count,
            "Total agents should equal active + stale"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 4: SESSION BINDING
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: Agent can be bound to exactly one session.
    ///
    /// GIVEN: An agent is not bound to any session
    /// WHEN: The agent is bound to session S
    /// THEN: The agent is bound to session S
    /// AND: The agent is not bound to any other session
    #[test]
    fn prop_agent_single_session_binding(
        agent_id in agent_id_strategy(),
        session_id in session_id_strategy(),
        current_time in timestamp_strategy(),
        ttl in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, current_time);

        // Verify no initial binding
        prop_assert!(agent.session_binding.is_none());

        // Bind to session
        agent.bind_to_session(&session_id, current_time, ttl);

        // Verify binding
        prop_assert!(
            agent.is_bound_to(&session_id, current_time),
            "Agent should be bound to session {}",
            session_id
        );

        // Verify not bound to other sessions
        prop_assert!(
            !agent.is_bound_to("other-session", current_time),
            "Agent should not be bound to other sessions"
        );
    }

    /// Property: Session binding expires after TTL.
    ///
    /// GIVEN: An agent is bound to session S with TTL X
    /// WHEN: We check binding at time T + X
    /// THEN: The binding has expired
    #[test]
    fn prop_session_binding_expires(
        agent_id in agent_id_strategy(),
        session_id in session_id_strategy(),
        bound_at in timestamp_strategy(),
        ttl in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, bound_at);
        agent.bind_to_session(&session_id, bound_at, ttl);

        // Check at expiry time
        let expiry_time = bound_at.saturating_add(ttl);
        prop_assert!(
            !agent.is_bound_to(&session_id, expiry_time),
            "Binding should be expired at TTL boundary"
        );

        // Check after expiry
        let after_expiry = bound_at.saturating_add(ttl).saturating_add(1);
        prop_assert!(
            !agent.is_bound_to(&session_id, after_expiry),
            "Binding should be expired after TTL"
        );
    }

    /// Property: Binding can be released explicitly.
    ///
    /// GIVEN: An agent is bound to a session
    /// WHEN: The binding is released
    /// THEN: The agent is no longer bound to any session
    #[test]
    fn prop_binding_can_be_released(
        agent_id in agent_id_strategy(),
        session_id in session_id_strategy(),
        current_time in timestamp_strategy(),
        ttl in timeout_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, current_time);
        agent.bind_to_session(&session_id, current_time, ttl);

        // Verify bound
        prop_assert!(agent.is_bound_to(&session_id, current_time));

        // Release binding
        agent.release_session();

        // Verify released
        prop_assert!(
            !agent.is_bound_to(&session_id, current_time),
            "Agent should not be bound after release"
        );
        prop_assert!(
            agent.session_binding.is_none(),
            "Session binding should be None after release"
        );
    }

    /// Property: Only one agent per session (exclusivity).
    ///
    /// GIVEN: Multiple agents in a registry
    /// WHEN: Some agents are bound to sessions using exclusive binding
    /// THEN: Each session has at most one agent bound
    #[test]
    fn prop_one_agent_per_session_exclusivity(
        agent_ids in unique_agent_ids_strategy(2, 10),
        session_ids in proptest::collection::hash_set(session_id_strategy(), 1..5),
        current_time in timestamp_strategy(),
        ttl in timeout_strategy(),
    ) {
        let mut registry = AgentRegistry::new();

        // Register all agents
        for id in &agent_ids {
            let agent = Agent::new(id.clone(), current_time);
            let _ = registry.register(agent);
        }

        // Bind agents to sessions using exclusive binding (first wins)
        let session_vec: Vec<String> = session_ids.into_iter().collect();
        for (i, id) in agent_ids.iter().enumerate() {
            if let Some(session_id) = session_vec.get(i % session_vec.len()) {
                // Use exclusive binding - will fail if session already bound
                let _ = registry.bind_agent_to_session(id, session_id, current_time, ttl);
            }
        }

        // Check exclusivity: count agents per session
        let mut session_agent_counts: HashMap<String, usize> = HashMap::new();
        for agent in registry.agents.values() {
            if let Some(ref binding) = agent.session_binding {
                if !binding.is_expired_at(current_time) {
                    *session_agent_counts.entry(binding.session_id.clone()).or_default() += 1;
                }
            }
        }

        // Verify exclusivity
        for (session_id, count) in &session_agent_counts {
            prop_assert!(
                *count <= 1,
                "Session {} should have at most 1 agent bound, but has {}",
                session_id,
                count
            );
        }
    }

    /// Property: Session binding survives heartbeat (if within TTL).
    ///
    /// GIVEN: An agent is bound to session S with TTL X
    /// WHEN: The agent sends a heartbeat at time < T + X
    /// THEN: The agent is still bound to session S
    #[test]
    fn prop_binding_survives_heartbeat(
        agent_id in agent_id_strategy(),
        session_id in session_id_strategy(),
        bound_at in timestamp_strategy(),
        heartbeat_time_delta in 1_u64..100,
        ttl_min in 100_u64..3600,  // Ensure TTL is larger than heartbeat delta
    ) {
        // Heartbeat time is always before TTL expires (heartbeat_delta < ttl_min)
        let heartbeat_time = bound_at.saturating_add(heartbeat_time_delta);

        let mut agent = Agent::new(agent_id, bound_at);
        agent.bind_to_session(&session_id, bound_at, ttl_min);

        // Verify bound before heartbeat
        prop_assert!(agent.is_bound_to(&session_id, bound_at));

        // Send heartbeat
        agent.heartbeat(heartbeat_time);

        // Verify still bound after heartbeat (heartbeat happens before TTL expires)
        prop_assert!(
            agent.is_bound_to(&session_id, heartbeat_time),
            "Binding should survive heartbeat when heartbeat is within TTL"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 5: STATE MACHINE VALIDITY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: All state transitions are valid according to state machine.
    ///
    /// GIVEN: Any pair of agent states
    /// WHEN: We validate the transition
    /// THEN: Only allowed transitions return true
    #[test]
    fn prop_state_transitions_valid(
        from_status in agent_status_strategy(),
        to_status in agent_status_strategy(),
    ) {
        let can_transition = from_status.can_transition_to(to_status);

        // Verify terminal states have no outgoing transitions (except to self)
        if from_status.is_terminal() && from_status != to_status {
            prop_assert!(
                !can_transition,
                "Terminal state {:?} should not transition to {:?}",
                from_status,
                to_status
            );
        }

        // Define expected valid transitions
        let is_expected_valid = match from_status {
            AgentStatus::Pending => {
                matches!(to_status, AgentStatus::Active | AgentStatus::Stopped | AgentStatus::Pending)
            }
            AgentStatus::Active => {
                matches!(to_status, AgentStatus::Idle | AgentStatus::Stopped | AgentStatus::Failed | AgentStatus::Active)
            }
            AgentStatus::Idle => {
                matches!(to_status, AgentStatus::Active | AgentStatus::Stopped | AgentStatus::Idle)
            }
            AgentStatus::Stopped | AgentStatus::Failed => {
                to_status == from_status
            }
        };

        prop_assert_eq!(
            can_transition, is_expected_valid,
            "Transition from {:?} to {:?}: can_transition={} but expected={}",
            from_status, to_status, can_transition, is_expected_valid
        );
    }

    /// Property: Self-transitions are always allowed.
    ///
    /// GIVEN: Any agent state
    /// WHEN: We check transition to same state
    /// THEN: It's always valid
    #[test]
    fn prop_self_transitions_always_valid(status in agent_status_strategy()) {
        prop_assert!(
            status.can_transition_to(status),
            "Self-transition for {:?} should always be valid",
            status
        );
    }

    /// Property: Terminal states cannot transition to other states.
    #[test]
    fn prop_terminal_states_no_exit(status in agent_status_strategy()) {
        prop_assume!(status.is_terminal());

        for other in AgentStatus::all() {
            if *other != status {
                prop_assert!(
                    !status.can_transition_to(*other),
                    "Terminal state {:?} should not transition to {:?}",
                    status,
                    other
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 6: COMBINED INVARIANTS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: All agent invariants hold together.
    ///
    /// INVARIANTS:
    /// 1. Agent IDs are unique
    /// 2. Heartbeats update last_seen correctly
    /// 3. Stale detection works correctly
    /// 4. Session bindings are exclusive
    #[test]
    fn prop_all_agent_invariants_hold(
        agent_ids in unique_agent_ids_strategy(2, 5),
        session_ids in proptest::collection::hash_set(session_id_strategy(), 1..3),
        registered_at in timestamp_strategy(),
        current_time in timestamp_strategy(),
        timeout in timeout_strategy(),
    ) {
        prop_assume!(current_time >= registered_at);

        let mut registry = AgentRegistry::new();

        // Register all agents
        for id in &agent_ids {
            let agent = Agent::new(id.clone(), registered_at);
            let _ = registry.register(agent);
        }

        // Send heartbeats
        for id in &agent_ids {
            if let Some(agent) = registry.agents.get_mut(id) {
                agent.heartbeat(current_time);
            }
        }

        // Bind to sessions using exclusive binding
        let session_vec: Vec<String> = session_ids.into_iter().collect();
        for (i, id) in agent_ids.iter().enumerate() {
            if let Some(session_id) = session_vec.get(i % session_vec.len()) {
                // Use exclusive binding - will fail if session already bound
                let _ = registry.bind_agent_to_session(id, session_id, current_time, timeout);
            }
        }

        // Invariant 1: ID uniqueness
        let all_ids: HashSet<&String> = registry.agents.keys().collect();
        let ids_unique = all_ids.len() == agent_ids.len();

        // Invariant 2: Heartbeats updated last_seen
        let heartbeats_correct = registry.agents.values().all(|a| a.last_seen == current_time);

        // Invariant 3: Stale detection (none should be stale since we just heartbeated)
        let none_stale = registry.agents.values().all(|a| !a.is_stale_at(current_time, timeout));

        // Invariant 4: Session exclusivity
        let session_agent_counts: HashMap<String, usize> = registry
            .agents
            .values()
            .filter_map(|a| a.session_binding.as_ref())
            .filter(|b| !b.is_expired_at(current_time))
            .fold(HashMap::new(), |mut acc, b| {
                *acc.entry(b.session_id.clone()).or_default() += 1;
                acc
            });
        let sessions_exclusive = session_agent_counts.values().all(|&c| c <= 1);

        prop_assert!(
            ids_unique && heartbeats_correct && none_stale && sessions_exclusive,
            "All invariants should hold: ids_unique={}, heartbeats_correct={}, none_stale={}, sessions_exclusive={}",
            ids_unique, heartbeats_correct, none_stale, sessions_exclusive
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
    fn test_stub_agent_creation() {
        let agent = Agent::new("agent-12345678".to_string(), 1000);
        assert_eq!(agent.id, "agent-12345678");
        assert_eq!(agent.status, AgentStatus::Pending);
        assert_eq!(agent.last_seen, 1000);
        assert!(agent.session_binding.is_none());
    }

    #[test]
    fn test_stub_agent_status_terminal() {
        assert!(!AgentStatus::Pending.is_terminal());
        assert!(!AgentStatus::Active.is_terminal());
        assert!(!AgentStatus::Idle.is_terminal());
        assert!(AgentStatus::Stopped.is_terminal());
        assert!(AgentStatus::Failed.is_terminal());
    }

    #[test]
    fn test_stub_agent_heartbeat() {
        let mut agent = Agent::new("agent-1".to_string(), 1000);
        assert_eq!(agent.last_seen, 1000);
        assert_eq!(agent.heartbeat_count, 0);

        agent.heartbeat(2000);
        assert_eq!(agent.last_seen, 2000);
        assert_eq!(agent.heartbeat_count, 1);
    }

    #[test]
    fn test_stub_agent_stale_detection() {
        let mut agent = Agent::new("agent-1".to_string(), 1000);
        agent.heartbeat(1000);

        // Not stale within timeout
        assert!(!agent.is_stale_at(1500, 1000)); // 500 < 1000

        // Stale after timeout
        assert!(agent.is_stale_at(2001, 1000)); // 1001 > 1000
    }

    #[test]
    fn test_stub_session_binding() {
        let mut agent = Agent::new("agent-1".to_string(), 1000);
        agent.bind_to_session("session-1", 1000, 500);

        assert!(agent.is_bound_to("session-1", 1000));
        assert!(!agent.is_bound_to("session-2", 1000));

        // Binding expires after TTL
        assert!(!agent.is_bound_to("session-1", 1500));
    }

    #[test]
    fn test_stub_state_transitions() {
        // Pending can go to Active or Stopped
        assert!(AgentStatus::Pending.can_transition_to(AgentStatus::Active));
        assert!(AgentStatus::Pending.can_transition_to(AgentStatus::Stopped));
        assert!(!AgentStatus::Pending.can_transition_to(AgentStatus::Idle));

        // Active can go to Idle, Stopped, or Failed
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Idle));
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Stopped));
        assert!(AgentStatus::Active.can_transition_to(AgentStatus::Failed));

        // Terminal states can't transition
        assert!(!AgentStatus::Stopped.can_transition_to(AgentStatus::Pending));
        assert!(!AgentStatus::Failed.can_transition_to(AgentStatus::Active));
    }

    /// GREEN PHASE: Registry properly rejects duplicate IDs.
    #[test]
    fn test_registry_rejects_duplicate_ids() {
        let mut registry = AgentRegistry::new();

        let agent1 = Agent::new("agent-duplicate".to_string(), 1000);
        let agent2 = Agent::new("agent-duplicate".to_string(), 1000);

        // First registration succeeds
        let result1 = registry.register(agent1);
        assert!(result1.is_ok(), "First registration should succeed");

        // Second registration with same ID fails
        let result2 = registry.register(agent2);
        assert!(result2.is_err(), "Duplicate registration should fail");
        assert!(
            matches!(result2, Err(AgentError::DuplicateId(_))),
            "Error should be DuplicateId"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVERSARIAL PROPERTY TESTS (RED QUEEN)
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(deterministic_config())]

    /// Adversarial: Heartbeat with time going backwards should still update last_seen.
    ///
    /// This tests what happens when system clocks are out of sync.
    /// GIVEN: An agent heartbeats at time T2
    /// WHEN: A subsequent heartbeat at time T1 < T2
    /// THEN: last_seen becomes T1 (no monotonicity enforcement)
    #[test]
    fn adv_heartbeat_non_monotonic_time(
        agent_id in agent_id_strategy(),
        t1 in timestamp_strategy(),
        t2_delta in 1_u64..1000,
    ) {
        let t2 = t1.saturating_add(t2_delta);
        let mut agent = Agent::new(agent_id, t1);

        // Heartbeat at later time
        agent.heartbeat(t2);
        prop_assert_eq!(agent.last_seen, t2);

        // Heartbeat at earlier time (simulating clock drift)
        agent.heartbeat(t1);
        prop_assert_eq!(agent.last_seen, t1, "Heartbeat accepts non-monotonic time");
    }

    /// Adversarial: Zero timeout should make agent immediately stale.
    ///
    /// GIVEN: An agent sends heartbeat at time T
    /// AND: Timeout is 0
    /// WHEN: We check staleness at time T
    /// THEN: Agent IS stale (boundary condition with zero timeout)
    #[test]
    fn adv_zero_timeout_immediate_stale(
        agent_id in agent_id_strategy(),
        heartbeat_time in timestamp_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, heartbeat_time);
        agent.heartbeat(heartbeat_time);

        // With zero timeout, agent is stale immediately
        prop_assert!(
            agent.is_stale_at(heartbeat_time, 0),
            "Agent should be stale with zero timeout"
        );
    }

    /// Adversarial: Maximum timeout values don't cause overflow.
    ///
    /// GIVEN: An agent heartbeats at max timestamp
    /// AND: Timeout is large
    /// WHEN: We check staleness
    /// THEN: No overflow, behaves correctly
    #[test]
    fn adv_large_timestamps_no_overflow(
        agent_id in agent_id_strategy(),
        heartbeat_time in 1_700_000_000_u64..u64::MAX - 3600,
        timeout in 1_u64..3600,
    ) {
        let mut agent = Agent::new(agent_id, heartbeat_time);
        agent.heartbeat(heartbeat_time);

        // Should not panic or overflow
        let is_stale = agent.is_stale_at(heartbeat_time, timeout);
        prop_assert!(!is_stale, "Agent should not be stale immediately after heartbeat");

        // Check at boundary
        let boundary_time = heartbeat_time.saturating_add(timeout);
        let is_stale_at_boundary = agent.is_stale_at(boundary_time, timeout);
        prop_assert!(is_stale_at_boundary, "Agent should be stale at timeout boundary");
    }

    /// Adversarial: Session binding with zero TTL expires immediately.
    ///
    /// GIVEN: An agent is bound to a session with TTL = 0
    /// WHEN: We check binding at bound_at time
    /// THEN: The binding is expired
    #[test]
    fn adv_zero_ttl_immediate_expiry(
        agent_id in agent_id_strategy(),
        session_id in session_id_strategy(),
        bound_at in timestamp_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, bound_at);
        agent.bind_to_session(&session_id, bound_at, 0);

        // Binding expires immediately with zero TTL
        prop_assert!(
            !agent.is_bound_to(&session_id, bound_at),
            "Binding should be expired with zero TTL"
        );
    }

    /// Adversarial: Cannot bind non-existent agent to session.
    ///
    /// GIVEN: An empty registry
    /// WHEN: We try to bind a non-existent agent
    /// THEN: Returns NotFound error
    #[test]
    fn adv_bind_nonexistent_agent_fails(
        agent_id in agent_id_strategy(),
        session_id in session_id_strategy(),
        current_time in timestamp_strategy(),
        ttl in timeout_strategy(),
    ) {
        let mut registry = AgentRegistry::new();

        let result = registry.bind_agent_to_session(&agent_id, &session_id, current_time, ttl);
        prop_assert!(
            matches!(result, Err(AgentError::NotFound(_))),
            "Binding non-existent agent should fail with NotFound"
        );
    }

    /// Adversarial: Session exclusivity prevents double binding.
    ///
    /// GIVEN: Agent A is bound to session S
    /// WHEN: Agent B tries to bind to session S
    /// THEN: Agent B's binding fails with SessionAlreadyBound
    #[test]
    fn adv_session_exclusivity_enforced(
        agent_id1 in agent_id_strategy(),
        agent_id2 in agent_id_strategy(),
        session_id in session_id_strategy(),
        current_time in timestamp_strategy(),
        ttl in timeout_strategy(),
    ) {
        // Ensure different agent IDs
        prop_assume!(agent_id1 != agent_id2);

        let mut registry = AgentRegistry::new();

        // Register both agents
        let _ = registry.register(Agent::new(agent_id1.clone(), current_time));
        let _ = registry.register(Agent::new(agent_id2.clone(), current_time));

        // First binding succeeds
        let result1 = registry.bind_agent_to_session(&agent_id1, &session_id, current_time, ttl);
        prop_assert!(result1.is_ok(), "First binding should succeed");

        // Second binding to same session fails
        let result2 = registry.bind_agent_to_session(&agent_id2, &session_id, current_time, ttl);
        prop_assert!(
            matches!(result2, Err(AgentError::SessionAlreadyBound(_))),
            "Second binding should fail with SessionAlreadyBound"
        );
    }

    /// Adversarial: Expired bindings don't block new bindings.
    ///
    /// GIVEN: Agent A's binding to session S has expired
    /// WHEN: Agent B tries to bind to session S
    /// THEN: Agent B's binding succeeds
    #[test]
    fn adv_expired_binding_allows_rebinding(
        agent_id1 in agent_id_strategy(),
        agent_id2 in agent_id_strategy(),
        session_id in session_id_strategy(),
        bound_at in timestamp_strategy(),
        ttl in 1_u64..100,
    ) {
        // Ensure different agent IDs
        prop_assume!(agent_id1 != agent_id2);

        let mut registry = AgentRegistry::new();

        // Register both agents
        let _ = registry.register(Agent::new(agent_id1.clone(), bound_at));
        let _ = registry.register(Agent::new(agent_id2.clone(), bound_at));

        // Bind first agent
        let result1 = registry.bind_agent_to_session(&agent_id1, &session_id, bound_at, ttl);
        prop_assert!(result1.is_ok(), "First binding should succeed");

        // Move time past expiry
        let after_expiry = bound_at.saturating_add(ttl).saturating_add(1);

        // Second binding should succeed now that first is expired
        let result2 = registry.bind_agent_to_session(&agent_id2, &session_id, after_expiry, ttl);
        prop_assert!(result2.is_ok(), "Binding should succeed after previous binding expired");
    }

    /// Adversarial: Heartbeat count saturates at u64::MAX.
    ///
    /// GIVEN: An agent with heartbeat_count near u64::MAX
    /// WHEN: More heartbeats are sent
    /// THEN: Count saturates at u64::MAX, doesn't overflow
    #[test]
    fn adv_heartbeat_count_saturates(
        agent_id in agent_id_strategy(),
        registered_at in timestamp_strategy(),
    ) {
        let mut agent = Agent::new(agent_id, registered_at);
        // Manually set count near max (simulating many heartbeats)
        agent.heartbeat_count = u64::MAX - 1;

        agent.heartbeat(registered_at + 1);
        prop_assert_eq!(agent.heartbeat_count, u64::MAX, "Should saturate at u64::MAX");

        // Another heartbeat should stay at max
        agent.heartbeat(registered_at + 2);
        prop_assert_eq!(agent.heartbeat_count, u64::MAX, "Should stay at u64::MAX");
    }

    /// Adversarial: Rapid heartbeat sequence handles edge cases.
    ///
    /// GIVEN: An agent receives many heartbeats in rapid succession
    /// WHEN: All heartbeats happen at the same timestamp
    /// THEN: Each heartbeat increments count, last_seen stays same
    #[test]
    fn adv_rapid_same_time_heartbeats(
        agent_id in agent_id_strategy(),
        registered_at in timestamp_strategy(),
        heartbeat_count in 1_u64..100,
    ) {
        let mut agent = Agent::new(agent_id, registered_at);
        let same_time = registered_at + 1000;

        for _ in 0..heartbeat_count {
            agent.heartbeat(same_time);
        }

        prop_assert_eq!(agent.last_seen, same_time);
        prop_assert_eq!(agent.heartbeat_count, heartbeat_count);
    }

    /// Adversarial: State machine rejects invalid transitions comprehensively.
    ///
    /// GIVEN: All possible state pairs
    /// WHEN: We validate transitions
    /// THEN: Only valid state machine transitions are allowed
    #[test]
    fn adv_state_machine_comprehensive(
        from_status in agent_status_strategy(),
        to_status in agent_status_strategy(),
    ) {
        let can_transition = from_status.can_transition_to(to_status);

        // Verify specific invalid transitions
        let is_known_invalid = match (from_status, to_status) {
            // Pending cannot go to Idle or Failed
            (AgentStatus::Pending, AgentStatus::Idle) => true,
            (AgentStatus::Pending, AgentStatus::Failed) => true,
            // Active cannot go to Pending
            (AgentStatus::Active, AgentStatus::Pending) => true,
            // Idle cannot go to Pending or Failed
            (AgentStatus::Idle, AgentStatus::Pending) => true,
            (AgentStatus::Idle, AgentStatus::Failed) => true,
            // Terminal states cannot exit
            (AgentStatus::Stopped, AgentStatus::Pending) => true,
            (AgentStatus::Stopped, AgentStatus::Active) => true,
            (AgentStatus::Stopped, AgentStatus::Idle) => true,
            (AgentStatus::Stopped, AgentStatus::Failed) => true,
            (AgentStatus::Failed, AgentStatus::Pending) => true,
            (AgentStatus::Failed, AgentStatus::Active) => true,
            (AgentStatus::Failed, AgentStatus::Idle) => true,
            (AgentStatus::Failed, AgentStatus::Stopped) => true,
            _ => false,
        };

        if is_known_invalid {
            prop_assert!(!can_transition, "Invalid transition {:?} -> {:?} should be rejected", from_status, to_status);
        } else if from_status == to_status {
            prop_assert!(can_transition, "Self-transition {:?} should be allowed", from_status);
        }
    }

    /// Adversarial: Registry with many agents handles scale correctly.
    ///
    /// GIVEN: A registry with maximum test agents
    /// WHEN: We perform various operations
    /// THEN: All invariants are maintained
    #[test]
    fn adv_large_registry_scale(
        agent_ids in unique_agent_ids_strategy(50, 100),
        registered_at in timestamp_strategy(),
        current_time in timestamp_strategy(),
    ) {
        prop_assume!(current_time >= registered_at);

        let mut registry = AgentRegistry::new();

        // Register all agents
        let mut registration_count = 0usize;
        for id in &agent_ids {
            let agent = Agent::new(id.clone(), registered_at);
            if registry.register(agent).is_ok() {
                registration_count = registration_count.saturating_add(1);
            }
        }

        // Verify all registered
        prop_assert_eq!(registration_count, agent_ids.len());
        prop_assert_eq!(registry.agents.len(), agent_ids.len());

        // All IDs unique
        let unique_count = registry.agents.keys().collect::<HashSet<_>>().len();
        prop_assert_eq!(unique_count, agent_ids.len());
    }
}
