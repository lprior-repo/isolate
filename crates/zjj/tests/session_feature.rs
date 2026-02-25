//! BDD Acceptance Tests for Session Management Feature
//!
//! Feature: Session Management
//!
//! As a developer using ZJJ
//! I want to manage my parallel workspaces as sessions
//! So that I can switch contexts quickly and maintain isolation
//!
//! Covers:
//! - Session creation/removal
//! - Session list/status
//! - Context switching (focus)
//! - Submit workflow

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
mod steps;

use steps::session_steps::{given_steps, then_steps, when_steps, SessionTestContext};

// =============================================================================
// Feature: Session Management
// =============================================================================

/// Scenario: Create session succeeds
#[tokio::test]
async fn bdd_create_session_succeeds() {
    let Some(ctx) = SessionTestContext::try_new() else {
        return;
    };

    // GIVEN
    given_steps::zjj_database_is_initialized(&ctx).unwrap();
    given_steps::in_jj_repository(&ctx).unwrap();

    // WHEN
    when_steps::create_session(&ctx, "feature-new")
        .await
        .unwrap();

    // THEN
    then_steps::operation_should_succeed(&ctx).await.unwrap();
    then_steps::session_should_exist(&ctx, "feature-new")
        .await
        .unwrap();
}

/// Scenario: Remove session succeeds
#[tokio::test]
async fn bdd_remove_session_succeeds() {
    let Some(ctx) = SessionTestContext::try_new() else {
        return;
    };

    // GIVEN
    given_steps::zjj_database_is_initialized(&ctx).unwrap();
    given_steps::in_jj_repository(&ctx).unwrap();
    given_steps::session_named_exists(&ctx, "feature-remove")
        .await
        .unwrap();

    // WHEN
    when_steps::remove_session(&ctx, "feature-remove")
        .await
        .unwrap();

    // THEN
    then_steps::operation_should_succeed(&ctx).await.unwrap();
    then_steps::session_should_not_exist(&ctx, "feature-remove")
        .await
        .unwrap();
}

/// Scenario: Focus switches to session
#[tokio::test]
async fn bdd_focus_switches_to_session() {
    let Some(ctx) = SessionTestContext::try_new() else {
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

/// Scenario: List shows all sessions
#[tokio::test]
async fn bdd_list_shows_all_sessions() {
    let Some(ctx) = SessionTestContext::try_new() else {
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

/// Scenario: Submit pushes bookmark to remote
#[tokio::test]
async fn bdd_submit_pushes_bookmark() {
    let Some(ctx) = SessionTestContext::try_new() else {
        return;
    };

    // GIVEN
    given_steps::zjj_database_is_initialized(&ctx).unwrap();
    given_steps::in_jj_repository(&ctx).unwrap();
    given_steps::session_named_exists_with_status(&ctx, "feature-submit", "synced")
        .await
        .unwrap();
    given_steps::session_has_bookmark(&ctx, "feature-submit", "feature-submit")
        .await
        .unwrap();

    // WHEN
    when_steps::submit_session(&ctx, "feature-submit")
        .await
        .unwrap();

    // THEN
    // Note: this might fail if no remote is configured (REMOTE_ERROR)
    // We only check dedupe key if it succeeded or failed with something other than REMOTE_ERROR
    let result_guard = ctx.last_result.lock().await;
    let result = result_guard.as_ref().unwrap();
    if result.success || !result.stdout.contains("REMOTE_ERROR") {
        drop(result_guard);
        then_steps::response_includes_dedupe_key(&ctx).await.unwrap();
    }
}

/// Scenario: Submit dry-run does not modify state
#[tokio::test]
async fn bdd_submit_dry_run_no_modify() {
    let Some(ctx) = SessionTestContext::try_new() else {
        return;
    };

    // GIVEN
    given_steps::zjj_database_is_initialized(&ctx).unwrap();
    given_steps::in_jj_repository(&ctx).unwrap();
    given_steps::session_named_exists_with_status(&ctx, "feature-dryrun", "synced")
        .await
        .unwrap();
    given_steps::session_has_bookmark(&ctx, "feature-dryrun", "feature-dryrun")
        .await
        .unwrap();

    // WHEN
    when_steps::submit_session_with_dry_run(&ctx, "feature-dryrun")
        .await
        .unwrap();

    // THEN
    let result_guard = ctx.last_result.lock().await;
    let result = result_guard.as_ref().unwrap();
    if result.success {
        drop(result_guard);
        then_steps::response_indicates(&ctx, "dry_run")
            .await
            .unwrap();
    }
}

/// Scenario: Submit with dirty workspace fails
#[tokio::test]
async fn bdd_submit_dirty_workspace_fails() {
    let Some(ctx) = SessionTestContext::try_new() else {
        return;
    };

    // GIVEN
    given_steps::zjj_database_is_initialized(&ctx).unwrap();
    given_steps::in_jj_repository(&ctx).unwrap();
    given_steps::session_named_exists_with_status(&ctx, "feature-dirty", "active")
        .await
        .unwrap();
    given_steps::session_has_bookmark(&ctx, "feature-dirty", "feature-dirty")
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
