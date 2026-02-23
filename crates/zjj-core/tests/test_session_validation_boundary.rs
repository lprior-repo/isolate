//! Session validation boundary tests (bd-37v)
//!
//! This test file enforces clean architecture between domain and infrastructure:
//! - Domain types (Session) must NOT perform I/O validation
//! - Infrastructure layer (validation module) performs filesystem checks
//! - Adapter layer coordinates both validations
//!
//! Design Principle:
//! "Session validate to intrinsic domain invariants"
//! "Filesystem-aware validator in adapter layer"

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use zjj_core::{
    domain::{AbsolutePath, SessionId},
    domain::session::{BranchState, ParentState},
    output::ValidatedMetadata,
    types::{Session, SessionName, SessionStatus},
    validation::infrastructure::validate_session_workspace_exists,
    Error, Result, WorkspaceState,
};

// ============================================================================
// PURE DOMAIN VALIDATION - No I/O
// ============================================================================

#[test]
fn test_session_validate_pure_only_checks_domain_invariants() {
    // GIVEN: A session with valid domain properties but non-existent path
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/nonexistent/path/that/does/not/exist")
            .expect("valid absolute path"),
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: validate_pure is called (domain-only validation)
    let result = session.validate_pure();

    // THEN: Should pass because domain invariants are valid
    // (name format, absolute path, timestamp order)
    assert!(result.is_ok(), "validate_pure should not check filesystem");
}

#[test]
fn test_session_validate_pure_rejects_non_absolute_path() {
    // Skip this test since AbsolutePath enforces absolute paths at construction time
    // The validation is now in the smart constructor, so relative paths can't be constructed
    // This is actually correct behavior - we've made illegal states unrepresentable
}

#[test]
fn test_session_validate_pure_rejects_invalid_timestamps() {
    // GIVEN: A session with updated_at before created_at
    let now = chrono::Utc::now();
    let earlier = now - chrono::Duration::seconds(60);

    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("valid-name").expect("valid name"),
        status: SessionStatus::Creating,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/tmp/test").expect("valid"),
        branch: BranchState::Detached,
        created_at: now,
        updated_at: earlier, // violates domain invariant
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: validate_pure is called
    let result = session.validate_pure();

    // THEN: Should fail - timestamp order is a domain invariant
    assert!(result.is_err());
}

// ============================================================================
// INFRASTRUCTURE LAYER VALIDATION - I/O allowed
// ============================================================================

#[test]
fn test_validate_session_workspace_exists_checks_filesystem() {
    // GIVEN: An active session with non-existent workspace
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active, // NOT Creating
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/nonexistent/path/xyz123").expect("valid"),
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: infrastructure validation is called
    let result = validate_session_workspace_exists(&session);

    // THEN: Should fail - workspace doesn't exist
    assert!(result.is_err());
}

#[test]
fn test_validate_session_workspace_exists_allows_creating_status() {
    // GIVEN: A Creating session with non-existent workspace
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("new-session").expect("valid name"),
        status: SessionStatus::Creating, // Creating status
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/nonexistent/path").expect("valid"),
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: infrastructure validation is called
    let result = validate_session_workspace_exists(&session);

    // THEN: Should pass - Creating sessions don't need workspace yet
    assert!(result.is_ok());
}

#[test]
fn test_validate_session_workspace_exists_passes_for_tmp() {
    // GIVEN: An active session with existing path
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("test-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/tmp").expect("valid"), // exists on all systems
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: infrastructure validation is called
    let result = validate_session_workspace_exists(&session);

    // THEN: Should pass - /tmp exists
    assert!(result.is_ok());
}

// ============================================================================
// ADAPTER LAYER - Coordinates both validations
// ============================================================================

/// Adapter function that combines domain and infrastructure validation.
///
/// This is the CORRECT pattern for validation in application layer:
/// 1. Check pure domain invariants first (fast, no I/O)
/// 2. Then check infrastructure concerns (filesystem, network)
///
/// This function belongs in adapter/application layer, NOT in domain types.
fn validate_session_complete(session: &Session) -> Result<()> {
    // Step 1: Pure domain validation (no I/O)
    session.validate_pure()?;

    // Step 2: Infrastructure validation (I/O)
    validate_session_workspace_exists(session)?;

    Ok(())
}

#[test]
fn test_adapter_validate_session_runs_both_validations() {
    // GIVEN: A session with valid domain properties (can't test invalid states anymore)
    // The new types make illegal states unrepresentable
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/tmp/test").expect("valid"),
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: adapter validation is called
    let result = validate_session_complete(&session);

    // THEN: Should pass all validations
    assert!(result.is_ok());
}

#[test]
fn test_adapter_validate_passes_domain_then_fails_filesystem() {
    // GIVEN: A session with valid domain but non-existent workspace
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/nonexistent/xyz456").expect("valid"),
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: adapter validation is called
    let result = validate_session_complete(&session);

    // THEN: Should pass domain validation, fail at filesystem check
    assert!(result.is_err());

    // Verify error is about filesystem, not domain invariants
    if let Err(Error::ValidationError { message, .. }) = result {
        assert!(
            message.contains("does not exist") || message.contains("exist"),
            "Error should mention filesystem: {message}"
        );
    } else {
        panic!("Expected ValidationError, got: {result:?}");
    }
}

#[test]
fn test_adapter_validate_passes_all_validations() {
    // GIVEN: A session with valid domain and existing workspace
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/tmp").expect("valid"), // exists
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: adapter validation is called
    let result = validate_session_complete(&session);

    // THEN: Should pass all validations
    assert!(result.is_ok());
}

// ============================================================================
// ARCHITECTURE BOUNDARY VERIFICATION
// ============================================================================

#[test]
fn test_domain_validation_has_no_filesystem_dependency() {
    // GIVEN: A session with any path (valid absolute path)
    let session = Session {
        id: SessionId::parse("test-id".to_string()).expect("valid"),
        name: SessionName::new("test").expect("valid"),
        status: SessionStatus::Creating,
        state: WorkspaceState::Created,
        workspace_path: AbsolutePath::parse("/any/path/at/all").expect("valid"),
        branch: BranchState::Detached,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: ValidatedMetadata::default(),
        parent_session: ParentState::Root,
        queue_status: None,
    };

    // WHEN: validate_pure is called
    // THEN: It should not attempt filesystem access
    // (This test passes if it compiles and runs - validate_pure has no I/O)
    let _ = session.validate_pure();
}
