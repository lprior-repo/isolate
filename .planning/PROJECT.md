# zjj: Technical Excellence & AI-First CLI

## What This Is

A comprehensive technical excellence initiative for zjj (JJ+Zellij session manager). This work eliminates all technical debt, verifies MVP feature completion, refactors for optimal performance, and transforms the codebase and CLI into an AI-native development experience. The goal is a world-class, production-ready tool that's trivial for AI to work on and with.

## Core Value

**Zero compromise on quality.** When complete, zjj will be technical debt-free, feature-complete for MVP, optimally performant, and the gold standard for AI-friendly Rust CLI tools—both in codebase clarity and CLI interaction design.

## Requirements

### Validated

**Existing Capabilities** (from codebase analysis):

- ✓ Functional programming patterns with zero panics/unwraps — existing
- ✓ SQLite-based session persistence with Beads integration — existing
- ✓ JJ workspace management via contracts — existing
- ✓ Zellij tab integration for session switching — existing
- ✓ Comprehensive error handling using Result types — existing
- ✓ Moon-based build system (not raw cargo) — existing
- ✓ Security: parameterized queries, command injection protection — existing

### Active

**Technical Debt Elimination:**

- [ ] Fix benchmark configuration API mismatch (benches/config_operations.rs:113)
- [ ] Implement change detection in hints system (hints.rs:417)
- [ ] Resolve tokio test macro incompatibility with clippy::expect_used
- [ ] Add workspace path escape validation (commands/add.rs)
- [ ] Optimize string allocation patterns (999 to_string/to_owned calls)
- [ ] Reduce clone usage prevalence (106 occurrences)
- [ ] Split large files into maintainable submodules (7 files >900 lines)

**MVP Command Verification:**

- [ ] Verify `jjz init` fully implemented and tested
- [ ] Verify `jjz add <name>` fully implemented and tested
- [ ] Verify `jjz list` fully implemented and tested
- [ ] Verify `jjz remove <name>` fully implemented and tested
- [ ] Verify `jjz focus <name>` fully implemented and tested

**Test Coverage:**

- [ ] Hook execution edge cases (non-UTF8 output, timeouts, large output)
- [ ] Database corruption recovery scenarios
- [ ] Concurrent session operations
- [ ] JJ version compatibility matrix
- [ ] Zellij integration failure modes
- [ ] Workspace cleanup on failure atomicity

**AI-Native CLI Design:**

- [ ] JSON output mode for all commands (--json flag)
- [ ] Structured error messages with correction guidance
- [ ] Command composability (pipe-friendly output)
- [ ] Machine-readable status codes
- [ ] CLI self-documentation (help text optimized for AI parsing)

**Performance Optimization:**

- [ ] Profile critical paths (add, sync, list commands)
- [ ] Optimize hot paths to use &str and Cow<str>
- [ ] Database connection pooling configuration
- [ ] Reduce memory allocations in frequent operations

**Codebase Health:**

- [ ] Refactor beads.rs (2135 lines) into query/filter modules
- [ ] Split commands/add.rs (1515 lines) into validation/workspace submodules
- [ ] Extract common patterns into reusable abstractions
- [ ] Improve code documentation for AI context

### Out of Scope

- Multi-threaded async runtime (single-threaded is correct for CLI) — architecture decision
- PostgreSQL migration (SQLite is appropriate for single-user CLI) — complexity not justified
- Telemetry/observability (can be v2 if needed) — privacy-first approach
- GUI or TUI interface (CLI-first tool) — out of scope
- Support for non-Zellij terminal multiplexers — focused tool philosophy

## Context

**Codebase State:**
- Rust CLI tool with 2-crate structure (zjj-core library, zjj binary)
- 1,479 lines of codebase documentation in .planning/codebase/
- Comprehensive security audit (zero vulnerabilities as of 2026-01-11)
- Strong functional programming foundation with explicit error handling
- Moon build system enforces strict quality gates

**Technical Environment:**
- Dependencies: JJ (Jujutsu VCS), Zellij (terminal multiplexer), Beads (issue tracking)
- Stack: Rust, SQLx (async SQLite), tokio (single-threaded runtime)
- Quality: Zero panics, zero unwraps, comprehensive clippy rules

**Known Issues:**
- 10 high-priority technical debt items documented in CONCERNS.md
- Test coverage gaps in edge cases and integration scenarios
- Performance overhead from excessive string allocations and cloning
- Large files indicating need for modularization

## Constraints

- **Code Quality**: Zero panics, zero unwraps — non-negotiable functional safety
- **Build System**: Moon only, never raw cargo commands — project standard
- **Lint Configuration**: Never modify clippy rules, fix code not rules — explicit requirement
- **API Contracts**: JJ and Zellij integration via contracts pattern — architectural decision
- **Beads Integration**: Hard requirement, always sync with .beads/beads.db — workflow integration
- **Testing**: Comprehensive coverage including edge cases — quality requirement
- **Performance**: Sub-second command response times — user experience requirement

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Zero unwraps/panics | Functional safety, no runtime crashes | — Pending validation |
| Moon build system | Consistent quality gates, no cargo drift | ✓ Good (enforces standards) |
| Single-threaded tokio | CLI tool doesn't need multi-threaded overhead | ✓ Good (minimal overhead) |
| SQLite over PostgreSQL | Single-user CLI, no concurrency needs | ✓ Good (appropriate scale) |
| Beads hard integration | Workflow alignment, issue tracking required | — Pending (verify integration complete) |
| AI-native CLI design | Future-proof for AI agents using tool | — Pending implementation |

---
*Last updated: 2026-01-16 after project initialization*
