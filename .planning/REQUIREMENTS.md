# Requirements: zjj Technical Excellence Initiative

**Defined:** 2026-01-16
**Core Value:** Zero compromise on quality—technical debt-free, feature-complete MVP, optimally performant, AI-native CLI experience.

## v1 Requirements

### Technical Debt (DEBT)

- [x] **DEBT-01**: Benchmark configuration uses correct `load_config()` API instead of non-existent `ConfigLoader`
- [x] **DEBT-02**: Hints system implements actual change detection via `jj status` parsing (not hardcoded `false`)
- [x] **DEBT-03**: Tokio test macro compatibility resolved with clippy::expect_used deny rule
- [x] **DEBT-04**: Workspace paths validated to prevent directory escape attacks (reject `..` components)
- [ ] **DEBT-05**: String allocation patterns optimized in hot paths (use `&str` and `Cow<str>`)
- [ ] **DEBT-06**: Clone usage reduced through structural sharing with `im` crate
- [ ] **DEBT-07**: Large files split into submodules (no file >800 lines)

### MVP Commands (CMD)

- [x] **CMD-01**: `zjj init` command fully implemented, tested, and verified functional
- [x] **CMD-02**: `zjj add <name>` command fully implemented, tested, and verified functional
- [x] **CMD-03**: `zjj list` command fully implemented, tested, and verified functional
- [x] **CMD-04**: `zjj remove <name>` command fully implemented, tested, and verified functional
- [x] **CMD-05**: `zjj focus <name>` command fully implemented, tested, and verified functional

### Test Coverage (TEST)

- [x] **TEST-01**: Hook execution handles non-UTF8 output, timeouts, and large output
- [x] **TEST-02**: Database corruption recovery scenarios tested and handled
- [x] **TEST-03**: Concurrent session operations tested for race conditions
- [x] **TEST-04**: JJ version compatibility matrix established and tested
- [x] **TEST-05**: Zellij integration failure modes comprehensively tested
- [x] **TEST-06**: Workspace cleanup atomicity verified on command failure

### AI-Native CLI (AI)

- [ ] **AI-01**: All commands support `--json` flag for structured output
- [ ] **AI-02**: Error messages structured with correction guidance for AI agents
- [ ] **AI-03**: Command output optimized for pipe composability
- [ ] **AI-04**: Machine-readable exit codes implemented consistently
- [ ] **AI-05**: Help text formatted for optimal AI parsing and understanding

### Performance (PERF)

- [ ] **PERF-01**: Critical command paths profiled (add, sync, list)
- [ ] **PERF-02**: Hot paths optimized to use `&str` and `Cow<str>` instead of `String`
- [ ] **PERF-03**: Database connection pool size explicitly configured
- [ ] **PERF-04**: Memory allocations minimized in frequent operations

### Codebase Health (HEALTH)

- [ ] **HEALTH-01**: beads.rs (2135 lines) refactored into query/filter modules
- [ ] **HEALTH-02**: commands/add.rs (1515 lines) split into validation/workspace submodules
- [ ] **HEALTH-03**: Common patterns extracted into reusable abstractions
- [ ] **HEALTH-04**: Code documentation improved for AI code navigation and modification

## v2 Requirements (Deferred)

### Observability
- **OBS-01**: Opt-in anonymous telemetry for error reporting
- **OBS-02**: Performance metrics collection and dashboard

### Extended Platform Support
- **PLAT-01**: TUI interface for interactive workflows
- **PLAT-02**: Support for tmux as alternative multiplexer

### Advanced Features
- **ADV-01**: Session templates for common workflows
- **ADV-02**: Automatic session cleanup for stale workspaces
- **ADV-03**: Integration with GitHub/GitLab for PR workflows

## Out of Scope

| Feature | Reason |
|---------|--------|
| Multi-threaded async runtime | Single-threaded is optimal for CLI tool; multi-threading adds unnecessary overhead |
| PostgreSQL migration | SQLite is appropriate for single-user CLI; no concurrency justification |
| GUI interface | CLI-first tool philosophy; graphical interface out of scope |
| Non-Zellij multiplexer support | Focused tool philosophy; deep Zellij integration is core value |
| Real-time collaboration | Single-user tool; collaboration via git is sufficient |
| Cloud sync | Local-first design; git provides sync mechanism |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEBT-01 | Phase 2 | Complete |
| DEBT-02 | Phase 2 | Complete |
| DEBT-03 | Phase 2 | Complete |
| DEBT-04 | Phase 1 | Complete |
| DEBT-05 | Phase 7 | Pending |
| DEBT-06 | Phase 7 | Pending |
| DEBT-07 | Phase 10 | Pending |
| CMD-01 | Phase 3 | Complete |
| CMD-02 | Phase 3 | Complete |
| CMD-03 | Phase 3 | Complete |
| CMD-04 | Phase 3 | Complete |
| CMD-05 | Phase 3 | Complete |
| TEST-01 | Phase 4 | Complete |
| TEST-02 | Phase 4 | Complete |
| TEST-03 | Phase 4 | Complete |
| TEST-04 | Phase 5 | Complete |
| TEST-05 | Phase 5 | Complete |
| TEST-06 | Phase 5 | Complete |
| AI-01 | Phase 8 | Pending |
| AI-02 | Phase 8 | Pending |
| AI-03 | Phase 9 | Pending |
| AI-04 | Phase 8 | Pending |
| AI-05 | Phase 9 | Pending |
| PERF-01 | Phase 6 | Pending |
| PERF-02 | Phase 6 | Pending |
| PERF-03 | Phase 6 | Pending |
| PERF-04 | Phase 6 | Pending |
| HEALTH-01 | Phase 10 | Pending |
| HEALTH-02 | Phase 10 | Pending |
| HEALTH-03 | Phase 10 | Pending |
| HEALTH-04 | Phase 10 | Pending |

**Coverage:**
- v1 requirements: 29 total
- Mapped to phases: 29 ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-01-16*
*Last updated: 2026-01-16 after initial definition*
