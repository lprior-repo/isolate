# External Integrations

**Analysis Date:** 2026-01-16

## APIs & External Services

**CLI Tools (System Commands):**
- JJ (Jujutsu) - Version control system
  - SDK/Client: Direct process execution via `std::process::Command`
  - Detection: `which::which("jj")` in `crates/zjj/src/cli.rs:120`
  - Commands: `jj root`, `jj workspace add`, `jj rebase`, `jj git init`, `jj commit`
  - Usage: Core dependency for workspace management

- Zellij - Terminal multiplexer
  - SDK/Client: Direct process execution via `std::process::Command`
  - Detection: `which::which("zellij")` in `crates/zjj/src/cli.rs:177`
  - Commands: `zellij action go-to-tab-name`, `zellij run`
  - Usage: Session management and tab navigation

- Beads (bd) - Issue tracking tool
  - SDK/Client: SQLite database direct access at `.beads/beads.db`
  - Config: `.beads/config.yaml`
  - Integration: `crates/zjj-core/src/beads.rs` provides query layer
  - Usage: Issue tracking integration for sessions

## Data Storage

**Databases:**
- SQLite (embedded)
  - Connection: Local filesystem paths (user data dir and `.beads/beads.db`)
  - Client: SQLx 0.8 with async connection pooling
  - Usage:
    - Session persistence in `crates/zjj/src/db.rs`
    - Beads issue database at `.beads/beads.db`
  - Schema: Embedded SQL in `crates/zjj/src/db.rs:17` (no migration files)
  - Features: Triggers for automatic timestamp updates, indexed queries

**File Storage:**
- Local filesystem only
  - Session database: XDG data directory (via `directories` crate)
  - Beads database: `.beads/beads.db` in repository root
  - Configuration: TOML files in `.beads/config.yaml`
  - Logs: Stderr with `tracing-subscriber`

**Caching:**
- None - Direct SQLite queries only

## Authentication & Identity

**Auth Provider:**
- None - Local-only tool
  - No authentication required
  - No network communication
  - All operations are local filesystem and process spawning

## Monitoring & Observability

**Error Tracking:**
- None - Errors propagated via `Result<T, Error>` types

**Logs:**
- Structured logging via `tracing` 0.1
- Output: Stderr with ANSI colors
- Configuration: Dynamic log levels (requires Rust nightly)
- Format: Human-readable with `tracing-subscriber`
- Implementation: `crates/zjj/src/main.rs` and `crates/zjj-core/src` modules

## CI/CD & Deployment

**Hosting:**
- GitHub Releases - Binary distribution
  - Workflow: `.github/workflows/release.yml`
  - Artifacts: Compiled binaries for Linux and macOS

**CI Pipeline:**
- GitHub Actions
  - Workflow: `.github/workflows/ci.yml`
  - Jobs: format, clippy, test, security (cargo-audit), build, coverage (tarpaulin), docs
  - Matrix: Ubuntu and macOS runners with stable Rust
  - Security: cargo-audit for dependency vulnerabilities
  - Coverage: cargo-tarpaulin with Codecov upload

**Benchmarks:**
- GitHub Actions
  - Workflow: `.github/workflows/benchmarks.yml`
  - Tool: Criterion with HTML reports
  - Benchmarks: Config operations, validation functions

## Environment Configuration

**Required env vars:**
- None - All configuration via files or command-line flags

**Optional env vars:**
- `BEADS_DB` - Override beads database path
- `BD_ACTOR` - Default actor for audit trails
- `BEADS_AUTO_START_DAEMON` - Auto-start beads daemon
- `BEADS_FLUSH_DEBOUNCE` - Debounce interval for auto-flush
- `BEADS_SYNC_BRANCH` - Git branch for beads sync
- `RUST_BACKTRACE` - Rust backtrace level (CI only)
- `CARGO_TERM_COLOR` - Cargo output coloring (CI only)
- `CODECOV_TOKEN` - Codecov upload token (CI secret)

**Secrets location:**
- No secrets required (local-only tool)
- GitHub Actions secrets: `CODECOV_TOKEN` for coverage uploads

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## File System Watching

**File Watcher:**
- Implementation: `crates/zjj-core/src/watcher.rs`
- Library: `notify` 6.x with `notify-debouncer-mini` 0.4
- Usage: Watch for configuration and database changes
- Debouncing: Built-in via `notify-debouncer-mini`

## External Tool Requirements

**Runtime Dependencies (must be in PATH):**
1. `jj` (Jujutsu) - Version control operations
2. `zellij` - Terminal session management
3. `bd` (Beads) - Optional, for issue tracking integration

**Detection Strategy:**
- `which` crate (7.0) used to locate executables
- Graceful degradation: CLI checks availability before operations
- Error messages guide users to install missing tools

---

*Integration audit: 2026-01-16*
