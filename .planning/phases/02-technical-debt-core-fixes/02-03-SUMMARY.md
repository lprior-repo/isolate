---
phase: 02-technical-debt-core-fixes
plan: 03
subsystem: testing
tags: [async, tokio, clippy, testing, patterns]

# Dependency graph
requires:
  - phase: 01-critical-security-validation
    provides: Secure foundation
provides:
  - Async testing pattern that avoids tokio::test clippy conflict
  - Direct unit test coverage for async functions
  - Reusable test helper pattern
affects: [testing, code-quality]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Test helper function for async tests without tokio::test macro"
    - "Manual runtime creation with unwrap_or_else for test setup"

key-files:
  created: []
  modified:
    - crates/zjj-core/src/watcher.rs
    - crates/zjj-core/src/beads.rs
    - .planning/codebase/TESTING.md

key-decisions:
  - "Test Helper approach (Option 3) chosen over manual-runtime or integration-only"
  - "run_async() helper provides reusable pattern with minimal boilerplate"
  - "Maintains zero-unwrap policy: unwrap_or_else with explicit panic message"

patterns-established:
  - "Async unit tests use run_async() helper to avoid #[tokio::test] clippy conflict"

# Metrics
duration: 15min
completed: 2026-01-16
---

# Phase 02 Plan 03: Async Testing Strategy Summary

**Async unit tests now possible with test helper pattern - tokio::test clippy conflict resolved**

## Performance

- **Duration:** 15 min
- **Started:** 2026-01-16T15:45:00Z
- **Completed:** 2026-01-16T16:00:00Z
- **Tasks:** 2 (Decision + Implementation)
- **Files modified:** 3

## Accomplishments
- Implemented run_async() test helper in watcher.rs and beads.rs
- Enabled test_query_beads_status_no_beads (watcher.rs)
- Enabled test_query_beads_empty_path (beads.rs)
- Documented async testing pattern in TESTING.md
- All 202 tests passing (DEBT-03 resolved)

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement async testing strategy** - `2a4e46b` (feat)
2. **Documentation update** - `fc109e3` (docs) - separate commit for audit doc

## Files Created/Modified
- `crates/zjj-core/src/watcher.rs` - Added run_async() helper, enabled async test
- `crates/zjj-core/src/beads.rs` - Added run_async() helper, enabled async test
- `.planning/codebase/TESTING.md` - Documented async testing pattern with rationale

## Decisions Made

**Async Testing Strategy: Test Helper (Option 3)**
- **Rationale:** Best balance of testing coverage and code quality
- **Alternatives considered:**
  1. Manual Runtime - Too much boilerplate per test
  2. Integration Tests Only - Loses unit test granularity
  3. Test Helper - ✅ Chosen: reusable, clean, maintains coverage
- **Benefits:**
  - Reusable pattern across codebase
  - Less boilerplate than manual runtime per test
  - Direct unit test coverage maintained
  - Follows zero-unwrap policy (unwrap_or_else with panic)
  - Avoids #[tokio::test] macro clippy conflict

**Helper Implementation:**
```rust
fn run_async<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        panic!("Failed to create tokio runtime for test: {e}");
    });
    runtime.block_on(f());
}
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added TempDir import to watcher.rs tests**
- **Found during:** Test implementation
- **Issue:** TempDir not in scope for test module
- **Fix:** Added `use tempfile::TempDir;` to test module imports
- **Files modified:** crates/zjj-core/src/watcher.rs
- **Verification:** Compiles without errors, test passes
- **Committed in:** 2a4e46b (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (missing import)
**Impact on plan:** Minor - expected compiler feedback, no scope creep

## Issues Encountered

None - straightforward implementation after decision made.

## Next Phase Readiness

- DEBT-03 fully closed: Async testing pattern implemented and documented
- Pattern can be reused for future async unit tests
- All 202 tests passing with zero clippy violations
- Ready for remaining technical debt items (performance optimization)

---
*Phase: 02-technical-debt-core-fixes*
*Completed: 2026-01-16*
