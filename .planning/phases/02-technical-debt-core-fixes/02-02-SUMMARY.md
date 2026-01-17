---
phase: 02-technical-debt-core-fixes
plan: 02
subsystem: core
tags: [jj, hints, status-detection, testing, functional-programming]

# Dependency graph
requires:
  - phase: 01-critical-security-validation
    provides: Path validation security foundation
provides:
  - Change detection system via JJ status parsing
  - Hints system accurately reports uncommitted changes
  - SystemContext reflects actual repository state
affects: [dashboard, status-display, session-management]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Graceful error degradation in advisory systems
    - Functional error handling with Result unwrapping

key-files:
  created: []
  modified:
    - crates/zjj-core/src/jj.rs
    - crates/zjj-core/src/hints.rs

key-decisions:
  - "Use check_in_jj_repo() to get repo path within hints (no SystemState API change)"
  - "Graceful degradation: unwrap_or(false) for change detection failures"
  - "JJ status pattern matching for change detection (Working copy changes, Modified files, etc.)"

patterns-established:
  - "Advisory systems degrade gracefully on errors (return safe default, don't crash)"
  - "Change detection via JJ status output patterns (simple, reliable)"

# Metrics
duration: 4min
completed: 2026-01-16
---

# Phase 2 Plan 02: Change Detection Summary

**Hints system now detects uncommitted changes via JJ status parsing with graceful error handling**

## Performance

- **Duration:** 4 min
- **Started:** 2026-01-16T14:50:21Z
- **Completed:** 2026-01-16T14:54:31Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- Implemented `has_uncommitted_changes()` function in jj.rs using JJ status parsing
- Replaced stubbed `has_changes` field with actual repository state detection
- Added unit test demonstrating change detection functionality
- Hints system now accurately reports repository state (DEBT-02 closed)

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement has_uncommitted_changes in jj.rs** - `aa9d04f` (feat)
2. **Task 2: Replace stubbed has_changes with actual detection** - `1cc5ed8` (feat)
3. **Task 3: Add test for change detection** - `c9b9166` (test)

## Files Created/Modified
- `crates/zjj-core/src/jj.rs` - Added `has_uncommitted_changes()` function and unit test
- `crates/zjj-core/src/hints.rs` - Replaced stubbed change detection with actual JJ status check

## Decisions Made

**1. Get repo path via check_in_jj_repo() rather than modifying SystemState**
- **Rationale:** Avoids architectural change to SystemState API, simpler implementation
- **Trade-off:** Two function calls instead of using cached path, but hints are advisory and infrequent

**2. Graceful degradation with unwrap_or(false)**
- **Rationale:** Hints are advisory, failing to detect changes shouldn't break the system
- **Pattern:** Advisory features return safe defaults on error rather than propagating failures

**3. JJ status pattern matching approach**
- **Rationale:** JJ status output has consistent markers ("Working copy changes:", "Modified files:", etc.)
- **Alternative considered:** Parsing full diff output (rejected: too complex, unnecessary)
- **Benefit:** Simple, reliable, maintainable

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added check_in_jj_repo() call to get repo path**
- **Found during:** Task 2 (Replace stubbed has_changes)
- **Issue:** Plan assumed `state.repo_path` existed, but SystemState doesn't have this field
- **Fix:** Call `check_in_jj_repo()` to get repo path, chain with has_uncommitted_changes
- **Files modified:** crates/zjj-core/src/hints.rs
- **Verification:** Compiles successfully, gracefully handles both function results
- **Committed in:** 1cc5ed8 (Task 2 commit)

**2. [Rule 1 - Bug] Removed unnecessary anyhow imports**
- **Found during:** Task 1 (Implement has_uncommitted_changes)
- **Issue:** Plan specified anyhow imports, but function uses existing Error type pattern
- **Fix:** Removed `use anyhow::{bail, Context}` to match existing jj.rs error handling
- **Files modified:** crates/zjj-core/src/jj.rs
- **Verification:** Compiles without warnings, follows project conventions
- **Committed in:** aa9d04f (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 missing critical, 1 bug)
**Impact on plan:** Both fixes necessary for correct implementation. No scope creep.

## Issues Encountered

None - plan executed smoothly with minor necessary adjustments.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- DEBT-02 fully closed: Hints system accurately reflects repository state
- Change detection foundation ready for dashboard and status displays
- Pattern established for graceful degradation in advisory features
- Ready to proceed with remaining technical debt items

---
*Phase: 02-technical-debt-core-fixes*
*Completed: 2026-01-16*
