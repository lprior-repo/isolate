# Architecture

**Analysis Date:** 2026-01-16

## Pattern Overview

**Overall:** Functional Core, Imperative Shell with Railway-Oriented Programming

**Key Characteristics:**
- Pure functional core with zero unwraps/panics (enforced by clippy)
- Async-first with SQLite connection pooling (no Arc<Mutex<>>)
- Railway-Oriented Programming: all fallible operations return `Result<T, Error>`
- Strict separation: pure functions (business logic) vs imperative shell (I/O)
- Type-driven error handling with custom Error enum

## Layers

**zjj-core (Core Library):**
- Purpose: Pure functional utilities and external integrations (JJ, Zellij, Beads)
- Location: `crates/zjj-core/src/`
- Contains: Error types, result extensions, JJ workspace ops, Zellij control, Beads DB queries
- Depends on: anyhow, thiserror, sqlx, tokio, serde
- Used by: zjj CLI binary

**zjj (CLI Binary):**
- Purpose: User-facing command interface and session orchestration
- Location: `crates/zjj/src/`
- Contains: CLI parsing, command handlers, session DB, JSON output formatting
- Depends on: zjj-core, clap, ratatui, crossterm
- Used by: End users via `zjj` binary

**Commands Layer:**
- Purpose: Individual command implementations (add, remove, focus, sync, etc.)
- Location: `crates/zjj/src/commands/`
- Contains: One file per command, each with options struct and run function
- Depends on: Session DB, zjj-core integrations
- Used by: Main CLI dispatcher

**Persistence Layer:**
- Purpose: SQLite-based session state management
- Location: `crates/zjj/src/db.rs`
- Contains: SessionDb with connection pooling, schema embedded as SQL string
- Depends on: sqlx async runtime
- Used by: All commands that read/write session state

## Data Flow

**Session Creation (add command):**

1. User runs `zjj add <name>` → main.rs parses CLI args via clap
2. Dispatcher routes to `commands::add::run_with_options()`
3. Validation: name checked via `validate_session_name()` (pure function)
4. DB transaction: `SessionDb::create()` inserts record with status=Creating
5. Side effects: `jj::workspace_create()` creates JJ workspace
6. Side effects: Zellij tab created via shell command
7. Status update: DB status set to Active via `SessionDb::update()`
8. Hooks: Optional post-create hooks executed if not disabled
9. Response: JSON or human-readable output via json_output.rs

**State Management:**
- All session state persists in SQLite (`~/.zjj/sessions.db`)
- No in-memory cache (database is source of truth)
- Connection pooling prevents lock contention (min 1, max 5 connections)
- Timestamps auto-updated via SQLite trigger

## Key Abstractions

**Result<T> Type Alias:**
- Purpose: Unified error handling across entire codebase
- Examples: `crates/zjj-core/src/result.rs`, used everywhere
- Pattern: `type Result<T> = std::result::Result<T, Error>`

**Session:**
- Purpose: Represents JJ workspace + Zellij tab pair
- Examples: `crates/zjj/src/session.rs`
- Pattern: Data struct with validation functions (validate_session_name, validate_status_transition)

**SessionDb:**
- Purpose: Async database facade with connection pooling
- Examples: `crates/zjj/src/db.rs`
- Pattern: Clone-able handle wrapping SqlitePool, all methods return Result<T>

**Error Enum:**
- Purpose: Type-safe error variants with context
- Examples: `crates/zjj-core/src/error.rs`
- Pattern: Custom enum with Display impl, From conversions for external errors

**Functional Core Functions:**
- Purpose: Pure logic with no side effects (testable, composable)
- Examples: `validate_database_path()`, `build_session()`, `serialize_sessions()` in db.rs
- Pattern: Takes immutable inputs, returns Result, never touches I/O

## Entry Points

**Main Binary:**
- Location: `crates/zjj/src/main.rs`
- Triggers: User runs `zjj <command>`
- Responsibilities: CLI parsing, error formatting, tokio runtime setup, command dispatch

**CLI Builder:**
- Location: `build_cli()` function in main.rs
- Triggers: Called by main() before argument parsing
- Responsibilities: Define all subcommands with clap, set help text, configure flags

**Async Runtime:**
- Location: `tokio::runtime::Runtime::new()` in main()
- Triggers: On binary startup
- Responsibilities: Execute async command handlers, manage SQLite connection pool

## Error Handling

**Strategy:** Railway-Oriented Programming with typed errors

**Patterns:**
- All fallible operations return `Result<T, Error>` (never panic/unwrap)
- `?` operator propagates errors up the call stack
- Match expressions or combinators (map, and_then) handle success/failure paths
- Top-level main() catches errors and formats for user output (JSON or human-readable)
- Database errors wrapped in Error::DatabaseError with context strings
- Hook failures include stdout/stderr/exit code in Error::HookFailed

## Cross-Cutting Concerns

**Logging:** tracing crate with env filter (level=INFO default, configurable via RUST_LOG)

**Validation:**
- Session names validated early via `validate_session_name()` (pure function)
- ASCII-only, must start with letter, max 64 chars, alphanumeric + dash/underscore
- Database schema uses CHECK constraints for status enum
- No SQL injection risk (parameterized queries only)

**Authentication:** Not applicable (local CLI tool, no remote auth)

---

*Architecture analysis: 2026-01-16*
