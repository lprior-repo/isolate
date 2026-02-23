//! KIRK Contracts for Queue CLI operations.
//!
//! Queue manages the merge train for sessions.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for adding a session to the queue.
#[derive(Debug, Clone)]
pub struct EnqueueInput {
    /// Session name
    pub session: String,
    /// Priority (lower = higher priority)
    pub priority: Option<u32>,
}

/// Input for removing a session from the queue.
#[derive(Debug, Clone)]
pub struct DequeueInput {
    /// Session name
    pub session: String,
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
    pub session: String,
    /// Queue position (1-indexed)
    pub position: u32,
    /// Queue status
    pub status: String,
}

/// Result of queue listing.
#[derive(Debug, Clone)]
pub struct QueueListResult {
    /// Queue entries
    pub entries: Vec<QueueResult>,
    /// Total count
    pub total: usize,
    /// Currently processing (if any)
    pub processing: Option<String>,
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

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate a queue status.
    ///
    /// # Errors
    /// Returns `ContractError` if the status is invalid.
    pub fn validate_status(status: &str) -> Result<(), ContractError> {
        match status {
            "pending" | "processing" | "completed" | "failed" | "cancelled" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: pending, processing, completed, failed, cancelled",
            )),
        }
    }

    /// Validate a priority value.
    ///
    /// # Errors
    /// Returns `ContractError` if the priority is invalid.
    pub fn validate_priority(priority: u32) -> Result<(), ContractError> {
        // Priority 0 is highest, 1000 is lowest
        if priority > 1000 {
            return Err(ContractError::invalid_input(
                "priority",
                "must be between 0 and 1000",
            ));
        }
        Ok(())
    }
}

impl Contract<EnqueueInput, QueueResult> for QueueContracts {
    fn preconditions(input: &EnqueueInput) -> Result<(), ContractError> {
        if input.session.trim().is_empty() {
            return Err(ContractError::invalid_input("session", "cannot be empty"));
        }

        if let Some(priority) = input.priority {
            Self::validate_priority(priority)?;
        }

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
        if result.position == 0 {
            return Err(ContractError::PostconditionFailed {
                name: "valid_position",
                description: "Queue position must be >= 1",
            });
        }
        if result.status != "pending" {
            return Err(ContractError::PostconditionFailed {
                name: "initial_status",
                description: "New queue entries must have status 'pending'",
            });
        }
        Ok(())
    }
}

impl Contract<DequeueInput, ()> for QueueContracts {
    fn preconditions(input: &DequeueInput) -> Result<(), ContractError> {
        if input.session.trim().is_empty() {
            return Err(ContractError::invalid_input("session", "cannot be empty"));
        }
        Ok(())
    }

    fn invariants(_input: &DequeueInput) -> Vec<Invariant> {
        vec![Self::INV_ORDERED_BY_PRIORITY]
    }

    fn postconditions(_input: &DequeueInput, _result: &()) -> Result<(), ContractError> {
        // Session should no longer be in queue - verified by caller
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
        let positions: Vec<u32> = result.entries.iter().map(|e| e.position).collect();

        for (idx, &pos) in positions.iter().enumerate() {
            let expected =
                u32::try_from(idx + 1).map_err(|_| ContractError::PostconditionFailed {
                    name: "position_overflow",
                    description: "Too many entries for position numbering",
                })?;
            if pos != expected {
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
    fn test_validate_status_valid() {
        assert!(QueueContracts::validate_status("pending").is_ok());
        assert!(QueueContracts::validate_status("processing").is_ok());
        assert!(QueueContracts::validate_status("completed").is_ok());
        assert!(QueueContracts::validate_status("failed").is_ok());
        assert!(QueueContracts::validate_status("cancelled").is_ok());
    }

    #[test]
    fn test_validate_status_invalid() {
        assert!(QueueContracts::validate_status("waiting").is_err());
        assert!(QueueContracts::validate_status("done").is_err());
    }

    #[test]
    fn test_validate_priority_valid() {
        assert!(QueueContracts::validate_priority(0).is_ok());
        assert!(QueueContracts::validate_priority(500).is_ok());
        assert!(QueueContracts::validate_priority(1000).is_ok());
    }

    #[test]
    fn test_validate_priority_invalid() {
        assert!(QueueContracts::validate_priority(1001).is_err());
    }

    #[test]
    fn test_enqueue_contract_preconditions() {
        let input = EnqueueInput {
            session: "test-session".to_string(),
            priority: Some(10),
        };
        assert!(QueueContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_enqueue_contract_preconditions_empty_session() {
        let input = EnqueueInput {
            session: String::new(),
            priority: None,
        };
        assert!(QueueContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_enqueue_contract_postconditions() {
        let input = EnqueueInput {
            session: "test-session".to_string(),
            priority: None,
        };
        let result = QueueResult {
            session: "test-session".to_string(),
            position: 1,
            status: "pending".to_string(),
        };
        assert!(QueueContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_enqueue_contract_postconditions_wrong_status() {
        let input = EnqueueInput {
            session: "test-session".to_string(),
            priority: None,
        };
        let result = QueueResult {
            session: "test-session".to_string(),
            position: 1,
            status: "processing".to_string(),
        };
        assert!(QueueContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_queue_contract_postpositions_consecutive() {
        let input = ListQueueInput::default();
        let result = QueueListResult {
            entries: vec![
                QueueResult {
                    session: "s1".to_string(),
                    position: 1,
                    status: "pending".to_string(),
                },
                QueueResult {
                    session: "s2".to_string(),
                    position: 2,
                    status: "pending".to_string(),
                },
            ],
            total: 2,
            processing: None,
        };
        assert!(QueueContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_queue_contract_postpositions_not_consecutive() {
        let input = ListQueueInput::default();
        let result = QueueListResult {
            entries: vec![
                QueueResult {
                    session: "s1".to_string(),
                    position: 1,
                    status: "pending".to_string(),
                },
                QueueResult {
                    session: "s2".to_string(),
                    position: 3, // Gap!
                    status: "pending".to_string(),
                },
            ],
            total: 2,
            processing: None,
        };
        assert!(QueueContracts::postconditions(&input, &result).is_err());
    }
}
