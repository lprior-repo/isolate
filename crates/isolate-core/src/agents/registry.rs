//! Agent registry with heartbeat tracking.
//!
//! Tracks active agents via `SQLite`, using timestamps to detect stale agents.

use std::fmt;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use crate::Error;

/// Result of a successful agent registration.
#[derive(Debug, Clone)]
pub struct RegistrationResult {
    /// The validated agent ID.
    pub agent_id: String,
    /// Whether this was a new registration (vs update of existing).
    pub is_new: bool,
    /// When the agent was registered.
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat timestamp.
    pub last_seen: DateTime<Utc>,
}

/// Heartbeat request from an agent.
///
/// Agent sends periodic heartbeats to indicate liveness.
/// The optional `command` field describes current work.
#[derive(Debug, Clone)]
pub struct HeartbeatRequest {
    /// Unique agent identifier.
    pub agent_id: String,
    /// Optional command describing current work.
    pub command: Option<String>,
}

/// Heartbeat response after successful heartbeat.
#[derive(Debug, Clone)]
pub struct HeartbeatOutput {
    /// The agent ID that sent the heartbeat.
    pub agent_id: String,
    /// Timestamp of the heartbeat (RFC3339 format).
    pub timestamp: String,
    /// Response message.
    pub message: String,
}

/// Errors that can occur during agent operations.
#[derive(Debug, Clone, Error)]
pub enum AgentError {
    /// Agent ID is empty or contains only whitespace.
    #[error("Agent ID cannot be empty")]
    EmptyId,

    /// Agent ID contains invalid characters.
    #[error("Agent ID contains invalid characters: {0}")]
    InvalidCharacters(String),

    /// Agent ID is a reserved keyword.
    #[error("Agent ID '{0}' is a reserved keyword")]
    ReservedKeyword(String),

    /// Agent ID is too long.
    #[error("Agent ID exceeds maximum length of {0} characters")]
    TooLong(usize),
}

impl AgentError {
    /// Convert to the general Error type.
    #[must_use]
    pub fn into_error(self) -> Error {
        match self {
            Self::EmptyId => Error::ValidationError {
                message: "Agent ID cannot be empty".to_string(),
                field: Some("agent_id".to_string()),
                value: None,
                constraints: vec!["non-empty".to_string()],
            },
            Self::InvalidCharacters(chars) => Error::ValidationError {
                message: format!("Agent ID contains invalid characters: {chars}"),
                field: Some("agent_id".to_string()),
                value: None,
                constraints: vec!["alphanumeric, dash, underscore only".to_string()],
            },
            Self::ReservedKeyword(keyword) => Error::ValidationError {
                message: format!("Agent ID '{keyword}' is a reserved keyword"),
                field: Some("agent_id".to_string()),
                value: Some(keyword),
                constraints: vec!["not a reserved keyword".to_string()],
            },
            Self::TooLong(max) => Error::ValidationError {
                message: format!("Agent ID exceeds maximum length of {max} characters"),
                field: Some("agent_id".to_string()),
                value: None,
                constraints: [format!("max {max} characters")].to_vec(),
            },
        }
    }
}

/// Reserved keywords that cannot be used as agent IDs.
const RESERVED_KEYWORDS: &[&str] = &["null", "undefined", "true", "false", "none", "default"];

/// Maximum allowed length for agent ID.
const MAX_AGENT_ID_LENGTH: usize = 128;

/// Validated agent ID newtype.
///
/// This type ensures that once created, the agent ID is always valid.
/// Validation happens at construction time (parse, don't validate pattern).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);

impl AgentId {
    /// Parse and validate an agent ID.
    ///
    /// Returns an error if the ID is empty, contains invalid characters,
    /// is a reserved keyword, or exceeds the maximum length.
    pub fn parse(s: impl Into<String>) -> Result<Self, AgentError> {
        let s = s.into();

        // Check for empty
        if s.trim().is_empty() {
            return Err(AgentError::EmptyId);
        }

        // Check length
        if s.len() > MAX_AGENT_ID_LENGTH {
            return Err(AgentError::TooLong(MAX_AGENT_ID_LENGTH));
        }

        // Check for invalid characters (alphanumeric, dash, underscore only)
        let invalid_chars: Vec<char> = s
            .chars()
            .filter(|c| !c.is_alphanumeric() && *c != '-' && *c != '_')
            .collect();

        if !invalid_chars.is_empty() {
            let chars_str: String = invalid_chars.iter().collect();
            return Err(AgentError::InvalidCharacters(chars_str));
        }

        // Check for reserved keywords (case-insensitive)
        let lower = s.to_lowercase();
        if RESERVED_KEYWORDS.contains(&lower.as_str()) {
            return Err(AgentError::ReservedKeyword(s));
        }

        Ok(Self(s))
    }

    /// Get the string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// An agent registry backed by `SQLite`.
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    db: SqlitePool,
    timeout_secs: u64,
}

/// An active agent record.
#[derive(Debug, Clone)]
pub struct ActiveAgent {
    /// Unique agent identifier.
    pub agent_id: String,
    /// Last heartbeat timestamp.
    pub last_seen: DateTime<Utc>,
    /// When the agent first registered.
    pub registered_at: DateTime<Utc>,
    /// Current session the agent is working on.
    pub current_session: Option<String>,
    /// Current command the agent is executing.
    pub current_command: Option<String>,
    /// Number of actions performed by the agent.
    pub actions_count: u64,
}

impl AgentRegistry {
    /// Create a new registry with the given pool and timeout.
    ///
    /// Creates the agents table if it does not exist.
    pub async fn new(db: SqlitePool, timeout_secs: u64) -> Result<Self, Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agents (
                agent_id TEXT PRIMARY KEY,
                last_seen TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                current_session TEXT,
                current_command TEXT,
                actions_count INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create agents table: {e}")))?;

        Ok(Self { db, timeout_secs })
    }

    /// Register an agent with validation.
    ///
    /// Validates the agent ID before registration.
    /// Returns a `RegistrationResult` with details about the registration.
    pub async fn register(&self, agent_id: &str) -> Result<RegistrationResult, Error> {
        // Parse and validate the agent ID (calculation layer)
        let validated_id = AgentId::parse(agent_id).map_err(AgentError::into_error)?;

        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();

        // Check if this is a new registration or update
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT agent_id FROM agents WHERE agent_id = ?1")
                .bind(validated_id.as_str())
                .fetch_optional(&self.db)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to check existing agent: {e}"))
                })?;

        let is_new = existing.is_none();

        sqlx::query(
            "INSERT INTO agents (agent_id, last_seen, registered_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?2",
        )
        .bind(validated_id.as_str())
        .bind(&now_rfc3339)
        .bind(&now_rfc3339)
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to register agent: {e}")))?;

        Ok(RegistrationResult {
            agent_id: validated_id.to_string(),
            is_new,
            registered_at: now,
            last_seen: now,
        })
    }

    /// Register an agent without validation (for internal use).
    ///
    /// This bypasses validation and should only be used when the agent ID
    /// has already been validated.
    pub async fn register_unchecked(&self, agent_id: &str) -> Result<RegistrationResult, Error> {
        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();

        // Check if this is a new registration or update
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT agent_id FROM agents WHERE agent_id = ?1")
                .bind(agent_id)
                .fetch_optional(&self.db)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to check existing agent: {e}"))
                })?;

        let is_new = existing.is_none();

        sqlx::query(
            "INSERT INTO agents (agent_id, last_seen, registered_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?2",
        )
        .bind(agent_id)
        .bind(&now_rfc3339)
        .bind(&now_rfc3339)
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to register agent: {e}")))?;

        Ok(RegistrationResult {
            agent_id: agent_id.to_string(),
            is_new,
            registered_at: now,
            last_seen: now,
        })
    }

    /// Update an agent's heartbeat timestamp.
    pub async fn heartbeat(&self, agent_id: &str) -> Result<(), Error> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
            .bind(&now)
            .bind(agent_id)
            .execute(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to heartbeat agent: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Agent not found: {agent_id}")));
        }
        Ok(())
    }

    /// Process an agent heartbeat with full contract.
    ///
    /// Updates `last_seen` timestamp, increments `actions_count`,
    /// and optionally updates `current_command`.
    ///
    /// # Contract
    ///
    /// - Precondition: Agent with given ID must exist
    /// - Postcondition Q1: `last_seen` updated to current time
    /// - Postcondition Q2: `actions_count` incremented by 1
    /// - Postcondition Q3: `current_command` updated if command provided
    /// - Postcondition Q4: `current_command` unchanged if command is None
    pub async fn heartbeat_with_request(
        &self,
        request: HeartbeatRequest,
    ) -> Result<HeartbeatOutput, Error> {
        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();

        // Execute query - update both timestamp and actions_count
        // current_command is updated via separate query if provided
        let result = sqlx::query(
            "UPDATE agents 
             SET last_seen = ?1, 
                 actions_count = actions_count + 1
             WHERE agent_id = ?2",
        )
        .bind(&now_rfc3339)
        .bind(&request.agent_id)
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to heartbeat agent: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Agent not found: {}",
                request.agent_id
            )));
        }

        // Update current_command if provided (separate query for simplicity)
        if let Some(ref cmd) = request.command {
            sqlx::query("UPDATE agents SET current_command = ?1 WHERE agent_id = ?2")
                .bind(cmd)
                .bind(&request.agent_id)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to update command: {e}")))?;
        }

        Ok(HeartbeatOutput {
            agent_id: request.agent_id,
            timestamp: now_rfc3339,
            message: "Heartbeat received".to_string(),
        })
    }

    /// Get all active agents (`last_seen` within timeout).
    pub async fn get_active(&self) -> Result<Vec<ActiveAgent>, Error> {
        let timeout_secs = i64::try_from(self.timeout_secs).map_or(i64::MAX, |v| v);
        let cutoff = Utc::now() - chrono::Duration::seconds(timeout_secs);
        let cutoff_str = cutoff.to_rfc3339();

        let rows: Vec<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT agent_id, last_seen, registered_at, current_session, current_command, actions_count
         FROM agents WHERE last_seen >= ?1",
        )
        .bind(&cutoff_str)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get active agents: {e}")))?;

        rows.into_iter()
            .map(
                |(
                    agent_id,
                    last_seen,
                    registered_at,
                    current_session,
                    current_command,
                    actions_count,
                )| {
                    let last_seen = DateTime::parse_from_rfc3339(&last_seen)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| {
                            Error::ParseError(format!("Invalid last_seen timestamp: {e}"))
                        })?;
                    let registered_at = DateTime::parse_from_rfc3339(&registered_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| {
                            Error::ParseError(format!("Invalid registered_at timestamp: {e}"))
                        })?;
                    let actions_count = u64::try_from(actions_count).map_or(0, |v| v);
                    Ok(ActiveAgent {
                        agent_id,
                        last_seen,
                        registered_at,
                        current_session,
                        current_command,
                        actions_count,
                    })
                },
            )
            .collect()
    }

    /// Get a specific agent by ID.
    pub async fn get(&self, agent_id: &str) -> Result<ActiveAgent, Error> {
        let row: Option<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT agent_id, last_seen, registered_at, current_session, current_command, actions_count
         FROM agents WHERE agent_id = ?1",
        )
        .bind(agent_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get agent: {e}")))?;

        row.map(
            |(
                agent_id,
                last_seen,
                registered_at,
                current_session,
                current_command,
                actions_count,
            )| {
                let last_seen = DateTime::parse_from_rfc3339(&last_seen)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| Error::ParseError(format!("Invalid last_seen timestamp: {e}")))?;
                let registered_at = DateTime::parse_from_rfc3339(&registered_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| {
                        Error::ParseError(format!("Invalid registered_at timestamp: {e}"))
                    })?;
                let actions_count = u64::try_from(actions_count).map_or(0, |v| v);
                Ok::<ActiveAgent, Error>(ActiveAgent {
                    agent_id,
                    last_seen,
                    registered_at,
                    current_session,
                    current_command,
                    actions_count,
                })
            },
        )
        .transpose()?
        .ok_or_else(|| Error::NotFound(format!("Agent not found: {agent_id}")))
    }

    /// Unregister an agent, removing its record.
    pub async fn unregister(&self, agent_id: &str) -> Result<(), Error> {
        let result = sqlx::query("DELETE FROM agents WHERE agent_id = ?1")
            .bind(agent_id)
            .execute(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to unregister agent: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Agent not found: {agent_id}")));
        }
        Ok(())
    }

    /// Check if an agent exists.
    pub async fn exists(&self, agent_id: &str) -> Result<bool, Error> {
        let result: Option<(String,)> =
            sqlx::query_as("SELECT agent_id FROM agents WHERE agent_id = ?1")
                .bind(agent_id)
                .fetch_optional(&self.db)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to check agent existence: {e}"))
                })?;

        Ok(result.is_some())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn test_pool() -> Result<SqlitePool, Error> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to create test pool: {e}")))
    }

    // ==========================================================================
    // AgentId Tests
    // ==========================================================================

    #[test]
    fn test_agent_id_parse_valid() {
        let id = AgentId::parse("agent-001").expect("Should parse valid ID");
        assert_eq!(id.as_str(), "agent-001");
    }

    #[test]
    fn test_agent_id_parse_with_underscore() {
        let id = AgentId::parse("agent_test_123").expect("Should parse valid ID");
        assert_eq!(id.as_str(), "agent_test_123");
    }

    #[test]
    fn test_agent_id_parse_empty() {
        let result = AgentId::parse("");
        assert!(matches!(result, Err(AgentError::EmptyId)));
    }

    #[test]
    fn test_agent_id_parse_whitespace_only() {
        let result = AgentId::parse("   ");
        assert!(matches!(result, Err(AgentError::EmptyId)));
    }

    #[test]
    fn test_agent_id_parse_reserved_keyword_null() {
        let result = AgentId::parse("null");
        assert!(matches!(result, Err(AgentError::ReservedKeyword(_))));
    }

    #[test]
    fn test_agent_id_parse_reserved_keyword_undefined() {
        let result = AgentId::parse("undefined");
        assert!(matches!(result, Err(AgentError::ReservedKeyword(_))));
    }

    #[test]
    fn test_agent_id_parse_reserved_keyword_true() {
        let result = AgentId::parse("true");
        assert!(matches!(result, Err(AgentError::ReservedKeyword(_))));
    }

    #[test]
    fn test_agent_id_parse_reserved_keyword_false() {
        let result = AgentId::parse("false");
        assert!(matches!(result, Err(AgentError::ReservedKeyword(_))));
    }

    #[test]
    fn test_agent_id_parse_reserved_keyword_case_insensitive() {
        let result = AgentId::parse("NULL");
        assert!(matches!(result, Err(AgentError::ReservedKeyword(_))));
    }

    #[test]
    fn test_agent_id_parse_invalid_characters() {
        let result = AgentId::parse("agent@123");
        assert!(matches!(result, Err(AgentError::InvalidCharacters(_))));
    }

    #[test]
    fn test_agent_id_parse_too_long() {
        let long_id = "a".repeat(MAX_AGENT_ID_LENGTH + 1);
        let result = AgentId::parse(long_id);
        assert!(matches!(result, Err(AgentError::TooLong(_))));
    }

    #[test]
    fn test_agent_id_parse_max_length_ok() {
        let max_id = "a".repeat(MAX_AGENT_ID_LENGTH);
        let id = AgentId::parse(max_id).expect("Should parse max length ID");
        assert_eq!(id.as_str().len(), MAX_AGENT_ID_LENGTH);
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::parse("test-agent").expect("Should parse");
        assert_eq!(format!("{id}"), "test-agent");
    }

    // ==========================================================================
    // Registration Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_register_creates_agent() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.register("agent-1").await?;
        assert!(result.is_new);
        assert_eq!(result.agent_id, "agent-1");

        let active = registry.get_active().await?;
        assert_eq!(active.len(), 1);
        assert_eq!(active.first().map(|a| a.agent_id.as_str()), Some("agent-1"));
        Ok(())
    }

    #[tokio::test]
    async fn test_register_idempotent() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result1 = registry.register("agent-1").await?;
        assert!(result1.is_new);

        let result2 = registry.register("agent-1").await?;
        assert!(!result2.is_new);

        let active = registry.get_active().await?;
        assert_eq!(active.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_register_invalid_id_fails() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.register("").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 1); // Validation error
        Ok(())
    }

    #[tokio::test]
    async fn test_register_reserved_keyword_fails() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.register("null").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 1); // Validation error
        Ok(())
    }

    #[tokio::test]
    async fn test_register_invalid_characters_fails() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.register("agent@123").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 1); // Validation error
        Ok(())
    }

    // ==========================================================================
    // Heartbeat Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_heartbeat_updates_last_seen() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        registry.register("agent-1").await?;

        let before = registry.get_active().await?;
        let before_ts = before
            .first()
            .ok_or_else(|| Error::NotFound("No agents found".to_string()))?
            .last_seen;

        // Small delay to ensure timestamp differs
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        registry.heartbeat("agent-1").await?;

        let after = registry.get_active().await?;
        let after_ts = after
            .first()
            .ok_or_else(|| Error::NotFound("No agents found".to_string()))?
            .last_seen;
        assert!(after_ts >= before_ts);
        Ok(())
    }

    #[tokio::test]
    async fn test_heartbeat_unknown_agent_returns_error() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.heartbeat("nonexistent").await;
        assert!(result.is_err());
        Ok(())
    }

    // ==========================================================================
    // Active Agents Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_stale_agent_not_in_active_list() -> Result<(), Error> {
        let pool = test_pool().await?;
        // Use 0 second timeout so everything is immediately stale
        let registry = AgentRegistry::new(pool, 0).await?;

        registry.register("agent-1").await?;

        // Small delay to ensure agent becomes stale
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let active = registry.get_active().await?;
        assert!(active.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_stale_agent_restored_by_heartbeat() -> Result<(), Error> {
        let pool = test_pool().await?;
        // Short timeout
        let registry = AgentRegistry::new(pool, 1).await?;

        registry.register("agent-1").await?;

        // Wait for staleness
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let stale = registry.get_active().await?;
        assert!(stale.is_empty());

        // Re-heartbeat restores
        registry.heartbeat("agent-1").await?;

        let active = registry.get_active().await?;
        assert_eq!(active.len(), 1);
        Ok(())
    }

    // ==========================================================================
    // Unregister Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_unregister_removes_agent() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        registry.register("agent-1").await?;
        registry.unregister("agent-1").await?;

        let active = registry.get_active().await?;
        assert!(active.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_unregister_unknown_agent_returns_error() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.unregister("nonexistent").await;
        assert!(result.is_err());
        Ok(())
    }

    // ==========================================================================
    // Get Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_get_existing_agent() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        registry.register("agent-1").await?;

        let agent = registry.get("agent-1").await?;
        assert_eq!(agent.agent_id, "agent-1");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent_agent() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let result = registry.get("nonexistent").await;
        assert!(result.is_err());
        Ok(())
    }

    // ==========================================================================
    // Exists Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_exists_returns_true_for_existing() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        registry.register("agent-1").await?;

        let exists = registry.exists("agent-1").await?;
        assert!(exists);
        Ok(())
    }

    #[tokio::test]
    async fn test_exists_returns_false_for_nonexistent() -> Result<(), Error> {
        let pool = test_pool().await?;
        let registry = AgentRegistry::new(pool, 60).await?;

        let exists = registry.exists("nonexistent").await?;
        assert!(!exists);
        Ok(())
    }
}
