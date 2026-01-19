# Bead zjj-jwwd Analysis: Normalize Dry-Run Output Structures

## Executive Summary

Three independent dry-run implementations exist across `add`, `sync`, and `remove` commands with inconsistent field naming, structure patterns, and operation definitions. This analysis identifies the specific inconsistencies and provides a roadmap for normalization.

**Complexity Classification**: MODERATE (4 hours estimated)
**Risk Level**: LOW (contained changes, good test coverage)
**Breaking Changes**: YES (output structure changes)

---

## Current State

### 1. Add Command Dry-Run (`crates/zjj/src/commands/add/dry_run.rs`)

**Output Structure**:
```rust
pub struct AddDryRunOutput {
    pub success: bool,
    pub dry_run: bool,
    pub plan: AddDryRunPlan,
}

pub struct AddDryRunPlan {
    pub session_name: String,
    pub workspace_path: String,
    pub branch: String,
    pub layout_template: String,
    pub zellij_tab_name: String,
    pub will_open_zellij: bool,
    pub will_run_hooks: bool,
    pub operations: im::Vector<PlannedOperation>,
}

pub struct PlannedOperation {
    pub action: String,
    pub target: String,
    pub details: Option<String>,
}
```

**Characteristics**:
- Uses `im::Vector` for operations
- Operations have 3 fields: action, target, details
- No explicit ordering (implicit Vec ordering)
- Stores template name in `layout_template`
- Uses `will_*` prefix for flags

**Location in codebase**: Defined in `commands/add/dry_run.rs` (non-standard)

---

### 2. Sync Command Dry-Run (`crates/zjj/src/commands/sync/dry_run.rs`)

**Output Structure**:
```rust
pub struct SyncDryRunOutput<'a> {
    pub success: bool,
    pub dry_run: bool,
    pub plan: &'a SyncDryRunPlan,
}

pub struct SyncDryRunPlan {
    pub session_name: Option<String>,  // None for all-sessions
    pub sessions_to_sync: Vec<SyncSessionPlan>,
    pub target_branch: String,
    pub target_branch_source: String,
    pub total_count: usize,
    pub operations_per_session: Vec<String>,
}

pub struct SyncSessionPlan {
    pub name: String,
    pub workspace_path: String,
    pub workspace_exists: bool,
    pub status: String,
    pub can_sync: bool,
    pub skip_reason: Option<String>,
}
```

**Characteristics**:
- Returns borrowed reference (`&'a SyncDryRunPlan`)
- Session name is `Option<String>` (distinguishes single vs all)
- Per-session details in separate struct
- Operations are string descriptions, not structured
- Stores target branch detection source for transparency
- No explicit operation ordering

**Location in codebase**: Defined in `json_output.rs` (standard)

---

### 3. Remove Command Dry-Run (`crates/zjj/src/commands/remove/dry_run.rs`)

**Output Structure** (in `json_output.rs`):
```rust
pub struct RemoveDryRunOutput<'a> {
    pub success: bool,
    pub dry_run: bool,
    pub plan: &'a RemoveDryRunPlan,
}

pub struct RemoveDryRunPlan {
    pub session_name: String,
    pub session_id: i64,
    pub workspace_path: String,
    pub workspace_exists: bool,
    pub zellij_tab: String,
    pub inside_zellij: bool,
    pub would_run_hooks: bool,
    pub would_merge: bool,
    pub planned_operations: Vec<PlannedRemoveOperation>,
    pub warnings: Option<Vec<String>>,
}

pub struct PlannedRemoveOperation {
    pub order: u32,
    pub action: String,
    pub description: String,
    pub target: Option<String>,
    pub reversible: bool,
}
```

**Characteristics**:
- Returns borrowed reference (`&'a RemoveDryRunPlan`)
- Has most comprehensive operation structure
- Operations have 5 fields: order, action, description, target, reversible
- Explicitly tracks operation ordering
- Includes warnings field for context
- Uses `would_*` prefix for flags
- Includes `inside_zellij` flag

**Location in codebase**: Defined in `json_output.rs` (standard)

---

## Inconsistencies Identified

### Critical Issues

#### 1. **Module Organization Inconsistency** (Non-Breaking but Poor Practice)
- `AddDryRunOutput` defined in `commands/add/dry_run.rs`
- `SyncDryRunOutput` and `RemoveDryRunOutput` defined in `json_output.rs`
- Violates "all JSON output types in one place" principle

**Impact**: Harder to locate, maintain, and discover output structure

---

#### 2. **Operation Structure Divergence** (Breaking)
Most critical inconsistency. Three different structures:

| Field | Add | Sync | Remove |
|-------|-----|------|--------|
| `action` | Yes | String desc | Yes |
| `target` | Yes | N/A | Yes |
| `details` | Option<String> | N/A | N/A |
| `description` | No | No | Yes |
| `order` | Implicit | Implicit | Yes |
| `reversible` | No | No | Yes |

**Examples from code**:

Add (line 44-48):
```rust
pub struct PlannedOperation {
    pub action: String,
    pub target: String,
    pub details: Option<String>,
}
```

Remove (line 86-94):
```rust
pub struct PlannedRemoveOperation {
    pub order: u32,
    pub action: String,
    pub description: String,
    pub target: Option<String>,
    pub reversible: bool,
}
```

Sync (lines 224-255): No structured operations, just string descriptions:
```rust
pub operations_per_session: Vec<String>,
```

**Impact**:
- Clients can't write generic operation handler
- JSON consumers face unpredictable structure
- Harder to add common features (e.g., operation documentation)

---

#### 3. **Session Context Handling** (Breaking)
- **Add**: Always single session (`session_name: String`)
- **Remove**: Always single session (`session_name: String`)
- **Sync**: Can be single or all (`session_name: Option<String>`)

**Problem**: Sync's Option makes it ambiguous. None could mean "all sessions" or "unknown".

**From code** (sync/dry_run.rs lines 44-46):
```rust
Ok(SyncDryRunPlan {
    session_name: Some(name.to_string()),  // Single
    ...
})
```

And (lines 123-125):
```rust
Ok(SyncDryRunPlan {
    session_name: None,  // All sessions
    ...
})
```

**Impact**: Type doesn't distinguish intent; requires documentation

---

#### 4. **Flag Tracking Inconsistency** (Breaking)

Add tracks (lines 37-38):
```rust
pub will_open_zellij: bool,
pub will_run_hooks: bool,
```

Remove tracks (lines 78-81):
```rust
pub inside_zellij: bool,
pub would_run_hooks: bool,
pub would_merge: bool,
```

Sync tracks:
```rust
// Nothing! Just target_branch_source
```

**Problem**: Different prefix conventions (`will_*` vs `would_*`)
**Problem**: Remove includes state context (`inside_zellij`) but Add/Sync don't

---

#### 5. **Referential Borrowing Inconsistency** (Breaking)
- Add: Returns owned `AddDryRunOutput` with owned `AddDryRunPlan`
- Sync: Returns `SyncDryRunOutput<'a>` with borrowed `&'a SyncDryRunPlan`
- Remove: Returns `RemoveDryRunOutput<'a>` with borrowed `&'a RemoveDryRunPlan`

**Impact**: Different lifetime requirements complicate trait definitions

---

#### 6. **Multi-Session Support** (Breaking)
- Add: Single session only
- Remove: Single session only
- Sync: Both single and all sessions with per-session details (`SyncSessionPlan` array)

**Problem**: Only Sync can easily extend to batch; Add/Remove would need restructuring

---

## Test Coverage Implications

Test file: `crates/zjj/tests/test_session_name_field.rs`

Critical test at lines 161-190:
```rust
#[test]
fn test_remove_dry_run_output_has_session_name() {
    // ...
    let plan = &json["plan"];
    assert!(
        plan.get("session_name").is_some(),
        "Dry-run plan must have 'session_name' field"
    );
    assert_eq!(
        plan["session_name"], "test-dry-run",
        "Plan session_name must match"
    );
}
```

This test validates that dry-run plans have `session_name` field. Any normalization must maintain this contract.

---

## Dependency Analysis

### Internal Dependencies
- `serde::Serialize` - All structures use it
- `im::Vector` - Only AddDryRunPlan uses it (could switch to Vec)
- `json_output.rs` - Central location for output types

### No External Breaking Changes
- These types are internal JSON output only
- No public crate API depends on them
- Only breaking within CLI command output contracts

---

## Complexity Breakdown

### Phase 1: Design (0.5h)
- Decide on unified operation struct
- Decide on single/multi session representation
- Document field naming conventions

### Phase 2: Implementation (2h)
- Create `UnifiedDryRunOperation` (or similar)
- Migrate Add operations to new structure
- Update Sync to use structured operations
- Remove already uses similar structure

### Phase 3: Consolidation (0.75h)
- Move `AddDryRunOutput` to `json_output.rs`
- Consolidate all `*DryRunOutput` types
- Update imports in command modules

### Phase 4: Testing & Validation (0.75h)
- Update all dry-run tests
- Validate JSON output shapes
- Ensure test_session_name_field passes
- Add tests for new normalized structure

**Total**: ~4 hours

---

## Recommendations (Priority Order)

### P1 (MUST FIX - Blocks Normalization)

**1. Create Unified Operation Structure**
```rust
#[derive(Debug, Clone, Serialize)]
pub struct DryRunOperation {
    pub order: u32,
    pub action: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub reversible: bool,  // Can be set to true if uncertain
}
```

Benefits:
- Consolidates all operation fields
- Serves all three commands
- Supports future extensions

**2. Consolidate All Types in json_output.rs**
- Move `AddDryRunOutput` and `AddDryRunPlan` from `commands/add/dry_run.rs`
- Keep implementation logic in respective command modules
- Output types are serialization concerns, not logic concerns

### P2 (SHOULD FIX - Improves Consistency)

**3. Standardize Flag Naming**
Adopt consistent naming:
```rust
pub would_run_hooks: bool,      // Consistent "would_*"
pub would_merge: bool,
pub would_open_zellij: bool,
pub inside_zellij: bool,        // State, not action
```

**4. Ensure Explicit Operation Ordering**
- All operation vecs must have `order: u32` field
- Add/Sync currently rely on vec ordering (fragile)
- Makes order intention explicit

### P3 (NICE TO HAVE - Future-Proofing)

**5. Type-Safe Single/Multi Session**
Instead of `Option<String>`:
```rust
#[derive(Debug, Serialize)]
pub enum SyncScope {
    #[serde(rename = "single")]
    Single(String),
    #[serde(rename = "all")]
    All,
}

pub struct SyncDryRunPlan {
    pub scope: SyncScope,
    // ...
}
```

Makes intent type-safe.

---

## Validation Checklist

Before merging normalization:

- [ ] All `*DryRunOutput` types defined in `json_output.rs`
- [ ] All `*DryRunPlan` types defined in `json_output.rs`
- [ ] All operations use unified `DryRunOperation` struct
- [ ] Operation ordering explicit in all outputs
- [ ] Flag names consistent (`would_*` convention)
- [ ] Add/Sync/Remove produce identical JSON structure for operations
- [ ] All existing tests pass
- [ ] `test_remove_dry_run_output_has_session_name` passes
- [ ] New test for unified structure added
- [ ] No manual JSON serialization workarounds

---

## Related Beads

- **zjj-gyr**: Add dry-run (implements AddDryRunPlan)
- **zjj-g80p**: Help JSON output (related output normalization)
- **zjj-xi2j**: Batch operations (may share normalized structure)

---

## Code Locations Quick Reference

| Concern | File | Lines |
|---------|------|-------|
| Add plan definition | `commands/add/dry_run.rs` | 20-48 |
| Add implementation | `commands/add/dry_run.rs` | 63-99 |
| Sync plan definition | `json_output.rs` | 133-161 |
| Sync implementation | `commands/sync/dry_run.rs` | 22-131 |
| Remove plan definition | `json_output.rs` | 68-95 |
| Remove implementation | `commands/remove/dry_run.rs` | 20-115 |
| Test validation | `tests/test_session_name_field.rs` | 161-190 |
