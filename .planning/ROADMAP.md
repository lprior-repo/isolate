# Roadmap: zjj Technical Excellence Initiative

## Overview

Transform zjj into a production-ready, AI-native CLI tool through systematic elimination of technical debt, comprehensive testing, performance optimization, and codebase health improvements. Each phase delivers verifiable improvements toward zero-compromise quality.

## Phases

**Phase Numbering:**
- Integer phases (1-10): Planned milestone work
- Decimal phases (X.1, X.2): Urgent insertions (marked with INSERTED)

- [x] **Phase 1: Critical Security & Validation**
- [x] **Phase 2: Technical Debt - Core Fixes**
- [x] **Phase 3: MVP Command Verification**
- [x] **Phase 4: Test Infrastructure**
- [x] **Phase 5: Integration Testing**
- [ ] **Phase 6: Performance Foundation**
- [ ] **Phase 7: Memory Optimization**
- [ ] **Phase 8: AI-Native CLI Core**
- [ ] **Phase 9: AI-Native CLI Polish**
- [ ] **Phase 10: Codebase Health**

## Phase Details

### Phase 1: Critical Security & Validation

**Goal**: Eliminate directory traversal vulnerability
**Depends on**: Nothing (first phase - critical security fix)
**Requirements**: DEBT-04
**Success Criteria** (what must be TRUE):
  1. Workspace paths reject `..` components with clear error
  2. Path canonicalization prevents symlink escapes outside repo
  3. Security tests verify boundary enforcement
**Research**: Unlikely (standard path validation patterns)
**Plans**: TBD (run /gsd:plan-phase 1 to break down)

Plans:
- [x] 01-01: Path Validation with Canonicalization (completed 2026-01-16)
- [x] 01-02: Absolute Path Rejection (gap closure, completed 2026-01-16)

### Phase 2: Technical Debt - Core Fixes

**Goal**: Fix broken APIs and stubbed functionality
**Depends on**: Phase 1
**Requirements**: DEBT-01, DEBT-02, DEBT-03
**Success Criteria** (what must be TRUE):
  1. ✅ Benchmarks execute using correct `load_config()` API
  2. ✅ Hints system detects actual JJ status changes
  3. ✅ Async tests can be written with test helper (documented alternative)
**Research**: Unlikely (internal API fixes)
**Status**: COMPLETE (2026-01-16)

Plans:
- [x] 02-01: Benchmark Config API Fix (completed 2026-01-16)
- [x] 02-02: Change Detection Implementation (completed 2026-01-16)
- [x] 02-03: Async Testing Strategy (completed 2026-01-16)

### Phase 3: MVP Command Verification

**Goal**: All core CLI commands fully functional and tested
**Depends on**: Phase 2
**Requirements**: CMD-01, CMD-02, CMD-03, CMD-04, CMD-05
**Success Criteria** (what must be TRUE):
  1. ✅ `zjj init` initializes project with all required components (15 tests)
  2. ✅ `zjj add` creates session with JJ workspace and Zellij tab (20+ tests)
  3. ✅ `zjj list` displays all sessions with correct formatting (11+ tests)
  4. ✅ `zjj remove` cleanly removes session and all artifacts (10+ tests, atomic)
  5. ✅ `zjj focus` switches to correct Zellij tab (13+ tests)
**Research**: Unlikely (verification of existing code)
**Status**: COMPLETE (2026-01-16) - All commands verified functional with comprehensive test coverage

Plans:
- [x] Manual verification via test analysis (completed 2026-01-16)

### Phase 4: Test Infrastructure

**Goal**: Edge case and failure mode test coverage
**Depends on**: Phase 3
**Requirements**: TEST-01, TEST-02, TEST-03
**Success Criteria** (what must be TRUE):
  1. ✅ Hook execution handles non-UTF8, timeouts, large output without panics
  2. ✅ Database corruption scenarios are tested and recovered
  3. ✅ Concurrent session operations don't cause race conditions
**Research**: Unlikely (standard testing patterns)
**Status**: COMPLETE (2026-01-16) - Comprehensive test coverage verified across test_error_scenarios.rs and error_recovery.rs

Plans:
- [x] Manual verification via test analysis (completed 2026-01-16)

### Phase 5: Integration Testing

**Goal**: External dependency compatibility verified
**Depends on**: Phase 4
**Requirements**: TEST-04, TEST-05, TEST-06
**Success Criteria** (what must be TRUE):
  1. ✅ JJ version compatibility matrix documented and tested
  2. ✅ Zellij integration failures handled gracefully
  3. ✅ Workspace cleanup is atomic on command failure
**Research**: Completed (JJ version format, breaking changes)
**Research topics**: JJ version detection methods (implemented), Zellij IPC protocol (tested)
**Status**: COMPLETE (2026-01-16) - All integration testing criteria met

Plans:
- [x] 05-ASSESSMENT: Verify Zellij and cleanup tests (completed 2026-01-16)
- [x] 05-VERSION-COMPAT: Implement JJ version detection (completed 2026-01-16)

### Phase 6: Performance Foundation

**Goal**: Hot paths profiled and baseline optimizations applied
**Depends on**: Phase 5
**Requirements**: PERF-01, PERF-02, PERF-03, PERF-04
**Success Criteria** (what must be TRUE):
  1. Flame graphs generated for add, sync, list commands
  2. String allocations reduced by 30% in profiled hot paths
  3. Database connection pool explicitly configured
  4. Memory allocation hotspots identified and minimized
**Research**: Unlikely (standard Rust profiling)
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 7: Memory Optimization

**Goal**: String and clone overhead eliminated through functional patterns
**Depends on**: Phase 6
**Requirements**: DEBT-05, DEBT-06
**Success Criteria** (what must be TRUE):
  1. Hot paths use `&str` and `Cow<str>` instead of `String` clones
  2. `im` crate's structural sharing leveraged for collections
  3. Benchmarks show measurable performance improvement
  4. Clone usage reduced by 40% in frequently-called code
**Research**: Unlikely (existing `im` crate usage)
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 8: AI-Native CLI Core

**Goal**: Structured output and error handling for AI agents
**Depends on**: Phase 7
**Requirements**: AI-01, AI-02, AI-04
**Success Criteria** (what must be TRUE):
  1. All commands support `--json` flag with consistent schema
  2. Errors include machine-readable codes and correction guidance
  3. Exit codes are consistent and documented (0=success, 1=user error, 2=system error, etc.)
**Research**: Unlikely (standard JSON serialization)
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 9: AI-Native CLI Polish

**Goal**: Composability and discoverability for AI agents
**Depends on**: Phase 8
**Requirements**: AI-03, AI-05
**Success Criteria** (what must be TRUE):
  1. Commands detect TTY vs pipe and format output accordingly
  2. Output is pipe-friendly (silent mode, no ANSI in pipes)
  3. Help text is structured for AI parsing with clear examples
**Research**: Unlikely (standard CLI patterns)
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 10: Codebase Health

**Goal**: Code is modular, well-documented, and AI-navigable
**Depends on**: Phase 9
**Requirements**: DEBT-07, HEALTH-01, HEALTH-02, HEALTH-03, HEALTH-04
**Success Criteria** (what must be TRUE):
  1. No file exceeds 800 lines
  2. Large files split into logical submodules with clear boundaries
  3. Common patterns extracted into reusable abstractions
  4. Code has sufficient documentation for AI code modification
**Research**: Unlikely (internal refactoring)
**Plans**: TBD

Plans:
- [ ] TBD

## Progress

**Execution Order:**
Phases execute sequentially: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Critical Security & Validation | 2/2 | Complete | 2026-01-16 |
| 2. Technical Debt - Core Fixes | 3/3 | Complete | 2026-01-16 |
| 3. MVP Command Verification | 1/1 | Complete | 2026-01-16 |
| 4. Test Infrastructure | 1/1 | Complete | 2026-01-16 |
| 5. Integration Testing | 2/2 | Complete | 2026-01-16 |
| 6. Performance Foundation | 0/TBD | Not started | - |
| 7. Memory Optimization | 0/TBD | Not started | - |
| 8. AI-Native CLI Core | 0/TBD | Not started | - |
| 9. AI-Native CLI Polish | 0/TBD | Not started | - |
| 10. Codebase Health | 0/TBD | Not started | - |
