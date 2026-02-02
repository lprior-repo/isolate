//! Bead repository trait for managing bead status
//!
//! This module provides a trait for interacting with the beads database,
//! allowing for easy testing and potential future backends.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use thiserror::Error;

use super::newtypes::{BeadId, WorkspaceName};

/// Bead repository errors
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BeadError {
    #[error("Bead not found: {0}")]
    NotFound(String),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Corrupt JSON: {0}")]
    CorruptJson(String),

    #[error("File locking error: {0}")]
    LockError(String),
}

/// Trait for bead repository operations
pub trait BeadRepository {
    /// Find bead by workspace name
    fn find_by_workspace(&self, workspace: &WorkspaceName) -> Result<Option<BeadId>, BeadError>;

    /// Update bead status
    fn update_status(&mut self, id: &BeadId, status: &str) -> Result<(), BeadError>;
}

/// Mock bead repository for testing
#[derive(Debug, Clone)]
pub struct MockBeadRepository {
    beads: Arc<Mutex<HashMap<String, BeadData>>>,
    workspace_to_bead: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Debug, Clone)]
struct BeadData {
    id: String,
    status: String,
}

impl MockBeadRepository {
    /// Create a new MockBeadRepository
    pub fn new() -> Self {
        Self {
            beads: Arc::new(Mutex::new(HashMap::new())),
            workspace_to_bead: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a bead for testing
    pub fn add_bead(&self, id: String, workspace: String, status: String) {
        if let Ok(mut beads) = self.beads.lock() {
            beads.insert(
                id.clone(),
                BeadData {
                    id: id.clone(),
                    status,
                },
            );
        }
        if let Ok(mut mapping) = self.workspace_to_bead.lock() {
            mapping.insert(workspace, id);
        }
    }

    /// Get bead status
    pub fn get_status(&self, id: &str) -> Option<String> {
        self.beads.lock().ok()?.get(id).map(|b| b.status.clone())
    }
}

impl Default for MockBeadRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl BeadRepository for MockBeadRepository {
    fn find_by_workspace(&self, workspace: &WorkspaceName) -> Result<Option<BeadId>, BeadError> {
        let bead_id = self
            .workspace_to_bead
            .lock()
            .map_err(|e| BeadError::LockError(format!("Lock poisoned: {}", e)))?
            .get(workspace.as_str())
            .cloned();

        match bead_id {
            Some(id) => BeadId::new(id)
                .map(Some)
                .map_err(|e| BeadError::DatabaseError(e.to_string())),
            None => Ok(None),
        }
    }

    fn update_status(&mut self, id: &BeadId, status: &str) -> Result<(), BeadError> {
        // Validate status
        let valid_statuses = ["open", "in_progress", "closed", "blocked"];
        if !valid_statuses.contains(&status) {
            return Err(BeadError::InvalidStatus(format!(
                "Status must be one of: {:?}",
                valid_statuses
            )));
        }

        let mut beads = self
            .beads
            .lock()
            .map_err(|e| BeadError::LockError(format!("Lock poisoned: {}", e)))?;

        if let Some(bead) = beads.get_mut(id.as_str()) {
            bead.status = status.to_string();
            Ok(())
        } else {
            Err(BeadError::NotFound(id.as_str().to_string()))
        }
    }
}
