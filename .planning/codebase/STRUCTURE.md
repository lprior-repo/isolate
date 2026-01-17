# Codebase Structure

**Analysis Date:** 2026-01-16

## Directory Layout

```
zjj/
├── crates/
│   ├── zjj-core/        # Core library (error handling, JJ/Zellij/Beads integrations)
│   └── zjj/             # CLI binary (commands, session DB, main entry point)
├── docs/                # User documentation (guides, architecture notes)
├── .planning/           # GSD planning artifacts
│   └── codebase/        # Codebase mapping documents (you are here)
├── schemas/             # JSON schemas for configuration
├── scripts/             # Build/release automation
├── .beads/              # Beads issue tracking database
├── .jjz/                # Session state (created by jjz init)
├── Cargo.toml           # Workspace manifest
├── moon.yml             # Moon build configuration (REQUIRED - use moon, not cargo)
├── .clippy.toml         # Clippy lint rules (DO NOT MODIFY)
└── rustfmt.toml         # Rust formatting rules
```

## Directory Purposes

**crates/zjj-core:**
- Purpose: Reusable functional core library
- Contains: Error types, JJ workspace ops, Zellij control, Beads integration, config parsing
- Key files: `src/lib.rs` (module exports), `src/error.rs` (Error enum), `src/jj.rs`, `src/zellij.rs`, `src/beads.rs`

**crates/zjj:**
- Purpose: CLI application binary
- Contains: Main entry point, command implementations, session persistence, CLI parsing
- Key files: `src/main.rs` (entry point), `src/cli.rs` (CLI helpers), `src/db.rs` (SessionDb), `src/session.rs` (Session type)

**crates/zjj/src/commands:**
- Purpose: Individual command implementations
- Contains: One .rs file per command (add.rs, remove.rs, focus.rs, sync.rs, etc.)
- Key files: `mod.rs` (command exports), each command has options struct and run function

**docs/**
- Purpose: User-facing documentation
- Contains: Installation guides, usage examples, architecture diagrams
- Key files: Markdown files for different topics

**schemas/**
- Purpose: JSON schema definitions for validation
- Contains: Schema files for configuration, output formats
- Key files: .json schema files

**.beads/**
- Purpose: Issue tracking database (managed by Beads)
- Contains: SQLite database for issues/tasks
- Generated: Yes (by Beads CLI)
- Committed: Yes (SQLite file is version controlled)

**.jjz/**
- Purpose: Session state storage
- Contains: sessions.db (SQLite), backups/, config.toml
- Generated: Yes (by `jjz init`)
- Committed: No (added to .gitignore)

## Key File Locations

**Entry Points:**
- `crates/zjj/src/main.rs`: Binary entry point, CLI dispatcher, error formatting
- `crates/zjj/src/lib.rs`: Library interface for benchmarks/tests
- `crates/zjj-core/src/lib.rs`: Core library module exports

**Configuration:**
- `Cargo.toml`: Workspace config, shared dependencies, lint rules (forbid unwrap/panic)
- `moon.yml`: Build tasks (run moon, not cargo directly)
- `.clippy.toml`: Clippy configuration (DO NOT MODIFY per CLAUDE.md)
- `rustfmt.toml`: Code formatting rules
- `rust-toolchain.toml`: Rust version pinning

**Core Logic:**
- `crates/zjj/src/db.rs`: Session database (SQLite pool, CRUD operations)
- `crates/zjj/src/session.rs`: Session type and validation
- `crates/zjj-core/src/error.rs`: Error type definitions
- `crates/zjj-core/src/result.rs`: Result type alias and extensions
- `crates/zjj-core/src/jj.rs`: JJ workspace lifecycle management
- `crates/zjj-core/src/zellij.rs`: Zellij tab control
- `crates/zjj-core/src/beads.rs`: Beads issue tracking integration

**Testing:**
- `crates/zjj-core/tests/`: Integration tests for core library
- `crates/zjj/tests/`: Integration tests for CLI
- `crates/zjj/benches/`: Criterion benchmarks (config_operations, validation)
- Inline unit tests at bottom of each module (using #[cfg(test)])

## Naming Conventions

**Files:**
- Rust source: snake_case (e.g., `session.rs`, `json_output.rs`)
- Binary name: `jjz` (defined in Cargo.toml, not zjj)
- Config files: lowercase with dots (e.g., `.clippy.toml`, `moon.yml`)

**Directories:**
- Snake_case for multi-word (e.g., `zjj-core`, not `zjj_core` because it's a crate name with hyphen)
- Lowercase single word (e.g., `crates`, `docs`, `schemas`)

**Rust Identifiers:**
- Functions: snake_case (e.g., `validate_session_name`, `run_with_options`)
- Types/Structs: PascalCase (e.g., `SessionDb`, `AddOptions`)
- Constants: SCREAMING_SNAKE_CASE (e.g., `SCHEMA` in db.rs)
- Modules: snake_case (e.g., `mod json_output`, `mod commands`)

## Where to Add New Code

**New Command:**
- Primary code: `crates/zjj/src/commands/<command_name>.rs`
- Tests: Inline in same file or `crates/zjj/tests/integration/`
- Register: Add to `commands/mod.rs` and wire into main.rs dispatcher

**New Core Integration:**
- Implementation: `crates/zjj-core/src/<integration_name>.rs`
- Export: Add to `crates/zjj-core/src/lib.rs` pub mod list
- Tests: `crates/zjj-core/tests/` or inline

**New Session Field:**
- Add to Session struct: `crates/zjj/src/session.rs`
- Update database schema: Modify SCHEMA const in `crates/zjj/src/db.rs`
- Update serialization: SessionUpdate struct and parse_session_row function
- Migration: No automated migrations (manual schema updates)

**New Error Variant:**
- Add to Error enum: `crates/zjj-core/src/error.rs`
- Implement Display: Add match arm in Display impl
- Add From conversion if external error type

**New Utility Function:**
- Shared helpers: `crates/zjj-core/src/functional.rs` (if pure/functional)
- CLI-specific: `crates/zjj/src/cli.rs`
- Core library only: Keep in relevant module (jj.rs, zellij.rs, etc.)

## Special Directories

**.moon/cache:**
- Purpose: Moon build system cache
- Generated: Yes (by moon commands)
- Committed: No (.gitignore excludes it)

**target/**
- Purpose: Cargo build artifacts
- Generated: Yes (by cargo/moon)
- Committed: No (.gitignore excludes it)

**.git/**
- Purpose: Version control metadata
- Generated: Yes (by git init)
- Committed: N/A (git internals)

**.jj/**
- Purpose: Jujutsu VCS metadata (this repo uses JJ)
- Generated: Yes (by jj init)
- Committed: No (JJ internals)

**.codanna/**
- Purpose: Codanna code search index
- Generated: Yes (by codanna tool)
- Committed: No (.gitignore excludes it)

**.claude/**
- Purpose: Claude Code artifacts
- Generated: Yes (by Claude Code)
- Committed: No (.gitignore excludes it)

**examples/**
- Purpose: Example code demonstrating zjj usage
- Generated: No (manually created)
- Committed: Yes

**tools/audit:**
- Purpose: Custom cargo-audit wrapper
- Generated: No (custom tooling)
- Committed: Yes

---

*Structure analysis: 2026-01-16*
