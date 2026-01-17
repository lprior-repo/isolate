#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Beads filtering, sorting, and pagination operations
//!
//! This module provides pure functional operations for querying, filtering,
//! sorting, and paginating bead issues. All functions are immutable and panic-free.
//!
//! The module is organized into:
//! - **predicates**: Individual filter predicates that check if an issue matches criteria
//! - **operations**: Complex filtering operations that compose predicates into pipelines

mod operations;
mod predicates;

pub use operations::{all_match, any_match, apply_query, filter_issues, paginate, sort_issues};

use strum::{Display, EnumString};

use crate::beads::types::{IssueStatus, IssueType, Priority};

/// Filter criteria for querying beads
#[derive(Debug, Clone, Default)]
pub struct BeadFilter {
    pub status: Vec<IssueStatus>,
    pub issue_type: Vec<IssueType>,
    pub priority_min: Option<Priority>,
    pub priority_max: Option<Priority>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
    pub has_parent: bool,
    pub blocked_only: bool,
    pub search_text: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl BeadFilter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_status(self, status: IssueStatus) -> Self {
        Self {
            status: self
                .status
                .into_iter()
                .chain(std::iter::once(status))
                .collect(),
            ..self
        }
    }

    #[must_use]
    pub fn with_statuses(self, statuses: impl IntoIterator<Item = IssueStatus>) -> Self {
        Self {
            status: self.status.into_iter().chain(statuses).collect(),
            ..self
        }
    }

    #[must_use]
    pub fn with_type(self, issue_type: IssueType) -> Self {
        Self {
            issue_type: self
                .issue_type
                .into_iter()
                .chain(std::iter::once(issue_type))
                .collect(),
            ..self
        }
    }

    #[must_use]
    pub fn with_priority_range(self, min: Priority, max: Priority) -> Self {
        Self {
            priority_min: Some(min),
            priority_max: Some(max),
            ..self
        }
    }

    #[must_use]
    pub fn with_label(self, label: impl Into<String>) -> Self {
        Self {
            labels: self
                .labels
                .into_iter()
                .chain(std::iter::once(label.into()))
                .collect(),
            ..self
        }
    }

    #[must_use]
    pub fn with_assignee(self, assignee: impl Into<String>) -> Self {
        Self {
            assignee: Some(assignee.into()),
            ..self
        }
    }

    #[must_use]
    pub fn with_parent(self, parent: impl Into<String>) -> Self {
        Self {
            parent: Some(parent.into()),
            ..self
        }
    }

    #[must_use]
    pub fn blocked_only(self) -> Self {
        Self {
            blocked_only: true,
            ..self
        }
    }

    #[must_use]
    pub fn with_search(self, text: impl Into<String>) -> Self {
        Self {
            search_text: Some(text.into()),
            ..self
        }
    }

    #[must_use]
    pub fn limit(self, n: usize) -> Self {
        Self {
            limit: Some(n),
            ..self
        }
    }

    #[must_use]
    pub fn offset(self, n: usize) -> Self {
        Self {
            offset: Some(n),
            ..self
        }
    }
}

/// Sort field for beads queries
#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum BeadSort {
    #[strum(to_string = "priority")]
    Priority,

    #[strum(to_string = "created")]
    Created,

    #[strum(to_string = "updated")]
    Updated,

    #[strum(to_string = "closed")]
    Closed,

    #[strum(to_string = "status")]
    Status,

    #[strum(to_string = "title")]
    Title,

    #[strum(to_string = "id")]
    Id,
}

/// Sort direction
#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum SortDirection {
    #[strum(to_string = "asc")]
    Asc,

    #[strum(to_string = "desc")]
    Desc,
}

/// Complete query specification for beads
#[derive(Debug, Clone)]
pub struct BeadQuery {
    pub filter: BeadFilter,
    pub sort: BeadSort,
    pub direction: SortDirection,
    pub include_closed: bool,
}

impl Default for BeadQuery {
    fn default() -> Self {
        Self {
            filter: BeadFilter::new(),
            sort: BeadSort::Priority,
            direction: SortDirection::Desc,
            include_closed: false,
        }
    }
}

impl BeadQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn filter(self, filter: BeadFilter) -> Self {
        Self { filter, ..self }
    }

    #[must_use]
    pub fn sort_by(self, sort: BeadSort) -> Self {
        Self { sort, ..self }
    }

    #[must_use]
    pub fn direction(self, direction: SortDirection) -> Self {
        Self { direction, ..self }
    }

    #[must_use]
    pub fn include_closed(self, include: bool) -> Self {
        Self {
            include_closed: include,
            ..self
        }
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_filter_new() {
        let filter = BeadFilter::new();
        assert!(filter.status.is_empty());
        assert!(filter.issue_type.is_empty());
        assert!(filter.labels.is_empty());
    }

    #[test]
    fn test_bead_filter_chaining() {
        let filter = BeadFilter::new()
            .with_status(IssueStatus::Open)
            .with_status(IssueStatus::InProgress)
            .with_type(IssueType::Bug)
            .with_label("urgent")
            .with_priority_range(Priority::P0, Priority::P2)
            .limit(10);

        assert_eq!(filter.status.len(), 2);
        assert_eq!(filter.issue_type.len(), 1);
        assert_eq!(filter.labels.len(), 1);
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_bead_query_default() {
        let query = BeadQuery::new();
        assert_eq!(query.sort, BeadSort::Priority);
        assert_eq!(query.direction, SortDirection::Desc);
        assert!(!query.include_closed);
    }

    #[test]
    fn test_bead_query_chaining() {
        let query = BeadQuery::new()
            .filter(BeadFilter::new().with_status(IssueStatus::Open))
            .sort_by(BeadSort::Created)
            .direction(SortDirection::Asc)
            .include_closed(true);

        assert_eq!(query.sort, BeadSort::Created);
        assert_eq!(query.direction, SortDirection::Asc);
        assert!(query.include_closed);
    }
}
