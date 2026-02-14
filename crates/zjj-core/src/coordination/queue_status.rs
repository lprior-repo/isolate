//! Queue Status State Machine (Pure Domain Logic)
//!
//! This module contains the state machine for merge queue lifecycle with:
//! - `QueueStatus` enum for queue entry states
//! - Pure transition validation (no DB, no async, no side effects)
//! - Zero tokio/sqlx imports - pure domain logic only
//!
//! # State Machine
//!
//! ```text
//! pending -> claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
//!     |          |          |           |              |            |
//!     v          v          v           v              v            v
//! cancelled  failed_retryable/failed_terminal/cancelled (from each state)
//!
//! failed_retryable -> pending (manual retry)
//! failed_retryable -> cancelled
//! ```
//!
//! Terminal states: `merged`, `failed_terminal`, `cancelled`

use std::{fmt, str::FromStr};

use thiserror::Error;

use crate::{Error, Result};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// STATE MACHINE ERROR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for invalid queue state transitions.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid state transition: cannot transition from {from} to {to}")]
pub struct TransitionError {
    pub from: QueueStatus,
    pub to: QueueStatus,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE STATUS STATE MACHINE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// State machine for merge queue item lifecycle.
///
/// Valid transitions:
/// - pending -> claimed
/// - claimed -> rebasing
/// - rebasing -> testing
/// - testing -> `ready_to_merge`
/// - `ready_to_merge` -> merging
/// - merging -> merged
/// - `claimed|rebasing|testing|ready_to_merge|merging` -> `failed_retryable`
/// - `claimed|rebasing|testing|ready_to_merge|merging` -> `failed_terminal`
/// - `pending|claimed|rebasing|testing|ready_to_merge|failed_retryable` -> cancelled
/// - `failed_retryable` -> pending (manual retry path)
///
/// Terminal states (no outgoing transitions): merged, `failed_terminal`, cancelled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    /// Item is waiting to be claimed by an agent.
    Pending,
    /// Item has been claimed by an agent and is being prepared.
    Claimed,
    /// Item is currently being rebased onto the target branch.
    Rebasing,
    /// Item is undergoing testing/validation.
    Testing,
    /// Item has passed all checks and is ready for merge.
    ReadyToMerge,
    /// Item is actively being merged.
    Merging,
    /// Item has been successfully merged.
    Merged,
    /// Item failed but can be retried manually.
    FailedRetryable,
    /// Item failed with an unrecoverable error.
    FailedTerminal,
    /// Item was cancelled before completion.
    Cancelled,
}

impl QueueStatus {
    /// Returns the string representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Rebasing => "rebasing",
            Self::Testing => "testing",
            Self::ReadyToMerge => "ready_to_merge",
            Self::Merging => "merging",
            Self::Merged => "merged",
            Self::FailedRetryable => "failed_retryable",
            Self::FailedTerminal => "failed_terminal",
            Self::Cancelled => "cancelled",
        }
    }

    /// Returns true if this status is terminal (no valid outgoing transitions).
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Merged | Self::FailedTerminal | Self::Cancelled)
    }

    /// Returns true if a transition from `self` to `target` is valid.
    #[must_use]
    pub fn can_transition_to(&self, target: Self) -> bool {
        self.validate_transition(target).is_ok()
    }

    /// Validates that a transition from `self` to `target` is allowed.
    ///
    /// Returns `Ok(())` if the transition is valid, or a `TransitionError` if not.
    pub fn validate_transition(&self, target: Self) -> std::result::Result<(), TransitionError> {
        if self == &target {
            return Ok(());
        }

        if self.is_terminal() {
            return Err(TransitionError {
                from: *self,
                to: target,
            });
        }

        let is_valid = match self {
            Self::Pending => matches!(target, Self::Claimed | Self::Cancelled),
            Self::Claimed => matches!(
                target,
                Self::Pending
                    | Self::Rebasing
                    | Self::FailedRetryable
                    | Self::FailedTerminal
                    | Self::Cancelled
            ),
            Self::Rebasing => matches!(
                target,
                Self::Testing | Self::FailedRetryable | Self::FailedTerminal | Self::Cancelled
            ),
            Self::Testing => matches!(
                target,
                Self::ReadyToMerge | Self::FailedRetryable | Self::FailedTerminal | Self::Cancelled
            ),
            Self::ReadyToMerge => matches!(
                target,
                Self::Merging | Self::FailedRetryable | Self::FailedTerminal | Self::Cancelled
            ),
            Self::Merging => matches!(
                target,
                Self::Merged | Self::FailedRetryable | Self::FailedTerminal
            ),
            Self::FailedRetryable => matches!(target, Self::Pending | Self::Cancelled),
            Self::Merged | Self::FailedTerminal | Self::Cancelled => false,
        };

        if is_valid {
            Ok(())
        } else {
            Err(TransitionError {
                from: *self,
                to: target,
            })
        }
    }

    /// Returns all possible queue statuses as a slice.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Pending,
            Self::Claimed,
            Self::Rebasing,
            Self::Testing,
            Self::ReadyToMerge,
            Self::Merging,
            Self::Merged,
            Self::FailedRetryable,
            Self::FailedTerminal,
            Self::Cancelled,
        ]
    }
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for QueueStatus {
    type Err = Error;

    #[allow(clippy::match_same_arms)]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "claimed" => Ok(Self::Claimed),
            "rebasing" => Ok(Self::Rebasing),
            "testing" => Ok(Self::Testing),
            "ready_to_merge" => Ok(Self::ReadyToMerge),
            "merging" => Ok(Self::Merging),
            "merged" => Ok(Self::Merged),
            "failed_retryable" => Ok(Self::FailedRetryable),
            "failed_terminal" => Ok(Self::FailedTerminal),
            "cancelled" => Ok(Self::Cancelled),
            "processing" => Ok(Self::Claimed),
            "completed" => Ok(Self::Merged),
            "failed" => Ok(Self::FailedTerminal),
            _ => Err(Error::InvalidConfig(format!("Invalid queue status: {s}"))),
        }
    }
}

impl TryFrom<String> for QueueStatus {
    type Error = Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// WORKSPACE QUEUE STATE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Workspace state for the queue state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceQueueState {
    #[default]
    Created,
    Working,
    Ready,
    Merged,
    Abandoned,
    Conflict,
}

impl WorkspaceQueueState {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Working => "working",
            Self::Ready => "ready",
            Self::Merged => "merged",
            Self::Abandoned => "abandoned",
            Self::Conflict => "conflict",
        }
    }
}

impl std::str::FromStr for WorkspaceQueueState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "created" => Ok(Self::Created),
            "working" => Ok(Self::Working),
            "ready" => Ok(Self::Ready),
            "merged" => Ok(Self::Merged),
            "abandoned" => Ok(Self::Abandoned),
            "conflict" => Ok(Self::Conflict),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid workspace queue state: {s}"
            ))),
        }
    }
}

impl TryFrom<String> for WorkspaceQueueState {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Self::from_str(&s)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE EVENT TYPE (Pure Domain)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Event types for queue entry lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueEventType {
    /// Entry created.
    Created,
    /// Entry claimed by worker.
    Claimed,
    /// State transition occurred.
    Transitioned,
    /// Entry failed.
    Failed,
    /// Entry retried.
    Retried,
    /// Entry cancelled.
    Cancelled,
    /// Entry merged.
    Merged,
    /// Worker heartbeat.
    Heartbeat,
}

impl QueueEventType {
    /// Returns the string representation of this event type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Claimed => "claimed",
            Self::Transitioned => "transitioned",
            Self::Failed => "failed",
            Self::Retried => "retried",
            Self::Cancelled => "cancelled",
            Self::Merged => "merged",
            Self::Heartbeat => "heartbeat",
        }
    }
}

impl fmt::Display for QueueEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for QueueEventType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "claimed" => Ok(Self::Claimed),
            "transitioned" => Ok(Self::Transitioned),
            "failed" => Ok(Self::Failed),
            "retried" => Ok(Self::Retried),
            "cancelled" => Ok(Self::Cancelled),
            "merged" => Ok(Self::Merged),
            "heartbeat" => Ok(Self::Heartbeat),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid queue event type: {s}"
            ))),
        }
    }
}

impl TryFrom<String> for QueueEventType {
    type Error = Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // --- Valid Happy Path Transitions ---

    #[test]
    fn test_pending_to_claimed_is_valid() {
        assert!(QueueStatus::Pending.can_transition_to(QueueStatus::Claimed));
        assert!(QueueStatus::Pending
            .validate_transition(QueueStatus::Claimed)
            .is_ok());
    }

    #[test]
    fn test_claimed_to_rebasing_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::Rebasing));
        assert!(QueueStatus::Claimed
            .validate_transition(QueueStatus::Rebasing)
            .is_ok());
    }

    #[test]
    fn test_rebasing_to_testing_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::Testing));
        assert!(QueueStatus::Rebasing
            .validate_transition(QueueStatus::Testing)
            .is_ok());
    }

    #[test]
    fn test_testing_to_ready_to_merge_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::ReadyToMerge));
        assert!(QueueStatus::Testing
            .validate_transition(QueueStatus::ReadyToMerge)
            .is_ok());
    }

    #[test]
    fn test_ready_to_merge_to_merging_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::Merging));
        assert!(QueueStatus::ReadyToMerge
            .validate_transition(QueueStatus::Merging)
            .is_ok());
    }

    #[test]
    fn test_merging_to_merged_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::Merged));
        assert!(QueueStatus::Merging
            .validate_transition(QueueStatus::Merged)
            .is_ok());
    }

    // --- Valid Failure Transitions ---

    #[test]
    fn test_claimed_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_rebasing_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_testing_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_ready_to_merge_to_failed_retryable_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_merging_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_claimed_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_rebasing_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_testing_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_ready_to_merge_to_failed_terminal_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_merging_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::FailedTerminal));
    }

    // --- Valid Cancel Transitions ---

    #[test]
    fn test_pending_to_cancelled_is_valid() {
        assert!(QueueStatus::Pending.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_claimed_to_cancelled_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_rebasing_to_cancelled_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_testing_to_cancelled_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_ready_to_merge_to_cancelled_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_failed_retryable_to_cancelled_is_valid() {
        assert!(QueueStatus::FailedRetryable.can_transition_to(QueueStatus::Cancelled));
    }

    // --- Valid Retry Path ---

    #[test]
    fn test_failed_retryable_to_pending_is_valid() {
        assert!(QueueStatus::FailedRetryable.can_transition_to(QueueStatus::Pending));
    }

    // --- Idempotent Transitions (same state) ---

    #[test]
    fn test_same_state_transition_is_always_valid() {
        for status in QueueStatus::all() {
            assert!(
                status.can_transition_to(*status),
                "{status:?} should be able to transition to itself"
            );
        }
    }

    // --- Terminal State Tests ---

    #[test]
    fn test_merged_is_terminal() {
        assert!(QueueStatus::Merged.is_terminal());
    }

    #[test]
    fn test_failed_terminal_is_terminal() {
        assert!(QueueStatus::FailedTerminal.is_terminal());
    }

    #[test]
    fn test_cancelled_is_terminal() {
        assert!(QueueStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_non_terminal_states_are_not_terminal() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::FailedRetryable,
        ] {
            assert!(!status.is_terminal(), "{status:?} should not be terminal");
        }
    }

    // --- Invalid Transition Edge Cases ---

    #[test]
    fn test_merged_cannot_transition_to_anything() {
        let terminal = QueueStatus::Merged;
        for target in QueueStatus::all() {
            if *target != terminal {
                assert!(
                    !terminal.can_transition_to(*target),
                    "Merged should not transition to {target:?}"
                );
            }
        }
    }

    #[test]
    fn test_failed_terminal_cannot_transition_to_anything() {
        let terminal = QueueStatus::FailedTerminal;
        for target in QueueStatus::all() {
            if *target != terminal {
                assert!(
                    !terminal.can_transition_to(*target),
                    "FailedTerminal should not transition to {target:?}"
                );
            }
        }
    }

    #[test]
    fn test_cancelled_cannot_transition_to_anything() {
        let terminal = QueueStatus::Cancelled;
        for target in QueueStatus::all() {
            if *target != terminal {
                assert!(
                    !terminal.can_transition_to(*target),
                    "Cancelled should not transition to {target:?}"
                );
            }
        }
    }

    #[test]
    fn test_pending_cannot_skip_to_testing() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::Testing));
    }

    #[test]
    fn test_pending_cannot_skip_to_ready_to_merge() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::ReadyToMerge));
    }

    #[test]
    fn test_pending_cannot_skip_to_merging() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::Merging));
    }

    #[test]
    fn test_pending_cannot_go_directly_to_merged() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::Merged));
    }

    #[test]
    fn test_pending_cannot_go_to_failed_retryable_directly() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_pending_cannot_go_to_failed_terminal_directly() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_claimed_cannot_skip_to_ready_to_merge() {
        assert!(!QueueStatus::Claimed.can_transition_to(QueueStatus::ReadyToMerge));
    }

    #[test]
    fn test_claimed_cannot_skip_to_merging() {
        assert!(!QueueStatus::Claimed.can_transition_to(QueueStatus::Merging));
    }

    #[test]
    fn test_claimed_cannot_go_directly_to_merged() {
        assert!(!QueueStatus::Claimed.can_transition_to(QueueStatus::Merged));
    }

    #[test]
    fn test_failed_terminal_cannot_retry() {
        assert!(!QueueStatus::FailedTerminal.can_transition_to(QueueStatus::Pending));
    }

    #[test]
    fn test_cancelled_cannot_retry() {
        assert!(!QueueStatus::Cancelled.can_transition_to(QueueStatus::Pending));
    }

    #[test]
    fn test_merged_cannot_be_cancelled() {
        assert!(!QueueStatus::Merged.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_failed_terminal_cannot_be_cancelled() {
        assert!(!QueueStatus::FailedTerminal.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_rebasing_cannot_skip_to_merging() {
        assert!(!QueueStatus::Rebasing.can_transition_to(QueueStatus::Merging));
    }

    #[test]
    fn test_testing_cannot_skip_to_merging() {
        assert!(!QueueStatus::Testing.can_transition_to(QueueStatus::Merging));
    }

    // --- Transition Error Tests ---

    #[test]
    fn test_transition_error_contains_from_and_to() {
        let err = QueueStatus::Merged.validate_transition(QueueStatus::Pending);
        assert!(err.is_err());
        let transition_err = err.err();
        assert!(transition_err.is_some());
        let err = transition_err.unwrap();
        assert_eq!(err.from, QueueStatus::Merged);
        assert_eq!(err.to, QueueStatus::Pending);
    }

    #[test]
    fn test_transition_error_display() {
        let err = TransitionError {
            from: QueueStatus::Merged,
            to: QueueStatus::Pending,
        };
        let display = err.to_string();
        assert!(display.contains("merged"));
        assert!(display.contains("pending"));
        assert!(display.contains("invalid state transition"));
    }

    // --- Display and FromStr Tests ---

    #[test]
    fn test_status_display_roundtrip() -> Result<()> {
        for status in QueueStatus::all() {
            let s = status.to_string();
            let parsed = QueueStatus::from_str(&s);
            assert!(parsed.is_ok(), "Failed to parse '{s}' back to QueueStatus");
            assert_eq!(parsed?, *status);
        }
        Ok(())
    }

    #[test]
    fn test_as_str_matches_display() {
        for status in QueueStatus::all() {
            assert_eq!(status.as_str(), status.to_string());
        }
    }

    #[test]
    fn test_backward_compat_processing_maps_to_claimed() -> Result<()> {
        let status = QueueStatus::from_str("processing")?;
        assert_eq!(status, QueueStatus::Claimed);
        Ok(())
    }

    #[test]
    fn test_backward_compat_completed_maps_to_merged() -> Result<()> {
        let status = QueueStatus::from_str("completed")?;
        assert_eq!(status, QueueStatus::Merged);
        Ok(())
    }

    #[test]
    fn test_backward_compat_failed_maps_to_failed_terminal() -> Result<()> {
        let status = QueueStatus::from_str("failed")?;
        assert_eq!(status, QueueStatus::FailedTerminal);
        Ok(())
    }

    #[test]
    fn test_invalid_status_string_returns_error() {
        let result = QueueStatus::from_str("invalid_status");
        assert!(result.is_err());
    }

    // --- TryFrom<String> Tests ---

    #[test]
    fn test_try_from_string_valid() -> Result<()> {
        let status = QueueStatus::try_from("pending".to_string());
        assert!(status.is_ok());
        assert_eq!(status?, QueueStatus::Pending);
        Ok(())
    }

    #[test]
    fn test_try_from_string_invalid() {
        let result = QueueStatus::try_from("not_a_status".to_string());
        assert!(result.is_err());
    }

    // --- All States Test ---

    #[test]
    fn test_all_returns_all_statuses() {
        let all = QueueStatus::all();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&QueueStatus::Pending));
        assert!(all.contains(&QueueStatus::Claimed));
        assert!(all.contains(&QueueStatus::Rebasing));
        assert!(all.contains(&QueueStatus::Testing));
        assert!(all.contains(&QueueStatus::ReadyToMerge));
        assert!(all.contains(&QueueStatus::Merging));
        assert!(all.contains(&QueueStatus::Merged));
        assert!(all.contains(&QueueStatus::FailedRetryable));
        assert!(all.contains(&QueueStatus::FailedTerminal));
        assert!(all.contains(&QueueStatus::Cancelled));
    }

    // --- WorkspaceQueueState Tests ---

    #[test]
    fn test_workspace_queue_state_as_str() {
        assert_eq!(WorkspaceQueueState::Created.as_str(), "created");
        assert_eq!(WorkspaceQueueState::Working.as_str(), "working");
        assert_eq!(WorkspaceQueueState::Ready.as_str(), "ready");
        assert_eq!(WorkspaceQueueState::Merged.as_str(), "merged");
        assert_eq!(WorkspaceQueueState::Abandoned.as_str(), "abandoned");
        assert_eq!(WorkspaceQueueState::Conflict.as_str(), "conflict");
    }

    #[test]
    fn test_workspace_queue_state_from_str() -> Result<()> {
        assert_eq!(
            WorkspaceQueueState::from_str("created")?,
            WorkspaceQueueState::Created
        );
        assert_eq!(
            WorkspaceQueueState::from_str("working")?,
            WorkspaceQueueState::Working
        );
        assert_eq!(
            WorkspaceQueueState::from_str("ready")?,
            WorkspaceQueueState::Ready
        );
        assert_eq!(
            WorkspaceQueueState::from_str("merged")?,
            WorkspaceQueueState::Merged
        );
        assert_eq!(
            WorkspaceQueueState::from_str("abandoned")?,
            WorkspaceQueueState::Abandoned
        );
        assert_eq!(
            WorkspaceQueueState::from_str("conflict")?,
            WorkspaceQueueState::Conflict
        );
        Ok(())
    }

    #[test]
    fn test_workspace_queue_state_default() {
        assert_eq!(WorkspaceQueueState::default(), WorkspaceQueueState::Created);
    }

    // --- QueueEventType Tests ---

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(QueueEventType::Created.as_str(), "created");
        assert_eq!(QueueEventType::Claimed.as_str(), "claimed");
        assert_eq!(QueueEventType::Transitioned.as_str(), "transitioned");
        assert_eq!(QueueEventType::Failed.as_str(), "failed");
        assert_eq!(QueueEventType::Retried.as_str(), "retried");
        assert_eq!(QueueEventType::Cancelled.as_str(), "cancelled");
        assert_eq!(QueueEventType::Merged.as_str(), "merged");
        assert_eq!(QueueEventType::Heartbeat.as_str(), "heartbeat");
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(QueueEventType::Created.to_string(), "created");
        assert_eq!(QueueEventType::Claimed.to_string(), "claimed");
    }

    #[test]
    fn test_event_type_from_str() -> Result<()> {
        assert_eq!(
            QueueEventType::from_str("created")?,
            QueueEventType::Created
        );
        assert_eq!(
            QueueEventType::from_str("claimed")?,
            QueueEventType::Claimed
        );
        assert_eq!(
            QueueEventType::from_str("transitioned")?,
            QueueEventType::Transitioned
        );
        assert_eq!(QueueEventType::from_str("failed")?, QueueEventType::Failed);
        assert_eq!(
            QueueEventType::from_str("retried")?,
            QueueEventType::Retried
        );
        assert_eq!(
            QueueEventType::from_str("cancelled")?,
            QueueEventType::Cancelled
        );
        assert_eq!(QueueEventType::from_str("merged")?, QueueEventType::Merged);
        assert_eq!(
            QueueEventType::from_str("heartbeat")?,
            QueueEventType::Heartbeat
        );
        Ok(())
    }

    #[test]
    fn test_event_type_invalid_returns_error() {
        assert!(QueueEventType::from_str("invalid").is_err());
    }
}
