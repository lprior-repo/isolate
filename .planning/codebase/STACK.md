# Technology Stack

**Analysis Date:** 2026-01-16

## Languages

**Primary:**
- Rust 2021 Edition - All codebase (binaries and libraries)
- Rust nightly-2025-12-15 - Required for dynamic log levels in tracing macros

**Secondary:**
- Shell (Bash) - Install scripts and CI helpers

## Runtime

**Environment:**
- Rust nightly-2025-12-15 (pinned version, managed via `rust-toolchain.toml`)
- Target: x86_64-unknown-linux-gnu (primary), supports macOS

**Package Manager:**
- Cargo (via rustup)
- Lockfile: Present (`Cargo.lock`)

## Frameworks

**Core:**
- Tokio 1.x - Async runtime with multi-threaded executor
- Clap 4.5 - CLI argument parsing and command structure
- SQLx 0.8 - Async database operations with compile-time query validation

**Testing:**
- Rust built-in test framework - Unit and integration tests
- Criterion 0.5 - Benchmarking framework with HTML reports
- Tokio-test 0.4 - Async test utilities
- Proptest 1.0 - Property-based testing
- Assert_cmd 2.x - CLI testing with predicates
- Serial_test 3.2 - Test serialization for shared resources
- Tempfile 3.x - Temporary directories for tests

**Build/Dev:**
- Moon >=1.20.0 - Task runner and monorepo orchestration (wraps cargo commands)
- Cargo - Native Rust build tool (invoked via Moon tasks)
- Rustfmt - Code formatting (configured via `rustfmt.toml`)
- Clippy - Linting with strict rules (configured via `.clippy.toml`)

## Key Dependencies

**Critical:**
- `sqlx` 0.8 - SQLite connection pooling and migrations (runtime-tokio, tls-rustls features)
- `tokio` 1.x - Async runtime for all I/O operations
- `clap` 4.5 - CLI interface with completion generation
- `zjj-core` (internal) - Core library with error handling and functional utilities

**Infrastructure:**
- `anyhow` 1.0 - Error handling and context propagation
- `thiserror` 2.0.17 - Custom error type derivation
- `serde` 1.0 / `serde_json` 1.0 - Serialization for JSON output and config
- `toml` 0.8 / `toml_edit` 0.22 - Configuration file parsing
- `directories` 5.0/6.0 - XDG Base Directory specification
- `tracing` 0.1 / `tracing-subscriber` 0.3 - Structured logging with dynamic levels
- `which` 7.0 - Find executables in PATH (for jj, zellij, bd)

**Functional Programming:**
- `im` 15.1 - Immutable persistent data structures (HashMap, Vector)
- `tap` 1.0 - Pipe operator for functional composition
- `either` 1.13 - Either/Result helpers
- `itertools` 0.13 - Iterator extensions

**UI/TUI:**
- `ratatui` 0.30 - Terminal UI framework
- `crossterm` 0.27 - Cross-platform terminal manipulation

**Utilities:**
- `chrono` 0.4 - Date/time handling (std, clock, serde features)
- `regex` 1.11 - Regular expression support
- `fs2` 0.4 - File locking for concurrent access
- `notify` 6.x / `notify-debouncer-mini` 0.4 - File system watching
- `strum` 0.26 - Enum utilities (derive Display, EnumString)
- `futures` 0.3 - Future combinators

## Configuration

**Environment:**
- No `.env` files present
- Configuration via TOML files and SQLite database
- XDG Base Directory compliant (uses `directories` crate)
- Beads configuration in `.beads/config.yaml`

**Build:**
- `Cargo.toml` - Workspace root with shared lint configuration
- `rust-toolchain.toml` - Pins to nightly-2025-12-15
- `rustfmt.toml` - Formatting rules (max_width=100, Unix newlines)
- `.clippy.toml` - Clippy configuration (zero-unwrap enforcement)
- `.moon/workspace.yml` - Moon workspace configuration
- `.moon/toolchain.yml` - Moon toolchain settings (uses system Rust)

**Lint Configuration (Workspace-level):**
- `unsafe_code = "forbid"` - No unsafe code allowed
- `unwrap_used = "forbid"` - Zero unwrap law
- `expect_used = "forbid"` - Zero expect law
- `panic = "forbid"` - No panics allowed
- `todo = "forbid"` - No TODOs in production code
- `unimplemented = "forbid"` - No unimplemented macros
- `arithmetic_side_effects = "deny"` - Checked arithmetic only
- Clippy levels: all/pedantic/nursery = warn, correctness/suspicious = deny

## Platform Requirements

**Development:**
- Rust nightly-2025-12-15 (via rustup)
- Moon >=1.20.0 (task runner)
- JJ (Jujutsu) - Version control (external dependency)
- Zellij - Terminal multiplexer (external dependency)
- Beads (bd) - Issue tracking tool (external dependency)

**Production:**
- Single binary deployment (`zjj`)
- No runtime dependencies (static linking via rustls)
- Target platforms: Linux (primary), macOS (tested in CI)
- Release optimization: LTO enabled, size-optimized, symbols stripped

---

*Stack analysis: 2026-01-16*
