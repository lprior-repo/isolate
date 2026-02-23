//! State Machine Transition Tests
//!
//! Exhaustive tests for all state machine transitions in the DDD domain layer.
//!
//! Tests:
//! - `SessionStatus` transitions (`creating` -> `active`/`paused`/`completed`/`failed`)
//! - `WorkspaceState` transitions (`creating` -> `ready` -> `active` -> `cleaning` -> `removed`)
//! - `AgentState` transitions (`idle` <-> `active`, any -> `error`/`offline`)
//! - `ClaimState` transitions (`unclaimed` -> `claimed` -> `expired` -> `unclaimed`)
//! - `BranchState` transitions (`detached` <-> `on_branch`)
//! - `ParentState` transitions (`root` <-> `child_of`)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use zjj_core::cli_contracts::SessionContracts;
use zjj_core::domain::{
    agent::AgentState,
    identifiers::{AgentId, SessionName},
    queue::ClaimState,
    session::{BranchState, ParentState},
    workspace::WorkspaceState,
};

// ═══════════════════════════════════════════════════════════════════════════
// ARBITRARY STRATEGY IMPLEMENTATIONS (for proptest)
// ═══════════════════════════════════════════════════════════════════════════

// Newtype wrappers for proptest to avoid orphan rule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PropWorkspaceState(WorkspaceState);

impl Arbitrary for PropWorkspaceState {
    type Parameters = ();

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(PropWorkspaceState(WorkspaceState::Creating)),
            Just(PropWorkspaceState(WorkspaceState::Ready)),
            Just(PropWorkspaceState(WorkspaceState::Active)),
            Just(PropWorkspaceState(WorkspaceState::Cleaning)),
            Just(PropWorkspaceState(WorkspaceState::Removed)),
        ]
        .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl From<PropWorkspaceState> for WorkspaceState {
    fn from(value: PropWorkspaceState) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PropAgentState(AgentState);

impl Arbitrary for PropAgentState {
    type Parameters = ();

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(PropAgentState(AgentState::Idle)),
            Just(PropAgentState(AgentState::Active)),
            Just(PropAgentState(AgentState::Offline)),
            Just(PropAgentState(AgentState::Error)),
        ]
        .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl From<PropAgentState> for AgentState {
    fn from(value: PropAgentState) -> Self {
        value.0
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to create a test agent ID without unwrap
fn test_agent_id(id: &str) -> AgentId {
    match AgentId::parse(id) {
        Ok(agent_id) => agent_id,
        Err(e) => panic!("Failed to parse test agent ID '{id}': {e}"),
    }
}

/// Helper to create a test session name without unwrap
fn test_session_name(name: &str) -> SessionName {
    match SessionName::parse(name) {
        Ok(session_name) => session_name,
        Err(e) => panic!("Failed to parse test session name '{name}': {e}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATUS TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_session_status_all_valid_transitions() {
    // Test all valid transitions defined in SessionContracts
    let valid_transitions = [
        ("creating", "active"),
        ("creating", "failed"),
        ("active", "paused"),
        ("active", "completed"),
        ("paused", "active"),
        ("paused", "completed"),
    ];

    for (from, to) in valid_transitions {
        assert!(
            SessionContracts::is_valid_transition(from, to),
            "Transition {from} -> {to} should be valid"
        );
    }
}

#[test]
fn test_session_status_invalid_transitions() {
    let invalid_transitions = [
        ("creating", "paused"),
        ("creating", "completed"),
        ("active", "creating"),
        ("active", "failed"),
        ("paused", "creating"),
        ("paused", "failed"),
        ("completed", "active"),
        ("completed", "creating"),
        ("failed", "creating"),
        ("failed", "active"),
    ];

    for (from, to) in invalid_transitions {
        assert!(
            !SessionContracts::is_valid_transition(from, to),
            "Transition {from} -> {to} should be invalid"
        );
    }
}

#[test]
fn test_session_status_all_status_values() {
    let valid_statuses = ["creating", "active", "paused", "completed", "failed"];

    for status in valid_statuses {
        assert!(
            SessionContracts::validate_status(status).is_ok(),
            "{status} should be a valid status"
        );
    }
}

#[test]
fn test_session_status_invalid_values() {
    let invalid_statuses = ["pending", "running", "blocked", "cancelled", "unknown"];

    for status in invalid_statuses {
        assert!(
            SessionContracts::validate_status(status).is_err(),
            "{status} should be an invalid status"
        );
    }
}

// Property: Transitions are deterministic
proptest! {
    #[test]
    fn prop_session_transitions_deterministic(from in ".*", to in ".*") {
        let result1 = SessionContracts::is_valid_transition(&from, &to);
        let result2 = SessionContracts::is_valid_transition(&from, &to);
        prop_assert_eq!(result1, result2);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE STATE TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_workspace_state_all_states() {
    let states = WorkspaceState::all();
    assert_eq!(states.len(), 5);
}

#[test]
fn test_workspace_state_valid_transitions_creating() {
    let current = WorkspaceState::Creating;
    let valid = current.valid_transitions();

    // Creating can go to Ready or Removed (failed creation)
    assert_eq!(valid.len(), 2);
    assert!(valid.contains(&WorkspaceState::Ready));
    assert!(valid.contains(&WorkspaceState::Removed));
}

#[test]
fn test_workspace_state_valid_transitions_ready() {
    let current = WorkspaceState::Ready;
    let valid = current.valid_transitions();

    // Ready can go to Active, Cleaning, or Removed
    assert_eq!(valid.len(), 3);
    assert!(valid.contains(&WorkspaceState::Active));
    assert!(valid.contains(&WorkspaceState::Cleaning));
    assert!(valid.contains(&WorkspaceState::Removed));
}

#[test]
fn test_workspace_state_valid_transitions_active() {
    let current = WorkspaceState::Active;
    let valid = current.valid_transitions();

    // Active can go to Cleaning or Removed
    assert_eq!(valid.len(), 2);
    assert!(valid.contains(&WorkspaceState::Cleaning));
    assert!(valid.contains(&WorkspaceState::Removed));
}

#[test]
fn test_workspace_state_valid_transitions_cleaning() {
    let current = WorkspaceState::Cleaning;
    let valid = current.valid_transitions();

    // Cleaning can only go to Removed
    assert_eq!(valid.len(), 1);
    assert!(valid.contains(&WorkspaceState::Removed));
}

#[test]
fn test_workspace_state_valid_transitions_removed() {
    let current = WorkspaceState::Removed;
    let valid = current.valid_transitions();

    // Removed is terminal
    assert!(valid.is_empty());
}

#[test]
fn test_workspace_state_is_removed_consistent() {
    // Check that is_terminal() matches Removed state
    assert!(WorkspaceState::Removed.is_terminal());
    assert!(!WorkspaceState::Creating.is_terminal());
    assert!(!WorkspaceState::Ready.is_terminal());
    assert!(!WorkspaceState::Active.is_terminal());
    assert!(!WorkspaceState::Cleaning.is_terminal());
}

#[test]
fn test_workspace_state_workflow_complete() {
    // Test the full lifecycle: Creating -> Ready -> Active -> Cleaning -> Removed
    let mut state = WorkspaceState::Creating;

    assert!(state.can_transition_to(&WorkspaceState::Ready));
    state = WorkspaceState::Ready;

    assert!(state.can_transition_to(&WorkspaceState::Active));
    state = WorkspaceState::Active;

    assert!(state.can_transition_to(&WorkspaceState::Cleaning));
    state = WorkspaceState::Cleaning;

    assert!(state.can_transition_to(&WorkspaceState::Removed));
    state = WorkspaceState::Removed;

    assert!(state.is_terminal());
}

#[test]
fn test_workspace_state_no_invalid_transitions() {
    let invalid = [
        (WorkspaceState::Creating, WorkspaceState::Creating),
        (WorkspaceState::Creating, WorkspaceState::Active),
        (WorkspaceState::Creating, WorkspaceState::Cleaning),
        (WorkspaceState::Ready, WorkspaceState::Creating),
        (WorkspaceState::Ready, WorkspaceState::Ready),
        (WorkspaceState::Active, WorkspaceState::Creating),
        (WorkspaceState::Active, WorkspaceState::Ready),
        (WorkspaceState::Active, WorkspaceState::Active),
        (WorkspaceState::Cleaning, WorkspaceState::Creating),
        (WorkspaceState::Cleaning, WorkspaceState::Ready),
        (WorkspaceState::Cleaning, WorkspaceState::Active),
        (WorkspaceState::Cleaning, WorkspaceState::Cleaning),
        (WorkspaceState::Removed, WorkspaceState::Creating),
        (WorkspaceState::Removed, WorkspaceState::Ready),
        (WorkspaceState::Removed, WorkspaceState::Active),
        (WorkspaceState::Removed, WorkspaceState::Cleaning),
    ];

    for (from, to) in invalid {
        assert!(
            !from.can_transition_to(&to),
            "Invalid transition {from:?} -> {to:?} should be rejected"
        );
    }
}

// Property: Once Removed, always Removed
proptest! {
    #[test]
    fn prop_workspace_removed_is_terminal(state in any::<PropWorkspaceState>()) {
        let state: WorkspaceState = state.into();
        let transitions = state.valid_transitions();
        prop_assert!(!matches!(state, WorkspaceState::Removed) || transitions.is_empty());
    }
}

// Property: Active always requires going through Ready first
proptest! {
    #[test]
    fn prop_workspace_active_requires_ready(states in prop::collection::vec(any::<PropWorkspaceState>(), 2..5)) {
        let states: Vec<WorkspaceState> = states.into_iter().map(Into::into).collect();
        // In any VALID path, Active must come after Ready or another Active
        for window in states.windows(2) {
            if matches!(window[1], WorkspaceState::Active) {
                // Only check if transition is valid
                let can_transition = window[0].can_transition_to(&window[1]);
                if can_transition {
                    prop_assert!(matches!(window[0], WorkspaceState::Ready) || matches!(window[0], WorkspaceState::Active));
                }
            }
        }
    }
}

// Property: Transitions are deterministic
proptest! {
    #[test]
    fn prop_workspace_transitions_deterministic(from in any::<PropWorkspaceState>(), to in any::<PropWorkspaceState>()) {
        let from: WorkspaceState = from.into();
        let to: WorkspaceState = to.into();
        let result1 = from.can_transition_to(&to);
        let result2 = from.can_transition_to(&to);
        prop_assert_eq!(result1, result2);
    }
}

// Property: State transition lists are consistent with can_transition_to
proptest! {
    #[test]
    fn prop_workspace_transition_list_consistency(state in any::<PropWorkspaceState>()) {
        let state: WorkspaceState = state.into();
        let transitions = state.valid_transitions();
        for target in WorkspaceState::all() {
            let in_list = transitions.contains(&target);
            let can_trans = state.can_transition_to(&target);
            prop_assert_eq!(in_list, can_trans,
                "Inconsistency for {:?} -> {:?}: in_list={}", state, target, in_list);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AGENT STATE TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_agent_state_all_states() {
    let states = AgentState::all();
    assert_eq!(states.len(), 4);
}

#[test]
fn test_agent_state_valid_transitions_idle() {
    let current = AgentState::Idle;
    let valid = current.valid_transitions();

    // Idle can go to Active, Offline, or Error
    assert_eq!(valid.len(), 3);
    assert!(valid.contains(&AgentState::Active));
    assert!(valid.contains(&AgentState::Offline));
    assert!(valid.contains(&AgentState::Error));
}

#[test]
fn test_agent_state_valid_transitions_active() {
    let current = AgentState::Active;
    let valid = current.valid_transitions();

    // Active can go to Idle, Offline, or Error
    assert_eq!(valid.len(), 3);
    assert!(valid.contains(&AgentState::Idle));
    assert!(valid.contains(&AgentState::Offline));
    assert!(valid.contains(&AgentState::Error));
}

#[test]
fn test_agent_state_valid_transitions_offline() {
    let current = AgentState::Offline;
    let valid = current.valid_transitions();

    // Offline can go to Idle or Error
    assert_eq!(valid.len(), 2);
    assert!(valid.contains(&AgentState::Idle));
    assert!(valid.contains(&AgentState::Error));
}

#[test]
fn test_agent_state_valid_transitions_error() {
    let current = AgentState::Error;
    let valid = current.valid_transitions();

    // Error can only go to Offline
    assert_eq!(valid.len(), 1);
    assert!(valid.contains(&AgentState::Offline));
}

#[test]
fn test_agent_state_idle_active_cycle() {
    // Idle <-> Active should be bidirectional
    assert!(AgentState::Idle.can_transition_to(&AgentState::Active));
    assert!(AgentState::Active.can_transition_to(&AgentState::Idle));
}

#[test]
fn test_agent_state_no_self_loops() {
    for state in AgentState::all() {
        assert!(
            !state.can_transition_to(&state),
            "{state:?} should not allow self-transition"
        );
    }
}

#[test]
fn test_agent_state_error_only_to_offline() {
    let from_error = AgentState::Error.valid_transitions();
    assert_eq!(from_error.len(), 1);
    assert!(from_error.contains(&AgentState::Offline));
}

#[test]
fn test_agent_state_reachable_from_initial() {
    // Starting from Idle, verify all states are reachable
    let mut reachable = vec![AgentState::Idle];
    let mut frontier = vec![AgentState::Idle];

    while let Some(current) = frontier.pop() {
        for next in current.valid_transitions() {
            if !reachable.contains(&next) {
                reachable.push(next);
                frontier.push(next);
            }
        }
    }

    // All states should be reachable from Idle
    for state in AgentState::all() {
        assert!(
            reachable.contains(&state),
            "{state:?} should be reachable from Idle"
        );
    }
}

// Property: Transitions are deterministic
proptest! {
    #[test]
    fn prop_agent_transitions_deterministic(from in any::<PropAgentState>(), to in any::<PropAgentState>()) {
        let from: AgentState = from.into();
        let to: AgentState = to.into();
        let result1 = from.can_transition_to(&to);
        let result2 = from.can_transition_to(&to);
        prop_assert_eq!(result1, result2);
    }
}

// Property: All states have at least one transition except possibly error states
proptest! {
    #[test]
    fn prop_agent_has_transitions(state in any::<PropAgentState>()) {
        let state: AgentState = state.into();
        let transitions = state.valid_transitions();
        // All agent states should have at least one outgoing transition
        prop_assert!(!transitions.is_empty());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAIM STATE (QUEUE) TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_claim_state_unclaimed_transitions() {
    let current = ClaimState::Unclaimed;
    let valid_types = current.valid_transition_types();

    assert_eq!(valid_types.len(), 1);
    assert!(valid_types.contains(&"Claimed"));

    // Test actual transition
    let claimed = ClaimState::Claimed {
        agent: test_agent_id("agent-1"),
        claimed_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
    };
    assert!(current.can_transition_to(&claimed));
}

#[test]
fn test_claim_state_claimed_transitions() {
    let agent = test_agent_id("agent-1");
    let current = ClaimState::Claimed {
        agent: agent.clone(),
        claimed_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
    };
    let valid_types = current.valid_transition_types();

    assert_eq!(valid_types.len(), 2);
    assert!(valid_types.contains(&"Expired"));
    assert!(valid_types.contains(&"Unclaimed"));

    // Test transitions
    let expired = ClaimState::Expired {
        previous_agent: agent.clone(),
        expired_at: chrono::Utc::now(),
    };
    assert!(current.can_transition_to(&expired));
    assert!(current.can_transition_to(&ClaimState::Unclaimed));
}

#[test]
fn test_claim_state_expired_transitions() {
    let agent = test_agent_id("agent-1");
    let current = ClaimState::Expired {
        previous_agent: agent,
        expired_at: chrono::Utc::now(),
    };
    let valid_types = current.valid_transition_types();

    assert_eq!(valid_types.len(), 1);
    assert!(valid_types.contains(&"Unclaimed"));

    // Test transition
    assert!(current.can_transition_to(&ClaimState::Unclaimed));
}

#[test]
fn test_claim_state_no_self_loops() {
    // Unclaimed cannot stay Unclaimed (should be explicit state change)
    assert!(!ClaimState::Unclaimed.can_transition_to(&ClaimState::Unclaimed));

    // Claimed cannot stay Claimed
    let claimed = ClaimState::Claimed {
        agent: test_agent_id("agent-1"),
        claimed_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
    };
    assert!(!claimed.can_transition_to(&claimed));

    // Expired cannot stay Expired
    let expired = ClaimState::Expired {
        previous_agent: test_agent_id("agent-1"),
        expired_at: chrono::Utc::now(),
    };
    assert!(!expired.can_transition_to(&expired));
}

#[test]
fn test_claim_state_claim_lifecycle() {
    // Test full lifecycle: Unclaimed -> Claimed -> Expired -> Unclaimed
    let agent = test_agent_id("agent-1");

    let mut state = ClaimState::Unclaimed;

    // Claim it
    let claimed = ClaimState::Claimed {
        agent: agent.clone(),
        claimed_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
    };
    assert!(state.can_transition_to(&claimed));
    state = claimed;

    // Expire it
    let expired = ClaimState::Expired {
        previous_agent: agent.clone(),
        expired_at: chrono::Utc::now(),
    };
    assert!(state.can_transition_to(&expired));
    state = expired;

    // Reclaim it
    assert!(state.can_transition_to(&ClaimState::Unclaimed));
}

// ═══════════════════════════════════════════════════════════════════════════
// BRANCH STATE TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_branch_state_detached_to_on_branch() {
    let detached = BranchState::Detached;
    let on_branch = BranchState::OnBranch {
        name: "main".to_string(),
    };

    assert!(detached.can_transition_to(&on_branch));
}

#[test]
fn test_branch_state_on_branch_to_detached() {
    let on_branch = BranchState::OnBranch {
        name: "main".to_string(),
    };
    let detached = BranchState::Detached;

    assert!(on_branch.can_transition_to(&detached));
}

#[test]
fn test_branch_state_on_branch_to_on_branch() {
    let main = BranchState::OnBranch {
        name: "main".to_string(),
    };
    let feature = BranchState::OnBranch {
        name: "feature".to_string(),
    };

    assert!(main.can_transition_to(&feature));
}

#[test]
fn test_branch_state_detached_no_self_loop() {
    let detached = BranchState::Detached;
    assert!(!detached.can_transition_to(&detached));
}

#[test]
fn test_branch_state_all_valid_branch_transitions() {
    let branches = vec!["main", "develop", "feature", "hotfix"];

    for from in &branches {
        for to in &branches {
            let from_state = BranchState::OnBranch {
                name: from.to_string(),
            };
            let to_state = BranchState::OnBranch {
                name: to.to_string(),
            };

            // All branch-to-branch transitions are valid
            assert!(
                from_state.can_transition_to(&to_state),
                "Branch {from} -> {to} should be valid"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PARENT STATE TRANSITIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parent_state_root_is_immutable() {
    let root = ParentState::Root;

    // Root cannot transition to anything
    assert!(!root.can_transition_to(&ParentState::Root));
    assert!(!root.can_transition_to(&ParentState::ChildOf {
        parent: test_session_name("new-parent"),
    }));
}

#[test]
fn test_parent_state_child_cannot_become_root() {
    let child = ParentState::ChildOf {
        parent: test_session_name("parent"),
    };

    assert!(!child.can_transition_to(&ParentState::Root));
}

#[test]
fn test_parent_state_child_can_change_parent() {
    let old_parent = test_session_name("old-parent");
    let new_parent = test_session_name("new-parent");

    let child = ParentState::ChildOf {
        parent: old_parent.clone(),
    };
    let new_child = ParentState::ChildOf {
        parent: new_parent.clone(),
    };

    assert!(child.can_transition_to(&new_child));
}

#[test]
fn test_parent_state_no_transitions_from_root() {
    let root = ParentState::Root;
    let all_states = vec![
        ParentState::Root,
        ParentState::ChildOf {
            parent: test_session_name("any"),
        },
    ];

    for target in all_states {
        assert!(
            !root.can_transition_to(&target),
            "Root should not allow any transitions"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE MACHINE INVARIANTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_all_state_machines_have_initial_states() {
    // Verify each state machine has a clear initial state

    // Session: Creating (first defined)
    // Workspace: Creating (first defined)
    // Agent: Idle (first defined)

    // Just verify the ordering in the all() methods
    assert_eq!(WorkspaceState::all()[0], WorkspaceState::Creating);
    assert_eq!(AgentState::all()[0], AgentState::Idle);
}

#[test]
fn test_all_state_machines_have_terminal_states() {
    // Verify each state machine has at least one terminal state

    // Workspace: Removed
    let workspace_terminals = WorkspaceState::all()
        .iter()
        .filter(|s| s.valid_transitions().is_empty())
        .count();
    assert_eq!(workspace_terminals, 1, "WorkspaceState should have 1 terminal state");

    // Session: Completed, Failed (2 terminal states)
    // Agent: No true terminal states (can always transition)
}

#[test]
fn test_all_state_transitions_are_validated() {
    // Verify that every state has transition validation logic

    for state in WorkspaceState::all() {
        let _ = state.valid_transitions(); // Should not panic
    }

    for state in AgentState::all() {
        let _ = state.valid_transitions(); // Should not panic
    }

    // Test ClaimState
    let _ = ClaimState::Unclaimed.valid_transition_types();
    let _ = ClaimState::Claimed {
        agent: test_agent_id("test"),
        claimed_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now(),
    }
    .valid_transition_types();
    let _ = ClaimState::Expired {
        previous_agent: test_agent_id("test"),
        expired_at: chrono::Utc::now(),
    }
    .valid_transition_types();

    // Test BranchState
    let _ = BranchState::Detached.can_transition_to(&BranchState::Detached);
    let _ = BranchState::OnBranch {
        name: "main".to_string(),
    }
    .can_transition_to(&BranchState::Detached);

    // Test ParentState
    let _ = ParentState::Root.can_transition_to(&ParentState::Root);
    let _ = ParentState::ChildOf {
        parent: test_session_name("parent"),
    }
    .can_transition_to(&ParentState::Root);
}

#[test]
fn test_no_unreachable_states() {
    // Verify all states are reachable from the initial state

    // Workspace: All states reachable from Creating
    let mut workspace_reachable = vec![WorkspaceState::Creating];
    let mut workspace_frontier = vec![WorkspaceState::Creating];

    while let Some(current) = workspace_frontier.pop() {
        for next in current.valid_transitions() {
            if !workspace_reachable.contains(&next) {
                workspace_reachable.push(next);
                workspace_frontier.push(next);
            }
        }
    }

    for state in WorkspaceState::all() {
        assert!(
            workspace_reachable.contains(&state),
            "WorkspaceState {:?} is not reachable from Creating",
            state
        );
    }

    // Agent: All states reachable from Idle
    let mut agent_reachable = vec![AgentState::Idle];
    let mut agent_frontier = vec![AgentState::Idle];

    while let Some(current) = agent_frontier.pop() {
        for next in current.valid_transitions() {
            if !agent_reachable.contains(&next) {
                agent_reachable.push(next);
                agent_frontier.push(next);
            }
        }
    }

    for state in AgentState::all() {
        assert!(
            agent_reachable.contains(&state),
            "AgentState {:?} is not reachable from Idle",
            state
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SUMMARY AND DOCUMENTATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_state_machine_summary() {
    // This test documents all state machines and their transitions

    // SessionStatus: 5 states, 2 terminal
    // Creating -> Active, Failed
    // Active -> Paused, Completed
    // Paused -> Active, Completed
    // Completed -> (terminal)
    // Failed -> (terminal)

    // WorkspaceState: 5 states, 1 terminal
    // Creating -> Ready, Removed
    // Ready -> Active, Cleaning, Removed
    // Active -> Cleaning, Removed
    // Cleaning -> Removed
    // Removed -> (terminal)

    let workspace_states = WorkspaceState::all().len();
    let workspace_terminal = WorkspaceState::all()
        .iter()
        .filter(|s| s.valid_transitions().is_empty())
        .count();

    assert_eq!(workspace_states, 5);
    assert_eq!(workspace_terminal, 1);

    // AgentState: 4 states, 0 terminal
    // Idle <-> Active
    // Any -> Offline
    // Any -> Error
    // Error -> Offline
    // Offline -> Idle

    let agent_states = AgentState::all().len();
    let agent_terminal = AgentState::all()
        .iter()
        .filter(|s| s.valid_transitions().is_empty())
        .count();

    assert_eq!(agent_states, 4);
    assert_eq!(agent_terminal, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_session_workspace_state_consistency() {
    // Session and Workspace states should be consistent
    // When Session is Creating, Workspace should be Creating
    // When Session is Active, Workspace should be Ready or Active

    // This is an example of how states might need to be coordinated
    // The actual coordination logic would be in higher-level code
    let session_creating = "creating";
    let workspace_creating = WorkspaceState::Creating;

    // Both are in creation phase
    let can_create_session =
        SessionContracts::is_valid_transition(session_creating, "active");
    assert!(can_create_session);
    assert!(!workspace_creating.valid_transitions().is_empty());
}

#[test]
fn test_agent_claim_state_interaction() {
    // Agent and Claim states interact
    // When Agent is Active, it can have Claims
    // When Agent is Offline, its Claims should expire

    let agent_active = AgentState::Active;
    let agent_offline = AgentState::Offline;

    // Active agent can go to offline
    assert!(agent_active.can_transition_to(&agent_offline));

    // Claimed state can expire
    let agent = test_agent_id("agent-1");
    let claimed = ClaimState::Claimed {
        agent: agent.clone(),
        claimed_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
    };
    let expired = ClaimState::Expired {
        previous_agent: agent,
        expired_at: chrono::Utc::now(),
    };

    assert!(claimed.can_transition_to(&expired));
}
