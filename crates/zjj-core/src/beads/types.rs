//! Beads domain types and errors
//!
//! This module defines core types for the beads issue tracking system:
//! - `BeadsError`: Error types for database, I/O, and parsing operations
//! - `IssueStatus`: Workflow states (Open, InProgress, Blocked, Deferred, Closed)
//! - `IssueType`: Issue classifications (Bug, Feature, Task, Epic, etc.)
//! - `Priority`: Priority levels (P0-P4) with custom serialization
//! - `BeadIssue`: Main issue entity with predicate methods

use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BeadsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Failed to read file {path}: {source}")]
    FileReadFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse JSON at line {line}: {source}")]
    JsonParseFailed {
        line: usize,
        source: serde_json::Error,
    },
}

impl From<sqlx::Error> for BeadsError {
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseError(err.to_string())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Serialize,
    Deserialize,
    Hash,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    #[strum(to_string = "open")]
    Open,

    #[strum(to_string = "in_progress")]
    #[serde(rename = "in_progress", alias = "inprogress")]
    InProgress,

    #[strum(to_string = "blocked")]
    Blocked,

    #[strum(to_string = "deferred")]
    Deferred,

    #[strum(to_string = "closed")]
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    #[strum(to_string = "bug")]
    Bug,

    #[strum(to_string = "feature")]
    Feature,

    #[strum(to_string = "task")]
    Task,

    #[strum(to_string = "epic")]
    Epic,

    #[strum(to_string = "chore")]
    Chore,

    #[strum(to_string = "merge-request")]
    MergeRequest,

    #[strum(to_string = "event")]
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

// ============================================================================
// Priority Serialization & Conversion
// ============================================================================
// Custom serialization for Priority to handle both integer and string formats
impl Serialize for Priority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.to_u32())
    }
}

impl<'de> Deserialize<'de> for Priority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PriorityVisitor;

        impl serde::de::Visitor<'_> for PriorityVisitor {
            type Value = Priority;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an integer between 0 and 4, or a string like \"p0\"")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Priority, E>
            where
                E: serde::de::Error,
            {
                #[allow(clippy::cast_possible_truncation)]
                Priority::from_u32(value as u32)
                    .ok_or_else(|| E::custom(format!("priority value out of range: {value}")))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Priority, E>
            where
                E: serde::de::Error,
            {
                if value < 0 {
                    return Err(E::custom(format!("priority cannot be negative: {value}")));
                }
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                Priority::from_u32(value as u32)
                    .ok_or_else(|| E::custom(format!("priority value out of range: {value}")))
            }

            fn visit_str<E>(self, value: &str) -> Result<Priority, E>
            where
                E: serde::de::Error,
            {
                match value.to_lowercase().as_str() {
                    "p0" | "0" => Ok(Priority::P0),
                    "p1" | "1" => Ok(Priority::P1),
                    "p2" | "2" => Ok(Priority::P2),
                    "p3" | "3" => Ok(Priority::P3),
                    "p4" | "4" => Ok(Priority::P4),
                    _ => Err(E::custom(format!("unknown priority: {value}"))),
                }
            }
        }

        deserializer.deserialize_any(PriorityVisitor)
    }
}

impl Priority {
    /// Convert a u32 to a Priority, or None if out of range [0, 4]
    #[must_use]
    pub const fn from_u32(n: u32) -> Option<Self> {
        match n {
            0 => Some(Self::P0),
            1 => Some(Self::P1),
            2 => Some(Self::P2),
            3 => Some(Self::P3),
            4 => Some(Self::P4),
            _ => None,
        }
    }

    /// Convert Priority to its numeric representation (0-4)
    #[must_use]
    pub const fn to_u32(&self) -> u32 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    /// Convert Priority to a human-readable string (e.g., "P0", "P1")
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadIssue {
    pub id: String,
    pub title: String,
    pub status: IssueStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(alias = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<IssueType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vector<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vector<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<Vector<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// BeadIssue Methods
// ============================================================================
impl BeadIssue {
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.status == IssueStatus::Blocked
            || self.blocked_by.as_ref().is_some_and(|v| !v.is_empty())
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == IssueStatus::Open || self.status == IssueStatus::InProgress
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.status == IssueStatus::Closed
    }

    #[must_use]
    pub fn is_deferred(&self) -> bool {
        self.status == IssueStatus::Deferred
    }

    #[must_use]
    pub const fn has_priority(&self) -> bool {
        self.priority.is_some()
    }

    #[must_use]
    pub const fn has_description(&self) -> bool {
        self.description.is_some()
    }

    #[must_use]
    pub const fn has_assignee(&self) -> bool {
        self.assignee.is_some()
    }

    #[must_use]
    pub fn has_dependencies(&self) -> bool {
        self.depends_on.as_ref().is_some_and(|v| !v.is_empty())
    }

    #[must_use]
    pub fn has_labels(&self) -> bool {
        self.labels.as_ref().is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use im::vector;

    use super::*;

    /// Test helper to create a minimal BeadIssue with defaults
    fn minimal_issue(id: &str, title: &str, status: IssueStatus) -> BeadIssue {
        BeadIssue {
            id: id.to_string(),
            title: title.to_string(),
            status,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        }
    }

    #[test]
    fn test_bead_issue_is_blocked() {
        let mut blocked = minimal_issue("test", "Test", IssueStatus::Blocked);
        blocked.blocked_by = Some(vector!["other".to_string()]);

        let unblocked = minimal_issue("test2", "Test2", IssueStatus::Open);

        assert!(blocked.is_blocked());
        assert!(!unblocked.is_blocked());
    }

    #[test]
    fn test_bead_issue_is_open() {
        let open = minimal_issue("test", "Test", IssueStatus::Open);
        let in_progress = minimal_issue("test2", "Test2", IssueStatus::InProgress);
        let mut closed = minimal_issue("test3", "Test3", IssueStatus::Closed);
        closed.closed_at = Some(Utc::now());

        assert!(open.is_open());
        assert!(in_progress.is_open());
        assert!(!closed.is_open());
    }

    #[test]
    fn test_priority_to_u32() {
        assert_eq!(Priority::P0.to_u32(), 0);
        assert_eq!(Priority::P1.to_u32(), 1);
        assert_eq!(Priority::P2.to_u32(), 2);
        assert_eq!(Priority::P3.to_u32(), 3);
        assert_eq!(Priority::P4.to_u32(), 4);
    }

    #[test]
    fn test_priority_from_u32() {
        assert_eq!(Priority::from_u32(0), Some(Priority::P0));
        assert_eq!(Priority::from_u32(1), Some(Priority::P1));
        assert_eq!(Priority::from_u32(2), Some(Priority::P2));
        assert_eq!(Priority::from_u32(3), Some(Priority::P3));
        assert_eq!(Priority::from_u32(4), Some(Priority::P4));
        assert_eq!(Priority::from_u32(5), None);
    }

    #[test]
    fn test_priority_as_str() {
        assert_eq!(Priority::P0.as_str(), "P0");
        assert_eq!(Priority::P1.as_str(), "P1");
        assert_eq!(Priority::P2.as_str(), "P2");
        assert_eq!(Priority::P3.as_str(), "P3");
        assert_eq!(Priority::P4.as_str(), "P4");
    }

    #[test]
    fn test_priority_serialization_roundtrip() -> Result<(), serde_json::Error> {
        let original = Priority::P1;
        let serialized = serde_json::to_value(original)?;
        let deserialized: Priority = serde_json::from_value(serialized)?;
        assert_eq!(original, deserialized);
        Ok(())
    }

    #[test]
    fn test_priority_deserialization_from_string() -> Result<(), serde_json::Error> {
        let p0_str = serde_json::json!("p0");
        let p0: Priority = serde_json::from_value(p0_str)?;
        assert_eq!(p0, Priority::P0);

        let p1_str = serde_json::json!("P1");
        let p1: Priority = serde_json::from_value(p1_str)?;
        assert_eq!(p1, Priority::P1);
        Ok(())
    }

    #[test]
    fn test_bead_issue_predicates() {
        let issue = minimal_issue("test", "Test", IssueStatus::Open);
        assert!(issue.is_open());
        assert!(!issue.is_blocked());
        assert!(!issue.is_closed());
        assert!(!issue.is_deferred());
        assert!(!issue.has_priority());
        assert!(!issue.has_description());
        assert!(!issue.has_assignee());
        assert!(!issue.has_dependencies());
        assert!(!issue.has_labels());
    }

    #[test]
    fn test_issue_status_serialization() -> Result<(), serde_json::Error> {
        assert_eq!(
            serde_json::to_value(IssueStatus::Open)?,
            serde_json::json!("open")
        );
        assert_eq!(
            serde_json::to_value(IssueStatus::InProgress)?,
            serde_json::json!("in_progress")
        );
        Ok(())
    }

    #[test]
    fn test_issue_type_serialization() -> Result<(), serde_json::Error> {
        assert_eq!(
            serde_json::to_value(IssueType::Bug)?,
            serde_json::json!("bug")
        );
        assert_eq!(
            serde_json::to_value(IssueType::Feature)?,
            serde_json::json!("feature")
        );
        Ok(())
    }
}
