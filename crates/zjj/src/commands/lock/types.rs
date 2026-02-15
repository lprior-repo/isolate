//! Types for lock/unlock commands

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(dead_code)]

use std::{future::Future, pin::Pin};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::SessionDb;

/// Trait for checking session existence.
///
/// This abstraction allows tests to bypass session validation
/// while production code uses the real session database.
pub trait SessionExists: Send + Sync {
    fn session_exists(
        &self,
        session_name: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<bool>> + Send + 'static>>;
}

/// Production implementation that checks against the session database.
pub struct ProductionSessionValidator {
    db: SessionDb,
}

impl ProductionSessionValidator {
    #[must_use]
    pub fn new(db: SessionDb) -> Self {
        Self { db }
    }
}

impl SessionExists for ProductionSessionValidator {
    fn session_exists(
        &self,
        session_name: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<bool>> + Send + 'static>> {
        let db = self.db.clone();
        let session_name = session_name.to_string();
        Box::pin(async move {
            db.get(&session_name)
                .await
                .map(|opt| opt.is_some())
                .map_err(|e| anyhow::anyhow!("Failed to check session existence: {e}"))
        })
    }
}

#[derive(Debug, Clone)]
pub struct LockArgs {
    pub session: String,
    pub agent_id: Option<String>,
    pub ttl: u64,
}

#[derive(Debug, Clone)]
pub struct UnlockArgs {
    pub session: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockOutput {
    pub success: bool,
    pub session: String,
    pub locked: bool,
    pub lock_id: Option<String>,
    pub holder: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockOutput {
    pub success: bool,
    pub session: String,
    pub released: bool,
}
