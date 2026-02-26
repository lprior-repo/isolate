#![allow(clippy::uninlined_format_args, clippy::useless_format, clippy::unnecessary_wraps)]
//! BDD Test: Session Creation Scenario
//!
//! Feature: Session Creation
//!   As a Isolate user
//!   I want to create sessions with valid names
//!   So that I can manage my workspaces.
//!
//! This test demonstrates the BDD (Given/When/Then) pattern for testing
//! session creation functionality with zero panics and Result-based error handling.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::path::PathBuf;

mod test_foundation;

// Re-export for convenience
#[allow(unused_imports)]
pub use test_foundation::bdd::{BddContext, BddError, ScenarioBuilder, StepType};

/// Session creation result type
#[derive(Debug, Clone)]
pub struct SessionCreationResult {
    /// The session name
    pub name: String,
    /// The workspace path
    pub workspace_path: PathBuf,
    /// Whether creation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Create a session with the given name.
///
/// # Arguments
///
/// * `name` - The session name
/// * `repo_path` - The repository path
///
/// # Errors
///
/// Returns an error string if session creation fails.
pub fn create_session(
    name: &str,
    repo_path: &std::path::Path,
) -> Result<SessionCreationResult, String> {
    // Validate the session name
    if name.is_empty() {
        return Err("session name cannot be empty".to_string());
    }

    if name.len() > 64 {
        return Err(format!(
            "session name exceeds maximum length of 64 characters"
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "session name must contain only alphanumeric characters, hyphens, and underscores"
                .to_string(),
        );
    }

    // In a real implementation, this would create the workspace
    let workspace_path = repo_path.join("workspaces").join(name);

    Ok(SessionCreationResult {
        name: name.to_string(),
        workspace_path,
        success: true,
        error: None,
    })
}

/// Verify a session was created successfully.
///
/// # Arguments
///
/// * `result` - The session creation result
///
/// # Errors
///
/// Returns an error string if verification fails.
pub fn verify_session_created(result: &SessionCreationResult) -> Result<(), String> {
    if !result.success {
        return result.error.clone().map_or_else(
            || Err("session creation failed with unknown error".to_string()),
            Err,
        );
    }

    if result.name.is_empty() {
        return Err("session name should not be empty after creation".to_string());
    }

    Ok(())
}

// ============================================================================
// BDD SCENARIOS
// ============================================================================

/// Scenario: Create session with valid name
///
/// GIVEN a valid session name "feature-123"
/// WHEN I create a session with that name
/// THEN the session should be created successfully
/// AND the session name should match the input
#[test]
fn bdd_create_session_with_valid_name() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "feature-123";
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(session_name, &repo_path)?;

    // Assert (Then)
    verify_session_created(&result)?;
    assert_eq!(result.name, session_name, "Session name should match input");
    assert!(
        result
            .workspace_path
            .to_string_lossy()
            .contains(session_name),
        "Workspace path should contain session name"
    );

    Ok(())
}

/// Scenario: Create session with name containing underscores
///
/// GIVEN a valid session name `"my_feature_branch"`
/// WHEN I create a session with that name
/// THEN the session should be created successfully
#[test]
fn bdd_create_session_with_underscores() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "my_feature_branch";
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(session_name, &repo_path)?;

    // Assert (Then)
    verify_session_created(&result)?;
    assert_eq!(result.name, session_name);

    Ok(())
}

/// Scenario: Reject session with empty name
///
/// GIVEN an empty session name ""
/// WHEN I attempt to create a session with that name
/// THEN the creation should fail
/// AND an appropriate error message should be returned
#[test]
fn bdd_reject_empty_session_name() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "";
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(session_name, &repo_path);

    // Assert (Then)
    assert!(result.is_err(), "Empty session name should be rejected");
    let error = result.expect_err("should be error");
    assert!(
        error.contains("empty"),
        "Error message should mention 'empty': {}",
        error
    );

    Ok(())
}

/// Scenario: Reject session with name too long
///
/// GIVEN a session name with 65 characters
/// WHEN I attempt to create a session with that name
/// THEN the creation should fail
/// AND an appropriate error message should be returned
#[test]
fn bdd_reject_too_long_session_name() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "a".repeat(65);
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(&session_name, &repo_path);

    // Assert (Then)
    assert!(result.is_err(), "Too long session name should be rejected");
    let error = result.expect_err("should be error");
    assert!(
        error.contains("exceeds") || error.contains("length"),
        "Error message should mention length constraint: {}",
        error
    );

    Ok(())
}

/// Scenario: Reject session with special characters
///
/// GIVEN a session name with special characters "session@name!"
/// WHEN I attempt to create a session with that name
/// THEN the creation should fail
/// AND an appropriate error message should be returned
#[test]
fn bdd_reject_special_characters() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "session@name!";
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(session_name, &repo_path);

    // Assert (Then)
    assert!(
        result.is_err(),
        "Session name with special characters should be rejected"
    );
    let error = result.expect_err("should be error");
    assert!(
        error.contains("alphanumeric") || error.contains("invalid"),
        "Error message should mention character constraints: {}",
        error
    );

    Ok(())
}

/// Scenario: Session name at boundary length (exactly 64 chars)
///
/// GIVEN a session name with exactly 64 characters
/// WHEN I create a session with that name
/// THEN the session should be created successfully
#[test]
fn bdd_accept_exactly_max_length_name() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "a".repeat(64);
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(&session_name, &repo_path)?;

    // Assert (Then)
    verify_session_created(&result)?;
    assert_eq!(result.name.len(), 64);

    Ok(())
}

/// Scenario: Session name with mixed valid characters
///
/// GIVEN a session name "Feature-123_test"
/// WHEN I create a session with that name
/// THEN the session should be created successfully
#[test]
fn bdd_accept_mixed_valid_characters() -> Result<(), String> {
    // Arrange (Given)
    let session_name = "Feature-123_test";
    let repo_path = std::env::temp_dir();

    // Act (When)
    let result = create_session(session_name, &repo_path)?;

    // Assert (Then)
    verify_session_created(&result)?;
    assert_eq!(result.name, session_name);

    Ok(())
}

// ============================================================================
// BDD Context-based tests (alternative pattern)
// ============================================================================

#[test]
fn test_bdd_context_usage() -> Result<(), String> {
    let mut ctx = BddContext::new();

    // Given: Store the session name
    ctx.set("test-session");

    // When: Retrieve and process
    let name = ctx.get().ok_or("context should have value")?;

    // Then: Verify
    assert_eq!(name, "test-session");

    Ok(())
}

/// Test the scenario builder pattern
#[test]
fn test_scenario_builder() {
    let scenario = ScenarioBuilder::new("Create session with valid name")
        .given("a valid session name")
        .when("I create a session")
        .then("the session should exist")
        .build();

    assert!(scenario.contains("Create session with valid name"));
    assert!(scenario.contains("Given: a valid session name"));
    assert!(scenario.contains("When: I create a session"));
    assert!(scenario.contains("Then: the session should exist"));
}
