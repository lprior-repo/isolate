# Dry-Run Output Structure Comparison

## Visual Overview

### Command: ADD
```
AddDryRunOutput
├── success: bool
├── dry_run: bool
└── plan: AddDryRunPlan
    ├── session_name: String
    ├── workspace_path: String
    ├── branch: String
    ├── layout_template: String
    ├── zellij_tab_name: String
    ├── will_open_zellij: bool
    ├── will_run_hooks: bool
    └── operations: im::Vector<PlannedOperation>
        └── (per operation)
            ├── action: String
            ├── target: String
            └── details: Option<String>
```

**File**: `crates/zjj/src/commands/add/dry_run.rs`
**Wrapper Location**: `commands/add/dry_run.rs` (INCONSISTENT)

---

### Command: SYNC
```
SyncDryRunOutput<'a>
├── success: bool
├── dry_run: bool
└── plan: &'a SyncDryRunPlan
    ├── session_name: Option<String>      // Some(name) or None (all)
    ├── sessions_to_sync: Vec<SyncSessionPlan>
    │   └── (per session)
    │       ├── name: String
    │       ├── workspace_path: String
    │       ├── workspace_exists: bool
    │       ├── status: String
    │       ├── can_sync: bool
    │       └── skip_reason: Option<String>
    ├── target_branch: String
    ├── target_branch_source: String
    ├── total_count: usize
    └── operations_per_session: Vec<String>    // Just strings!
```

**File**: `crates/zjj/src/commands/sync/dry_run.rs`
**Wrapper Location**: `json_output.rs` (STANDARD)

---

### Command: REMOVE
```
RemoveDryRunOutput<'a>
├── success: bool
├── dry_run: bool
└── plan: &'a RemoveDryRunPlan
    ├── session_name: String
    ├── session_id: i64
    ├── workspace_path: String
    ├── workspace_exists: bool
    ├── zellij_tab: String
    ├── inside_zellij: bool
    ├── would_run_hooks: bool
    ├── would_merge: bool
    ├── planned_operations: Vec<PlannedRemoveOperation>
    │   └── (per operation)
    │       ├── order: u32
    │       ├── action: String
    │       ├── description: String
    │       ├── target: Option<String>
    │       └── reversible: bool
    └── warnings: Option<Vec<String>>
```

**File**: `crates/zjj/src/commands/remove/dry_run.rs`
**Wrapper Location**: `json_output.rs` (STANDARD)

---

## Field Comparison Matrix

### Session Context
| Field | Add | Sync | Remove |
|-------|-----|------|--------|
| `session_name` | String | Option<String> | String |
| `session_id` | ❌ | ❌ | i64 |
| `workspace_path` | ✓ | ✓ | ✓ |
| `workspace_exists` | ❌ | ✓ | ✓ |
| `zellij_tab` | As `zellij_tab_name` | ❌ | ✓ |

### Execution Context
| Field | Add | Sync | Remove |
|-------|-----|------|--------|
| `will_run_hooks` | ✓ | ❌ | As `would_run_hooks` |
| `will_open_zellij` | ✓ | ❌ | ❌ |
| `would_merge` | ❌ | ❌ | ✓ |
| `inside_zellij` | ❌ | ❌ | ✓ |
| `target_branch` | ❌ | ✓ | ❌ |
| `target_branch_source` | ❌ | ✓ | ❌ |

### Operations
| Field | Add | Sync | Remove |
|-------|-----|------|--------|
| Structure type | `PlannedOperation` | `Vec<String>` | `PlannedRemoveOperation` |
| Explicit ordering | ❌ (implicit) | ❌ (implicit) | ✓ |
| `action` field | ✓ | String desc | ✓ |
| `description` field | ❌ | Entire string | ✓ |
| `target` field | ✓ | N/A | ✓ |
| `details` field | ✓ | N/A | ❌ |
| `reversible` field | ❌ | N/A | ✓ |

---

## Inconsistency Severity Ratings

### CRITICAL (Breaking Fixes Required)

1. **Operation Structure Divergence**
   - Severity: CRITICAL
   - Impact: Can't build generic operation handlers
   - Fix: Create `DryRunOperation` struct usable by all

2. **Sync Operations as Strings**
   - Severity: CRITICAL
   - Impact: No machine-readable operation details
   - Fix: Convert to structured operations

### HIGH (Should Fix)

3. **Module Organization**
   - Severity: HIGH
   - Impact: Types scattered across modules
   - Fix: All *DryRunOutput to `json_output.rs`

4. **Session Context Ambiguity**
   - Severity: HIGH
   - Impact: Option<String> intent unclear
   - Fix: Use enum or clearer naming

### MEDIUM (Nice to Fix)

5. **Flag Naming Inconsistency**
   - Severity: MEDIUM
   - Impact: Cognitive load, inconsistent contracts
   - Fix: Standardize to `would_*` prefix

6. **Borrowing Inconsistency**
   - Severity: MEDIUM
   - Impact: Can't abstract with traits
   - Fix: Choose one pattern (suggest: all owned)

7. **Operation Ordering Ambiguity**
   - Severity: MEDIUM
   - Impact: Fragile implicit ordering
   - Fix: All operations have explicit `order: u32`

---

## Proposed Unified Structure

```rust
/// Unified dry-run output wrapper for all commands
#[derive(Debug, Serialize)]
pub struct DryRunOutput<P> {
    pub success: bool,
    pub dry_run: bool,
    pub plan: P,
}

/// Unified operation type used by all commands
#[derive(Debug, Clone, Serialize)]
pub struct DryRunOperation {
    pub order: u32,
    pub action: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub reversible: bool,
}

/// Add command dry-run plan
#[derive(Debug, Serialize)]
pub struct AddDryRunPlan {
    pub session_name: String,
    pub workspace_path: String,
    pub branch: String,
    pub layout_template: String,
    pub zellij_tab_name: String,
    pub would_open_zellij: bool,      // Normalized to "would_*"
    pub would_run_hooks: bool,        // Normalized to "would_*"
    pub operations: Vec<DryRunOperation>,
}

/// Sync command dry-run plan
#[derive(Debug, Serialize)]
pub struct SyncDryRunPlan {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    pub sessions_to_sync: Vec<SyncSessionPlan>,
    pub target_branch: String,
    pub target_branch_source: String,
    pub total_count: usize,
    pub operations_per_session: Vec<DryRunOperation>,  // Now structured!
}

/// Remove command dry-run plan
#[derive(Debug, Serialize)]
pub struct RemoveDryRunPlan {
    pub session_name: String,
    pub session_id: i64,
    pub workspace_path: String,
    pub workspace_exists: bool,
    pub zellij_tab: String,
    pub inside_zellij: bool,
    pub would_run_hooks: bool,
    pub would_merge: bool,
    pub planned_operations: Vec<DryRunOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}
```

Benefits:
- ✓ All operations have consistent structure
- ✓ All commands use same `DryRunOperation`
- ✓ Explicit ordering via `order: u32`
- ✓ Standardized flag naming (`would_*`)
- ✓ All types in `json_output.rs`
- ✓ Generic wrapper pattern enables future commands
- ✓ No behavioral changes, only structure

---

## Migration Path

### Stage 1: Definition (json_output.rs)
- Define `DryRunOperation` struct
- Move `AddDryRunOutput` to json_output.rs
- Keep existing types for backward compat if needed

### Stage 2: Add Command
- Update `PlannedOperation` → `DryRunOperation`
- Convert `im::Vector` → `Vec`
- Rename flags: `will_*` → `would_*`
- Add `order` field to operations

### Stage 3: Sync Command
- Convert string operations to `DryRunOperation`
- Add explicit ordering to operations
- Add missing context fields if needed

### Stage 4: Remove Command
- Rename `PlannedRemoveOperation` → `DryRunOperation`
- Update imports everywhere

### Stage 5: Testing
- Run existing tests (should pass)
- Add tests for new structure
- Verify JSON serialization

### Stage 6: Documentation
- Update examples
- Update any API docs

---

## JSON Output Examples

### Current Add Output
```json
{
  "success": true,
  "dry_run": true,
  "plan": {
    "session_name": "feature-x",
    "operations": [
      {
        "action": "create_db",
        "target": ".zjj/state.db",
        "details": null
      }
    ]
  }
}
```

### Current Sync Output
```json
{
  "success": true,
  "dry_run": true,
  "plan": {
    "session_name": null,
    "operations_per_session": [
      "Rebase workspace onto main",
      "Update last_synced timestamp"
    ]
  }
}
```

### Normalized Output (All Commands)
```json
{
  "success": true,
  "dry_run": true,
  "plan": {
    "session_name": "feature-x",
    "operations": [
      {
        "order": 1,
        "action": "create_db",
        "description": "Create session database",
        "target": ".zjj/state.db",
        "details": null,
        "reversible": false
      }
    ]
  }
}
```

Structure is now consistent and machine-parseable!

---

## Testing Validation

Key tests to ensure pass:
- ✓ `test_remove_dry_run_output_has_session_name` - Already defined
- ✓ All operations have `action` and `description`
- ✓ All operations have `order` (incrementing from 1)
- ✓ JSON serialization round-trips
- ✓ No panic or error on edge cases

Example test:
```rust
#[test]
fn test_dry_run_operations_normalized() {
    let add_result = harness.zjj(&["add", "test", "--dry-run", "--json"]);
    let add_json: serde_json::Value = serde_json::from_str(&add_result.stdout).unwrap();

    // Verify structure
    let ops = &add_json["plan"]["operations"];
    assert!(ops.is_array());

    for (idx, op) in ops.as_array().unwrap().iter().enumerate() {
        assert!(op.get("order").is_some());
        assert!(op.get("action").is_some());
        assert!(op.get("description").is_some());
        assert_eq!(op["order"].as_u64().unwrap(), (idx + 1) as u64);
    }
}
```
