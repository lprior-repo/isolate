//! Exhaustive state machine transition tests
//!
//! Tests all valid and invalid state transitions for all domain state machines.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use zjj_core::domain::aggregates::{
    queue_entry::QueueEntryMetadata, Bead, QueueEntry, Session, Workspace,
};
use zjj_core::domain::identifiers::{
    AgentId, BeadId, SessionId, SessionName, WorkspaceName,
};
use zjj_core::domain::queue::ClaimState;
use zjj_core::domain::session::{BranchState, ParentState};
use zjj_core::domain::workspace::WorkspaceState;

// ============================================================================
// BEAD STATE MACHINE TRANSITIONS
// ============================================================================

#[test]
fn test_bead_state_open_transitions() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");

    assert!(bead.is_open());

    // Open -> InProgress (valid)
    let in_progress = bead.start();
    assert!(in_progress.is_ok());

    // Open -> Blocked (valid)
    let blocked = bead.block();
    assert!(blocked.is_ok());

    // Open -> Deferred (valid)
    let deferred = bead.defer();
    assert!(deferred.is_ok());

    // Open -> Closed (valid)
    let closed = bead.close();
    assert!(closed.is_ok());
}

#[test]
fn test_bead_state_in_progress_transitions() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");
    let in_progress = bead.start().expect("transition valid");

    assert!(in_progress.is_in_progress());

    // InProgress -> Open (valid)
    let open = in_progress.open();
    assert!(open.is_ok());

    // InProgress -> Blocked (valid)
    let blocked = in_progress.block();
    assert!(blocked.is_ok());

    // InProgress -> Deferred (valid)
    let deferred = in_progress.defer();
    assert!(deferred.is_ok());

    // InProgress -> Closed (valid)
    let closed = in_progress.close();
    assert!(closed.is_ok());
}

#[test]
fn test_bead_state_blocked_transitions() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");
    let blocked = bead.start().and_then(|b| b.block()).expect("transition valid");

    assert!(blocked.is_blocked());

    // Blocked -> Open (valid)
    let open = blocked.open();
    assert!(open.is_ok());

    // Blocked -> InProgress (valid)
    let in_progress = blocked.start();
    assert!(in_progress.is_ok());

    // Blocked -> Deferred (valid)
    let deferred = blocked.defer();
    assert!(deferred.is_ok());

    // Blocked -> Closed (valid)
    let closed = blocked.close();
    assert!(closed.is_ok());
}

#[test]
fn test_bead_state_deferred_transitions() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");
    let deferred = bead.start().and_then(|b| b.defer()).expect("transition valid");

    assert!(deferred.is_deferred());

    // Deferred -> Open (valid)
    let open = deferred.open();
    assert!(open.is_ok());

    // Deferred -> InProgress (valid)
    let in_progress = deferred.start();
    assert!(in_progress.is_ok());

    // Deferred -> Blocked (valid)
    let blocked = deferred.block();
    assert!(blocked.is_ok());

    // Deferred -> Closed (valid)
    let closed = deferred.close();
    assert!(closed.is_ok());
}

#[test]
fn test_bead_state_closed_is_terminal() {
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");
    let closed = bead.close().expect("transition valid");

    assert!(closed.is_closed());

    // Closed is terminal - no transitions allowed
    assert!(matches!(closed.open(), Err(zjj_core::domain::aggregates::BeadError::CannotModifyClosed)));
    assert!(matches!(closed.start(), Err(zjj_core::domain::aggregates::BeadError::CannotModifyClosed)));
    assert!(matches!(closed.block(), Err(zjj_core::domain::aggregates::BeadError::CannotModifyClosed)));
    assert!(matches!(closed.defer(), Err(zjj_core::domain::aggregates::BeadError::CannotModifyClosed)));
    assert!(matches!(closed.close(), Err(zjj_core::domain::aggregates::BeadError::CannotModifyClosed)));
}

#[test]
fn test_bead_state_all_possible_transitions() {
    // Test that all valid state transitions are reachable
    let id = BeadId::parse("bd-abc123").expect("valid id");
    let bead = Bead::new(id, "Test", None::<String>).expect("valid bead");

    // Open -> InProgress -> Blocked -> Deferred -> Open -> Closed
    let path1 = bead
        .start()
        .and_then(|b| b.block())
        .and_then(|b| b.defer())
        .and_then(|b| b.open())
        .and_then(|b| b.close());

    assert!(path1.is_ok());

    // Open -> Blocked -> InProgress -> Deferred -> Closed
    let id2 = BeadId::parse("bd-abc1232").expect("valid id");
    let bead2 = Bead::new(id2, "Test", None::<String>).expect("valid bead");

    let path2 = bead2
        .block()
        .and_then(|b| b.start())
        .and_then(|b| b.defer())
        .and_then(|b| b.close());

    assert!(path2.is_ok());
}

// ============================================================================
// SESSION BRANCH STATE TRANSITIONS
// ============================================================================

#[test]
fn test_branch_state_detached_transitions() {
    let id = SessionId::parse("sess-branch-detached").expect("valid id");
    let name = SessionName::parse("test").expect("valid name");

    let session = Session::new_root(
        id,
        name,
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // Detached -> OnBranch (valid)
    let on_main = session.transition_branch(BranchState::OnBranch {
        name: "main".to_string(),
    });
    assert!(on_main.is_ok());

    // Detached -> Detached (invalid, no self-loop)
    let result = session.transition_branch(BranchState::Detached);
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::SessionError::InvalidBranchTransition { .. })
    ));
}

#[test]
fn test_branch_state_on_branch_transitions() {
    let id = SessionId::parse("sess-branch-on").expect("valid id");
    let name = SessionName::parse("test").expect("valid name");

    let session = Session::new_root(
        id,
        name,
        BranchState::OnBranch {
            name: "main".to_string(),
        },
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // OnBranch -> Detached (valid)
    let detached = session.transition_branch(BranchState::Detached);
    assert!(detached.is_ok());

    // OnBranch -> OnBranch (switching branches, valid)
    let feature = session.transition_branch(BranchState::OnBranch {
        name: "feature".to_string(),
    });
    assert!(feature.is_ok());
}

#[test]
fn test_branch_state_all_transitions() {
    let id = SessionId::parse("sess-branch-all").expect("valid id");
    let name = SessionName::parse("test").expect("valid name");

    let session = Session::new_root(
        id,
        name,
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // Detached -> OnBranch(main) -> OnBranch(feature) -> Detached -> OnBranch(develop)
    let path = session
        .transition_branch(BranchState::OnBranch {
            name: "main".to_string(),
        })
        .and_then(|s| {
            s.transition_branch(BranchState::OnBranch {
                name: "feature".to_string(),
            })
        })
        .and_then(|s| s.transition_branch(BranchState::Detached))
        .and_then(|s| {
            s.transition_branch(BranchState::OnBranch {
                name: "develop".to_string(),
            })
        });

    assert!(path.is_ok());
}

// ============================================================================
// SESSION PARENT STATE TRANSITIONS
// ============================================================================

#[test]
fn test_parent_state_root_transitions() {
    let id = SessionId::parse("sess-parent-root").expect("valid id");
    let name = SessionName::parse("root").expect("valid name");

    let session = Session::new_root(
        id,
        name,
        BranchState::Detached,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    assert!(session.is_root());

    // Root -> ChildOf (invalid - root cannot become child)
    let new_parent = ParentState::ChildOf {
        parent: SessionName::parse("parent").expect("valid name"),
    };
    let result = session.transition_parent(new_parent);
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::SessionError::CannotModifyRootParent)
    ));

    // Root -> Root (invalid, no self-loop)
    let result = session.transition_parent(ParentState::Root);
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::SessionError::InvalidParentTransition { .. })
    ));
}

#[test]
fn test_parent_state_child_transitions() {
    let id = SessionId::parse("sess-parent-child").expect("valid id");
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

    assert!(session.is_child());

    // ChildOf -> Root (invalid)
    let result = session.transition_parent(ParentState::Root);
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::SessionError::InvalidParentTransition { .. })
    ));

    // ChildOf -> ChildOf (valid - change parent)
    let parent2 = SessionName::parse("parent2").expect("valid name");
    let new_parent = ParentState::ChildOf {
        parent: parent2.clone(),
    };
    let updated = session.transition_parent(new_parent);
    assert!(updated.is_ok());
    assert_eq!(updated.unwrap().parent_name(), Some(&parent2));
}

#[test]
fn test_parent_state_adoption_chain() {
    // Test that a child can be adopted multiple times
    let id = SessionId::parse("sess-adoption").expect("valid id");
    let name = SessionName::parse("child").expect("valid name");

    let parent1 = SessionName::parse("parent1").expect("valid name");
    let parent2 = SessionName::parse("parent2").expect("valid name");
    let parent3 = SessionName::parse("parent3").expect("valid name");

    let session = Session::new_child(
        id,
        name,
        BranchState::Detached,
        parent1,
        std::path::PathBuf::from("/tmp"),
    )
    .expect("session created");

    // Adopted by parent2
    let session = session
        .transition_parent(ParentState::ChildOf {
            parent: parent2.clone(),
        })
        .expect("adoption 1 valid");

    // Adopted by parent3
    let session = session
        .transition_parent(ParentState::ChildOf {
            parent: parent3.clone(),
        })
        .expect("adoption 2 valid");

    assert_eq!(session.parent_name(), Some(&parent3));
}

// ============================================================================
// WORKSPACE STATE TRANSITIONS
// ============================================================================

#[test]
fn test_workspace_state_creating_transitions() {
    let name = WorkspaceName::parse("test-creating").expect("valid name");
    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    assert!(workspace.is_creating());

    // Creating -> Ready (valid)
    let ready = workspace.mark_ready();
    assert!(ready.is_ok());

    // Creating -> Removed (valid)
    let removed = workspace.mark_removed();
    assert!(removed.is_ok());

    // Creating -> Active (invalid - must go through Ready first)
    assert!(!workspace.state.can_transition_to(&WorkspaceState::Active));
    let result = workspace.mark_active();
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::WorkspaceError::InvalidStateTransition { .. })
    ));

    // Creating -> Cleaning (invalid)
    assert!(!workspace.state.can_transition_to(&WorkspaceState::Cleaning));
    let result = workspace.start_cleaning();
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::WorkspaceError::InvalidStateTransition { .. })
    ));
}

#[test]
fn test_workspace_state_ready_transitions() {
    let name = WorkspaceName::parse("test-ready").expect("valid name");
    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");
    let ready = workspace.mark_ready().expect("transition valid");

    assert!(ready.is_ready());

    // Ready -> Active (valid)
    let active = ready.mark_active();
    assert!(active.is_ok());

    // Ready -> Cleaning (valid)
    let cleaning = ready.start_cleaning();
    assert!(cleaning.is_ok());

    // Ready -> Removed (valid)
    let removed = ready.mark_removed();
    assert!(removed.is_ok());

    // Ready -> Ready (invalid - no self-loop)
    assert!(!ready.state.can_transition_to(&WorkspaceState::Ready));

    // Ready -> Creating (invalid)
    assert!(!ready.state.can_transition_to(&WorkspaceState::Creating));
}

#[test]
fn test_workspace_state_active_transitions() {
    let name = WorkspaceName::parse("test-active").expect("valid name");
    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");
    let active = workspace
        .mark_ready()
        .and_then(|w| w.mark_active())
        .expect("transitions valid");

    assert!(active.is_active());

    // Active -> Cleaning (valid)
    let cleaning = active.start_cleaning();
    assert!(cleaning.is_ok());

    // Active -> Removed (valid)
    let removed = active.mark_removed();
    assert!(removed.is_ok());

    // Active -> Ready (invalid - backwards transition)
    assert!(!active.state.can_transition_to(&WorkspaceState::Ready));

    // Active -> Creating (invalid)
    assert!(!active.state.can_transition_to(&WorkspaceState::Creating));
}

#[test]
fn test_workspace_state_cleaning_transitions() {
    let name = WorkspaceName::parse("test-cleaning").expect("valid name");
    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");
    let cleaning = workspace
        .mark_ready()
        .and_then(|w| w.mark_active())
        .and_then(|w| w.start_cleaning())
        .expect("transitions valid");

    assert!(cleaning.is_cleaning());

    // Cleaning -> Removed (valid)
    let removed = cleaning.mark_removed();
    assert!(removed.is_ok());

    // Cleaning -> Cleaning (invalid - no self-loop)
    assert!(!cleaning.state.can_transition_to(&WorkspaceState::Cleaning));

    // All other transitions invalid
    assert!(!cleaning.state.can_transition_to(&WorkspaceState::Creating));
    assert!(!cleaning.state.can_transition_to(&WorkspaceState::Ready));
    assert!(!cleaning.state.can_transition_to(&WorkspaceState::Active));
}

#[test]
fn test_workspace_state_removed_is_terminal() {
    let name = WorkspaceName::parse("test-removed").expect("valid name");
    let workspace = Workspace::create(name, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");
    let removed = workspace.mark_removed().expect("transition valid");

    assert!(removed.is_removed());
    assert!(removed.is_terminal());

    // Removed is terminal - no transitions allowed
    for state in WorkspaceState::all() {
        assert!(!removed.state.can_transition_to(&state));
    }

    // Trying to use the transition methods should fail
    assert!(removed.mark_ready().is_err());
    assert!(removed.mark_active().is_err());
    assert!(removed.start_cleaning().is_err());
}

#[test]
fn test_workspace_state_all_valid_paths() {
    // Test all valid state transition paths

    // Path 1: Creating -> Ready -> Active -> Cleaning -> Removed
    let name1 = WorkspaceName::parse("test-path1").expect("valid name");
    let workspace1 = Workspace::create(name1, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    let path1 = workspace1
        .mark_ready()
        .and_then(|w| w.mark_active())
        .and_then(|w| w.start_cleaning())
        .and_then(|w| w.mark_removed());

    assert!(path1.is_ok());

    // Path 2: Creating -> Ready -> Cleaning -> Removed
    let name2 = WorkspaceName::parse("test-path2").expect("valid name");
    let workspace2 = Workspace::create(name2, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    let path2 = workspace2
        .mark_ready()
        .and_then(|w| w.start_cleaning())
        .and_then(|w| w.mark_removed());

    assert!(path2.is_ok());

    // Path 3: Creating -> Ready -> Removed
    let name3 = WorkspaceName::parse("test-path3").expect("valid name");
    let workspace3 = Workspace::create(name3, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    let path3 = workspace3.mark_ready().and_then(|w| w.mark_removed());

    assert!(path3.is_ok());

    // Path 4: Creating -> Removed
    let name4 = WorkspaceName::parse("test-path4").expect("valid name");
    let workspace4 = Workspace::create(name4, std::path::PathBuf::from("/tmp"))
        .expect("workspace created");

    let path4 = workspace4.mark_removed();

    assert!(path4.is_ok());
}

// ============================================================================
// QUEUE CLAIM STATE TRANSITIONS
// ============================================================================

#[test]
fn test_claim_state_unclaimed_transitions() {
    let workspace = WorkspaceName::parse("test-unclaimed").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    assert!(entry.is_unclaimed());

    // Unclaimed -> Claimed (valid)
    let claimed = entry.claim(agent.clone(), 300);
    assert!(claimed.is_ok());

    // Unclaimed -> Unclaimed (invalid - no self-loop)
    let result = entry.reclaim();
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::QueueEntryError::InvalidClaimTransition { .. })
    ));
}

#[test]
fn test_claim_state_claimed_transitions() {
    let workspace = WorkspaceName::parse("test-claimed").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");
    let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

    assert!(claimed.is_claimed());

    // Claimed -> Unclaimed (valid - release)
    let released = claimed.release(&agent);
    assert!(released.is_ok());
    assert!(released.unwrap().is_unclaimed());

    // Claimed -> Expired (valid - expire)
    let expired = claimed.expire_claim();
    assert!(expired.is_ok());
    assert!(expired.unwrap().is_expired());

    // Claimed -> Claimed (invalid - no self-loop, use refresh instead)
    let result = claimed.claim(agent, 300);
    assert!(matches!(
        result,
        Err(zjj_core::domain::aggregates::QueueEntryError::AlreadyClaimed(_))
    ));
}

#[test]
fn test_claim_state_expired_transitions() {
    let workspace = WorkspaceName::parse("test-expired").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    // Test Expired -> Unclaimed (valid - reclaim)
    let entry1 = QueueEntry::new(1, workspace.clone(), None, 0).expect("entry created");
    let claimed1 = entry1.claim(agent.clone(), 300).expect("claim valid");
    let expired1 = claimed1.expire_claim().expect("expire valid");

    assert!(expired1.is_expired());

    let reclaimed = expired1.reclaim();
    assert!(reclaimed.is_ok());
    assert!(reclaimed.unwrap().is_unclaimed());

    // Test Expired -> Expired (invalid - no self-loop)
    let entry2 = QueueEntry::new(2, workspace.clone(), None, 0).expect("entry created");
    let claimed2 = entry2.claim(agent.clone(), 300).expect("claim valid");
    let expired2 = claimed2.expire_claim().expect("expire valid");

    let result = expired2.expire_claim();
    // The result should be an error - checking just that it's an error
    assert!(result.is_err());

    // Test Expired -> Claimed (invalid - must reclaim first)
    // Expired entries have AlreadyClaimed error because they still have a claim holder
    let agent2 = AgentId::parse("agent-2").expect("valid agent");
    let result = expired2.claim(agent2, 300);
    // The result is AlreadyClaimed because is_unclaimed() returns false for Expired
    assert!(result.is_err());
}

#[test]
fn test_claim_state_claim_lifecycle() {
    let workspace = WorkspaceName::parse("test-lifecycle").expect("valid name");
    let agent1 = AgentId::parse("agent-1").expect("valid agent");
    let agent2 = AgentId::parse("agent-2").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");

    // Unclaimed -> Claimed by agent1
    let claimed1 = entry.claim(agent1.clone(), 300).expect("claim 1");
    assert!(claimed1.is_claimed());

    // Claimed -> Unclaimed (release)
    let released = claimed1.release(&agent1).expect("release");
    assert!(released.is_unclaimed());

    // Unclaimed -> Claimed by agent2
    let claimed2 = released.claim(agent2.clone(), 300).expect("claim 2");
    assert!(claimed2.is_claimed());

    // Claimed -> Expired
    let expired = claimed2.expire_claim().expect("expire");
    assert!(expired.is_expired());

    // Expired -> Unclaimed (reclaim)
    let reclaimed = expired.reclaim().expect("reclaim");
    assert!(reclaimed.is_unclaimed());

    // Unclaimed -> Claimed again
    let claimed3 = reclaimed.claim(agent1, 600).expect("claim 3");
    assert!(claimed3.is_claimed());
}

#[test]
fn test_claim_state_refresh_is_not_transition() {
    let workspace = WorkspaceName::parse("test-refresh").expect("valid name");
    let agent = AgentId::parse("agent-1").expect("valid agent");

    let entry = QueueEntry::new(1, workspace, None, 0).expect("entry created");
    let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

    // Refresh extends the claim but doesn't change state
    let refreshed = claimed.refresh_claim(&agent, 600).expect("refresh");

    assert!(refreshed.is_claimed());
    assert_eq!(refreshed.claim_holder(), Some(&agent));
}

// ============================================================================
// EXHAUSTIVE TRANSITION COVERAGE
// ============================================================================

#[test]
fn test_workspace_state_coverage() {
    // Test that we can reach all valid transitions from each state
    let states = WorkspaceState::all();

    for &from in &states {
        for &to in &states {
            let name = WorkspaceName::parse("test").expect("valid name");
            let workspace = Workspace::reconstruct(
                name,
                std::path::PathBuf::from("/tmp"),
                from,
            )
            .expect("reconstruct valid");

            // Check if can_transition_to matches
            let expected_valid = from.can_transition_to(&to);

            if expected_valid {
                // For valid transitions, verify we can use the transition method
                let result = match to {
                    WorkspaceState::Ready => workspace.mark_ready(),
                    WorkspaceState::Active => workspace.mark_active(),
                    WorkspaceState::Cleaning => workspace.start_cleaning(),
                    WorkspaceState::Removed => workspace.mark_removed(),
                    WorkspaceState::Creating => {
                        // No method for this, but can_transition_to should be false anyway
                        continue;
                    }
                };

                assert!(
                    result.is_ok(),
                    "Expected valid transition: {:?} -> {:?}",
                    from,
                    to
                );
            }
        }
    }
}

#[test]
fn test_branch_state_coverage() {
    // Test all branch state transitions
    let states = vec![BranchState::Detached, BranchState::OnBranch {
        name: "main".to_string(),
    }];

    for from in &states {
        for to in &states {
            let id = SessionId::parse("test").expect("valid id");
            let name = SessionName::parse("test").expect("valid name");

            let session = Session::new_root(
                id,
                name,
                from.clone(),
                std::path::PathBuf::from("/tmp"),
            )
            .expect("session created");

            let result = session.transition_branch(to.clone());

            let expected_valid = from.can_transition_to(to);

            if expected_valid {
                assert!(
                    result.is_ok(),
                    "Expected valid transition: {:?} -> {:?}",
                    from,
                    to
                );
            } else {
                assert!(
                    result.is_err(),
                    "Expected invalid transition: {:?} -> {:?}",
                    from,
                    to
                );
            }
        }
    }
}

#[test]
fn test_parent_state_coverage() {
    // Test all parent state transitions
    let parent = SessionName::parse("parent").expect("valid name");

    let states = vec![
        ParentState::Root,
        ParentState::ChildOf {
            parent: parent.clone(),
        },
    ];

    for from in &states {
        for to in &states {
            let id = SessionId::parse("test").expect("valid id");
            let name = SessionName::parse("test").expect("valid name");

            let session = match from {
                ParentState::Root => Session::new_root(
                    id,
                    name,
                    BranchState::Detached,
                    std::path::PathBuf::from("/tmp"),
                )
                .expect("session created"),
                ParentState::ChildOf { parent: p } => Session::new_child(
                    id,
                    name,
                    BranchState::Detached,
                    p.clone(),
                    std::path::PathBuf::from("/tmp"),
                )
                .expect("session created"),
            };

            let result = session.transition_parent(to.clone());

            // Root has special handling
            if matches!(from, ParentState::Root) {
                if matches!(to, ParentState::ChildOf { .. }) {
                    assert!(matches!(
                        result,
                        Err(zjj_core::domain::aggregates::SessionError::CannotModifyRootParent)
                    ));
                }
            } else {
                let expected_valid = from.can_transition_to(to);
                if expected_valid {
                    assert!(
                        result.is_ok(),
                        "Expected valid transition: {:?} -> {:?}",
                        from,
                        to
                    );
                } else {
                    assert!(
                        result.is_err(),
                        "Expected invalid transition: {:?} -> {:?}",
                        from,
                        to
                    );
                }
            }
        }
    }
}

#[test]
fn test_claim_state_coverage() {
    // Test all claim state transitions
    let agent = AgentId::parse("agent-1").expect("valid agent");
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::seconds(300);

    let states = vec![
        ClaimState::Unclaimed,
        ClaimState::Claimed {
            agent: agent.clone(),
            claimed_at: now,
            expires_at: expires,
        },
        ClaimState::Expired {
            previous_agent: agent.clone(),
            expired_at: now,
        },
    ];

    for from in &states {
        for to in &states {
            let workspace = WorkspaceName::parse("test").expect("valid name");

            let entry = QueueEntry::reconstruct(
                workspace,
                None,
                0,
                from.clone(),
                QueueEntryMetadata::new(1, now),
            )
            .expect("reconstruct valid");

            // Perform the transition
            let result = match (from, to) {
                (ClaimState::Unclaimed, ClaimState::Claimed { .. }) => {
                    entry.claim(agent.clone(), 300).map(|_| ())
                }
                (ClaimState::Claimed { .. }, ClaimState::Unclaimed) => {
                    entry.release(&agent).map(|_| ())
                }
                (ClaimState::Claimed { .. }, ClaimState::Expired { .. }) => {
                    entry.expire_claim().map(|_| ())
                }
                (ClaimState::Expired { .. }, ClaimState::Unclaimed) => {
                    entry.reclaim().map(|_| ())
                }
                _ => continue, // Skip invalid combinations
            };

            let expected_valid = from.can_transition_to(to);

            if expected_valid {
                assert!(
                    result.is_ok(),
                    "Expected valid transition: {:?} -> {:?}",
                    from,
                    to
                );
            }
        }
    }
}
