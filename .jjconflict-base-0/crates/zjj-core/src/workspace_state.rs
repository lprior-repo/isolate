//! Workspace Lifecycle State Machine
//!
//! Provides a type-safe state machine for workspace lifecycle management:
//! - `WorkspaceState` enum for runtime state representation
//! - Valid state transitions with exhaustive pattern matching
//! - Atomic state transition support for concurrent agents
//! - Railway-Oriented error handling with zero panics
//!
//! # State Machine
//!
//! ```text
//! Created -> Working        (start work)
//! Working -> Ready          (work complete)
//! Working -> Conflict       (merge conflict detected)
//! Working -> Abandoned      (manual abandon)
//! Ready -> Working          (needs more work)
//! Ready -> Merged           (successful merge)
//! Ready -> Conflict         (merge conflict on merge attempt)
//! Ready -> Abandoned        (decided not to merge)
//! Conflict -> Working       (conflict resolved)
//! Conflict -> Abandoned     (give up on conflict)
//! ```
//!
//! Terminal states: `Merged`, `Abandoned`

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE STATE ENUM
// ═══════════════════════════════════════════════════════════════════════════

/// Workspace lifecycle states for parallel agent coordination
///
/// This enum represents the lifecycle of a workspace from creation to
/// final merge or abandonment. It supports 40+ concurrent agents with
/// atomic state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceState {
    /// Workspace created, not yet actively worked on
    #[default]
    Created,
    /// Actively being worked on by an agent
    Working,
    /// Work complete, ready for merge review
    Ready,
    /// Successfully merged to main branch
    Merged,
    /// Manually abandoned by agent
    Abandoned,
    /// Merge conflict detected, needs resolution
    Conflict,
}

impl WorkspaceState {
    /// Returns all valid next states from current state.
    ///
    /// Uses exhaustive pattern matching to ensure all states are covered.
    /// This is the single source of truth for valid transitions.
    #[must_use]
    pub fn valid_next_states(self) -> Vec<Self> {
        match self {
            Self::Created => vec![Self::Working],
            Self::Working => vec![Self::Ready, Self::Conflict, Self::Abandoned],
            Self::Ready => vec![Self::Working, Self::Merged, Self::Conflict, Self::Abandoned],
            Self::Conflict => vec![Self::Working, Self::Abandoned],
            // Terminal states - no transitions out
            Self::Merged | Self::Abandoned => vec![],
        }
    }

    /// Returns true if this state can transition to the next state.
    ///
    /// Uses exhaustive matching via `valid_next_states()`.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.valid_next_states().contains(&next)
    }

    /// Returns true if this is a terminal state (no further transitions possible).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::Abandoned)
    }

    /// Returns true if this state indicates active work is happening.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Working | Self::Conflict)
    }

    /// Returns true if this state indicates work is complete (ready or merged).
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Ready | Self::Merged)
    }

    /// Returns all possible workspace states as a slice.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Created,
            Self::Working,
            Self::Ready,
            Self::Merged,
            Self::Abandoned,
            Self::Conflict,
        ]
    }
}

impl fmt::Display for WorkspaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Working => write!(f, "working"),
            Self::Ready => write!(f, "ready"),
            Self::Merged => write!(f, "merged"),
            Self::Abandoned => write!(f, "abandoned"),
            Self::Conflict => write!(f, "conflict"),
        }
    }
}

impl FromStr for WorkspaceState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "working" => Ok(Self::Working),
            "ready" => Ok(Self::Ready),
            "merged" => Ok(Self::Merged),
            "abandoned" => Ok(Self::Abandoned),
            "conflict" => Ok(Self::Conflict),
            _ => Err(Error::ValidationError(format!(
                "Invalid workspace state: '{s}'. Valid states: created, working, ready, merged, abandoned, conflict"
            ))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE TRANSITION
// ═══════════════════════════════════════════════════════════════════════════

/// A workspace state transition event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStateTransition {
    /// Source state
    pub from: WorkspaceState,
    /// Target state
    pub to: WorkspaceState,
    /// Timestamp of transition (UTC)
    pub timestamp: DateTime<Utc>,
    /// Reason for transition (human-readable)
    pub reason: String,
    /// Agent ID that performed the transition (for audit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

impl WorkspaceStateTransition {
    /// Create a new state transition
    #[must_use]
    pub fn new(from: WorkspaceState, to: WorkspaceState, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: reason.into(),
            agent_id: None,
        }
    }

    /// Create a new state transition with agent ID
    #[must_use]
    pub fn with_agent(
        from: WorkspaceState,
        to: WorkspaceState,
        reason: impl Into<String>,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: reason.into(),
            agent_id: Some(agent_id.into()),
        }
    }

    /// Validate that the transition is allowed
    ///
    /// # Errors
    ///
    /// Returns `Error::ValidationError` if the transition is not allowed.
    pub fn validate(&self) -> Result<()> {
        if self.from.can_transition_to(self.to) {
            Ok(())
        } else {
            Err(Error::ValidationError(format!(
                "Invalid workspace state transition: {} -> {}. Valid transitions from {} are: {:?}",
                self.from,
                self.to,
                self.from,
                self.from.valid_next_states()
            )))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE QUERY HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Filter predicate for workspace states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceStateFilter {
    /// Match a specific state
    State(WorkspaceState),
    /// Match any active state (Working, Conflict)
    Active,
    /// Match any complete state (Ready, Merged)
    Complete,
    /// Match any terminal state (Merged, Abandoned)
    Terminal,
    /// Match any non-terminal state
    NonTerminal,
    /// Match all states
    All,
}

impl WorkspaceStateFilter {
    /// Check if a workspace state matches this filter
    #[must_use]
    pub fn matches(self, state: WorkspaceState) -> bool {
        match self {
            Self::State(s) => state == s,
            Self::Active => state.is_active(),
            Self::Complete => state.is_complete(),
            Self::Terminal => state.is_terminal(),
            Self::NonTerminal => !state.is_terminal(),
            Self::All => true,
        }
    }
}

impl FromStr for WorkspaceStateFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "all" => Ok(Self::All),
            "active" => Ok(Self::Active),
            "complete" => Ok(Self::Complete),
            "terminal" => Ok(Self::Terminal),
            "non-terminal" | "nonterminal" => Ok(Self::NonTerminal),
            _ => {
                // Try parsing as a specific state
                WorkspaceState::from_str(s).map(Self::State)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE MACHINE CORRECTNESS TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_all_valid_transitions_succeed() {
        // Created -> Working
        assert!(WorkspaceState::Created.can_transition_to(WorkspaceState::Working));

        // Working -> Ready, Conflict, Abandoned
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Ready));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Working.can_transition_to(WorkspaceState::Abandoned));

        // Ready -> Working, Merged, Conflict, Abandoned
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Merged));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Conflict));
        assert!(WorkspaceState::Ready.can_transition_to(WorkspaceState::Abandoned));

        // Conflict -> Working, Abandoned
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Working));
        assert!(WorkspaceState::Conflict.can_transition_to(WorkspaceState::Abandoned));
    }

    #[test]
    fn test_invalid_transition_returns_error() {
        // Created cannot go directly to Ready, Merged, Abandoned, Conflict
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Ready));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Merged));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Abandoned));
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Conflict));

        // Working cannot go directly to Merged
        assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Merged));

        // Working cannot go back to Created
        assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Created));
    }

    #[test]
    fn test_terminal_states_reject_transitions() {
        // Merged is terminal
        assert!(WorkspaceState::Merged.is_terminal());
        assert!(WorkspaceState::Merged.valid_next_states().is_empty());
        for state in WorkspaceState::all() {
            assert!(!WorkspaceState::Merged.can_transition_to(*state));
        }

        // Abandoned is terminal
        assert!(WorkspaceState::Abandoned.is_terminal());
        assert!(WorkspaceState::Abandoned.valid_next_states().is_empty());
        for state in WorkspaceState::all() {
            assert!(!WorkspaceState::Abandoned.can_transition_to(*state));
        }
    }

    #[test]
    fn test_state_serialization_roundtrip() {
        for state in WorkspaceState::all() {
            // Test serde JSON roundtrip
            let json_result = serde_json::to_string(state);
            assert!(json_result.is_ok(), "Failed to serialize state: {state:?}");
            let Some(json) = json_result.ok() else {
                continue;
            };

            let parsed_result: std::result::Result<WorkspaceState, _> = serde_json::from_str(&json);
            assert!(
                parsed_result.is_ok(),
                "Failed to deserialize state: {state:?}"
            );
            let Some(parsed) = parsed_result.ok() else {
                continue;
            };
            assert_eq!(*state, parsed);
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SPECIFIC INVALID TRANSITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_created_cannot_go_to_merged() {
        assert!(!WorkspaceState::Created.can_transition_to(WorkspaceState::Merged));

        let transition =
            WorkspaceStateTransition::new(WorkspaceState::Created, WorkspaceState::Merged, "skip");
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_merged_cannot_transition() {
        for state in WorkspaceState::all() {
            assert!(
                !WorkspaceState::Merged.can_transition_to(*state),
                "Merged should not transition to {state:?}"
            );
        }
    }

    #[test]
    fn test_conflict_cannot_go_to_ready() {
        assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Ready));
    }

    #[test]
    fn test_conflict_cannot_go_to_merged() {
        assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Merged));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE DISPLAY AND PARSING
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_state_display() {
        assert_eq!(WorkspaceState::Created.to_string(), "created");
        assert_eq!(WorkspaceState::Working.to_string(), "working");
        assert_eq!(WorkspaceState::Ready.to_string(), "ready");
        assert_eq!(WorkspaceState::Merged.to_string(), "merged");
        assert_eq!(WorkspaceState::Abandoned.to_string(), "abandoned");
        assert_eq!(WorkspaceState::Conflict.to_string(), "conflict");
    }

    #[test]
    fn test_state_from_str() {
        assert_eq!(
            WorkspaceState::from_str("created").ok(),
            Some(WorkspaceState::Created)
        );
        assert_eq!(
            WorkspaceState::from_str("working").ok(),
            Some(WorkspaceState::Working)
        );
        assert_eq!(
            WorkspaceState::from_str("ready").ok(),
            Some(WorkspaceState::Ready)
        );
        assert_eq!(
            WorkspaceState::from_str("merged").ok(),
            Some(WorkspaceState::Merged)
        );
        assert_eq!(
            WorkspaceState::from_str("abandoned").ok(),
            Some(WorkspaceState::Abandoned)
        );
        assert_eq!(
            WorkspaceState::from_str("conflict").ok(),
            Some(WorkspaceState::Conflict)
        );
    }

    #[test]
    fn test_state_from_str_case_insensitive() {
        assert_eq!(
            WorkspaceState::from_str("CREATED").ok(),
            Some(WorkspaceState::Created)
        );
        assert_eq!(
            WorkspaceState::from_str("Working").ok(),
            Some(WorkspaceState::Working)
        );
        assert_eq!(
            WorkspaceState::from_str("READY").ok(),
            Some(WorkspaceState::Ready)
        );
    }

    #[test]
    fn test_state_from_str_invalid() {
        assert!(WorkspaceState::from_str("invalid").is_err());
        assert!(WorkspaceState::from_str("").is_err());
        assert!(WorkspaceState::from_str("active").is_err()); // Not a state name
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE PROPERTIES
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_is_terminal() {
        assert!(!WorkspaceState::Created.is_terminal());
        assert!(!WorkspaceState::Working.is_terminal());
        assert!(!WorkspaceState::Ready.is_terminal());
        assert!(WorkspaceState::Merged.is_terminal());
        assert!(WorkspaceState::Abandoned.is_terminal());
        assert!(!WorkspaceState::Conflict.is_terminal());
    }

    #[test]
    fn test_is_active() {
        assert!(!WorkspaceState::Created.is_active());
        assert!(WorkspaceState::Working.is_active());
        assert!(!WorkspaceState::Ready.is_active());
        assert!(!WorkspaceState::Merged.is_active());
        assert!(!WorkspaceState::Abandoned.is_active());
        assert!(WorkspaceState::Conflict.is_active());
    }

    #[test]
    fn test_is_complete() {
        assert!(!WorkspaceState::Created.is_complete());
        assert!(!WorkspaceState::Working.is_complete());
        assert!(WorkspaceState::Ready.is_complete());
        assert!(WorkspaceState::Merged.is_complete());
        assert!(!WorkspaceState::Abandoned.is_complete());
        assert!(!WorkspaceState::Conflict.is_complete());
    }

    #[test]
    fn test_all_states() {
        let all = WorkspaceState::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&WorkspaceState::Created));
        assert!(all.contains(&WorkspaceState::Working));
        assert!(all.contains(&WorkspaceState::Ready));
        assert!(all.contains(&WorkspaceState::Merged));
        assert!(all.contains(&WorkspaceState::Abandoned));
        assert!(all.contains(&WorkspaceState::Conflict));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE TRANSITION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_transition_validate_valid() {
        let transition = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "start work",
        );
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_transition_validate_invalid() {
        let transition = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Merged,
            "skip everything",
        );
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_transition_with_agent() {
        let transition = WorkspaceStateTransition::with_agent(
            WorkspaceState::Working,
            WorkspaceState::Ready,
            "work complete",
            "agent-42",
        );
        assert_eq!(transition.agent_id, Some("agent-42".to_string()));
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_transition_timestamp_is_recent() {
        let before = Utc::now();
        let transition =
            WorkspaceStateTransition::new(WorkspaceState::Created, WorkspaceState::Working, "test");
        let after = Utc::now();

        assert!(transition.timestamp >= before);
        assert!(transition.timestamp <= after);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // FILTER TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_filter_state() {
        let filter = WorkspaceStateFilter::State(WorkspaceState::Working);
        assert!(filter.matches(WorkspaceState::Working));
        assert!(!filter.matches(WorkspaceState::Ready));
    }

    #[test]
    fn test_filter_active() {
        let filter = WorkspaceStateFilter::Active;
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(filter.matches(WorkspaceState::Working));
        assert!(!filter.matches(WorkspaceState::Ready));
        assert!(!filter.matches(WorkspaceState::Merged));
        assert!(!filter.matches(WorkspaceState::Abandoned));
        assert!(filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_complete() {
        let filter = WorkspaceStateFilter::Complete;
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(!filter.matches(WorkspaceState::Working));
        assert!(filter.matches(WorkspaceState::Ready));
        assert!(filter.matches(WorkspaceState::Merged));
        assert!(!filter.matches(WorkspaceState::Abandoned));
        assert!(!filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_terminal() {
        let filter = WorkspaceStateFilter::Terminal;
        assert!(!filter.matches(WorkspaceState::Created));
        assert!(!filter.matches(WorkspaceState::Working));
        assert!(!filter.matches(WorkspaceState::Ready));
        assert!(filter.matches(WorkspaceState::Merged));
        assert!(filter.matches(WorkspaceState::Abandoned));
        assert!(!filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_non_terminal() {
        let filter = WorkspaceStateFilter::NonTerminal;
        assert!(filter.matches(WorkspaceState::Created));
        assert!(filter.matches(WorkspaceState::Working));
        assert!(filter.matches(WorkspaceState::Ready));
        assert!(!filter.matches(WorkspaceState::Merged));
        assert!(!filter.matches(WorkspaceState::Abandoned));
        assert!(filter.matches(WorkspaceState::Conflict));
    }

    #[test]
    fn test_filter_all() {
        let filter = WorkspaceStateFilter::All;
        for state in WorkspaceState::all() {
            assert!(filter.matches(*state));
        }
    }

    #[test]
    fn test_filter_from_str() {
        assert_eq!(
            WorkspaceStateFilter::from_str("all").ok(),
            Some(WorkspaceStateFilter::All)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("active").ok(),
            Some(WorkspaceStateFilter::Active)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("complete").ok(),
            Some(WorkspaceStateFilter::Complete)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("terminal").ok(),
            Some(WorkspaceStateFilter::Terminal)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("non-terminal").ok(),
            Some(WorkspaceStateFilter::NonTerminal)
        );
        assert_eq!(
            WorkspaceStateFilter::from_str("working").ok(),
            Some(WorkspaceStateFilter::State(WorkspaceState::Working))
        );
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EDGE CASE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_transition_with_empty_reason() {
        let transition =
            WorkspaceStateTransition::new(WorkspaceState::Created, WorkspaceState::Working, "");
        assert_eq!(transition.reason, "");
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_transition_with_long_reason() {
        let long_reason = "a".repeat(10000);
        let transition = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Working,
            long_reason.clone(),
        );
        assert_eq!(transition.reason, long_reason);
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_default_state_is_created() {
        assert_eq!(WorkspaceState::default(), WorkspaceState::Created);
    }

    #[test]
    fn test_state_eq_and_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(WorkspaceState::Working);
        set.insert(WorkspaceState::Working); // duplicate

        assert_eq!(set.len(), 1);
        assert!(set.contains(&WorkspaceState::Working));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE MACHINE EXHAUSTIVENESS TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_every_non_terminal_state_has_valid_transitions() {
        for state in WorkspaceState::all() {
            if !state.is_terminal() {
                assert!(
                    !state.valid_next_states().is_empty(),
                    "Non-terminal state {state:?} should have valid transitions"
                );
            }
        }
    }

    #[test]
    fn test_terminal_states_have_no_transitions() {
        for state in WorkspaceState::all() {
            if state.is_terminal() {
                assert!(
                    state.valid_next_states().is_empty(),
                    "Terminal state {state:?} should have no valid transitions"
                );
            }
        }
    }

    #[test]
    fn test_created_can_only_go_to_working() {
        let valid = WorkspaceState::Created.valid_next_states();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0], WorkspaceState::Working);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // RAILWAY-ORIENTED PROGRAMMING TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_transition_validate_returns_result_not_panic() {
        // Valid transition returns Ok
        let valid = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "valid",
        );
        let result = valid.validate();
        assert!(result.is_ok());

        // Invalid transition returns Err, not panic
        let invalid =
            WorkspaceStateTransition::new(WorkspaceState::Merged, WorkspaceState::Working, "bad");
        let result = invalid.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_from_str_returns_result_not_panic() {
        // Valid input returns Ok
        let result = WorkspaceState::from_str("working");
        assert!(result.is_ok());

        // Invalid input returns Err, not panic
        let result = WorkspaceState::from_str("not-a-state");
        assert!(result.is_err());
    }
}
