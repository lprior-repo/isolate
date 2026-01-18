//! Validation logic for the add command
//!
//! This module orchestrates the validation flow for creating a new session.
//! All validation logic has been extracted into specialized validator modules:
//! - `validators::name` - Session name format and rules
//! - `validators::exists` - Session existence in database
//! - `validators::workspace` - JJ workspace availability
//! - `validators::zellij` - Zellij running and accessible
//! - `validators::dependencies` - Required commands installed
//!
//! This module follows the Functional Core/Imperative Shell architecture,
//! delegating pure validation logic to specialized modules.

use std::path::Path;

use anyhow::Result;

use super::validators;

pub use validators::{
    validate_dependencies, validate_not_exists, validate_session_name,
    validate_workspace_available, validate_zellij_running,
};

/// Validate all prerequisites for creating a new session
///
/// This is a convenience function that runs all validation checks in sequence.
/// It delegates to specialized validators in the validators module.
///
/// # Arguments
/// * `name` - Session name to validate
/// * `session_db` - Database connection for checking existing sessions
/// * `repo_root` - Repository root path for workspace validation
/// * `no_open` - If true, skip Zellij validation (won't open tab)
///
/// # Errors
/// Returns the first validation error encountered
pub async fn validate_all(
    name: &str,
    session_db: &crate::database::SessionDb,
    repo_root: &Path,
    no_open: bool,
) -> Result<()> {
    validate_session_name(name)?;
    validate_not_exists(session_db, name).await?;
    validate_workspace_available(repo_root, name)?;

    // Only validate Zellij if we're going to open a tab
    if !no_open {
        validate_zellij_running()?;
    }

    validate_dependencies()?;
    Ok(())
}

// Tests for individual validators are located in their respective modules:
// - validators::name::tests - Session name validation tests
// - validators::exists::tests - Session existence checks
// - validators::workspace::tests - Workspace availability tests
// - validators::zellij::tests - Zellij availability tests
// - validators::dependencies::tests - Dependency checks
//
// Integration tests for validate_all are in the integration test suite.
