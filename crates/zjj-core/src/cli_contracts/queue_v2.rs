//! KIRK Contracts for Queue CLI operations.
//!
//! Queue manages the merge train for sessions.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::cli_contracts::{
    domain_types::{Priority, QueueStatus, SessionName},
    Contract, ContractError, Invariant, Postcondition, Precondition,
};

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for adding a session to the queue.
#[derive(Debug, Clone)]
pub struct EnqueueInput {
    /// Session name
    pub session: SessionName,
    /// Priority (lower = higher priority)
    pub priority: Option<Priority>,
}

/// Input for removing a session from the queue.
#[derive(Debug, Clone)]
pub struct DequeueInput {
    /// Session name
    pub session: SessionName,
}

/// Input for listing the queue.
#[derive(Debug, Clone, Default)]
pub struct ListQueueInput {
    /// Include completed entries
    pub include_completed: bool,
}

/// Input for processing the queue.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProcessQueueInput {
    /// Maximum entries to process
    pub max_entries: Option<usize>,
    /// Dry run mode
    pub dry_run: bool,
}

/// Result of queue operations.
#[derive(Debug, Clone)]
pub struct QueueResult {
    /// Session name
    pub session: SessionName,
    /// Queue position (1-indexed)
    pub position: QueuePosition,
    /// Queue status
    pub status: QueueStatus,
}

/// A validated queue position (must be >= 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueuePosition(u32);

impl QueuePosition {
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    pub fn validate(position: u32) -> Result<(), ContractError> {
        if position == 0 {
            return Err(ContractError::PostconditionFailed {
                name: "valid_position",
                description: "Queue position must be >= 1",
            });
        }
        Ok(())
    }
}

impl TryFrom<u32> for QueuePosition {
    type Error = ContractError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value))
    }
}

/// Result of queue listing.
#[derive(Debug, Clone)]
pub struct QueueListResult {
    /// Queue entries
    pub entries: Vec<QueueResult>,
    /// Total count
    pub total: usize,
    /// Currently processing (if any)
    pub processing: Option<SessionName>,
}

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Queue CLI operations.
pub struct QueueContracts;

impl QueueContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: session exists.
    pub const PRECOND_SESSION_EXISTS: Precondition =
        Precondition::new("session_exists", "Session must exist in the database");

    /// Precondition: session is not already queued.
    pub const PRECOND_NOT_ALREADY_QUEUED: Precondition = Precondition::new(
        "not_already_queued",
        "Session must not already be in the queue",
    );

    /// Precondition: session is ready for merge.
    pub const PRECOND_SESSION_READY: Precondition = Precondition::new(
        "session_ready",
        "Session must be in 'ready' state to join the queue",
    );

    /// Precondition: session is in queue (for dequeue).
    pub const PRECOND_IN_QUEUE: Precondition =
        Precondition::new("in_queue", "Session must be in the queue");

    /// Precondition: no session is currently processing (for add).
    pub const PRECOND_NO_PROCESSING: Precondition = Precondition::new(
        "no_processing",
        "Cannot add to queue while processing is in progress",
    );

    /// Precondition: queue is not at capacity.
    pub const PRECOND_QUEUE_CAPACITY: Precondition =
        Precondition::new("queue_capacity", "Queue must not exceed maximum capacity");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: queue positions are unique.
    pub const INV_POSITIONS_UNIQUE: Invariant =
        Invariant::documented("positions_unique", "Each queue entry has a unique position");

    /// Invariant: queue is ordered by priority.
    pub const INV_ORDERED_BY_PRIORITY: Invariant = Invariant::documented(
        "ordered_by_priority",
        "Queue entries are ordered by priority",
    );

    /// Invariant: only one session processing at a time.
    pub const INV_SINGLE_PROCESSING: Invariant =
        Invariant::documented("single_processing", "At most one session can be processing");

    /// Invariant: session appears at most once in queue.
    pub const INV_SESSION_UNIQUE: Invariant = Invariant::documented(
        "session_unique",
        "Each session appears at most once in queue",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: session was added to queue.
    pub const POST_ENQUEUED: Postcondition =
        Postcondition::new("enqueued", "Session is in queue with status 'pending'");

    /// Postcondition: session was removed from queue.
    pub const POST_DEQUEUED: Postcondition =
        Postcondition::new("dequeued", "Session is no longer in the queue");

    /// Postcondition: queue positions are consecutive.
    pub const POST_POSITIONS_CONSECUTIVE: Postcondition = Postcondition::new(
        "positions_consecutive",
        "Queue positions are 1, 2, 3, ... without gaps",
    );
}

impl Contract<EnqueueInput, QueueResult> for QueueContracts {
    fn preconditions(_input: &EnqueueInput) -> Result<(), ContractError> {
        // Validation is now done at the boundary when creating SessionName and Priority
        Ok(())
    }

    fn invariants(_input: &EnqueueInput) -> Vec<Invariant> {
        vec![
            Self::INV_POSITIONS_UNIQUE,
            Self::INV_SESSION_UNIQUE,
            Self::INV_SINGLE_PROCESSING,
        ]
    }

    fn postconditions(input: &EnqueueInput, result: &QueueResult) -> Result<(), ContractError> {
        if result.session != input.session {
            return Err(ContractError::PostconditionFailed {
                name: "session_matches",
                description: "Result session must match input",
            });
        }
        if result.status != QueueStatus::Pending {
            return Err(ContractError::PostconditionFailed {
                name: "initial_status",
                description: "New queue entries must have status 'pending'",
            });
        }
        Ok(())
    }
}

impl Contract<DequeueInput, ()> for QueueContracts {
    fn preconditions(_input: &DequeueInput) -> Result<(), ContractError> {
        Ok(())
    }

    fn invariants(_input: &DequeueInput) -> Vec<Invariant> {
        vec![Self::INV_ORDERED_BY_PRIORITY]
    }

    fn postconditions(_input: &DequeueInput, _result: &()) -> Result<(), ContractError> {
        Ok(())
    }
}

impl Contract<ListQueueInput, QueueListResult> for QueueContracts {
    fn preconditions(_input: &ListQueueInput) -> Result<(), ContractError> {
        Ok(())
    }

    fn invariants(_input: &ListQueueInput) -> Vec<Invariant> {
        vec![Self::INV_ORDERED_BY_PRIORITY, Self::INV_SINGLE_PROCESSING]
    }

    fn postconditions(
        _input: &ListQueueInput,
        result: &QueueListResult,
    ) -> Result<(), ContractError> {
        // Verify positions are consecutive starting from 1
        for (idx, entry) in result.entries.iter().enumerate() {
            let expected_position =
                u32::try_from(idx + 1).map_err(|_| ContractError::PostconditionFailed {
                    name: "position_overflow",
                    description: "Too many entries for position numbering",
                })?;
            let expected = QueuePosition::try_from(expected_position)?;
            if entry.position != expected {
                return Err(ContractError::PostconditionFailed {
                    name: "positions_consecutive",
                    description: "Positions must be consecutive starting from 1",
                });
            }
        }

        if result.entries.len() > result.total {
            return Err(ContractError::PostconditionFailed {
                name: "count_consistent",
                description: "Entry count must not exceed total",
            });
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_position_valid() {
        assert!(QueuePosition::try_from(1).is_ok());
        assert!(QueuePosition::try_from(100).is_ok());
    }

    #[test]
    fn test_queue_position_invalid() {
        assert!(QueuePosition::try_from(0).is_err());
    }

    #[test]
    fn test_enqueue_contract_preconditions() {
        macro_rules! unwrap_ok {
            ($expr:expr, $msg:expr) => {
                match $expr {
                    Ok(v) => v,
                    Err(e) => panic!("{}: {:?}", $msg, e),
                }
            };
        }

        let input = EnqueueInput {
            session: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            priority: Some(unwrap_ok!(Priority::try_from(10), "Failed to create Priority")),
        };
        assert!(QueueContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_enqueue_contract_postconditions() {
        macro_rules! unwrap_ok {
            ($expr:expr, $msg:expr) => {
                match $expr {
                    Ok(v) => v,
                    Err(e) => panic!("{}: {:?}", $msg, e),
                }
            };
        }

        let input = EnqueueInput {
            session: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            priority: None,
        };
        let result = QueueResult {
            session: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            position: unwrap_ok!(QueuePosition::try_from(1), "Failed to create QueuePosition"),
            status: QueueStatus::Pending,
        };
        assert!(QueueContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_enqueue_contract_postconditions_wrong_status() {
        macro_rules! unwrap_ok {
            ($expr:expr, $msg:expr) => {
                match $expr {
                    Ok(v) => v,
                    Err(e) => panic!("{}: {:?}", $msg, e),
                }
            };
        }

        let input = EnqueueInput {
            session: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            priority: None,
        };
        let result = QueueResult {
            session: unwrap_ok!(SessionName::try_from("test-session"), "Failed to create SessionName"),
            position: unwrap_ok!(QueuePosition::try_from(1), "Failed to create QueuePosition"),
            status: QueueStatus::Processing, // Wrong!
        };
        assert!(QueueContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_queue_contract_postconditions_consecutive() {
        macro_rules! unwrap_ok {
            ($expr:expr, $msg:expr) => {
                match $expr {
                    Ok(v) => v,
                    Err(e) => panic!("{}: {:?}", $msg, e),
                }
            };
        }

        let input = ListQueueInput::default();
        let result = QueueListResult {
            entries: vec![
                QueueResult {
                    session: unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName"),
                    position: unwrap_ok!(QueuePosition::try_from(1), "Failed to create QueuePosition"),
                    status: QueueStatus::Pending,
                },
                QueueResult {
                    session: unwrap_ok!(SessionName::try_from("s2"), "Failed to create SessionName"),
                    position: unwrap_ok!(QueuePosition::try_from(2), "Failed to create QueuePosition"),
                    status: QueueStatus::Pending,
                },
            ],
            total: 2,
            processing: None,
        };
        assert!(QueueContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_queue_contract_postconditions_not_consecutive() {
        macro_rules! unwrap_ok {
            ($expr:expr, $msg:expr) => {
                match $expr {
                    Ok(v) => v,
                    Err(e) => panic!("{}: {:?}", $msg, e),
                }
            };
        }

        let input = ListQueueInput::default();
        let result = QueueListResult {
            entries: vec![
                QueueResult {
                    session: unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName"),
                    position: unwrap_ok!(QueuePosition::try_from(1), "Failed to create QueuePosition"),
                    status: QueueStatus::Pending,
                },
                QueueResult {
                    session: unwrap_ok!(SessionName::try_from("s2"), "Failed to create SessionName"),
                    position: unwrap_ok!(QueuePosition::try_from(3), "Failed to create QueuePosition"), // Gap!
                    status: QueueStatus::Pending,
                },
            ],
            total: 2,
            processing: None,
        };
        assert!(QueueContracts::postconditions(&input, &result).is_err());
    }
}
