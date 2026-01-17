//! Session types and lifecycle management

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    contracts::{Constraint, ContextualHint, FieldContract, HasContract, HintType, TypeContract},
    Error, Result,
};

/// Session lifecycle states
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created (transient)
    Creating,
    /// Session is ready for use
    Active,
    /// Session exists but not currently in use
    Paused,
    /// Work completed, ready for removal
    Completed,
    /// Creation or hook failed
    Failed,
}

impl SessionStatus {
    /// Valid state transitions
    ///
    /// # Returns
    /// `true` if transition from current state to `next` is valid
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Creating | Self::Paused, Self::Active)
                | (Self::Creating, Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Completed)
        )
    }

    /// Allowed operations in this state
    pub const fn allowed_operations(self) -> &'static [Operation] {
        match self {
            Self::Creating => &[],
            Self::Active => &[
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ],
            Self::Paused => &[Operation::Status, Operation::Focus, Operation::Remove],
            Self::Completed | Self::Failed => &[Operation::Remove],
        }
    }

    /// Check if an operation is allowed in this state
    pub fn allows_operation(self, op: Operation) -> bool {
        self.allowed_operations().contains(&op)
    }
}

/// Operations that can be performed on sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// View session status
    Status,
    /// View diff
    Diff,
    /// Focus session
    Focus,
    /// Remove session
    Remove,
}

/// A session represents a parallel workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: String,

    /// Human-readable session name
    ///
    /// # Contract
    /// - MUST match regex: `^[a-zA-Z0-9_-]+$`
    /// - MUST be unique across all sessions
    /// - MUST NOT exceed 64 characters
    pub name: String,

    /// Current session status
    pub status: SessionStatus,

    /// Absolute path to workspace directory
    ///
    /// # Contract
    /// - MUST be absolute path
    /// - MUST exist if status != Creating
    pub workspace_path: PathBuf,

    /// Optional branch name
    ///
    /// # Contract
    /// - `Some` if session has explicit branch
    /// - `None` if using anonymous branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Creation timestamp (UTC)
    pub created_at: DateTime<Utc>,

    /// Last update timestamp (UTC)
    pub updated_at: DateTime<Utc>,

    /// Last sync timestamp (UTC, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,

    /// Arbitrary metadata (extensibility)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Session {
    /// Validate session invariants
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Name doesn't match regex
    /// - Workspace path is not absolute
    /// - Workspace doesn't exist (if status != Creating)
    /// - Timestamps are in wrong order
    pub fn validate(&self) -> Result<()> {
        // Name validation
        let name_regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+$")
            .map_err(|e| Error::validation_error(format!("Invalid regex: {e}")))?;

        if !name_regex.is_match(&self.name) {
            return Err(Error::validation_error(format!(
                "Session name '{}' must contain only alphanumeric characters, hyphens, and underscores",
                self.name
            )));
        }

        if self.name.len() > 64 {
            return Err(Error::validation_error(format!(
                "Session name '{}' exceeds maximum length of 64 characters",
                self.name
            )));
        }

        // Path validation
        if !self.workspace_path.is_absolute() {
            return Err(Error::validation_error(format!(
                "Workspace path '{}' must be absolute",
                self.workspace_path.display()
            )));
        }

        // Existence check (except during creation)
        if self.status != SessionStatus::Creating && !self.workspace_path.exists() {
            return Err(Error::validation_error(format!(
                "Workspace '{}' does not exist",
                self.workspace_path.display()
            )));
        }

        // Timestamp order
        if self.updated_at < self.created_at {
            return Err(Error::validation_error(
                "Updated timestamp cannot be before created timestamp".to_string(),
            ));
        }

        Ok(())
    }
}

impl HasContract for Session {
    fn contract() -> TypeContract {
        TypeContract::builder("Session")
            .description("A parallel workspace for isolating work")
            .field(
                "name",
                FieldContract::builder("name", "String")
                    .required()
                    .description("Human-readable session name")
                    .constraint(Constraint::Regex {
                        pattern: r"^[a-zA-Z0-9_-]+$".to_string(),
                        description: "alphanumeric with hyphens and underscores".to_string(),
                    })
                    .constraint(Constraint::Length {
                        min: Some(1),
                        max: Some(64),
                    })
                    .constraint(Constraint::Unique)
                    .example("feature-auth")
                    .example("bugfix-123")
                    .example("experiment_idea")
                    .build(),
            )
            .field(
                "status",
                FieldContract::builder("status", "SessionStatus")
                    .required()
                    .description("Current lifecycle state of the session")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "creating".to_string(),
                            "active".to_string(),
                            "paused".to_string(),
                            "completed".to_string(),
                            "failed".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "workspace_path",
                FieldContract::builder("workspace_path", "PathBuf")
                    .required()
                    .description("Absolute path to the workspace directory")
                    .constraint(Constraint::PathAbsolute)
                    .constraint(Constraint::Custom {
                        rule: "must exist if status != creating".to_string(),
                        description: "Workspace directory must exist for non-creating sessions"
                            .to_string(),
                    })
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::BestPractice,
                message: "Use descriptive session names that indicate the purpose of the work"
                    .to_string(),
                condition: None,
                related_to: Some("name".to_string()),
            })
            .hint(ContextualHint {
                hint_type: HintType::Warning,
                message: "Session name cannot be changed after creation".to_string(),
                condition: None,
                related_to: Some("name".to_string()),
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        self.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));

        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));

        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_session_status_allowed_operations() {
        assert_eq!(SessionStatus::Creating.allowed_operations().len(), 0);
        assert!(SessionStatus::Active.allows_operation(Operation::Status));
        assert!(SessionStatus::Active.allows_operation(Operation::Focus));
        assert!(SessionStatus::Paused.allows_operation(Operation::Remove));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Status));
    }

    #[test]
    fn test_session_validate_name_regex() {
        let session = Session {
            id: "id123".to_string(),
            name: "invalid name".to_string(), // contains space
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("/tmp/test"),
            branch: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: serde_json::Value::Null,
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_validate_path_not_absolute() {
        let session = Session {
            id: "id123".to_string(),
            name: "valid-name".to_string(),
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("relative/path"), // not absolute
            branch: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: serde_json::Value::Null,
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_validate_timestamps() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(60);

        let session = Session {
            id: "id123".to_string(),
            name: "valid-name".to_string(),
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("/tmp/test"),
            branch: None,
            created_at: now,
            updated_at: earlier, // updated before created!
            last_synced: None,
            metadata: serde_json::Value::Null,
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_contract() {
        let contract = Session::contract();
        assert_eq!(contract.name, "Session");
        assert!(!contract.fields.is_empty());
        assert!(contract.fields.contains_key("name"));
        assert!(contract.fields.contains_key("status"));
    }

    #[test]
    fn test_session_json_schema() {
        let schema = Session::json_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["title"], "Session");
        assert!(schema["properties"].is_object());
    }
}
