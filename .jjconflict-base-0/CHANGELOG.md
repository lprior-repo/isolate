# Changelog

All notable changes to this project will be documented in this file.

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
