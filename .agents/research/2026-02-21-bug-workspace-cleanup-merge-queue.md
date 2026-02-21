# Bug Report: Workspace Cleanup and Merge Queue Reliability

**Date**: 2026-02-21
**Investigator**: Bug Hunt Investigation
**Severity**: High
**Status**: ✅ FIXED

## Executive Summary

Two critical bugs were affecting workspace cleanup and merge queue reliability:

1. **Workspace cleanup is never triggered** - The "24h retention" mentioned in comments was not implemented
2. **Merged queue entries are never cleaned** - The cleanup method excluded `merged` status

Both issues have been fixed.

---

## Fixes Applied

### Fix 1: Queue Cleanup Now Includes All Terminal Statuses

**File:** `crates/zjj-core/src/coordination/queue.rs:1028-1068`

**Before:**
```rust
"DELETE FROM merge_queue WHERE status IN ('completed', 'failed')"
```

**After:**
```rust
const TERMINAL_STATUSES: &str = "'merged', 'failed_terminal', 'cancelled', 'completed', 'failed'";
// Now includes: merged, failed_terminal, cancelled, completed, failed
```

### Fix 2: Done Command Now Cleans Up By Default

**File:** `crates/zjj/src/commands/done/mod.rs:285-296`

**Before:**
```rust
let cleaned = if options.keep_workspace || !options.no_keep {
    false  // Never cleaned by default
} else { ... }
```

**After:**
```rust
let cleaned = if options.keep_workspace {
    false  // Only skip cleanup if explicitly requested
} else {
    cleanup_workspace(...).await?  // Clean up by default
};
```

### Fix 3: Periodic Cleanup Now Handles Completed Sessions

**File:** `crates/zjj/src/commands/clean/periodic_cleanup.rs`

Added:
1. New `completed_age_threshold` config (default: 24 hours)
2. Logic to detect completed sessions past retention period
3. Cleanup of completed sessions alongside orphans

---

## Summary of Changes

| File | Change |
|------|--------|
| `queue.rs` | Cleanup query now includes `merged`, `cancelled`, `failed_terminal` |
| `done/mod.rs` | Default behavior is now to cleanup workspace after merge |
| `periodic_cleanup.rs` | Added `completed_age_threshold` and logic to clean completed sessions |
| `clean/mod.rs` | Added `completed_age_threshold` to periodic cleanup config |

---

## Original Bug Report (Preserved for Reference)

### Bug 1: Workspace Cleanup Never Executes

**Location:** `crates/zjj/src/commands/done/mod.rs:285-289`

The `done` command had this cleanup logic:

```rust
let cleaned = if options.keep_workspace || !options.no_keep {
    false  // Never cleanup
} else {
    cleanup_workspace(...)
};
```

The default behavior (no flags) resulted in **NO CLEANUP** because:
- `keep_workspace = false` (default)
- `no_keep = false` (default)
- `false || !false = false || true = true` → NO CLEANUP

### Bug 2: Merged Queue Entries Never Cleaned

**Location:** `crates/zjj-core/src/coordination/queue.rs:1052-1053`

The `cleanup()` method only targeted `completed` and `failed` statuses, but successful merges use status `merged`.

---

## Verification

- ✅ Build passes (`cargo check --all-targets`)
- ✅ Queue tests pass (210 tests)
- ✅ No unwraps added to modified code
- ✅ Functional Rust patterns maintained
