# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-02-07

### Major Release - Production Ready

This release marks zjj as production-ready with comprehensive functional Rust refactoring, async architecture, and enhanced safety guarantees.

### Breaking Changes

#### Pervasive Functional Transformation
- **Zero unwrap law enforced**: All `unwrap()`, `expect()`, `panic!()` eliminated
- **Railway-Oriented Programming**: Error handling via `Result<T, E>` with proper propagation
- **Pure functional core**: Core logic is sync, deterministic, and side-effect-free
- **Persistent data structures**: `rpds` for structural sharing and immutability

#### Type-State Pattern for Safety
- **RepairExecutor**: Backup safety enforced via compile-time type-state
- **File locking**: Battle-tested `fs2` library replaces custom implementations
- **Extension locks**: Now extend from current expiration, not from `now`

#### Async Architecture Complete
- **Async shell, sync core**: I/O operations use `tokio`, core remains pure
- **Connection pooling**: `sqlx` with `SqlitePool` for efficient database access
- **Stream processing**: `futures-util` for async combinators

### Added
- Comprehensive test suite (4x faster with optimization rounds)
- SCCache compiler cache integration
- Workspace integrity verification
- Production readiness fixes (14 tasks completed)

### Performance
- **98.5% faster builds**: Moon + bazel-remote caching
- **4x test speedup**: Three optimization rounds on slow tests
- **50% avg batch speedup**: Round 1 and 2 test optimizations

### Fixed
- 100+ clippy and linting errors resolved
- All `items-after-statements` warnings eliminated
- Template storage thread safety with fs2 file locking
- Lock expiration bug in `extend_lock`

### Quality
- Zero unsafe code (forbidden by lint policy)
- Zero panics/unwraps in production code
- Exhaustive pattern matching enforced
- Thiserror for domain errors, anyhow for boundary errors

## [0.3.0] - 2026-01-27

### Breaking Changes

#### Database Layer Migration (rusqlite → sqlx)
- **Migrated from synchronous rusqlite to async sqlx** with connection pooling
- **All database operations are now async** using `async fn` and `.await`
- **Connection pooling** via `SqlitePool` replaces direct `Connection` management
- **Tokio runtime** is now required for all command execution

**Migration Notes for Contributors:**
- Command functions that access database must be `async fn`
- Database operations now use `sqlx::query()` instead of `rusqlite` methods
- Connection setup uses `SqlitePool::connect()` instead of `Connection::open()`
- All database calls require `.await` or `.spawn()` in async contexts

**Files Affected:**
- `crates/zjj-core/src/beads/` - Full async migration
- `crates/zjj/src/commands/` - All command handlers now async
- `crates/zjj/src/main.rs` - Entry point uses `#[tokio::main]`

### Added
- `zjj spawn` - One-command parallel isolation for AI agents
- Enhanced TUI dashboard with beads integration
- WebSocket-based real-time progress updates (experimental)

### Fixed
- Improved error handling with semantic exit codes
- Enhanced JSON output across all commands
- Reduced clone usage through structural sharing

### Performance
- 98.5% faster builds with Moon + bazel-remote integration
- Connection pooling reduces database overhead
- Parallel task execution across all crates

## [0.2.x] - Previous Releases

See [GitHub Releases](https://github.com/lprior-repo/zjj/releases) for version history prior to 0.3.0.
