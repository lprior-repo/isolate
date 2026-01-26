# Ralph Loop Phase 0-1: TRIAGE→RESEARCH SUMMARY

**Timestamp:** 2026-01-25 20:25 UTC
**Task:** Research next 5 open/in-progress beads for TDD15 workflow
**Status:** COMPLETE

## Beads Researched

### 1. zjj-gv3f - EPIC: State Tracking Infrastructure [COMPLEX - P0]

**Status:** IN_PROGRESS (Epic)

**Summary:**
- Foundational epic for AI observability covering state snapshots, diffs, and before/after tracking
- **Blocked by:** zjj-fl0d (CUE schema), zjj-txqd (History database)
- **Blocks:** 5+ child tasks including zjj-jakw, zjj-ioa3, zjj-05ut
- **Key Files:** session_state.rs, json.rs, status.rs, sync.rs, db.rs
- **Complexity:** VERY_HIGH - Multiple phases, coordination work

**Phase Routing:** Orchestration/coordination - skip phases 1-3 for child tasks

**Files:**
- `/home/lewis/src/zjj/.tdd15-cache/zjj-gv3f/research.json`
- `/home/lewis/src/zjj/.tdd15-cache/zjj-gv3f/progress.json`

---

### 2. zjj-jakw - Wrap StatusOutput in SchemaEnvelope [SIMPLE - P0]

**Status:** IN_PROGRESS (Task)

**Summary:**
- Wrap StatusOutput JSON in SchemaEnvelope with schema_type="single"
- Straightforward wrapping task, ~30 minutes effort
- **Key File:** crates/zjj/src/commands/status.rs (output_json function)
- **Dependency:** SchemaEnvelope already exists in json.rs (line 360-369)
- **No Breaking Changes:** Pure wrapping, API is stable

**Implementation Sketch:**
1. Create SessionStatusInfo struct with session data
2. Wrap with SchemaEnvelope::new("status-response", "single", status_info)
3. Serialize and print JSON
4. Add 3 RED tests for envelope validation

**Phase Routing:** SIMPLE - Skip phases 1-3, start at phase 4 (RED tests)

**Files:**
- `/home/lewis/src/zjj/.tdd15-cache/zjj-jakw/research.json`
- `/home/lewis/src/zjj/.tdd15-cache/zjj-jakw/progress.json`
- Existing: `crates/zjj/src/commands/status.rs`

---

### 3. zjj-ioa3 - Wrap SyncOutput in SchemaEnvelope [SIMPLE - P0]

**Status:** IN_PROGRESS (Task)

**Summary:**
- Similar to zjj-jakw but for sync command
- Wrap SyncOutput in SchemaEnvelope for version tracking and schema evolution
- ~35 minutes effort
- **Key File:** crates/zjj/src/commands/sync.rs (sync_session_with_options, sync_all_with_options)
- **Secondary File:** crates/zjj/src/json_output.rs (SyncOutput struct)

**Edge Cases:**
- Rebase conflicts (wrap error in envelope)
- Already up-to-date status (success wrapped)
- Session not found (NotFound error wrapped)

**Phase Routing:** SIMPLE - Skip phases 1-3, start at phase 4 (RED tests)

**Files:**
- `/home/lewis/src/zjj/.tdd15-cache/zjj-ioa3/research.json`
- `/home/lewis/src/zjj/.tdd15-cache/zjj-ioa3/progress.json`
- Existing: `crates/zjj/src/commands/sync.rs`, `crates/zjj/src/json_output.rs`

---

### 4. zjj-05ut - SchemaEnvelope wrapper missing on most JSON outputs [MEDIUM - P0]

**Status:** IN_PROGRESS (Task)

**Summary:**
- Large refactoring: Audit ALL JSON outputs and wrap with SchemaEnvelope
- ~20-30 JSON output locations across 10+ command files
- **Affected Commands:** add, remove, focus, clean, doctor, init, config, query, dashboard, diff, introspect
- **Estimated Effort:** 2-3 hours (Medium complexity due to scale, not technical difficulty)

**Audit Approach:**
1. Search for all `serde_json::to_string()` calls in commands/
2. Map each to its output struct
3. Determine schema_type (single vs array)
4. Batch refactor by command module
5. Name convention: "{command}-response"

**Phase Routing:** MEDIUM - Phases 1-12, iterate per command module

**Files:**
- `/home/lewis/src/zjj/.tdd15-cache/zjj-05ut/research.json`
- `/home/lewis/src/zjj/.tdd15-cache/zjj-05ut/progress.json`
- Affected: 10+ command files in `crates/zjj/src/commands/`

---

### 5. zjj-1ppy - Rename --filter-by-bead to --bead [SIMPLE - P1]

**Status:** IN_PROGRESS (Task)

**IMPORTANT:** Investigation revealed the target state ALREADY EXISTS

**Findings:**
- The --bead flag ALREADY EXISTS in main.rs (lines 100-104)
- list.rs run() ALREADY accepts bead parameter (line 42)
- No --filter-by-bead flag found in current codebase
- **Implication:** This work appears already completed or bead is outdated

**Verification Needed:**
1. Check git history for when rename happened
2. Confirm --filter-by-bead doesn't exist in other branches
3. Verify tests use new flag name

**If Implementation Needed:**
- Add `.alias("filter-by-bead")` to Arg::new("bead") for backward compatibility
- ~15 minutes effort

**Phase Routing:** SIMPLE (if needed) - Skip phases 1-3

**Files:**
- `/home/lewis/src/zjj/.tdd15-cache/zjj-1ppy/research.json`
- `/home/lewis/src/zjj/.tdd15-cache/zjj-1ppy/progress.json`
- Existing: `crates/zjj/src/main.rs`, `crates/zjj/src/commands/list.rs`

---

## TDD15 Phase Routing Summary

| Bead | Title | Complexity | Phases | Est. Time | Notes |
|------|-------|-----------|--------|-----------|-------|
| zjj-gv3f | EPIC: State Tracking | COMPLEX | N/A | Very High | Epic orchestration |
| zjj-jakw | Wrap StatusOutput | SIMPLE | 4-6 | 30 min | Straightforward |
| zjj-ioa3 | Wrap SyncOutput | SIMPLE | 4-6 | 35 min | Similar to jakw |
| zjj-05ut | Audit All Outputs | MEDIUM | 1-12 | 2-3 hrs | Large scope, simple pattern |
| zjj-1ppy | Rename --bead Flag | SIMPLE | 4-6 | Already Done? | Needs verification |

## Key Architectural Insights

1. **SchemaEnvelope Integration:** Core pattern throughout P0 work. Type is stable and well-defined in json.rs.

2. **Dependency Chain:** 
   - zjj-gv3f (Epic) depends on zjj-fl0d and zjj-txqd
   - zjj-jakw and zjj-ioa3 are child tasks, ready to work immediately
   - zjj-05ut is large refactoring, independent of blocking work

3. **OutputFormat Integration:** All command files already use OutputFormat. Check with `format.is_json()` before wrapping.

4. **No Breaking Changes:** All wrapping work is additive (envelope wrapper layer). Existing APIs preserved.

## Next Steps for Ralph Loop

1. **Immediate:** Start phases 4-6 for zjj-jakw (simplest, highest priority P0)
2. **Parallel:** Start phases 4-6 for zjj-ioa3 (similar difficulty)
3. **Sequence:** Start phases 1-12 for zjj-05ut (medium effort, large scope)
4. **Verify:** Check status of zjj-1ppy - may already be complete
5. **Blocked:** Monitor zjj-gv3f for zjj-fl0d, zjj-txqd completion

## Cache Files Created

All research documents and progress files are stored in:
```
.tdd15-cache/
├── zjj-gv3f/
│   ├── research.json
│   └── progress.json
├── zjj-jakw/
│   ├── research.json
│   └── progress.json
├── zjj-ioa3/
│   ├── research.json
│   └── progress.json
├── zjj-05ut/
│   ├── research.json
│   └── progress.json
├── zjj-1ppy/
│   ├── research.json
│   └── progress.json
└── RESEARCH_SUMMARY.md (this file)
```

---

**Research Completed By:** Claude Code Agent
**Ready for:** TDD15 Phase Execution (Phases 4-12)
