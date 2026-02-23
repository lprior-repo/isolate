//! `QueueEntry` aggregate root with business rules and invariants.
//!
//! The `QueueEntry` aggregate represents a work item in the distributed queue with:
//! - Unique identity (database-generated `i64`)
//! - Workspace to process
//! - Optional bead to process
//! - Priority (lower = higher priority)
//! - Claim state (Unclaimed, Claimed, Expired)
//! - Creation timestamp
//!
//! # Invariants
//!
//! 1. Queue entry IDs are unique
//! 2. Claim state transitions are restricted:
//!    - Unclaimed -> Claimed
//!    - Claimed -> Expired | Unclaimed
//!    - Expired -> Unclaimed
//!    - No self-loops
//! 3. Claims must have valid expiration timestamps
//! 4. Only unclaimed entries can be claimed
//! 5. Only the owning agent can release a claim

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Duration, Utc};

use thiserror::Error;

use crate::domain::identifiers::{AgentId, BeadId, WorkspaceName};
use crate::domain::queue::ClaimState;

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

/// Identity and metadata for queue entry reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueEntryMetadata {
    /// Entry ID
    pub id: i64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl QueueEntryMetadata {
    /// Create new metadata.
    #[must_use]
    pub const fn new(id: i64, created_at: DateTime<Utc>) -> Self {
        Self { id, created_at }
    }
}

/// Errors that can occur during queue entry operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum QueueEntryError {
    /// Invalid claim transition
    #[error("invalid claim transition: {from:?} -> {to:?}")]
    InvalidClaimTransition {
        from: ClaimState,
        to: ClaimState,
    },

    /// Entry is not claimed
    #[error("entry is not claimed")]
    NotClaimed,

    /// Entry is already claimed
    #[error("entry is already claimed by {0}")]
    AlreadyClaimed(AgentId),

    /// Entry is claimed by a different agent
    #[error("entry is claimed by {actual}, not {expected}")]
    NotOwner { actual: AgentId, expected: AgentId },

    /// Claim has expired
    #[error("claim has expired")]
    ClaimExpired,

    /// Invalid expiration time (must be in the future)
    #[error("expiration time must be in the future")]
    InvalidExpiration,

    /// Negative priority values are not allowed
    #[error("priority cannot be negative")]
    NegativePriority,

    /// Cannot modify entry in current state
    #[error("cannot modify entry in state: {0:?}")]
    CannotModify(ClaimState),
}

// ============================================================================
// QUEUE ENTRY AGGREGATE ROOT
// ============================================================================

/// Queue entry aggregate root.
///
/// Enforces all business rules and invariants for queue entries.
/// All state transitions go through validated methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    /// Unique entry ID (database-generated)
    pub id: i64,
    /// Workspace to process
    pub workspace: WorkspaceName,
    /// Optional bead to process
    pub bead: Option<BeadId>,
    /// Priority (lower = higher priority)
    pub priority: i32,
    /// Current claim state
    pub claim_state: ClaimState,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl QueueEntry {
    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Create a new unclaimed queue entry.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::NegativePriority` if priority is negative.
    pub fn new(
        id: i64,
        workspace: WorkspaceName,
        bead: Option<BeadId>,
        priority: i32,
    ) -> Result<Self, QueueEntryError> {
        if priority < 0 {
            return Err(QueueEntryError::NegativePriority);
        }

        Ok(Self {
            id,
            workspace,
            bead,
            priority,
            claim_state: ClaimState::Unclaimed,
            created_at: Utc::now(),
        })
    }

    /// Reconstruct a queue entry from persisted data.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::NegativePriority` if priority is negative.
    pub fn reconstruct(
        workspace: WorkspaceName,
        bead: Option<BeadId>,
        priority: i32,
        claim_state: ClaimState,
        metadata: QueueEntryMetadata,
    ) -> Result<Self, QueueEntryError> {
        if priority < 0 {
            return Err(QueueEntryError::NegativePriority);
        }

        Ok(Self {
            id: metadata.id,
            workspace,
            bead,
            priority,
            claim_state,
            created_at: metadata.created_at,
        })
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Check if entry is unclaimed.
    #[must_use]
    pub const fn is_unclaimed(&self) -> bool {
        self.claim_state.is_unclaimed()
    }

    /// Check if entry is claimed.
    #[must_use]
    pub const fn is_claimed(&self) -> bool {
        self.claim_state.is_claimed()
    }

    /// Check if entry claim has expired.
    #[must_use]
    pub const fn is_expired(&self) -> bool {
        matches!(self.claim_state, ClaimState::Expired { .. })
    }

    /// Get the current claim holder if claimed.
    #[must_use]
    pub const fn claim_holder(&self) -> Option<&AgentId> {
        self.claim_state.holder()
    }

    /// Check if entry is claimable (unclaimed and not expired).
    #[must_use]
    pub const fn is_claimable(&self) -> bool {
        self.is_unclaimed()
    }

    /// Check if a claim has expired based on current time.
    #[must_use]
    pub fn has_claim_expired(&self) -> bool {
        match &self.claim_state {
            ClaimState::Claimed { expires_at, .. } => *expires_at < Utc::now(),
            _ => false,
        }
    }

    // ========================================================================
    // CLAIM TRANSITION METHODS
    // ========================================================================

    /// Claim the entry for an agent.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::AlreadyClaimed` if entry is already claimed.
    /// Returns `QueueEntryError::InvalidExpiration` if duration is invalid.
    pub fn claim(
        &self,
        agent: AgentId,
        claim_duration_secs: i64,
    ) -> Result<Self, QueueEntryError> {
        if !self.is_unclaimed() {
            // Get the holder if it exists
            if let Some(holder) = self.claim_holder() {
                return Err(QueueEntryError::AlreadyClaimed(holder.clone()));
            }
            // Shouldn't happen but handle gracefully
            return Err(QueueEntryError::AlreadyClaimed(agent));
        }

        if claim_duration_secs <= 0 {
            return Err(QueueEntryError::InvalidExpiration);
        }

        let now = Utc::now();
        let expires_at = now + Duration::seconds(claim_duration_secs);

        let new_state = ClaimState::Claimed {
            agent,
            claimed_at: now,
            expires_at,
        };

        if !self.claim_state.can_transition_to(&new_state) {
            return Err(QueueEntryError::InvalidClaimTransition {
                from: self.claim_state.clone(),
                to: new_state,
            });
        }

        Ok(Self {
            claim_state: new_state,
            ..self.clone()
        })
    }

    /// Release a claim by the owning agent.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::NotClaimed` if entry is not claimed.
    /// Returns `QueueEntryError::NotOwner` if agent is not the claim holder.
    pub fn release(&self, agent: &AgentId) -> Result<Self, QueueEntryError> {
        let holder = self.claim_holder().ok_or(QueueEntryError::NotClaimed)?;

        if holder != agent {
            return Err(QueueEntryError::NotOwner {
                actual: holder.clone(),
                expected: agent.clone(),
            });
        }

        let new_state = ClaimState::Unclaimed;

        if !self.claim_state.can_transition_to(&new_state) {
            return Err(QueueEntryError::InvalidClaimTransition {
                from: self.claim_state.clone(),
                to: new_state,
            });
        }

        Ok(Self {
            claim_state: new_state,
            ..self.clone()
        })
    }

    /// Expire the current claim.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::NotClaimed` if entry is not claimed.
    pub fn expire_claim(&self) -> Result<Self, QueueEntryError> {
        let holder = self
            .claim_holder()
            .ok_or(QueueEntryError::NotClaimed)?
            .clone();

        let now = Utc::now();
        let new_state = ClaimState::Expired {
            previous_agent: holder,
            expired_at: now,
        };

        if !self.claim_state.can_transition_to(&new_state) {
            return Err(QueueEntryError::InvalidClaimTransition {
                from: self.claim_state.clone(),
                to: new_state,
            });
        }

        Ok(Self {
            claim_state: new_state,
            ..self.clone()
        })
    }

    /// Reclaim an expired entry (transition from Expired to Unclaimed).
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::InvalidClaimTransition` if state is not Expired.
    pub fn reclaim(&self) -> Result<Self, QueueEntryError> {
        if !self.is_expired() {
            return Err(QueueEntryError::InvalidClaimTransition {
                from: self.claim_state.clone(),
                to: ClaimState::Unclaimed,
            });
        }

        Ok(Self {
            claim_state: ClaimState::Unclaimed,
            ..self.clone()
        })
    }

    /// Refresh a claim (extend expiration time).
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::NotClaimed` if entry is not claimed.
    /// Returns `QueueEntryError::NotOwner` if agent is not the claim holder.
    /// Returns `QueueEntryError::InvalidExpiration` if duration is invalid.
    pub fn refresh_claim(
        &self,
        agent: &AgentId,
        claim_duration_secs: i64,
    ) -> Result<Self, QueueEntryError> {
        let holder = self.claim_holder().ok_or(QueueEntryError::NotClaimed)?;

        if holder != agent {
            return Err(QueueEntryError::NotOwner {
                actual: holder.clone(),
                expected: agent.clone(),
            });
        }

        if claim_duration_secs <= 0 {
            return Err(QueueEntryError::InvalidExpiration);
        }

        let now = Utc::now();
        let expires_at = now + Duration::seconds(claim_duration_secs);

        Ok(Self {
            claim_state: ClaimState::Claimed {
                agent: holder.clone(),
                claimed_at: now,
                expires_at,
            },
            ..self.clone()
        })
    }

    // ========================================================================
    // PRIORITY METHODS
    // ========================================================================

    /// Update the priority.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::CannotModify` if entry is claimed.
    /// Returns `QueueEntryError::NegativePriority` if priority is negative.
    pub fn update_priority(&self, new_priority: i32) -> Result<Self, QueueEntryError> {
        if self.is_claimed() {
            return Err(QueueEntryError::CannotModify(self.claim_state.clone()));
        }

        if new_priority < 0 {
            return Err(QueueEntryError::NegativePriority);
        }

        Ok(Self {
            priority: new_priority,
            ..self.clone()
        })
    }

    // ========================================================================
    // VALIDATION METHODS
    // ========================================================================

    /// Validate that the entry can be claimed.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::AlreadyClaimed` if entry is claimed.
    pub fn validate_can_claim(&self) -> Result<(), QueueEntryError> {
        if !self.is_unclaimed() {
            // Get the holder if it exists
            if let Some(holder) = self.claim_holder() {
                return Err(QueueEntryError::AlreadyClaimed(holder.clone()));
            }
            // Fallback - should not happen in practice
            let fallback = AgentId::parse("unknown").map_err(|_| QueueEntryError::InvalidExpiration)?;
            return Err(QueueEntryError::AlreadyClaimed(fallback));
        }
        Ok(())
    }

    /// Validate that an agent is the claim holder.
    ///
    /// # Errors
    ///
    /// Returns `QueueEntryError::NotClaimed` if entry is not claimed.
    /// Returns `QueueEntryError::NotOwner` if agent is not the holder.
    pub fn validate_is_owner(&self, agent: &AgentId) -> Result<(), QueueEntryError> {
        let holder = self.claim_holder().ok_or(QueueEntryError::NotClaimed)?;

        if holder != agent {
            return Err(QueueEntryError::NotOwner {
                actual: holder.clone(),
                expected: agent.clone(),
            });
        }

        Ok(())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(id: i64) -> QueueEntry {
        let workspace = WorkspaceName::parse("test-workspace").expect("valid name");
        QueueEntry::new(id, workspace, None, 0).expect("entry created")
    }

    #[test]
    fn test_create_entry() {
        let entry = create_test_entry(1);

        assert!(entry.is_unclaimed());
        assert!(!entry.is_claimed());
        assert!(entry.is_claimable());
        assert_eq!(entry.priority, 0);
    }

    #[test]
    fn test_claim_entry() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

        assert!(claimed.is_claimed());
        assert!(!claimed.is_unclaimed());
        assert_eq!(claimed.claim_holder(), Some(&agent));
    }

    #[test]
    fn test_claim_already_claimed() {
        let entry = create_test_entry(1);
        let agent1 = AgentId::parse("agent-1").expect("valid agent");
        let agent2 = AgentId::parse("agent-2").expect("valid agent");

        let claimed = entry.claim(agent1.clone(), 300).expect("claim valid");

        let result = claimed.claim(agent2, 300);
        assert!(matches!(result, Err(QueueEntryError::AlreadyClaimed(_))));
    }

    #[test]
    fn test_release_claim() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

        let released = claimed.release(&agent).expect("release valid");

        assert!(released.is_unclaimed());
        assert!(released.is_claimable());
    }

    #[test]
    fn test_release_not_owner() {
        let entry = create_test_entry(1);
        let agent1 = AgentId::parse("agent-1").expect("valid agent");
        let agent2 = AgentId::parse("agent-2").expect("valid agent");

        let claimed = entry.claim(agent1, 300).expect("claim valid");

        let result = claimed.release(&agent2);
        assert!(matches!(result, Err(QueueEntryError::NotOwner { .. })));
    }

    #[test]
    fn test_expire_claim() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        let claimed = entry.claim(agent, 300).expect("claim valid");

        let expired = claimed.expire_claim().expect("expire valid");

        assert!(expired.is_expired());
        assert!(!expired.is_claimed());
    }

    #[test]
    fn test_reclaim_expired() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        let claimed = entry.claim(agent, 300).expect("claim valid");
        let expired = claimed.expire_claim().expect("expire valid");

        let reclaimed = expired.reclaim().expect("reclaim valid");

        assert!(reclaimed.is_unclaimed());
        assert!(reclaimed.is_claimable());
    }

    #[test]
    fn test_refresh_claim() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        let claimed = entry.claim(agent.clone(), 300).expect("claim valid");

        let refreshed = claimed.refresh_claim(&agent, 600).expect("refresh valid");

        assert!(refreshed.is_claimed());
        assert_eq!(refreshed.claim_holder(), Some(&agent));
    }

    #[test]
    fn test_invalid_claim_duration() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        // Negative duration
        let result = entry.claim(agent.clone(), -1);
        assert!(matches!(result, Err(QueueEntryError::InvalidExpiration)));

        // Zero duration
        let result = entry.claim(agent, 0);
        assert!(matches!(result, Err(QueueEntryError::InvalidExpiration)));
    }

    #[test]
    fn test_negative_priority() {
        let workspace = WorkspaceName::parse("test").expect("valid name");

        let result = QueueEntry::new(1, workspace, None, -1);
        assert!(matches!(result, Err(QueueEntryError::NegativePriority)));
    }

    #[test]
    fn test_update_priority() {
        let entry = create_test_entry(1);

        let updated = entry.update_priority(5).expect("update valid");

        assert_eq!(updated.priority, 5);
    }

    #[test]
    fn test_cannot_update_priority_when_claimed() {
        let entry = create_test_entry(1);
        let agent = AgentId::parse("agent-1").expect("valid agent");

        let claimed = entry.claim(agent, 300).expect("claim valid");

        let result = claimed.update_priority(10);
        assert!(matches!(
            result,
            Err(QueueEntryError::CannotModify(_))
        ));
    }

    #[test]
    fn test_validate_can_claim() {
        let entry = create_test_entry(1);

        assert!(entry.validate_can_claim().is_ok());

        let agent = AgentId::parse("agent-1").expect("valid agent");
        let claimed = entry.claim(agent, 300).expect("claim valid");

        let result = claimed.validate_can_claim();
        assert!(matches!(result, Err(QueueEntryError::AlreadyClaimed(_))));
    }

    #[test]
    fn test_validate_is_owner() {
        let entry = create_test_entry(1);
        let agent1 = AgentId::parse("agent-1").expect("valid agent");
        let agent2 = AgentId::parse("agent-2").expect("valid agent");

        // Not claimed
        let result = entry.validate_is_owner(&agent1);
        assert!(matches!(result, Err(QueueEntryError::NotClaimed)));

        let claimed = entry.claim(agent1.clone(), 300).expect("claim valid");

        // Is owner
        assert!(claimed.validate_is_owner(&agent1).is_ok());

        // Not owner
        let result = claimed.validate_is_owner(&agent2);
        assert!(matches!(result, Err(QueueEntryError::NotOwner { .. })));
    }

    #[test]
    fn test_reconstruct() {
        let workspace = WorkspaceName::parse("test").expect("valid name");
        let bead = BeadId::parse("bd-1").expect("valid bead");
        let agent = AgentId::parse("agent-1").expect("valid agent");
        let now = Utc::now();
        let expires = now + Duration::seconds(300);

        let entry = QueueEntry::reconstruct(
            workspace,
            Some(bead),
            10,
            ClaimState::Claimed {
                agent,
                claimed_at: now,
                expires_at: expires,
            },
            QueueEntryMetadata::new(1, now),
        )
        .expect("reconstruct valid");

        assert_eq!(entry.id, 1);
        assert_eq!(entry.priority, 10);
        assert!(entry.is_claimed());
        assert!(entry.bead.is_some());
    }
}
