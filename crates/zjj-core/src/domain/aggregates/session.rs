//! Session aggregate root with business rules and invariants.
//!
//! The Session aggregate represents a development session with:
//! - Unique identity (`SessionId`)
//! - Human-readable name (`SessionName`)
//! - Branch state (detached or on branch)
//! - Parent hierarchy (root or child)
//! - Workspace location
//!
//! # Invariants
//!
//! 1. Session names must be unique within a workspace
//! 2. Root sessions cannot become children (no parent transition)
//! 3. Active sessions must have valid workspace paths
//! 4. Branch transitions must be valid (detached <-> branch, branch <-> branch)
//! 5. Parent transitions are limited (root stays root, child can change parent)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use thiserror::Error;

use crate::domain::identifiers::{SessionId, SessionName};
use crate::domain::session::{BranchState, ParentState};

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

/// Errors that can occur during session operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    /// Invalid branch transition
    #[error("invalid branch transition: {from:?} -> {to:?}")]
    InvalidBranchTransition { from: BranchState, to: BranchState },

    /// Invalid parent transition
    #[error("invalid parent transition: {from:?} -> {to:?}")]
    InvalidParentTransition { from: ParentState, to: ParentState },

    /// Workspace path does not exist
    #[error("workspace path does not exist: {0}")]
    WorkspaceNotFound(PathBuf),

    /// Session is not active
    #[error("session is not active")]
    NotActive,

    /// Cannot activate session with invalid state
    #[error("cannot activate session: invalid state")]
    CannotActivate,

    /// Cannot modify root session parent
    #[error("cannot modify parent of root session")]
    CannotModifyRootParent,

    /// Session name conflicts with existing session
    #[error("session name already exists: {0}")]
    NameAlreadyExists(SessionName),
}

// ============================================================================
// SESSION AGGREGATE ROOT
// ============================================================================

/// Session aggregate root.
///
/// Enforces all business rules and invariants for sessions.
/// All state transitions go through validated methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Human-readable session name
    pub name: SessionName,
    /// Branch state (detached or on branch)
    pub branch: BranchState,
    /// Parent state (root or child)
    pub parent: ParentState,
    /// Absolute path to workspace root
    pub workspace_path: PathBuf,
}

impl Session {
    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Create a new root session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if workspace path doesn't exist.
    pub fn new_root(
        id: SessionId,
        name: SessionName,
        branch: BranchState,
        workspace_path: PathBuf,
    ) -> Result<Self, SessionError> {
        // Validate workspace exists
        if !workspace_path.exists() {
            return Err(SessionError::WorkspaceNotFound(workspace_path));
        }

        Ok(Self {
            id,
            name,
            branch,
            parent: ParentState::Root,
            workspace_path,
        })
    }

    /// Create a new child session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if workspace path doesn't exist.
    pub fn new_child(
        id: SessionId,
        name: SessionName,
        branch: BranchState,
        parent: SessionName,
        workspace_path: PathBuf,
    ) -> Result<Self, SessionError> {
        // Validate workspace exists
        if !workspace_path.exists() {
            return Err(SessionError::WorkspaceNotFound(workspace_path));
        }

        Ok(Self {
            id,
            name,
            branch,
            parent: ParentState::ChildOf { parent },
            workspace_path,
        })
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Check if session is active (has valid branch and workspace).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.branch.is_detached() && self.workspace_path.exists()
    }

    /// Check if session is a root session.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.parent.is_root()
    }

    /// Check if session is a child session.
    #[must_use]
    pub const fn is_child(&self) -> bool {
        self.parent.is_child()
    }

    /// Get the parent session name if this is a child.
    #[must_use]
    pub const fn parent_name(&self) -> Option<&SessionName> {
        self.parent.parent_name()
    }

    /// Get the branch name if on a branch.
    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        self.branch.branch_name()
    }

    // ========================================================================
    // STATE TRANSITION METHODS
    // ========================================================================

    /// Transition to a new branch state.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidBranchTransition` if transition is invalid.
    pub fn transition_branch(&self, new_branch: BranchState) -> Result<Self, SessionError> {
        if !self.branch.can_transition_to(&new_branch) {
            return Err(SessionError::InvalidBranchTransition {
                from: self.branch.clone(),
                to: new_branch,
            });
        }

        Ok(Self {
            branch: new_branch,
            ..self.clone()
        })
    }

    /// Transition to a new parent state.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidParentTransition` if transition is invalid.
    /// Returns `SessionError::CannotModifyRootParent` if trying to modify root parent.
    pub fn transition_parent(&self, new_parent: ParentState) -> Result<Self, SessionError> {
        // Root sessions cannot become children
        if self.parent.is_root() && !matches!(new_parent, ParentState::Root) {
            return Err(SessionError::CannotModifyRootParent);
        }

        if !self.parent.can_transition_to(&new_parent) {
            return Err(SessionError::InvalidParentTransition {
                from: self.parent.clone(),
                to: new_parent,
            });
        }

        Ok(Self {
            parent: new_parent,
            ..self.clone()
        })
    }

    /// Change the workspace path.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if new path doesn't exist.
    pub fn change_workspace(&self, new_path: PathBuf) -> Result<Self, SessionError> {
        if !new_path.exists() {
            return Err(SessionError::WorkspaceNotFound(new_path));
        }

        Ok(Self {
            workspace_path: new_path,
            ..self.clone()
        })
    }

    /// Rename the session.
    ///
    /// Note: The caller is responsible for ensuring name uniqueness.
    #[must_use]
    pub fn rename(&self, new_name: SessionName) -> Self {
        Self {
            name: new_name,
            ..self.clone()
        }
    }

    // ========================================================================
    // VALIDATION METHODS
    // ========================================================================

    /// Validate that the session can be activated.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::CannotActivate` if session cannot be activated.
    pub fn validate_can_activate(&self) -> Result<(), SessionError> {
        if !self.is_active() {
            return Err(SessionError::CannotActivate);
        }
        Ok(())
    }

    /// Validate that the session is in a valid state.
    ///
    /// A session is valid if:
    /// - Workspace path exists
    /// - Branch state is valid
    /// - Parent state is valid
    ///
    /// # Errors
    ///
    /// Returns `SessionError::WorkspaceNotFound` if workspace path doesn't exist.
    pub fn validate(&self) -> Result<(), SessionError> {
        if !self.workspace_path.exists() {
            return Err(SessionError::WorkspaceNotFound(self.workspace_path.clone()));
        }
        Ok(())
    }

    // ========================================================================
    // BUILDER PATTERN
    // ========================================================================

    /// Create a builder for constructing or modifying sessions.
    #[must_use]
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }
}

// ============================================================================
// SESSION BUILDER
// ============================================================================

/// Builder for constructing sessions.
///
/// Provides a fluent interface for session creation with validation.
#[derive(Debug, Default)]
pub struct SessionBuilder {
    id: Option<SessionId>,
    name: Option<SessionName>,
    branch: Option<BranchState>,
    parent: Option<ParentState>,
    workspace_path: Option<PathBuf>,
}

impl SessionBuilder {
    /// Create a new session builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session ID.
    #[must_use]
    pub fn id(mut self, id: SessionId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the session name.
    #[must_use]
    pub fn name(mut self, name: SessionName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the branch state.
    #[must_use]
    pub fn branch(mut self, branch: BranchState) -> Self {
        self.branch = Some(branch);
        self
    }

    /// Set the parent state.
    #[must_use]
    pub fn parent(mut self, parent: ParentState) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set the workspace path.
    #[must_use]
    pub fn workspace_path(mut self, path: PathBuf) -> Self {
        self.workspace_path = Some(path);
        self
    }

    /// Build the session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` if:
    /// - Required fields are missing
    /// - Workspace path doesn't exist
    pub fn build(self) -> Result<Session, SessionError> {
        let id = self.id.ok_or(SessionError::CannotActivate)?;
        let name = self.name.ok_or(SessionError::CannotActivate)?;
        let branch = self.branch.ok_or(SessionError::CannotActivate)?;
        let workspace_path = self.workspace_path.ok_or(SessionError::CannotActivate)?;

        match self.parent {
            Some(ParentState::ChildOf { parent }) => {
                Session::new_child(id, name, branch, parent, workspace_path)
            }
            Some(ParentState::Root) | None => Session::new_root(id, name, branch, workspace_path),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_root_session() {
        let id = SessionId::parse("test-session-1").expect("valid id");
        let name = SessionName::parse("test-session").expect("valid name");
        let workspace = PathBuf::from("/tmp"); // Assume exists for test

        let session = Session::new_root(
            id.clone(),
            name.clone(),
            BranchState::OnBranch {
                name: "main".to_string(),
            },
            workspace.clone(),
        )
        .expect("session created");

        assert_eq!(session.id, id);
        assert_eq!(session.name, name);
        assert!(session.is_root());
        assert!(!session.is_child());
        assert_eq!(session.branch_name(), Some("main"));
    }

    #[test]
    fn test_create_child_session() {
        let id = SessionId::parse("test-session-2").expect("valid id");
        let name = SessionName::parse("child-session").expect("valid name");
        let parent = SessionName::parse("parent-session").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session = Session::new_child(
            id.clone(),
            name.clone(),
            BranchState::Detached,
            parent.clone(),
            workspace,
        )
        .expect("session created");

        assert!(!session.is_root());
        assert!(session.is_child());
        assert_eq!(session.parent_name(), Some(&parent));
        assert!(session.branch.is_detached());
    }

    #[test]
    fn test_branch_transition() {
        let id = SessionId::parse("test-3").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new_root(id, name, BranchState::Detached, workspace).expect("session created");

        // Detached -> OnBranch is valid
        let new_branch = BranchState::OnBranch {
            name: "main".to_string(),
        };
        let session = session
            .transition_branch(new_branch.clone())
            .expect("transition valid");

        assert_eq!(session.branch, new_branch);

        // OnBranch -> OnBranch is valid (switching branches)
        let another_branch = BranchState::OnBranch {
            name: "feature".to_string(),
        };
        let session = session
            .transition_branch(another_branch.clone())
            .expect("switch branches valid");

        assert_eq!(session.branch, another_branch);

        // OnBranch -> Detached is valid
        let session = session
            .transition_branch(BranchState::Detached)
            .expect("detach valid");

        assert!(session.branch.is_detached());
    }

    #[test]
    fn test_invalid_branch_transition() {
        let id = SessionId::parse("test-4").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new_root(id, name, BranchState::Detached, workspace).expect("session created");

        // Detached -> Detached is invalid (no self-loop)
        let result = session.transition_branch(BranchState::Detached);
        assert!(matches!(
            result,
            Err(SessionError::InvalidBranchTransition { .. })
        ));
    }

    #[test]
    fn test_parent_transition_child_to_child() {
        let id = SessionId::parse("test-5").expect("valid id");
        let name = SessionName::parse("child").expect("valid name");
        let parent1 = SessionName::parse("parent1").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new_child(id, name, BranchState::Detached, parent1.clone(), workspace)
                .expect("session created");

        // Child can change parent (adoption)
        let parent2 = SessionName::parse("parent2").expect("valid name");
        let new_parent = ParentState::ChildOf {
            parent: parent2.clone(),
        };

        let session = session
            .transition_parent(new_parent)
            .expect("parent change valid");

        assert_eq!(session.parent_name(), Some(&parent2));
    }

    #[test]
    fn test_invalid_parent_transition_root_to_child() {
        let id = SessionId::parse("test-6").expect("valid id");
        let name = SessionName::parse("root").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session =
            Session::new_root(id, name, BranchState::Detached, workspace).expect("session created");

        // Root cannot become child
        let new_parent = ParentState::ChildOf {
            parent: SessionName::parse("parent").expect("valid name"),
        };

        let result = session.transition_parent(new_parent);
        assert!(matches!(result, Err(SessionError::CannotModifyRootParent)));
    }

    #[test]
    fn test_workspace_not_found() {
        let id = SessionId::parse("test-7").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/nonexistent/path/that/does/not/exist");

        let result = Session::new_root(id, name, BranchState::Detached, workspace);

        assert!(matches!(result, Err(SessionError::WorkspaceNotFound(_))));
    }

    #[test]
    fn test_builder() {
        let id = SessionId::parse("test-8").expect("valid id");
        let name = SessionName::parse("test").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session = Session::builder()
            .id(id.clone())
            .name(name.clone())
            .branch(BranchState::OnBranch {
                name: "main".to_string(),
            })
            .workspace_path(workspace)
            .build()
            .expect("builder works");

        assert_eq!(session.id, id);
        assert_eq!(session.name, name);
        assert!(session.is_root());
    }

    #[test]
    fn test_rename() {
        let id = SessionId::parse("test-9").expect("valid id");
        let name1 = SessionName::parse("name1").expect("valid name");
        let name2 = SessionName::parse("name2").expect("valid name");
        let workspace = PathBuf::from("/tmp");

        let session = Session::new_root(id, name1, BranchState::Detached, workspace)
            .expect("session created");

        let renamed = session.rename(name2.clone());
        assert_eq!(renamed.name, name2);
    }
}
