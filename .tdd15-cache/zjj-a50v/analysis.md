# Bead Analysis: zjj-a50v - Standardize filter flag naming

**Date**: 2026-01-18
**Analyzed**: Using Codanna semantic search
**Status**: READY_FOR_IMPLEMENTATION

## Executive Summary

Bead **zjj-a50v** requires standardizing CLI filter flag naming. Currently, the `zjj list` command uses an inconsistent pattern:
- **Value filters** use: `--filter-by-{field}` (✓ consistent)
- **Presence filters** use: `--with-{field}` (✗ inconsistent)

This analysis identifies **2 flags needing rename**, affecting **2 files**, with **LOW implementation complexity** but **HIGH breaking change impact**.

---

## Current State

### All Filter Flags (4 total)

| Flag | Type | Status | Location |
|------|------|--------|----------|
| `--filter-by-bead <ID>` | Value-arg | ✓ Correct | args.rs:396-397 |
| `--filter-by-agent <ID>` | Value-arg | ✓ Correct | args.rs:402-403 |
| `--with-beads` | Boolean | ✗ **Needs rename** | args.rs:408-409 |
| `--with-agents` | Boolean | ✗ **Needs rename** | args.rs:414-415 |

### Problem: Naming Inconsistency

```bash
# Current (INCONSISTENT)
zjj list --filter-by-bead zjj-123      # value filter: --filter-by-*
zjj list --filter-by-agent agent-456   # value filter: --filter-by-*
zjj list --with-beads                  # presence filter: --with-*
zjj list --with-agents                 # presence filter: --with-*

# Proposed (CONSISTENT)
zjj list --filter-by-bead zjj-123      # value filter: --filter-by-*
zjj list --filter-by-agent agent-456   # value filter: --filter-by-*
zjj list --filter-by-has-beads         # presence filter: --filter-by-*
zjj list --filter-by-has-agents        # presence filter: --filter-by-*
```

---

## Affected Code

### File 1: `crates/zjj/src/cli/args.rs` (HIGH IMPACT)

**Lines affected**: 352-355, 396-418, 428-434, 446

**Changes needed**:
1. Line 396: `Arg::new("with-beads")` → `Arg::new("filter-by-has-beads")`
2. Line 408: `Arg::new("with-agents")` → `Arg::new("filter-by-has-agents")`
3. Lines 409 & 415: Update `.long()` calls
4. Lines 352-355: Update help text
5. Lines 428-434: Update examples
6. Line 446: Update documentation

**Example change**:
```rust
// Before
.arg(
    Arg::new("with-beads")
        .long("with-beads")
        .action(clap::ArgAction::SetTrue)
        .help("Show only sessions that have beads attached"),
)

// After
.arg(
    Arg::new("filter-by-has-beads")
        .long("filter-by-has-beads")
        .action(clap::ArgAction::SetTrue)
        .help("Show only sessions that have beads attached"),
)
```

### File 2: `crates/zjj/src/commands/routers/session.rs` (HIGH IMPACT)

**Lines affected**: 63-66 (handle_list_cmd function)

**Changes needed**:
1. Line 65: `sub_m.get_flag("with-beads")` → `sub_m.get_flag("filter-by-has-beads")`
2. Line 66: `sub_m.get_flag("with-agents")` → `sub_m.get_flag("filter-by-has-agents")`

**Example change**:
```rust
// Before
let filter = list::ListFilter {
    bead_id: sub_m.get_one::<String>("filter-by-bead").cloned(),
    agent_id: sub_m.get_one::<String>("filter-by-agent").cloned(),
    with_beads: sub_m.get_flag("with-beads"),
    with_agents: sub_m.get_flag("with-agents"),
};

// After
let filter = list::ListFilter {
    bead_id: sub_m.get_one::<String>("filter-by-bead").cloned(),
    agent_id: sub_m.get_one::<String>("filter-by-agent").cloned(),
    with_beads: sub_m.get_flag("filter-by-has-beads"),
    with_agents: sub_m.get_flag("filter-by-has-agents"),
};
```

### File 3: `crates/zjj/src/commands/list/data/types.rs` (LOW IMPACT)

**Impact**: NONE - Internal struct fields can remain unchanged.

The `ListFilter` struct fields (`with_beads`, `with_agents`) are internal and don't need renaming. Only the CLI flag names change.

---

## Implementation Breakdown

### Step 1: Update CLI Argument Definitions (args.rs)
- Rename two `Arg::new()` calls
- Update two `.long()` calls
- Update help text and documentation
- **Estimated time**: 20 minutes

### Step 2: Update Flag Parsing (session.rs)
- Update two `sub_m.get_flag()` calls
- **Estimated time**: 10 minutes

### Step 3: Add Tests
- Create test cases for flag parsing
- Test with new flag names
- **Estimated time**: 30 minutes

### Step 4: Run Tests & Validation
- `moon run :test` - Run all tests
- `moon run :ci` - Full CI pipeline
- Verify no regressions
- **Estimated time**: 20 minutes

### Step 5: Update Documentation
- Update CHANGELOG.md with breaking change notice
- **Estimated time**: 15 minutes

**Total Estimated Effort**: ~2-3 hours

---

## Breaking Change Assessment

### YES - This is a BREAKING CHANGE

#### Who is affected:
- Users with shell scripts using `--with-beads` or `--with-agents`
- Documentation/wikis referencing old flag names
- Users with shell aliases or functions

#### Severity: HIGH

Old scripts will fail with error: `error: unexpected argument '--with-beads'`

#### Migration Strategy:

1. **Option A (Hard Break)**: Remove old flags entirely
   - Clean, simple
   - Requires major version bump (0.x → 1.0)
   - Users must update scripts immediately

2. **Option B (Soft Deprecation)**: Support both names with warning
   - More user-friendly
   - Can be done in current version
   - Remove old names in next major version
   - Shows warning when old flags used

**Recommendation**: Option A for consistency, with clear CHANGELOG notes

---

## Risk Analysis

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|-----------|
| Breaking user scripts | HIGH | HIGH | Clear CHANGELOG, version bump, migration guide |
| Documentation outdated | MEDIUM | MEDIUM | Update examples in args.rs help text |
| Missing test coverage | LOW | MEDIUM | Add new tests for flag parsing |
| Incomplete rename | MEDIUM | LOW | Use Codanna to verify all instances found |

---

## Test Plan

### New Tests Required:

```rust
#[test]
fn test_filter_by_has_beads_flag() { /* ... */ }

#[test]
fn test_filter_by_has_agents_flag() { /* ... */ }

#[test]
fn test_multiple_filters_combined() { /* ... */ }
```

### Existing Tests:
- `crates/zjj/tests/p0_standardization_suite.rs` - May need updates
- Filter functionality tests - Already exist in list command tests

### Verification Checklist:
- [ ] `moon run :fmt-fix` passes
- [ ] `moon run :check` passes
- [ ] `moon run :test` passes (all tests)
- [ ] `moon run :ci` passes
- [ ] Manual test: `zjj list --filter-by-has-beads` works
- [ ] Manual test: `zjj list --filter-by-has-agents` works
- [ ] Help text shows new flags: `zjj list --help`

---

## Files Needing Changes

```
crates/zjj/src/cli/args.rs                    (HIGH: 30+ lines affected)
crates/zjj/src/commands/routers/session.rs    (HIGH: 2 lines affected)
crates/zjj/tests/p0_standardization_suite.rs  (LOW: Add ~20 lines for new tests)
CHANGELOG.md                                   (LOW: Add breaking change note)
```

---

## Complexity Assessment

**Complexity Level: LOW**

- Only 2 flags need renaming
- Changes are straightforward string replacements
- No logic changes needed
- Clear, linear implementation path
- Well-defined scope

**Confidence: HIGH** (95%)

Codanna analysis found all instances. No hidden dependencies or complex interactions.

---

## Standardization Rationale

### Why `--filter-by-has-*`?

1. **Consistency**: All filters follow `--filter-by-*` pattern
2. **Clarity**: `has-beads` clearly indicates "presence of beads"
3. **Discoverability**: Tab completion and `--help` show all filters together
4. **Scalability**: Easier to add new filters following same pattern

### Alternative Considered

- `--filter-has-beads` (shorter, but breaks `--filter-by-*` pattern)
- `--with-has-beads` (confusing double "with"/"has")
- `--filter-beads` (ambiguous: filter or include beads?)

**Winner**: `--filter-by-has-beads` ✓

---

## Recommendation

**PROCEED WITH IMPLEMENTATION** - This is a well-scoped, straightforward refactoring that improves CLI consistency. The breaking change is manageable with proper documentation and version communication.

**Priority**: P0 - Standardization/CLI
**Effort**: ~2-3 hours
**Risk**: Low technical, medium user-facing
**Impact**: High (improves long-term maintainability and user experience)

---

## Next Steps

1. Create feature branch for renaming
2. Update both Rust files
3. Add tests
4. Run full test suite
5. Update CHANGELOG with breaking change note
6. Create PR with clear description of renaming rationale
