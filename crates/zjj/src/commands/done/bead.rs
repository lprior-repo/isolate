//! Bead repository trait for managing bead status
//!
//! This module provides a trait for interacting with the beads database,
//! allowing for easy testing and potential future backends.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::{
    future::Future,
    pin::Pin,
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
