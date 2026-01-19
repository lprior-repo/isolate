# Bead Analysis: zjj-jwwd - Normalize Dry-Run Output Structures

**Analysis Date**: 2026-01-18  
**Status**: COMPLETE - Ready for Implementation  
**Complexity**: MODERATE (4 hours)  
**Risk Level**: LOW

## Quick Summary

Three independent dry-run implementations exist across `add`, `sync`, and `remove` commands with **5 major inconsistencies**:

1. **Operation structures differ** (Add: 3 fields, Sync: strings, Remove: 5 fields)
2. **Module organization scattered** (Add types in commands/, Sync+Remove in json_output/)
3. **Session context ambiguous** (Sync uses Option<String>, others use String)
4. **Flag naming inconsistent** (will_* vs would_*)
5. **Borrowing patterns differ** (Add owns, Sync+Remove borrow)

## Files in This Analysis

| File | Purpose |
|------|---------|
| `triage.json` | Structured data for tooling/automation |
| `analysis.md` | Detailed technical analysis (439 lines) |
| `structure_comparison.md` | Visual field-by-field comparison |
| `FINDINGS.txt` | Executive summary for humans |
| `README.md` | This file |

## Critical Issues (Priority P1)

### Issue 1: Operation Structure Divergence

**Add command** uses:
```rust
struct PlannedOperation {
  action: String,
  target: String,
  details: Option<String>,
}
```

**Sync command** uses:
```rust
Vec<String>  // Just text descriptions!
```

**Remove command** uses:
```rust
struct PlannedRemoveOperation {
  order: u32,
  action: String,
  description: String,
  target: Option<String>,
  reversible: bool,
}
```

**Impact**: Can't write generic operation handler. Sync operations aren't machine-readable.

### Issue 2: Module Organization

- `AddDryRunOutput` → `commands/add/dry_run.rs` ❌
- `SyncDryRunOutput` → `json_output.rs` ✓
- `RemoveDryRunOutput` → `json_output.rs` ✓

**Fix**: Move all to `json_output.rs` for single source of truth.

## Solution

Create unified operation struct used by all three commands:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DryRunOperation {
    pub order: u32,                      // Explicit ordering
    pub action: String,                  // e.g., "create_workspace"
    pub description: String,             // e.g., "Create JJ workspace"
    pub target: Option<String>,          // e.g., "/path/to/workspace"
    pub details: Option<String>,         // Additional context
    pub reversible: bool,                // Can be undone?
}
```

This single struct replaces:
- `PlannedOperation` (Add)
- `Vec<String>` (Sync)
- `PlannedRemoveOperation` (Remove)

**Benefits**:
- ✓ Consistent JSON output across commands
- ✓ Machine-readable operations everywhere
- ✓ Generic tooling possible
- ✓ Explicit ordering eliminates fragility
- ✓ Future-proof (easy to extend)

## Implementation Plan

### Phase 1: Consolidation (2h)
- [ ] Define `DryRunOperation` in `json_output.rs`
- [ ] Move `AddDryRunOutput` to `json_output.rs`
- [ ] Update all *DryRunPlan to use `Vec<DryRunOperation>`

### Phase 2: Migration (1.5h)
- [ ] Update Add: `PlannedOperation` → `DryRunOperation`
- [ ] Update Sync: `Vec<String>` → `Vec<DryRunOperation>`
- [ ] Update Remove: `PlannedRemoveOperation` → `DryRunOperation`
- [ ] Standardize flag naming to `would_*` prefix

### Phase 3: Testing (0.5h)
- [ ] Run existing tests (should still pass)
- [ ] Add tests for unified structure
- [ ] Verify JSON schemas consistent

## Validation Checklist

Before merging, ensure:

- [ ] `triage.json` aligns with implementation plan
- [ ] All operations have `order: u32` field
- [ ] All commands use `DryRunOperation` struct
- [ ] `test_remove_dry_run_output_has_session_name` passes
- [ ] All existing dry-run tests pass
- [ ] New tests added for normalized structure
- [ ] JSON output is valid and consistent
- [ ] No panics or unwraps in operation handling

## Code Locations

**Current implementations**:
- Add: `/home/lewis/src/zjj/crates/zjj/src/commands/add/dry_run.rs` (lines 20-48)
- Sync: `/home/lewis/src/zjj/crates/zjj/src/commands/sync/dry_run.rs` (lines 22-131)
- Remove: `/home/lewis/src/zjj/crates/zjj/src/commands/remove/dry_run.rs` (lines 20-115)
- Shared: `/home/lewis/src/zjj/crates/zjj/src/json_output.rs` (lines 60-161)

**Tests**:
- Main: `/home/lewis/src/zjj/crates/zjj/tests/test_session_name_field.rs` (lines 161-190)

## Related Issues

- **zjj-gyr**: Add dry-run implementation
- **zjj-g80p**: Help JSON output (related normalization)
- **zjj-xi2j**: Batch operations (may share structure)

## Effort Estimate

| Phase | Hours | Notes |
|-------|-------|-------|
| Design | 0.5 | Review and plan (done) |
| Implementation | 2.0 | Code changes across 3 commands |
| Testing | 1.0 | Run suite, write new tests |
| Documentation | 0.5 | Update examples/docs |
| **Total** | **~4.0** | Low risk, contained scope |

## Breaking Changes

✓ YES - Output structure changes (this is intended)

The changes are limited to JSON output format, not API or CLI behavior. Version bump recommended.

## How to Use This Analysis

1. **For implementation**: Use `triage.json` as structured requirements
2. **For design review**: Read `structure_comparison.md` for visual overview
3. **For stakeholders**: Share `FINDINGS.txt` for quick overview
4. **For detailed planning**: Read `analysis.md` (comprehensive)

## Questions?

Refer to the detailed analysis files or code locations above. All inconsistencies are documented in `analysis.md` with specific line numbers.

---

**Analysis Tools Used**: Codanna (semantic search), ripgrep, manual code review  
**Confidence Level**: HIGH (all inconsistencies verified with line numbers)
