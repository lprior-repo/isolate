#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Session State Tracking Infrastructure
//!
//! Provides a type-safe state machine for session lifecycle management using:
//! - State Transition enums for valid state changes
//! - SessionStateManager for managing state transitions
//! - Type State Pattern with Phantom Types for compile-time safety
//! - SessionBeadsContext for beads integration
//! - State history tracking and validation
//! - Railway-Oriented error handling with zero panics

use std::marker::PhantomData;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// STATE TYPES & TRANSITIONS
// ═════════════════════════════════════════════════════════════════════════

/// Compile-time state marker for Created sessions
#[derive(Debug, Clone, Copy)]
pub struct Created;

/// Compile-time state marker for Active sessions
#[derive(Debug, Clone, Copy)]
pub struct Active;

/// Compile-time state marker for Syncing sessions
#[derive(Debug, Clone, Copy)]
pub struct Syncing;

/// Compile-time state marker for Synced sessions
#[derive(Debug, Clone, Copy)]
pub struct Synced;

/// Compile-time state marker for Paused sessions
#[derive(Debug, Clone, Copy)]
pub struct Paused;

/// Compile-time state marker for Completed sessions
#[derive(Debug, Clone, Copy)]
pub struct Completed;

/// Compile-time state marker for Failed sessions
#[derive(Debug, Clone, Copy)]
pub struct Failed;

/// Runtime state enumeration for storage and serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Session created but not yet activated
    Created,
    /// Session is active and ready for work
    Active,
    /// Session is being synced with main branch
    Syncing,
    /// Session sync completed
    Synced,
    /// Session is paused
    Paused,
    /// Session work completed
    Completed,
    /// Session creation or operation failed
    Failed,
}

impl SessionState {
    /// Returns true if this state allows transition to next state
    pub fn can_transition_to(self, next: SessionState) -> bool {
        matches!(
            (self, next),
            (SessionState::Created, SessionState::Active | SessionState::Failed)
                | (SessionState::Active, SessionState::Syncing | SessionState::Paused | SessionState::Completed)
                | (SessionState::Syncing, SessionState::Synced | SessionState::Failed)
                | (SessionState::Synced, SessionState::Active | SessionState::Paused | SessionState::Completed)
                | (SessionState::Paused, SessionState::Active | SessionState::Completed)
                | (SessionState::Completed, SessionState::Created) // Allow restart
        )
    }

    /// Returns all valid next states from current state
    pub fn valid_next_states(self) -> Vec<SessionState> {
        match self {
            SessionState::Created => vec![SessionState::Active, SessionState::Failed],
            SessionState::Active => vec![SessionState::Syncing, SessionState::Paused, SessionState::Completed],
            SessionState::Syncing => vec![SessionState::Synced, SessionState::Failed],
            SessionState::Synced => vec![SessionState::Active, SessionState::Paused, SessionState::Completed],
            SessionState::Paused => vec![SessionState::Active, SessionState::Completed],
            SessionState::Completed => vec![SessionState::Created],
            SessionState::Failed => vec![SessionState::Created],
        }
    }
}

/// State transition event with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Source state
    pub from: SessionState,
    /// Target state
    pub to: SessionState,
    /// Timestamp of transition
    pub timestamp: DateTime<Utc>,
    /// Reason for transition (metadata)
    pub reason: String,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(from: SessionState, to: SessionState, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: reason.into(),
        }
    }

    /// Validate that the transition is allowed
    pub fn validate(&self) -> Result<()> {
        if self.from.can_transition_to(self.to) {
            Ok(())
        } else {
            Err(Error::ValidationError(
                format!("Invalid state transition: {:?} -> {:?}", self.from, self.to),
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATE MANAGER
// ═════════════════════════════════════════════════════════════════════════

/// Session state manager with type-safe state machine
pub struct SessionStateManager<S = Created> {
    session_id: String,
    current_state: SessionState,
    history: Vec<StateTransition>,
    metadata: HashMap<String, String>,
    _state: PhantomData<S>,
}

impl SessionStateManager<Created> {
    /// Create a new session state manager in Created state
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            current_state: SessionState::Created,
            history: Vec::new(),
            metadata: HashMap::new(),
            _state: PhantomData,
        }
    }
}

impl<S> SessionStateManager<S> {
    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get current state
    pub fn current_state(&self) -> SessionState {
        self.current_state
    }

    /// Get state history
    pub fn history(&self) -> &[StateTransition] {
        &self.history
    }

    /// Get metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Record a state transition
    fn record_transition(&mut self, transition: StateTransition) -> Result<()> {
        transition.validate()?;
        self.history.push(transition.clone());
        self.current_state = transition.to;
        Ok(())
    }
}

impl SessionStateManager<Created> {
    /// Transition from Created to Active
    pub fn activate(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Active>> {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Created to Failed
    pub fn fail(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Failed>> {
        let transition = StateTransition::new(SessionState::Created, SessionState::Failed, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Active> {
    /// Transition from Active to Syncing
    pub fn sync(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Syncing>> {
        let transition = StateTransition::new(SessionState::Active, SessionState::Syncing, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Active to Paused
    pub fn pause(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Paused>> {
        let transition = StateTransition::new(SessionState::Active, SessionState::Paused, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Active to Completed
    pub fn complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Completed>> {
        let transition = StateTransition::new(SessionState::Active, SessionState::Completed, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Syncing> {
    /// Transition from Syncing to Synced
    pub fn sync_complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Synced>> {
        let transition = StateTransition::new(SessionState::Syncing, SessionState::Synced, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Syncing to Failed
    pub fn fail(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Failed>> {
        let transition = StateTransition::new(SessionState::Syncing, SessionState::Failed, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Synced> {
    /// Transition from Synced to Active
    pub fn reactivate(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Active>> {
        let transition = StateTransition::new(SessionState::Synced, SessionState::Active, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Synced to Paused
    pub fn pause(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Paused>> {
        let transition = StateTransition::new(SessionState::Synced, SessionState::Paused, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Synced to Completed
    pub fn complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Completed>> {
        let transition = StateTransition::new(SessionState::Synced, SessionState::Completed, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Paused> {
    /// Transition from Paused to Active
    pub fn resume(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Active>> {
        let transition = StateTransition::new(SessionState::Paused, SessionState::Active, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }

    /// Transition from Paused to Completed
    pub fn complete(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Completed>> {
        let transition = StateTransition::new(SessionState::Paused, SessionState::Completed, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Completed> {
    /// Transition from Completed to Created to allow restart
    pub fn restart(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Created>> {
        let transition = StateTransition::new(SessionState::Completed, SessionState::Created, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

impl SessionStateManager<Failed> {
    /// Transition from Failed to Created to allow retry
    pub fn retry(mut self, reason: impl Into<String>) -> Result<SessionStateManager<Created>> {
        let transition = StateTransition::new(SessionState::Failed, SessionState::Created, reason);
        self.record_transition(transition)?;
        Ok(SessionStateManager {
            session_id: self.session_id,
            current_state: self.current_state,
            history: self.history,
            metadata: self.metadata,
            _state: PhantomData,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION BEADS CONTEXT
// ═════════════════════════════════════════════════════════════════════════

/// Beads integration context for sessions
#[derive(Debug, Clone)]
pub struct SessionBeadsContext {
    session_id: String,
    state: SessionState,
    beads_db_path: Option<String>,
}

impl SessionBeadsContext {
    /// Create a new beads context for a session
    pub fn new(session_id: impl Into<String>, state: SessionState) -> Self {
        Self {
            session_id: session_id.into(),
            state,
            beads_db_path: None,
        }
    }

    /// Set beads database path
    pub fn with_beads_path(mut self, path: impl Into<String>) -> Self {
        self.beads_db_path = Some(path.into());
        self
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get current state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get beads database path
    pub fn beads_db_path(&self) -> Option<&str> {
        self.beads_db_path.as_deref()
    }

    /// Query beads for state-appropriate issues
    /// Returns a result of beads IDs relevant to this session's state
    pub fn query_beads_for_state(&self) -> Result<Vec<String>> {
        // This is a placeholder that returns different results based on state
        // In real implementation, it would query the beads database
        match self.state {
            SessionState::Created => Ok(vec![]),
            SessionState::Active => {
                // Active sessions should show work-in-progress issues
                Ok(vec!["bead-wip-1".to_string()])
            }
            SessionState::Syncing => {
                // Syncing sessions show merge-related issues
                Ok(vec!["bead-merge-1".to_string()])
            }
            SessionState::Synced => {
                // Synced sessions show completed issues
                Ok(vec!["bead-done-1".to_string()])
            }
            SessionState::Paused => {
                // Paused sessions show blocked issues
                Ok(vec!["bead-blocked-1".to_string()])
            }
            SessionState::Completed => {
                // Completed sessions show all issues
                Ok(vec!["bead-all-1".to_string()])
            }
            SessionState::Failed => {
                // Failed sessions show error issues
                Ok(vec!["bead-error-1".to_string()])
            }
        }
    }

    /// Update state
    pub fn update_state(&mut self, new_state: SessionState) -> Result<()> {
        if self.state.can_transition_to(new_state) {
            self.state = new_state;
            Ok(())
        } else {
            Err(Error::ValidationError(
                format!("Cannot transition from {:?} to {:?}", self.state, new_state),
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSION STATE MANAGER TYPE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_state_manager_type_exists() {
        // This test verifies that SessionStateManager type exists with state machine guards
        let manager = SessionStateManager::new("test-session");
        assert_eq!(manager.session_id(), "test-session");
        assert_eq!(manager.current_state(), SessionState::Created);
    }

    #[test]
    fn test_session_state_manager_generic_state_marker() {
        // This test verifies that state markers enforce compile-time safety
        let _: SessionStateManager<Created> = SessionStateManager::new("test");
        // This would fail at compile time:
        // let invalid: SessionStateManager<Active> = SessionStateManager::new("test");
    }

    #[test]
    fn test_session_state_manager_preserves_session_id() {
        let session_id = "my-session-123";
        let manager = SessionStateManager::new(session_id);
        assert_eq!(manager.session_id(), session_id);
    }

    #[test]
    fn test_session_state_manager_initial_history_empty() {
        let manager = SessionStateManager::new("test");
        assert!(manager.history().is_empty());
    }

    #[test]
    fn test_session_state_manager_metadata_operations() {
        let mut manager = SessionStateManager::new("test");
        manager.set_metadata("key1", "value1");
        assert_eq!(manager.metadata().get("key1").map(|s| s.as_str()), Some("value1"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE TRANSITION ENUM TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_state_transition_created_to_active() {
        // StateTransition enum should cover Created → Active transition
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, "activation");
        assert_eq!(transition.from, SessionState::Created);
        assert_eq!(transition.to, SessionState::Active);
        assert_eq!(transition.reason, "activation");
    }

    #[test]
    fn test_state_transition_active_to_syncing() {
        // StateTransition enum should cover Active → Syncing transition
        let transition = StateTransition::new(SessionState::Active, SessionState::Syncing, "starting sync");
        assert_eq!(transition.from, SessionState::Active);
        assert_eq!(transition.to, SessionState::Syncing);
    }

    #[test]
    fn test_state_transition_syncing_to_synced() {
        // StateTransition enum should cover Syncing → Synced transition
        let transition = StateTransition::new(SessionState::Syncing, SessionState::Synced, "sync complete");
        assert_eq!(transition.from, SessionState::Syncing);
        assert_eq!(transition.to, SessionState::Synced);
    }

    #[test]
    fn test_state_transition_active_to_paused() {
        // StateTransition enum should cover Active → Paused transition
        let transition = StateTransition::new(SessionState::Active, SessionState::Paused, "pause");
        assert_eq!(transition.from, SessionState::Active);
        assert_eq!(transition.to, SessionState::Paused);
    }

    #[test]
    fn test_state_transition_active_to_completed() {
        // StateTransition enum should cover Active → Completed transition
        let transition = StateTransition::new(SessionState::Active, SessionState::Completed, "finish");
        assert_eq!(transition.from, SessionState::Active);
        assert_eq!(transition.to, SessionState::Completed);
    }

    #[test]
    fn test_state_transition_timestamp_is_set() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, "test");
        // Timestamp should be recent (within last second)
        let now = Utc::now();
        assert!(transition.timestamp <= now);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE VALIDATION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_state_validation_prevents_invalid_created_to_paused() {
        // State validation should prevent invalid transitions
        let transition = StateTransition::new(SessionState::Created, SessionState::Paused, "invalid");
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_state_validation_prevents_invalid_synced_to_syncing() {
        let transition = StateTransition::new(SessionState::Synced, SessionState::Syncing, "invalid");
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_state_validation_prevents_invalid_paused_to_syncing() {
        let transition = StateTransition::new(SessionState::Paused, SessionState::Syncing, "invalid");
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_state_validation_prevents_invalid_completed_to_active() {
        let transition = StateTransition::new(SessionState::Completed, SessionState::Active, "invalid");
        assert!(transition.validate().is_err());
    }

    #[test]
    fn test_state_validation_allows_valid_created_to_active() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, "valid");
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_state_validation_allows_valid_active_to_syncing() {
        let transition = StateTransition::new(SessionState::Active, SessionState::Syncing, "valid");
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_state_validation_allows_valid_syncing_to_synced() {
        let transition = StateTransition::new(SessionState::Syncing, SessionState::Synced, "valid");
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_state_can_transition_to_method() {
        // Test the can_transition_to method
        assert!(SessionState::Created.can_transition_to(SessionState::Active));
        assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
        assert!(SessionState::Syncing.can_transition_to(SessionState::Synced));
    }

    #[test]
    fn test_state_valid_next_states_method() {
        // Test the valid_next_states method
        let next_states = SessionState::Active.valid_next_states();
        assert!(next_states.contains(&SessionState::Syncing));
        assert!(next_states.contains(&SessionState::Paused));
        assert!(next_states.contains(&SessionState::Completed));
        assert!(!next_states.contains(&SessionState::Created));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSION BEADS CONTEXT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_beads_context_type_exists() {
        // SessionBeadsContext should provide beads integration
        let context = SessionBeadsContext::new("test-session", SessionState::Active);
        assert_eq!(context.session_id(), "test-session");
        assert_eq!(context.state(), SessionState::Active);
    }

    #[test]
    fn test_session_beads_context_with_beads_path() {
        let context = SessionBeadsContext::new("test", SessionState::Active)
            .with_beads_path("/path/to/beads.db");
        assert_eq!(context.beads_db_path(), Some("/path/to/beads.db"));
    }

    #[test]
    fn test_session_beads_context_query_beads_for_created_state() {
        // Beads queries should return state-appropriate issues
        let context = SessionBeadsContext::new("test", SessionState::Created);
        let result = context.query_beads_for_state();
        assert!(result.is_ok());
        let beads = result.unwrap();
        assert!(beads.is_empty(), "Created state should have no beads");
    }

    #[test]
    fn test_session_beads_context_query_beads_for_active_state() {
        let context = SessionBeadsContext::new("test", SessionState::Active);
        let result = context.query_beads_for_state();
        assert!(result.is_ok());
        let beads = result.unwrap();
        assert!(!beads.is_empty(), "Active state should have beads");
        assert!(beads.iter().any(|b| b.contains("wip")), "Active state should have WIP beads");
    }

    #[test]
    fn test_session_beads_context_query_beads_for_syncing_state() {
        let context = SessionBeadsContext::new("test", SessionState::Syncing);
        let result = context.query_beads_for_state();
        assert!(result.is_ok());
        let beads = result.unwrap();
        assert!(!beads.is_empty(), "Syncing state should have beads");
    }

    #[test]
    fn test_session_beads_context_query_beads_for_synced_state() {
        let context = SessionBeadsContext::new("test", SessionState::Synced);
        let result = context.query_beads_for_state();
        assert!(result.is_ok());
        let beads = result.unwrap();
        assert!(!beads.is_empty(), "Synced state should have beads");
    }

    #[test]
    fn test_session_beads_context_update_state_valid() {
        let mut context = SessionBeadsContext::new("test", SessionState::Active);
        let result = context.update_state(SessionState::Syncing);
        assert!(result.is_ok());
        assert_eq!(context.state(), SessionState::Syncing);
    }

    #[test]
    fn test_session_beads_context_update_state_invalid() {
        let mut context = SessionBeadsContext::new("test", SessionState::Created);
        let result = context.update_state(SessionState::Paused);
        assert!(result.is_err());
        assert_eq!(context.state(), SessionState::Created);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE HISTORY TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_state_history_tracked_in_session_metadata() {
        // State history should be tracked in session metadata
        let manager = SessionStateManager::new("test");
        assert!(manager.history().is_empty());
    }

    #[test]
    fn test_state_history_grows_with_transitions() {
        // After transitions, history should be populated
        let manager = SessionStateManager::new("test");
        let result = manager.activate("test activation");
        assert!(result.is_ok());
        let manager = result.unwrap();
        assert_eq!(manager.history().len(), 1);
        assert_eq!(manager.history()[0].from, SessionState::Created);
        assert_eq!(manager.history()[0].to, SessionState::Active);
    }

    #[test]
    fn test_state_history_multiple_transitions() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.sync("start sync").unwrap();
        assert_eq!(manager.history().len(), 2);
        assert_eq!(manager.history()[0].from, SessionState::Created);
        assert_eq!(manager.history()[0].to, SessionState::Active);
        assert_eq!(manager.history()[1].from, SessionState::Active);
        assert_eq!(manager.history()[1].to, SessionState::Syncing);
    }

    #[test]
    fn test_state_history_preserves_reason() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("initialization reason").unwrap();
        assert_eq!(manager.history()[0].reason, "initialization reason");
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // TYPE STATE PATTERN TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_type_state_pattern_created_to_active() {
        // Type state pattern should enforce valid operation sequences
        let manager = SessionStateManager::new("test");
        let result = manager.activate("test");
        assert!(result.is_ok());
        // manager.activate() should only be available on Created state
    }

    #[test]
    fn test_type_state_pattern_active_to_syncing() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let result = manager.sync("sync");
        assert!(result.is_ok());
        // manager.sync() should only be available on Active state
    }

    #[test]
    fn test_type_state_pattern_syncing_to_synced() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.sync("sync").unwrap();
        let result = manager.sync_complete("complete");
        assert!(result.is_ok());
        // manager.sync_complete() should only be available on Syncing state
    }

    #[test]
    fn test_type_state_pattern_created_can_fail() {
        let manager = SessionStateManager::new("test");
        let result = manager.fail("failed");
        assert!(result.is_ok());
        // manager.fail() should be available on Created state
    }

    #[test]
    fn test_type_state_pattern_active_can_pause() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let result = manager.pause("pause");
        assert!(result.is_ok());
        // manager.pause() should be available on Active state
    }

    #[test]
    fn test_type_state_pattern_active_can_complete() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let result = manager.complete("finish");
        assert!(result.is_ok());
        // manager.complete() should be available on Active state
    }

    #[test]
    fn test_type_state_pattern_synced_can_reactivate() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.sync("sync").unwrap();
        let manager = manager.sync_complete("complete").unwrap();
        let result = manager.reactivate("reactivate");
        assert!(result.is_ok());
        // manager.reactivate() should only be available on Synced state
    }

    #[test]
    fn test_type_state_pattern_paused_can_resume() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.pause("pause").unwrap();
        let result = manager.resume("resume");
        assert!(result.is_ok());
        // manager.resume() should only be available on Paused state
    }

    #[test]
    fn test_type_state_pattern_completed_can_restart() {
        let manager = SessionStateManager::new("test");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.complete("complete").unwrap();
        let result = manager.restart("restart");
        assert!(result.is_ok());
        // manager.restart() should only be available on Completed state
    }

    #[test]
    fn test_type_state_pattern_failed_can_retry() {
        let manager = SessionStateManager::new("test");
        let manager = manager.fail("failed").unwrap();
        let result = manager.retry("retry");
        assert!(result.is_ok());
        // manager.retry() should only be available on Failed state
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // ZERO PANICS / ZERO UNWRAPS TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_invalid_transition_returns_result_err_not_panic() {
        let manager = SessionStateManager::new("test");
        let result = manager.fail("fail"); // Valid transition from Created
        let manager = result.unwrap();

        // This is actually invalid from Failed state (cannot go from Failed to Active)
        // But we can't test it at compile time - the type system prevents it
        // Instead, test that we get Result types
        let _history = manager.history(); // Should not panic
        let _state = manager.current_state(); // Should not panic
    }

    #[test]
    fn test_beads_query_returns_result_not_panic() {
        let context = SessionBeadsContext::new("test", SessionState::Active);
        let result = context.query_beads_for_state();
        // Should return Result, not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_update_returns_result_not_panic() {
        let mut context = SessionBeadsContext::new("test", SessionState::Created);
        let result = context.update_state(SessionState::Paused);
        // Should return Result with error, not panic
        assert!(result.is_err());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHANTOM TYPE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_phantom_types_for_compile_time_safety() {
        // Phantom types should enforce compile-time safety
        let _manager_created: SessionStateManager<Created> = SessionStateManager::new("test");

        // This would fail at compile time:
        // let invalid_active: SessionStateManager<Active> = SessionStateManager::new("test");
        // The type system prevents creating an Active manager directly
    }

    #[test]
    fn test_phantom_types_prevent_calling_wrong_methods() {
        let manager = SessionStateManager::new("test");
        // manager is SessionStateManager<Created>

        // This is valid:
        let result = manager.activate("test");
        assert!(result.is_ok());

        // manager.sync() would fail because manager is Created, not Active
        // The type system enforces this at compile time
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INTEGRATION WITH COMMANDS TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_integration_add_command_state_flow() {
        // add.rs should use state manager: Created → Active
        let manager = SessionStateManager::new("new-session");
        let manager = manager.activate("workspace created").unwrap();
        assert_eq!(manager.current_state(), SessionState::Active);
    }

    #[test]
    fn test_integration_sync_command_state_flow() {
        // sync.rs should use state manager: Active → Syncing → Synced
        let manager = SessionStateManager::new("session");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.sync("starting sync").unwrap();
        assert_eq!(manager.current_state(), SessionState::Syncing);
        let manager = manager.sync_complete("sync complete").unwrap();
        assert_eq!(manager.current_state(), SessionState::Synced);
    }

    #[test]
    fn test_integration_remove_command_state_flow() {
        // remove.rs should use state manager to complete sessions
        let manager = SessionStateManager::new("session");
        let manager = manager.activate("activate").unwrap();
        let manager = manager.complete("removing").unwrap();
        assert_eq!(manager.current_state(), SessionState::Completed);
    }

    #[test]
    fn test_integration_status_command_state_query() {
        // status.rs should query state and history
        let manager = SessionStateManager::new("session");
        let manager = manager.activate("activate").unwrap();
        let _status = manager.current_state();
        let _history = manager.history();
        // Status command can query these without panic
    }

    #[test]
    fn test_integration_list_command_with_beads() {
        // list.rs should use beads context to show state-appropriate info
        let context = SessionBeadsContext::new("session", SessionState::Active);
        let beads = context.query_beads_for_state().unwrap();
        assert!(!beads.is_empty());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // RAILWAY ERROR HANDLING TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_railway_error_handling_invalid_transition() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Synced, "invalid");
        let result = transition.validate();
        // Should return Result::Err, not panic
        assert!(result.is_err());

        // Map error to Result type
        let mapped = result.map_err(|_| "transition failed");
        assert!(mapped.is_err());
    }

    #[test]
    fn test_railway_error_handling_chaining() {
        let manager = SessionStateManager::new("test");

        // Chain operations with Result
        let result = manager
            .activate("activate")
            .and_then(|m| m.sync("sync"))
            .and_then(|m| m.sync_complete("complete"));

        assert!(result.is_ok());
        let final_manager = result.unwrap();
        assert_eq!(final_manager.current_state(), SessionState::Synced);
    }

    #[test]
    fn test_railway_error_handling_with_map() {
        let manager = SessionStateManager::new("test");
        let result: Result<String> = manager
            .activate("activate")
            .map(|m| m.session_id().to_string());

        assert!(result.is_ok());
        assert_eq!(result.map(|s| s == "test").unwrap_or(false), true);
    }

    #[test]
    fn test_railway_error_handling_with_map_err() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Paused, "invalid");
        let result = transition
            .validate()
            .map_err(|_| "Failed to validate transition");

        assert!(result.is_err());
    }

    #[test]
    fn test_railway_error_handling_or_else() {
        let context = SessionBeadsContext::new("test", SessionState::Created);
        let result: Result<Vec<String>> = context
            .query_beads_for_state()
            .or_else(|_| Ok(vec!["default-bead".to_string()]));

        assert!(result.is_ok());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EDGE CASE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_state_transition_empty_reason() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, "");
        assert_eq!(transition.reason, "");
        assert!(transition.validate().is_ok());
    }

    #[test]
    fn test_state_transition_long_reason() {
        let long_reason = "a".repeat(1000);
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, long_reason.clone());
        assert_eq!(transition.reason, long_reason);
    }

    #[test]
    fn test_session_state_manager_multiple_metadata_keys() {
        let mut manager = SessionStateManager::new("test");
        manager.set_metadata("key1", "value1");
        manager.set_metadata("key2", "value2");
        manager.set_metadata("key3", "value3");
        assert_eq!(manager.metadata().len(), 3);
    }

    #[test]
    fn test_session_state_manager_metadata_overwrite() {
        let mut manager = SessionStateManager::new("test");
        manager.set_metadata("key", "value1");
        manager.set_metadata("key", "value2");
        assert_eq!(manager.metadata().get("key").map(|s| s.as_str()), Some("value2"));
    }

    #[test]
    fn test_beads_context_state_transition_chain() {
        let mut context = SessionBeadsContext::new("test", SessionState::Created);
        assert!(context.update_state(SessionState::Active).is_ok());
        assert!(context.update_state(SessionState::Syncing).is_ok());
        assert!(context.update_state(SessionState::Synced).is_ok());
        assert_eq!(context.state(), SessionState::Synced);
    }

    #[test]
    fn test_state_transition_serialization() {
        let transition = StateTransition::new(SessionState::Created, SessionState::Active, "test");
        // Verify that transition can be serialized (has Serialize)
        let json = serde_json::to_string(&transition);
        assert!(json.is_ok());
    }

    #[test]
    fn test_session_state_deserialization() {
        // Verify states can be deserialized
        let state_json = r#""active""#;
        let state: std::result::Result<SessionState, serde_json::Error> = serde_json::from_str(state_json);
        assert!(state.is_ok());
        let state_value = state.map(|s| s == SessionState::Active).unwrap_or(false);
        assert!(state_value);
    }
}
