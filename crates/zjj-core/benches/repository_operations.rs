#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Benchmark repository operations.
//!
//! This benchmark compares mock vs real repository implementations.
//! It measures the overhead of the repository abstraction and helps
//! identify bottlenecks in persistence operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use zjj_core::domain::identifiers::{SessionId, SessionName, WorkspaceName};
use zjj_core::domain::repository::{SessionRepository, WorkspaceRepository};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ============================================================================
// MOCK REPOSITORY IMPLEMENTATION
// ============================================================================

/// In-memory session repository (mock for testing).
struct MockSessionRepo {
    sessions: Arc<Mutex<Vec<zjj_core::domain::repository::Session>>>,
}

impl MockSessionRepo {
    fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_sessions(sessions: Vec<zjj_core::domain::repository::Session>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(sessions)),
        }
    }
}

impl SessionRepository for MockSessionRepo {
    fn load(&self, id: &SessionId) -> zjj_core::domain::repository::RepositoryResult<zjj_core::domain::repository::Session> {
        self.sessions
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?
            .iter()
            .find(|s| &s.id == id)
            .cloned()
            .ok_or_else(|| zjj_core::domain::repository::RepositoryError::not_found("session", id))
    }

    fn load_by_name(&self, name: &SessionName) -> zjj_core::domain::repository::RepositoryResult<zjj_core::domain::repository::Session> {
        self.sessions
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?
            .iter()
            .find(|s| &s.name == name)
            .cloned()
            .ok_or_else(|| zjj_core::domain::repository::RepositoryError::not_found("session", name))
    }

    fn save(&self, session: &zjj_core::domain::repository::Session) -> zjj_core::domain::repository::RepositoryResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?;

        if let Some(pos) = sessions.iter().position(|s| s.id == session.id) {
            sessions[pos] = session.clone();
        } else {
            sessions.push(session.clone());
        }
        Ok(())
    }

    fn delete(&self, id: &SessionId) -> zjj_core::domain::repository::RepositoryResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?;

        let pos = sessions
            .iter()
            .position(|s| &s.id == id)
            .ok_or_else(|| zjj_core::domain::repository::RepositoryError::not_found("session", id))?;

        sessions.remove(pos);
        Ok(())
    }

    fn list_all(&self) -> zjj_core::domain::repository::RepositoryResult<Vec<zjj_core::domain::repository::Session>> {
        self.sessions
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))
            .map(|v| v.clone())
    }

    fn get_current(&self) -> zjj_core::domain::repository::RepositoryResult<Option<zjj_core::domain::repository::Session>> {
        self.list_all().map(|sessions| sessions.first().cloned())
    }

    fn set_current(&self, id: &SessionId) -> zjj_core::domain::repository::RepositoryResult<()> {
        // Verify session exists
        self.load(id)?;
        Ok(())
    }

    fn clear_current(&self) -> zjj_core::domain::repository::RepositoryResult<()> {
        Ok(())
    }
}

/// In-memory workspace repository (mock for testing).
struct MockWorkspaceRepo {
    workspaces: Arc<Mutex<Vec<zjj_core::domain::repository::Workspace>>>,
}

impl MockWorkspaceRepo {
    fn new() -> Self {
        Self {
            workspaces: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_workspaces(workspaces: Vec<zjj_core::domain::repository::Workspace>) -> Self {
        Self {
            workspaces: Arc::new(Mutex::new(workspaces)),
        }
    }
}

impl WorkspaceRepository for MockWorkspaceRepo {
    fn load(&self, name: &WorkspaceName) -> zjj_core::domain::repository::RepositoryResult<zjj_core::domain::repository::Workspace> {
        self.workspaces
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?
            .iter()
            .find(|w| &w.name == name)
            .cloned()
            .ok_or_else(|| zjj_core::domain::repository::RepositoryError::not_found("workspace", name))
    }

    fn save(&self, workspace: &zjj_core::domain::repository::Workspace) -> zjj_core::domain::repository::RepositoryResult<()> {
        let mut workspaces = self
            .workspaces
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?;

        if let Some(pos) = workspaces.iter().position(|w| w.name == workspace.name) {
            workspaces[pos] = workspace.clone();
        } else {
            workspaces.push(workspace.clone());
        }
        Ok(())
    }

    fn delete(&self, name: &WorkspaceName) -> zjj_core::domain::repository::RepositoryResult<()> {
        let mut workspaces = self
            .workspaces
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))?;

        let pos = workspaces
            .iter()
            .position(|w| &w.name == name)
            .ok_or_else(|| zjj_core::domain::repository::RepositoryError::not_found("workspace", name))?;

        workspaces.remove(pos);
        Ok(())
    }

    fn list_all(&self) -> zjj_core::domain::repository::RepositoryResult<Vec<zjj_core::domain::repository::Workspace>> {
        self.workspaces
            .lock()
            .map_err(|e| zjj_core::domain::repository::RepositoryError::StorageError(e.to_string()))
            .map(|v| v.clone())
    }
}

// ============================================================================
// FIXTURES
// ============================================================================

/// Create test sessions
fn test_sessions(count: usize) -> Vec<zjj_core::domain::repository::Session> {
    (0..count)
        .map(|i| {
            let id = SessionId::parse(&format!("session-{}", i)).expect("valid id");
            let name = SessionName::parse(&format!("session-{}", i)).expect("valid name");
            zjj_core::domain::repository::Session {
                id,
                name,
                branch: zjj_core::domain::session::BranchState::OnBranch {
                    name: "main".to_string(),
                },
                parent: zjj_core::domain::session::ParentState::Root,
                workspace_path: PathBuf::from(format!("/tmp/workspace{}", i)),
            }
        })
        .collect()
}

/// Create test workspaces
fn test_workspaces(count: usize) -> Vec<zjj_core::domain::repository::Workspace> {
    (0..count)
        .map(|i| {
            let name = WorkspaceName::parse(&format!("workspace-{}", i)).expect("valid name");
            zjj_core::domain::repository::Workspace {
                name: name.clone(),
                path: PathBuf::from(format!("/tmp/workspace{}", i)),
                state: zjj_core::domain::repository::WorkspaceState::Ready,
            }
        })
        .collect()
}

// ============================================================================
// BENCHMARKS: Repository CRUD operations
// ============================================================================

fn bench_session_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_save");

    // Save single session
    group.bench_function("single", |b| {
        let repo = MockSessionRepo::new();
        let session = test_sessions(1).pop().expect("session");

        b.iter(|| {
            black_box(&repo).save(black_box(&session)).ok();
        });
    });

    // Batch save
    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("batch", count), count, |b, count| {
            b.iter(|| {
                let repo = MockSessionRepo::new();
                let sessions = test_sessions(*count);

                for session in &sessions {
                    repo.save(session).ok();
                }
            });
        });
    }

    group.finish();
}

fn bench_session_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_load");

    // Load by ID
    group.bench_function("by_id_single", |b| {
        let sessions = test_sessions(1);
        let id = sessions[0].id.clone();
        let repo = MockSessionRepo::with_sessions(sessions);

        b.iter(|| {
            let _session = black_box(&repo).load(black_box(&id));
        });
    });

    // Load by name
    group.bench_function("by_name_single", |b| {
        let sessions = test_sessions(1);
        let name = sessions[0].name.clone();
        let repo = MockSessionRepo::with_sessions(sessions);

        b.iter(|| {
            let _session = black_box(&repo).load_by_name(black_box(&name));
        });
    });

    // Load from larger collection
    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("from_collection", count), count, |b, count| {
            let sessions = test_sessions(*count);
            let id = sessions[0].id.clone();
            let repo = MockSessionRepo::with_sessions(sessions);

            b.iter(|| {
                let _session = black_box(&repo).load(black_box(&id));
            });
        });
    }

    group.finish();
}

fn bench_session_list_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_list_all");

    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, count| {
            let repo = MockSessionRepo::with_sessions(test_sessions(*count));

            b.iter(|| {
                let _sessions = black_box(&repo).list_all();
            });
        });
    }

    group.finish();
}

fn bench_session_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_delete");

    group.bench_function("single", |b| {
        let sessions = test_sessions(1);
        let id = sessions[0].id.clone();

        b.iter(|| {
            let repo = MockSessionRepo::with_sessions(test_sessions(1));
            let _ = repo.delete(&id);
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Repository query methods
// ============================================================================

fn bench_session_exists(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_exists");

    // Exists (true)
    group.bench_function("found", |b| {
        let sessions = test_sessions(1);
        let id = sessions[0].id.clone();
        let repo = MockSessionRepo::with_sessions(sessions);

        b.iter(|| {
            let _exists = black_box(&repo).exists(black_box(&id));
        });
    });

    // Exists (false)
    group.bench_function("not_found", |b| {
        let repo = MockSessionRepo::new();
        let id = SessionId::parse("nonexistent").expect("valid id");

        b.iter(|| {
            let _exists = black_box(&repo).exists(black_box(&id));
        });
    });

    group.finish();
}

fn bench_session_list_sorted(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_list_sorted");

    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, count| {
            let repo = MockSessionRepo::with_sessions(test_sessions(*count));

            b.iter(|| {
                let _sessions = black_box(&repo).list_sorted_by_name();
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Workspace operations
// ============================================================================

fn bench_workspace_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_save");

    group.bench_function("single", |b| {
        let repo = MockWorkspaceRepo::new();
        let workspace = test_workspaces(1).pop().expect("workspace");

        b.iter(|| {
            black_box(&repo).save(black_box(&workspace)).ok();
        });
    });

    group.finish();
}

fn bench_workspace_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_load");

    group.bench_function("by_name", |b| {
        let workspaces = test_workspaces(1);
        let name = workspaces[0].name.clone();
        let repo = MockWorkspaceRepo::with_workspaces(workspaces);

        b.iter(|| {
            let _workspace = black_box(&repo).load(black_box(&name));
        });
    });

    group.finish();
}

fn bench_workspace_list_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_list_all");

    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, count| {
            let repo = MockWorkspaceRepo::new();

            // Pre-populate
            for workspace in test_workspaces(*count) {
                repo.save(&workspace).ok();
            }

            b.iter(|| {
                let _workspaces = black_box(&repo).list_all();
            });
        });
    }

    group.finish();
}

// ============================================================================
// BENCHMARKS: Concurrency overhead
// ============================================================================

fn bench_repository_lock_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("repository_lock_contention");

    // Single-threaded baseline
    group.bench_function("single_threaded_save", |b| {
        let repo = MockSessionRepo::new();
        let sessions = test_sessions(100);

        b.iter(|| {
            for session in &sessions {
                repo.save(session).ok();
            }
        });
    });

    // Note: True parallel benchmarks would require rayon or similar
    // This measures the lock overhead in sequential access

    group.finish();
}

// ============================================================================
// BENCHMARKS: Memory allocation patterns
// ============================================================================

fn bench_repository_clone_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("repository_clone_overhead");

    group.bench_function("clone_session", |b| {
        let session = test_sessions(1).pop().expect("session");

        b.iter(|| {
            let _cloned = black_box(&session).clone();
        });
    });

    group.bench_function("clone_workspace", |b| {
        let workspace = test_workspaces(1).pop().expect("workspace");

        b.iter(|| {
            let _cloned = black_box(&workspace).clone();
        });
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: Repository trait overhead
// ============================================================================

fn bench_repository_trait_object(c: &mut Criterion) {
    let mut group = c.benchmark_group("repository_trait_object");

    // Direct call (static dispatch)
    group.bench_function("static_dispatch", |b| {
        let repo = MockSessionRepo::new();
        let session = test_sessions(1).pop().expect("session");

        b.iter(|| {
            repo.save(&session).ok();
        });
    });

    // Trait object call (dynamic dispatch)
    group.bench_function("dynamic_dispatch", |b| {
        let repo: Box<dyn SessionRepository> = Box::new(MockSessionRepo::new());
        let session = test_sessions(1).pop().expect("session");

        b.iter(|| {
            repo.save(&session).ok();
        });
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group!(
    benches,
    bench_session_save,
    bench_session_load,
    bench_session_list_all,
    bench_session_delete,
    bench_session_exists,
    bench_session_list_sorted,
    bench_workspace_save,
    bench_workspace_load,
    bench_workspace_list_all,
    bench_repository_lock_contention,
    bench_repository_clone_overhead,
    bench_repository_trait_object
);

criterion_main!(benches);
