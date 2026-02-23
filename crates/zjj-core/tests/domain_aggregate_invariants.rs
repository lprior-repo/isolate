//! Comprehensive aggregate invariant tests
//!
//! Tests all business rules and invariants enforced by aggregate roots.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::Utc;
use zjj_core::domain::aggregates::{
    bead::BeadTimestamps, Bead, BeadError, BeadState, QueueEntry, QueueEntryError, Session,
    SessionError, Workspace, WorkspaceError,
};
use zjj_core::domain::identifiers::{
    AgentId, BeadId, SessionId, SessionName, WorkspaceName,
};
use zjj_core::domain::queue::ClaimState;
use zjj_core::domain::session::{BranchState, ParentState};
use zjj_core::domain::workspace::WorkspaceState;

// ============================================================================
// BEAD AGGREGATE INVARIANTS
// ============================================================================

#[test]
fn test_bead_invariant_title_required() {
    let id = BeadId::parse("bd-abc123").expect("valid id");

    // Empty title should fail
    let result = Bead::new(id.clone(), "", None::<String>);
    assert!(matches!(result, Err(BeadError::InvalidTitle(_))));

    // Whitespace-only title should fail
    let result = Bead::new(id.clone(), "   ", None::<String>);
    assert!(matches!(result, Err(BeadError::InvalidTitle(_))));

    // Valid title should succeed
    let result = Bead::new(id, "Valid Title", None::<String>);
    assert!(result.is_ok());
}

#[test]
fn test_bead_invariant_closed_state_has_timestamp() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let now = Utc::now();

    // Reconstruct with closed state and timestamp
    let bead = Bead::reconstruct(
        id.clone(),
        "Test",
        None::<String>,
        BeadState::Closed { closed_at: now },
        BeadTimestamps::new(now - chrono::Duration::seconds(10), now),
    )
    .expect("reconstruct valid");

    assert!(bead.is_closed());
    assert_eq!(bead.closed_at(), Some(now));
}

#[test]
fn test_bead_invariant_cannot_modify_closed() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");
    let closed = bead.close().expect("close valid");

    // Cannot transition from closed
    assert!(matches!(closed.start(), Err(BeadError::CannotModifyClosed)));
    assert!(matches!(closed.block(), Err(BeadError::CannotModifyClosed)));
    assert!(matches!(closed.defer(), Err(BeadError::CannotModifyClosed)));

    // Cannot update fields
    assert!(matches!(
        closed.update_title("New"),
        Err(BeadError::CannotModifyClosed)
    ));
    assert!(matches!(
        closed.update_description(Some("New")),
        Err(BeadError::CannotModifyClosed)
    ));
}

#[test]
fn test_bead_invariant_timestamps_monotonic() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let now = Utc::now();
    let past = now - chrono::Duration::seconds(1);

    // Non-monotonic timestamps should fail
    let result = Bead::reconstruct(
        id,
        "Test",
        None::<String>,
        BeadState::Open,
        BeadTimestamps::new(now, past), // updated_at < created_at
    );

    assert!(matches!(
        result,
        Err(BeadError::NonMonotonicTimestamps { .. })
    ));
}

#[test]
fn test_bead_invariant_state_transitions_valid() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");

    // Open -> InProgress (valid)
    let in_progress = bead.start().expect("transition valid");
    assert!(in_progress.is_in_progress());

    // InProgress -> Blocked (valid)
    let blocked = in_progress.block().expect("transition valid");
    assert!(blocked.is_blocked());

    // Blocked -> Deferred (valid)
    let deferred = blocked.defer().expect("transition valid");
    assert!(deferred.is_deferred());

    // Deferred -> Open (valid)
    let open = deferred.open().expect("transition valid");
    assert!(open.is_open());
}

// ============================================================================
// SESSION AGGREGATE INVARIANTS
// ============================================================================

#[test]
fn test_session_invariant_workspace_must_exist() {
    let id = SessionId::parse("sess-test001").expect("valid id");
    let name = SessionName::parse("test-session").expect("valid name");

    // Non-existent workspace should fail
    let result = Session::new_root(
        id.clone(),
        name.clone(),
        BranchState::Detached,
        std::path::PathBuf::from("/nonexistent/workspace/path"),
    );

    assert!(matches!(
        result,
        Err(SessionError::WorkspaceNotFound(_))
    ));
}

#[test]
fn test_session_invariant_root_cannot_become_child() {
    let id = SessionId::parse("sess-test002").expect("valid id");
    let name = SessionName::parse("test-session").expect("valid name");

    // Use /tmp which should exist
    let session = Session::new_root(
        id,
        name,
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // Root cannot become child
    let new_parent = ParentState::ChildOf {
        parent: SessionName::parse("parent").expect("valid name"),
    };

    let result = session.transition_parent(new_parent);
    assert!(matches!(result, Err(SessionError::CannotModifyRootParent)));
}

#[test]
fn test_session_invariant_branch_transitions() {
    let id = SessionId::parse("sess-test003").expect("valid id");
    let name = SessionName::parse("test").expect("valid name");

    // Detached -> OnBranch (valid)
    let session = Session::new_root(
        id.clone(),
        name.clone(),
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    let on_branch = session
        .transition_branch(BranchState::OnBranch {
            name: "main".to_string(),
        })
        .expect("transition valid");

    // OnBranch -> OnBranch (switching branches, valid)
    let feature_branch = on_branch
        .transition_branch(BranchState::OnBranch {
            name: "feature".to_string(),
        })
        .expect("switch valid");

    // OnBranch -> Detached (valid)
    let detached = feature_branch
        .transition_branch(BranchState::Detached)
        .expect("detach valid");

    assert!(detached.branch.is_detached());
}

#[test]
fn test_session_invariant_invalid_branch_transition() {
    let id = SessionId::parse("sess-test004").expect("valid id");
    let name = SessionName::parse("test").expect("valid name");

    let session = Session::new_root(
        id,
        name,
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // Detached -> Detached (invalid, no self-loop)
    let result = session.transition_branch(BranchState::Detached);
    assert!(matches!(
        result,
        Err(SessionError::InvalidBranchTransition { .. })
    ));
}

#[test]
fn test_session_invariant_child_can_change_parent() {
    let id = SessionId::parse("sess-test005").expect("valid id");
    let name = SessionName::parse("child").expect("valid name");
    let parent1 = SessionName::parse("parent1").expect("valid name");

    let session = Session::new_child(
        id,
        name,
        BranchState::Detached,
        parent1,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // Child can change parent (adoption)
    let parent2 = SessionName::parse("parent2").expect("valid name");
    let new_parent = ParentState::ChildOf {
        parent: parent2.clone(),
    };

    let updated = session
        .transition_parent(new_parent)
        .expect("parent change valid");

    assert_eq!(updated.parent_name(), Some(&parent2));
}

// ============================================================================
// WORKSPACE AGGREGATE INVARIANTS
// ============================================================================

#[test]
fn test_workspace_invariant_path_must_exist() {
    let name = WorkspaceName::parse("test-workspace").expect("valid name");

    // Non-existent path should fail
    let result = Workspace::create(
        name.clone(),
        std::path::PathBuf::from("/nonexistent/path"),
    );

    assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));

    // Reconstruct should also validate
    let result = Workspace::reconstruct(
        name,
        std::path::PathBuf::from("/also/nonexistent"),
        WorkspaceState::Creating,
    );

    assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));
}

#[test]
fn test_workspace_invariant_state_transitions() {
    let name = WorkspaceName::parse("test-workspace").expect("valid name");

    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    // Creating -> Ready (valid)
    let ready = workspace.mark_ready().expect("transition valid");
    assert!(ready.is_ready());

    // Ready -> Active (valid)
    let active = ready.mark_active().expect("transition valid");
    assert!(active.is_active());

    // Active -> Cleaning (valid)
    let cleaning = active.start_cleaning().expect("transition valid");
    assert!(cleaning.is_cleaning());

    // Cleaning -> Removed (valid)
    let removed = cleaning.mark_removed().expect("transition valid");
    assert!(removed.is_removed());
    assert!(removed.is_terminal());
}

#[test]
fn test_workspace_invariant_cannot_skip_states() {
    let name = WorkspaceName::parse("test-workspace").expect("valid name");

    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    // Creating -> Active (invalid, must go through Ready first)
    // Use the can_transition_to on the state directly to test this
    assert!(!workspace.state.can_transition_to(&WorkspaceState::Active));
}

#[test]
fn test_workspace_invariant_removed_is_terminal() {
    let name = WorkspaceName::parse("test-workspace").expect("valid name");

    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    let removed = workspace.mark_removed().expect("transition valid");

    // Cannot transition from terminal state - verify state machine is terminal
    assert!(!removed.state.can_transition_to(&WorkspaceState::Creating));
    assert!(!removed.state.can_transition_to(&WorkspaceState::Ready));
    assert!(!removed.state.can_transition_to(&WorkspaceState::Active));

    // Attempting to call mark_active should fail due to invalid transition
    let result = removed.mark_active();
    assert!(matches!(
        result,
        Err(WorkspaceError::InvalidStateTransition { .. })
    ));
}

#[test]
fn test_workspace_invariant_only_ready_active_can_be_used() {
    let name = WorkspaceName::parse("test-workspace").expect("valid name");

    let workspace = Workspace::create(name.clone(), std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    // Creating cannot be used
    assert!(matches!(
        workspace.validate_can_use(),
        Err(WorkspaceError::CannotUse(_))
    ));

    let ready = workspace.mark_ready().expect("transition valid");

    // Ready can be used
    assert!(ready.validate_can_use().is_ok());

    let active = ready.mark_active().expect("transition valid");

    // Active can be used
    assert!(active.validate_can_use().is_ok());

    let cleaning = active.start_cleaning().expect("transition valid");

    // Cleaning cannot be used
    assert!(matches!(
        cleaning.validate_can_use(),
        Err(WorkspaceError::CannotUse(_))
    ));
}

// ============================================================================
// QUEUE ENTRY AGGREGATE INVARIANTS
// ============================================================================

#[test]
fn test_queue_entry_invariant_priority_non_negative() {
    let workspace = WorkspaceName::parse("test-workspace").expect("valid name");

    // Negative priority should fail
    let result = QueueEntry::new(1, workspace.clone(), None, -1);
    assert!(matches!(
        result,
        Err(QueueEntryError::NegativePriority)
    ));

    // Zero priority should succeed
    let result = QueueEntry::new(2, workspace.clone(), None, 0);
    assert!(result.is_ok());

    // Positive priority should succeed
    let result = QueueEntry::new(3, workspace, None, 10);
    assert!(result.is_ok());
}

#[test]
fn test_queue_entry_invariant_only_unclaimed_can_be_claimed() {
    let workspace = WorkspaceName::parse("test-workspace").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    // First claim should succeed
    let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

    // Second claim should fail
    let result = claimed.claim(AgentId::parse("agent-2").expect("valid agent"), 300);
    assert!(matches!(
        result,
        Err(QueueEntryError::AlreadyClaimed(_))
    ));
}

#[test]
fn test_queue_entry_invariant_claim_duration_positive() {
    let workspace = WorkspaceName::parse("test-workspace").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    // Negative duration should fail
    let result = entry.claim(agent.clone(), -1);
    assert!(matches!(
        result,
        Err(QueueEntryError::InvalidExpiration)
    ));

    // Zero duration should fail
    let result = entry.claim(AgentId::parse("agent-2").expect("valid agent"), 0);
    assert!(matches!(
        result,
        Err(QueueEntryError::InvalidExpiration)
    ));

    // Positive duration should succeed
    let entry2 = QueueEntry::new(2, WorkspaceName::parse("test-ws2").expect("valid name"), None, 0).expect("entry created");
    let result = entry2.claim(AgentId::parse("agent-3").expect("valid agent"), 300);
    assert!(result.is_ok());
}

#[test]
fn test_queue_entry_invariant_only_owner_can_release() {
    let workspace = WorkspaceName::parse("test-workspace").expect("valid name");
    let agent1 = AgentId::parse("agent-1").expect("valid agent");
    let agent2 = AgentId::parse("agent-2").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    let claimed = entry.claim(agent1.clone(), 300).expect("claim valid");

    // Non-owner cannot release
    let result = claimed.release(&agent2);
    assert!(matches!(result, Err(QueueEntryError::NotOwner { .. })));

    // Owner can release (need to re-claim since we already tried to release)
    let entry2 = QueueEntry::new(2, WorkspaceName::parse("test-ws2").expect("valid name"), None, 0).expect("entry created");
    let claimed2 = entry2.claim(agent1.clone(), 300).expect("claim valid");
    let result = claimed2.release(&agent1);
    assert!(result.is_ok());
}

#[test]
fn test_queue_entry_invariant_cannot_modify_when_claimed() {
    let workspace = WorkspaceName::parse("test-workspace").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    // Can update when unclaimed
    let updated = entry.update_priority(5).expect("update valid");
    assert_eq!(updated.priority, 5);

    let claimed = updated.claim(agent, 300).expect("claim valid");

    // Cannot update when claimed
    let result = claimed.update_priority(10);
    assert!(matches!(
        result,
        Err(QueueEntryError::CannotModify(_))
    ));
}

#[test]
fn test_queue_entry_invariant_expiration_must_be_future() {
    let workspace = WorkspaceName::parse("test-workspace").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    // Claim with positive duration should succeed
    let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

    if let ClaimState::Claimed { expires_at, .. } = &claimed.claim_state {
        assert!(*expires_at > Utc::now());
    } else {
        panic!("Expected claimed state");
    }

    // Refresh with positive duration should succeed
    let refreshed = claimed.refresh_claim(&agent, 600).expect("refresh valid");

    if let ClaimState::Claimed { expires_at, .. } = &refreshed.claim_state {
        assert!(*expires_at > Utc::now());
    } else {
        panic!("Expected claimed state");
    }
}

// ============================================================================
// CROSS-AGGREGATE INVARIANTS
// ============================================================================

#[test]
fn test_invariant_all_aggregates_have_valid_identifiers() {
    // Bead ID must be valid
    let bead_id = BeadId::parse("bd-abc123").expect("valid bead id");
    let bead = Bead::new(bead_id, "Test", None::<String>).expect("valid bead");
    assert_eq!(bead.id.as_str(), "bd-abc123");

    // Session ID must be valid
    let session_id = SessionId::parse("session-abc").expect("valid session id");
    let session_name = SessionName::parse("test-session").expect("valid name");
    let session = Session::new_root(
        session_id,
        session_name,
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("valid session");
    assert_eq!(session.id.as_str(), "session-abc");

    // Workspace name must be valid
    let workspace_name = WorkspaceName::parse("test-workspace").expect("valid name");
    let workspace = Workspace::create(workspace_name, std::path::PathBuf::from("/tmp"))
        .expect("valid workspace");
    assert_eq!(workspace.name.as_str(), "test-workspace");
}

#[test]
fn test_invariant_aggregates_enforce_immutability() {
    let workspace_name = WorkspaceName::parse("test").expect("valid name");
    let workspace = Workspace::create(workspace_name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    // State transitions return new instances
    let ready = workspace.mark_ready().expect("transition valid");

    // Original is unchanged
    assert!(workspace.is_creating());
    assert!(ready.is_ready());

    // Multiple transitions create independent instances
    let active = ready.mark_active().expect("transition valid");

    assert!(workspace.is_creating());
    assert!(ready.is_ready());
    assert!(active.is_active());
}

#[test]
fn test_invariant_validation_methods_enforce_rules() {
    let workspace_name = WorkspaceName::parse("test").expect("valid name");
    let workspace = Workspace::create(workspace_name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    // Validate ready fails for creating workspace
    assert!(matches!(
        workspace.validate_ready(),
        Err(WorkspaceError::NotReady(_))
    ));

    // Validate active fails for creating workspace
    assert!(matches!(
        workspace.validate_active(),
        Err(WorkspaceError::NotActive(_))
    ));

    let ready = workspace.mark_ready().expect("transition valid");

    // Validate ready succeeds for ready workspace
    assert!(ready.validate_ready().is_ok());

    // Validate active fails for ready workspace
    assert!(matches!(
        ready.validate_active(),
        Err(WorkspaceError::NotActive(_))
    ));
}
