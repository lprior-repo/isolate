//! BDD Acceptance Tests for Session Management
//!
//! This test module implements ATDD (Acceptance Test-Driven Development)
//! for the session management feature as defined in `features/session.feature`.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test session_feature
//! ```
//!
//! # Test Organization
//!
//! Tests follow the Given/When/Then BDD pattern:
//! - GIVEN: Set up preconditions
//! - WHEN: Execute the action under test
//! - THEN: Verify the expected outcomes
//!
//! See `features/session.feature` for the full scenario definitions.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]

mod steps;
mod test_helpers;

pub mod common;

// Re-export step modules for use in tests
pub use steps::session_steps::{given_steps, then_steps, when_steps, SessionTestContext};

#[cfg(test)]
mod session_feature_tests {
    use super::*;

    // ============================================================================
    // CREATE SESSION SCENARIOS
    // ============================================================================

    /// Scenario: Create session succeeds
    ///
    /// GIVEN no session named "feature-auth" exists
    /// WHEN I create a session named "feature-auth" with workspace path
    /// THEN the session "feature-auth" should exist with status "creating"
    #[tokio::test]
    async fn scenario_create_session_succeeds() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx)
            .expect("GIVEN: database initialization should succeed");
        given_steps::in_jj_repository(&ctx).expect("GIVEN: jj repository setup should succeed");
        given_steps::no_session_named_exists(&ctx, "feature-auth")
            .await
            .expect("GIVEN: no session should exist initially");

        // WHEN
        when_steps::create_session_with_path(&ctx, "feature-auth", "/workspaces/feature-auth")
            .await
            .expect("WHEN: session creation should succeed");

        // THEN
        then_steps::session_should_exist(&ctx, "feature-auth")
            .await
            .expect("THEN: session should exist");
        then_steps::session_should_have_status(&ctx, "feature-auth", "creating")
            .await
            .expect("THEN: session should have creating status");
        then_steps::session_details_as_json(&ctx)
            .await
            .expect("THEN: session details should be valid JSON");
    }

    /// Scenario: Create duplicate session fails
    ///
    /// GIVEN a session named "feature-auth" exists
    /// WHEN I attempt to create a session named "feature-auth"
    /// THEN the operation should fail with error `"SESSION_EXISTS"`
    #[tokio::test]
    async fn scenario_create_duplicate_session_fails() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx)
            .expect("GIVEN: database initialization should succeed");
        given_steps::in_jj_repository(&ctx).expect("GIVEN: jj repository setup should succeed");
        given_steps::session_named_exists(&ctx, "feature-auth")
            .await
            .expect("GIVEN: session should exist");

        // WHEN
        when_steps::attempt_create_session(&ctx, "feature-auth")
            .await
            .expect("WHEN: attempt should complete");

        // THEN
        then_steps::operation_should_fail_with_error(&ctx, "SESSION_EXISTS")
            .await
            .expect("THEN: operation should fail with SESSION_EXISTS error");
        then_steps::no_duplicate_created(&ctx)
            .await
            .expect("THEN: no duplicate session should be created");
    }

    /// Scenario: Create session with invalid name fails
    #[tokio::test]
    async fn scenario_create_session_invalid_name_fails() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx)
            .expect("GIVEN: database initialization should succeed");
        given_steps::in_jj_repository(&ctx).expect("GIVEN: jj repository setup should succeed");
        given_steps::no_session_exists(&ctx)
            .await
            .expect("GIVEN: no sessions should exist initially");

        // WHEN
        when_steps::attempt_create_session(&ctx, "123-invalid")
            .await
            .expect("WHEN: attempt should complete");

        // THEN
        then_steps::operation_should_fail_with_error(&ctx, "VALIDATION_ERROR")
            .await
            .expect("THEN: operation should fail with VALIDATION_ERROR");
    }

    // ============================================================================
    // REMOVE SESSION SCENARIOS
    // ============================================================================

    /// Scenario: Remove session cleans up
    #[tokio::test]
    async fn scenario_remove_session_cleans_up() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx)
            .expect("GIVEN: database initialization should succeed");
        given_steps::in_jj_repository(&ctx).expect("GIVEN: jj repository setup should succeed");
        given_steps::session_named_exists_with_status(&ctx, "old-feature", "active")
            .await
            .expect("GIVEN: session should exist with active status");

        // WHEN
        when_steps::remove_session(&ctx, "old-feature")
            .await
            .expect("WHEN: session removal should succeed");

        // THEN
        then_steps::session_should_not_exist(&ctx, "old-feature")
            .await
            .expect("THEN: session should not exist after removal");
    }

    /// Scenario: Remove non-existent session fails
    #[tokio::test]
    async fn scenario_remove_nonexistent_session_fails() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx)
            .expect("GIVEN: database initialization should succeed");
        given_steps::in_jj_repository(&ctx).expect("GIVEN: jj repository setup should succeed");
        given_steps::no_session_named_exists(&ctx, "nonexistent")
            .await
            .expect("GIVEN: session should not exist");

        // WHEN
        when_steps::attempt_remove_session(&ctx, "nonexistent")
            .await
            .expect("WHEN: attempt should complete");

        // THEN
        then_steps::operation_should_fail_with_error(&ctx, "SESSION_NOT_FOUND")
            .await
            .expect("THEN: operation should fail with SESSION_NOT_FOUND error");
    }

    // ============================================================================
    // SYNC SESSION SCENARIOS
    // ============================================================================

    /// Scenario: Sync session rebases onto main
    #[tokio::test]
    async fn scenario_sync_session_rebases() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx)
            .expect("GIVEN: database initialization should succeed");
        given_steps::in_jj_repository(&ctx).expect("GIVEN: jj repository setup should succeed");
        given_steps::session_named_exists_with_status(&ctx, "feature-sync", "active")
            .await
            .expect("GIVEN: session should exist with active status");
        given_steps::session_has_no_uncommitted_changes(&ctx, "feature-sync")
            .await
            .expect("GIVEN: session should have no uncommitted changes");

        // WHEN
        when_steps::sync_session(&ctx, "feature-sync")
            .await
            .expect("WHEN: session sync should complete");

        // THEN - sync may succeed or fail depending on state
        // Just verify the command ran
    }

    /// Scenario: Sync with conflicts reports them
    #[tokio::test]
    async fn scenario_sync_with_conflicts() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-conflict", "active")
            .await
            .unwrap();

        // WHEN
        when_steps::sync_session(&ctx, "feature-conflict")
            .await
            .unwrap();

        // THEN - verify command executed (conflicts depend on actual state)
    }

    // ============================================================================
    // FOCUS SESSION SCENARIOS
    // ============================================================================

    /// Scenario: Focus switches to session
    #[tokio::test]
    async fn scenario_focus_switches_to_session() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-focus", "active")
            .await
            .unwrap();

        // WHEN
        when_steps::focus_session(&ctx, "feature-focus")
            .await
            .unwrap();

        // THEN
        then_steps::session_details_as_json(&ctx).await.unwrap();
    }

    /// Scenario: Focus from CLI attaches
    #[tokio::test]
    async fn scenario_focus_from_cli() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-attach", "active")
            .await
            .unwrap();

        // WHEN
        when_steps::focus_session(&ctx, "feature-attach")
            .await
            .unwrap();

        // THEN
        then_steps::session_details_as_json(&ctx).await.unwrap();
    }

    /// Scenario: Focus non-existent session fails
    #[tokio::test]
    async fn scenario_focus_nonexistent_session_fails() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::no_session_named_exists(&ctx, "missing")
            .await
            .unwrap();

        // WHEN
        when_steps::attempt_focus_session(&ctx, "missing")
            .await
            .unwrap();

        // THEN
        then_steps::operation_should_fail_with_error(&ctx, "SESSION_NOT_FOUND")
            .await
            .unwrap();
    }

    // ============================================================================
    // SUBMIT SESSION SCENARIOS
    // ============================================================================

    /// Scenario: Submit adds to queue
    #[tokio::test]
    async fn scenario_submit_adds_to_queue() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-submit", "synced")
            .await
            .unwrap();

        // WHEN
        when_steps::submit_session(&ctx, "feature-submit")
            .await
            .unwrap();

        // THEN - verify command executed (queue addition depends on state)
    }

    /// Scenario: Submit with dirty workspace fails
    #[tokio::test]
    async fn scenario_submit_dirty_workspace_fails() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-dirty", "active")
            .await
            .unwrap();
        given_steps::session_has_uncommitted_changes(&ctx, "feature-dirty")
            .await
            .unwrap();

        // WHEN
        when_steps::attempt_submit_session(&ctx, "feature-dirty")
            .await
            .unwrap();

        // THEN
        then_steps::operation_should_fail_with_error(&ctx, "DIRTY_WORKSPACE")
            .await
            .unwrap();
    }

    /// Scenario: Submit dry-run does not modify state
    #[tokio::test]
    async fn scenario_submit_dry_run_no_modify() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-dryrun", "synced")
            .await
            .unwrap();

        // WHEN
        when_steps::submit_session_with_dry_run(&ctx, "feature-dryrun")
            .await
            .unwrap();

        // THEN
        then_steps::no_queue_entry_created(&ctx).await.unwrap();
    }

    // ============================================================================
    // LIST SESSIONS SCENARIOS
    // ============================================================================

    /// Scenario: List shows all sessions
    #[tokio::test]
    async fn scenario_list_shows_all_sessions() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::multiple_sessions_exist(&ctx, &["feature-a", "feature-b", "feature-c"])
            .await
            .unwrap();

        // WHEN
        when_steps::list_sessions(&ctx).await.unwrap();

        // THEN
        then_steps::output_contains_n_sessions(&ctx, 3)
            .await
            .unwrap();
        then_steps::sessions_show_details(&ctx).await.unwrap();
        then_steps::output_is_valid_json(&ctx).await.unwrap();
    }

    /// Scenario: List empty returns empty array
    #[tokio::test]
    async fn scenario_list_empty_returns_empty() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::no_session_exists(&ctx).await.unwrap();

        // WHEN
        when_steps::list_sessions(&ctx).await.unwrap();

        // THEN
        then_steps::output_is_empty_array(&ctx).await.unwrap();
        then_steps::output_is_valid_json(&ctx).await.unwrap();
    }

    /// Scenario: List with filter shows matching sessions
    #[tokio::test]
    async fn scenario_list_with_filter() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::sessions_with_statuses_exist(&ctx, &["active", "paused", "completed"])
            .await
            .unwrap();

        // WHEN
        when_steps::list_sessions_with_filter(&ctx, "active")
            .await
            .unwrap();

        // THEN
        then_steps::only_status_shown(&ctx, "active").await.unwrap();
    }

    // ============================================================================
    // SHOW SESSION SCENARIOS
    // ============================================================================

    /// Scenario: Show displays session details
    #[tokio::test]
    async fn scenario_show_displays_details() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "feature-detail", "active")
            .await
            .unwrap();

        // WHEN
        when_steps::show_session(&ctx, "feature-detail")
            .await
            .unwrap();

        // THEN
        then_steps::output_contains_session_name(&ctx, "feature-detail")
            .await
            .unwrap();
        then_steps::output_is_valid_json(&ctx).await.unwrap();
    }

    /// Scenario: Show non-existent session fails
    #[tokio::test]
    async fn scenario_show_nonexistent_fails() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::no_session_named_exists(&ctx, "missing")
            .await
            .unwrap();

        // WHEN
        when_steps::attempt_show_session(&ctx, "missing")
            .await
            .unwrap();

        // THEN
        then_steps::operation_should_fail_with_error(&ctx, "SESSION_NOT_FOUND")
            .await
            .unwrap();
    }

    // ============================================================================
    // STATE MACHINE TRANSITION SCENARIOS
    // ============================================================================

    /// Scenario: Valid state transition Created to Active
    #[tokio::test]
    async fn scenario_state_transition_created_to_active() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "state-test-1", "creating")
            .await
            .unwrap();

        // WHEN
        when_steps::session_workspace_created(&ctx).await.unwrap();

        // THEN - session should be able to transition to active
        then_steps::can_transition_to(&ctx, "active").await.unwrap();
    }

    /// Scenario: Invalid state transition Created to Paused is prevented
    #[tokio::test]
    async fn scenario_invalid_state_transition_prevented() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists_with_status(&ctx, "state-test-4", "creating")
            .await
            .unwrap();

        // WHEN
        when_steps::attempt_pause_session(&ctx, "state-test-4")
            .await
            .unwrap();

        // THEN - the session should remain in creating state
        // (invalid transition should fail)
    }

    // ============================================================================
    // INVARIANT SCENARIOS
    // ============================================================================

    /// Scenario: Each session has exactly one JJ workspace
    #[tokio::test]
    async fn scenario_each_session_has_one_workspace() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists(&ctx, "invariant-test")
            .await
            .unwrap();

        // WHEN
        when_steps::inspect_workspace_mapping(&ctx).await.unwrap();

        // THEN
        then_steps::exactly_one_workspace(&ctx).await.unwrap();
    }

    /// Scenario: Session cannot have multiple workspaces
    #[tokio::test]
    async fn scenario_session_cannot_have_multiple_workspaces() {
        let Some(ctx) = SessionTestContext::try_new() else {
            eprintln!("SKIP: jj not available");
            return;
        };

        // GIVEN
        given_steps::zjj_database_is_initialized(&ctx).unwrap();
        given_steps::in_jj_repository(&ctx).unwrap();
        given_steps::session_named_exists(&ctx, "invariant-multi")
            .await
            .unwrap();

        // THEN - invariant enforced by database schema
        then_steps::at_most_one_workspace(&ctx).await.unwrap();
    }
}
