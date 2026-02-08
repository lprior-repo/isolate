//! Bead repository trait for managing bead status
//!
//! This module provides a trait for interacting with the beads database,
//! allowing for easy testing and potential future backends.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use thiserror::Error;

use super::newtypes::{BeadId, WorkspaceName};
use crate::beads::{BeadRepository as RealBeadRepo, BeadStatus};

/// Bead repository errors
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BeadError {
    #[error("Bead not found: {0}")]
    NotFound(String),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[expect(dead_code)] // Reserved for future JSONL parsing errors
    #[error("Corrupt JSON: {0}")]
    CorruptJson(String),

    #[error("File locking error: {0}")]
    LockError(String),
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait for bead repository operations
pub trait BeadRepository: Send + Sync {
    /// Find bead by workspace name
    fn find_by_workspace<'a>(
        &'a self,
        workspace: &'a WorkspaceName,
    ) -> BoxFuture<'a, Result<Option<BeadId>, BeadError>>;

    /// Update bead status
    fn update_status<'a>(
        &'a mut self,
        id: &'a BeadId,
        status: &'a str,
    ) -> BoxFuture<'a, Result<(), BeadError>>;
}

/// Real bead repository implementation
pub struct RealBeadRepository {
    inner: RealBeadRepo,
}

impl RealBeadRepository {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self {
            inner: RealBeadRepo::new(root),
        }
    }
}

impl BeadRepository for RealBeadRepository {
    fn find_by_workspace<'a>(
        &'a self,
        workspace: &'a WorkspaceName,
    ) -> BoxFuture<'a, Result<Option<BeadId>, BeadError>> {
        Box::pin(async move {
            match self.inner.get_bead(workspace.as_str()).await {
                Ok(Some(bead)) => Ok(Some(
                    BeadId::new(bead.id).map_err(|e| BeadError::DatabaseError(e.to_string()))?,
                )),
                Ok(None) => Ok(None),
                Err(e) => Err(BeadError::DatabaseError(e.to_string())),
            }
        })
    }

    fn update_status<'a>(
        &'a mut self,
        id: &'a BeadId,
        status: &'a str,
    ) -> BoxFuture<'a, Result<(), BeadError>> {
        Box::pin(async move {
            let bead_status = status
                .parse::<BeadStatus>()
                .map_err(|e| BeadError::InvalidStatus(e.to_string()))?;
            self.inner
                .update_status(id.as_str(), bead_status)
                .await
                .map_err(|e| BeadError::DatabaseError(e.to_string()))
        })
    }
}

/// Mock bead repository for testing
#[derive(Debug, Clone)]
pub struct MockBeadRepository {
    beads: Arc<Mutex<HashMap<String, BeadData>>>,
    workspace_to_bead: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Debug, Clone)]
struct BeadData {
    #[expect(dead_code)] // Used for debugging/future features
    id: String,
    status: String,
}

impl MockBeadRepository {
    /// Create a new `MockBeadRepository`
    pub fn new() -> Self {
        Self {
            beads: Arc::new(Mutex::new(HashMap::new())),
            workspace_to_bead: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a bead for testing
    #[expect(dead_code)] // For future test expansion
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
    #[expect(dead_code)] // For future test expansion
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
    fn find_by_workspace<'a>(
        &'a self,
        workspace: &'a WorkspaceName,
    ) -> BoxFuture<'a, Result<Option<BeadId>, BeadError>> {
        Box::pin(async move {
            let bead_id = self
                .workspace_to_bead
                .lock()
                .map_err(|e| BeadError::LockError(format!("Lock poisoned: {e}")))?
                .get(workspace.as_str())
                .cloned();

            bead_id.map_or(Ok(None), |id| {
                BeadId::new(id)
                    .map(Some)
                    .map_err(|e| BeadError::DatabaseError(e.to_string()))
            })
        })
    }

    fn update_status<'a>(
        &'a mut self,
        id: &'a BeadId,
        status: &'a str,
    ) -> BoxFuture<'a, Result<(), BeadError>> {
        let status = status.to_string();
        Box::pin(async move {
            // Validate status
            let valid_statuses = ["open", "in_progress", "closed", "blocked"];
            if !valid_statuses.contains(&status.as_str()) {
                return Err(BeadError::InvalidStatus(format!(
                    "Status must be one of: {valid_statuses:?}"
                )));
            }

            let mut beads = self
                .beads
                .lock()
                .map_err(|e| BeadError::LockError(format!("Lock poisoned: {e}")))?;

            if let Some(bead) = beads.get_mut(id.as_str()) {
                bead.status = status;
                Ok(())
            } else {
                Err(BeadError::NotFound(id.as_str().to_string()))
            }
        })
    }
}
