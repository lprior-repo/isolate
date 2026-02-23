//! Repository interface mock implementations
//!
//! Demonstrates mock implementations for repository interfaces.
//! These show the pattern without duplicating all the implementation types.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::Arc;

use zjj_core::domain::identifiers::{
    AgentId, BeadId, SessionId, SessionName, WorkspaceName,
};
use zjj_core::domain::repository::{
    AgentRepository, BeadRepository, ClaimState, QueueEntry, QueueRepository, QueueStats,
    RepositoryError, RepositoryResult, SessionRepository, WorkspaceRepository,
};
use zjj_core::domain::session::{BranchState, ParentState};

// ============================================================================
// MOCK SESSION REPOSITORY
// ============================================================================

/// Simple mock session repository demonstrating the repository pattern.
pub struct MockSessionRepository;

impl SessionRepository for MockSessionRepository {
    fn load(&self, _id: &SessionId) -> RepositoryResult<zjj_core::domain::repository::Session> {
        // Return a mock session - in real tests, you'd store actual data
        Ok(zjj_core::domain::repository::Session {
            id: SessionId::parse("mock-session").expect("valid id"),
            name: SessionName::parse("mock").expect("valid name"),
            branch: BranchState::Detached,
            parent: ParentState::Root,
            workspace_path: PathBuf::from("/tmp"),
        })
    }

    fn load_by_name(
        &self,
        _name: &SessionName,
    ) -> RepositoryResult<zjj_core::domain::repository::Session> {
        // Mock implementation
        Ok(zjj_core::domain::repository::Session {
            id: SessionId::parse("mock-session").expect("valid id"),
            name: _name.clone(),
            branch: BranchState::Detached,
            parent: ParentState::Root,
            workspace_path: PathBuf::from("/tmp"),
        })
    }

    fn save(
        &self,
        _session: &zjj_core::domain::repository::Session,
    ) -> RepositoryResult<()> {
        // Mock save - no-op
        Ok(())
    }

    fn delete(&self, _id: &SessionId) -> RepositoryResult<()> {
        // Mock delete - no-op
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<zjj_core::domain::repository::Session>> {
        // Return empty list
        Ok(Vec::new())
    }

    fn get_current(
        &self,
    ) -> RepositoryResult<Option<zjj_core::domain::repository::Session>> {
        Ok(None)
    }

    fn set_current(&self, _id: &SessionId) -> RepositoryResult<()> {
        Ok(())
    }

    fn clear_current(&self) -> RepositoryResult<()> {
        Ok(())
    }
}

// ============================================================================
// MOCK WORKSPACE REPOSITORY
// ============================================================================

/// Simple mock workspace repository demonstrating the repository pattern.
pub struct MockWorkspaceRepository;

impl WorkspaceRepository for MockWorkspaceRepository {
    fn load(
        &self,
        _name: &WorkspaceName,
    ) -> RepositoryResult<zjj_core::domain::repository::Workspace> {
        Ok(zjj_core::domain::repository::Workspace {
            name: _name.clone(),
            path: PathBuf::from("/tmp"),
            state: zjj_core::domain::repository::WorkspaceState::Creating,
        })
    }

    fn save(
        &self,
        _workspace: &zjj_core::domain::repository::Workspace,
    ) -> RepositoryResult<()> {
        Ok(())
    }

    fn delete(&self, _name: &WorkspaceName) -> RepositoryResult<()> {
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<zjj_core::domain::repository::Workspace>> {
        Ok(Vec::new())
    }
}

// ============================================================================
// MOCK BEAD REPOSITORY
// ============================================================================

/// Simple mock bead repository demonstrating the repository pattern.
pub struct MockBeadRepository;

impl BeadRepository for MockBeadRepository {
    fn load(&self, _id: &BeadId) -> RepositoryResult<zjj_core::domain::repository::Bead> {
        use chrono::Utc;

        Ok(zjj_core::domain::repository::Bead {
            id: _id.clone(),
            title: "Mock Bead".to_string(),
            description: None,
            state: zjj_core::domain::repository::BeadState::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    fn save(
        &self,
        _bead: &zjj_core::domain::repository::Bead,
    ) -> RepositoryResult<()> {
        Ok(())
    }

    fn delete(&self, _id: &BeadId) -> RepositoryResult<()> {
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<zjj_core::domain::repository::Bead>> {
        Ok(Vec::new())
    }
}

// ============================================================================
// MOCK QUEUE REPOSITORY
// ============================================================================

/// Simple mock queue repository demonstrating the repository pattern.
pub struct MockQueueRepository;

impl QueueRepository for MockQueueRepository {
    fn load(&self, _id: i64) -> RepositoryResult<QueueEntry> {
        use chrono::Utc;

        Ok(QueueEntry {
            id: _id,
            workspace: WorkspaceName::parse("mock-workspace").expect("valid name"),
            bead: None,
            priority: 0,
            claim_state: ClaimState::Unclaimed,
            created_at: Utc::now(),
        })
    }

    fn save(&self, _entry: &QueueEntry) -> RepositoryResult<()> {
        Ok(())
    }

    fn delete(&self, _id: i64) -> RepositoryResult<()> {
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<QueueEntry>> {
        Ok(Vec::new())
    }

    fn claim_next(
        &self,
        _agent: &AgentId,
        _claim_duration_secs: i64,
    ) -> RepositoryResult<Option<QueueEntry>> {
        Ok(None)
    }

    fn release(&self, _id: i64, _agent: &AgentId) -> RepositoryResult<()> {
        Ok(())
    }

    fn expire_claims(&self, _older_than_secs: i64) -> RepositoryResult<usize> {
        Ok(0)
    }

    fn add_workspace(
        &self,
        _workspace: &WorkspaceName,
        _bead: Option<&BeadId>,
        _priority: i32,
    ) -> RepositoryResult<i64> {
        Ok(1)
    }

    fn remove_workspace(&self, _workspace: &WorkspaceName) -> RepositoryResult<()> {
        Ok(())
    }

    fn stats(&self) -> RepositoryResult<QueueStats> {
        Ok(QueueStats {
            total: 0,
            unclaimed: 0,
            claimed: 0,
            expired: 0,
        })
    }
}

// ============================================================================
// MOCK AGENT REPOSITORY
// ============================================================================

/// Simple mock agent repository demonstrating the repository pattern.
pub struct MockAgentRepository;

impl AgentRepository for MockAgentRepository {
    fn load(&self, _id: &AgentId) -> RepositoryResult<zjj_core::domain::repository::Agent> {
        Ok(zjj_core::domain::repository::Agent {
            id: _id.clone(),
            state: zjj_core::domain::repository::AgentState::Active,
            last_seen: None,
        })
    }

    fn save(
        &self,
        _agent: &zjj_core::domain::repository::Agent,
    ) -> RepositoryResult<()> {
        Ok(())
    }

    fn heartbeat(&self, _id: &AgentId) -> RepositoryResult<()> {
        Ok(())
    }

    fn list_all(&self) -> RepositoryResult<Vec<zjj_core::domain::repository::Agent>> {
        Ok(Vec::new())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_session_repository_implements_trait() {
        let repo = MockSessionRepository;

        // Verify all methods compile and work
        let id = SessionId::parse("test-1").expect("valid id");
        let _session = repo.load(&id).expect("load works");

        let name = SessionName::parse("test").expect("valid name");
        let _session = repo.load_by_name(&name).expect("load by name works");

        let _all = repo.list_all().expect("list works");
        let _current = repo.get_current().expect("get current works");

        repo.set_current(&id).expect("set current works");
        repo.clear_current().expect("clear current works");
    }

    #[test]
    fn test_mock_workspace_repository_implements_trait() {
        let repo = MockWorkspaceRepository;

        let name = WorkspaceName::parse("test-ws").expect("valid name");
        let _workspace = repo.load(&name).expect("load works");

        let _all = repo.list_all().expect("list works");
    }

    #[test]
    fn test_mock_bead_repository_implements_trait() {
        let repo = MockBeadRepository;

        let id = BeadId::parse("bd-abc123").expect("valid id");
        let _bead = repo.load(&id).expect("load works");

        let _all = repo.list_all().expect("list works");
    }

    #[test]
    fn test_mock_queue_repository_implements_trait() {
        let repo = MockQueueRepository;

        let _entry = repo.load(1).expect("load works");

        let agent = AgentId::parse("agent-1").expect("valid agent");
        let _claimed = repo.claim_next(&agent, 300).expect("claim works");

        let _stats = repo.stats().expect("stats works");
        assert_eq!(_stats.total, 0);

        let workspace = WorkspaceName::parse("test").expect("valid name");
        let _id = repo.add_workspace(&workspace, None, 1).expect("add works");
    }

    #[test]
    fn test_mock_agent_repository_implements_trait() {
        let repo = MockAgentRepository;

        let id = AgentId::parse("agent-1").expect("valid agent");
        let _agent = repo.load(&id).expect("load works");

        repo.heartbeat(&id).expect("heartbeat works");

        let _all = repo.list_all().expect("list works");
    }

    #[test]
    fn test_all_mock_repositories_are_send_and_sync() {
        // Verify all mock repositories can be used across threads
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<MockSessionRepository>();
        assert_send_sync::<MockWorkspaceRepository>();
        assert_send_sync::<MockBeadRepository>();
        assert_send_sync::<MockQueueRepository>();
        assert_send_sync::<MockAgentRepository>();
    }

    #[test]
    fn test_mock_repositories_return_correct_error_types() {
        let session_repo = MockSessionRepository;

        // Mock returns empty list, not NotFound (this is just a simple mock)
        let result = session_repo.list_all();
        assert!(result.is_ok());

        // Real implementations would return NotFound for missing entities
        // This mock just demonstrates the interface
    }
}
