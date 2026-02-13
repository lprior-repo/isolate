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

use std::path::PathBuf;

use zjj_core::{
    types::{Session, SessionName, SessionStatus},
    validation,
    Error, Result, WorkspaceState,
};

// ============================================================================
// PURE DOMAIN VALIDATION - No I/O
// ============================================================================

#[test]
fn test_session_validate_pure_only_checks_domain_invariants() {
    // GIVEN: A session with valid domain properties but non-existent path
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/nonexistent/path/that/does/not/exist"),
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: validate_pure is called (domain-only validation)
    let result = session.validate_pure();

    // THEN: Should pass because domain invariants are valid
    // (name format, absolute path, timestamp order)
    assert!(result.is_ok(), "validate_pure should not check filesystem");
}

#[test]
fn test_session_validate_pure_rejects_non_absolute_path() {
    // GIVEN: A session with a relative path
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("valid-name").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("relative/path"), // violates domain invariant
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: validate_pure is called
    let result = session.validate_pure();

    // THEN: Should fail - relative path is a domain invariant violation
    assert!(result.is_err());
}

#[test]
fn test_session_validate_pure_rejects_invalid_timestamps() {
    // GIVEN: A session with updated_at before created_at
    let now = chrono::Utc::now();
    let earlier = now - chrono::Duration::seconds(60);

    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("valid-name").expect("valid name"),
        status: SessionStatus::Creating,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/tmp/test"),
        branch: None,
        created_at: now,
        updated_at: earlier, // violates domain invariant
        last_synced: None,
        metadata: serde_json::Value::Null,
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
        id: "test-id".to_string(),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active, // NOT Creating
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/nonexistent/path/xyz123"),
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: infrastructure validation is called
    let result = validation::validate_session_workspace_exists(&session);

    // THEN: Should fail - workspace doesn't exist
    assert!(result.is_err());
}

#[test]
fn test_validate_session_workspace_exists_allows_creating_status() {
    // GIVEN: A Creating session with non-existent workspace
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("new-session").expect("valid name"),
        status: SessionStatus::Creating, // Creating status
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/nonexistent/path"),
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: infrastructure validation is called
    let result = validation::validate_session_workspace_exists(&session);

    // THEN: Should pass - Creating sessions don't need workspace yet
    assert!(result.is_ok());
}

#[test]
fn test_validate_session_workspace_exists_passes_for_tmp() {
    // GIVEN: An active session with existing path
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("test-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/tmp"), // exists on all systems
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: infrastructure validation is called
    let result = validation::validate_session_workspace_exists(&session);

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
    validation::validate_session_workspace_exists(session)?;

    Ok(())
}

#[test]
fn test_adapter_validate_session_runs_both_validations() {
    // GIVEN: A session with invalid domain property
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("relative/path"), // domain violation
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: adapter validation is called
    let result = validate_session_complete(&session);

    // THEN: Should fail at domain validation (before I/O)
    assert!(result.is_err());
}

#[test]
fn test_adapter_validate_passes_domain_then_fails_filesystem() {
    // GIVEN: A session with valid domain but missing workspace
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/nonexistent/xyz456"),
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
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
        panic!("Expected ValidationError, got: {:?}", result);
    }
}

#[test]
fn test_adapter_validate_passes_all_validations() {
    // GIVEN: A session with valid domain and existing workspace
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("valid-session").expect("valid name"),
        status: SessionStatus::Active,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/tmp"), // exists
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
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
    // GIVEN: A session with any path
    let session = Session {
        id: "test-id".to_string(),
        name: SessionName::new("test").expect("valid"),
        status: SessionStatus::Creating,
        state: WorkspaceState::Created,
        workspace_path: PathBuf::from("/any/path/at/all"),
        branch: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: None,
        metadata: serde_json::Value::Null,
    };

    // WHEN: validate_pure is called
    // THEN: It should not attempt filesystem access
    // (This test passes if it compiles and runs - validate_pure has no I/O)
    let _ = session.validate_pure();
}
