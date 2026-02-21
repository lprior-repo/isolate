# Scout Notes

## bd-1i1a: Add parent_workspace column (COMPLETE)

## Bead Requirements
- Add `parent_workspace` column to `merge_queue` table
- Column is nullable (Option<String>)
- No validation at DB level (validation at service layer)
- Used to track parent workspace reference for stacked PRs

## Existing Patterns to Follow

### 1. QueueEntry struct (queue_entities.rs:19-47)
```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QueueEntry {
    pub id: i64,
    pub workspace: String,
    // ... other fields
    #[sqlx(default, try_from = "String")]
    pub workspace_state: WorkspaceQueueState,
    // ... 
}
```

**Pattern for nullable columns:**
- Use `Option<T>` type
- Add `#[sqlx(default)]` for backward compatibility with existing rows

### 2. Database Schema (05_queue_tables.sql:27-85)
- Column definitions use `TEXT` for strings
- Nullable columns omit `NOT NULL`
- Self-references use `FOREIGN KEY` to same table

### 3. Similar nullable columns in QueueEntry
- `bead_id: Option<String>` - nullable string
- `agent_id: Option<String>` - nullable string
- `dedupe_key: Option<String>` - nullable string with UNIQUE
- `head_sha: Option<String>` - nullable string

## Implementation Plan

### 1. Add SQL column
Add to `sql_schemas/05_queue_tables.sql`:
```sql
-- Stack parent reference (for stacked PRs)
parent_workspace TEXT,
```

Add index for parent lookups:
```sql
-- Index for parent_workspace lookups (stack queries)
CREATE INDEX IF NOT EXISTS idx_merge_queue_parent_workspace ON merge_queue(parent_workspace);
```

### 2. Add Rust field
Add to `QueueEntry` struct:
```rust
/// Parent workspace for stacked PRs (None if not in a stack)
#[sqlx(default)]
pub parent_workspace: Option<String>,
```

**Key:** Use `#[sqlx(default)]` for backward compatibility

## Files to Modify
1. `sql_schemas/05_queue_tables.sql` - Add column + index
2. `crates/zjj-core/src/coordination/queue_entities.rs` - Add field

## No Changes Needed To
- `queue_status.rs` - No state machine changes
- Repository methods - This bead only adds the column, not methods
- Tests - Will be handled by ATDD test

---

## bd-3axf: Add stack_depth column

### Bead Requirements
- Add `stack_depth` column to `merge_queue` table
- Column is INTEGER with default 0
- Must be non-negative (CHECK constraint)
- Used to track depth in stack hierarchy

### Existing Patterns to Follow

1. **Default integer columns in QueueEntry:**
```rust
#[sqlx(default)]
pub attempt_count: i32,
```

2. **SQL CHECK constraint pattern (from status column):**
```sql
status TEXT NOT NULL DEFAULT 'pending'
    CHECK(status IN ('pending', ...))
```

### Implementation Plan

1. **SQL Schema (05_queue_tables.sql)** - Add after parent_workspace:
```sql
-- Stack depth (0 = root, 1 = first child, etc.)
stack_depth INTEGER NOT NULL DEFAULT 0 CHECK(stack_depth >= 0),
```

2. **Rust struct (queue_entities.rs)** - Add field:
```rust
/// Depth in stack hierarchy (0 = root, no parent)
#[sqlx(default)]
pub stack_depth: i32,
```

### Files to Modify
1. `sql_schemas/05_queue_tables.sql` - Add column
2. `crates/zjj-core/src/coordination/queue_entities.rs` - Add field

---

## bd-1idz: Add calculate_stack_depth function

### Bead Requirements
- Pure function that calculates depth from parent chain
- Traverses to root counting depth
- Returns error for cycles
- Returns 0 for no parent
- Function is pure, no panic paths

### Existing Patterns to Follow

1. **StackError enum (stack_error.rs):**
```rust
pub enum StackError {
    CycleDetected { workspace: String, cycle_path: Vec<String> },
    ParentNotFound { parent_workspace: String },
    DepthExceeded { current_depth: u32, max_depth: u32 },
    InvalidParent { workspace: String, reason: String },
}
```

2. **Pure function pattern:**
- Takes inputs, returns Result<T, E>
- No side effects
- Uses `?` operator for early returns
- Uses `map`/`and_then` for transformations

3. **QueueEntry has parent_workspace field:**
```rust
pub parent_workspace: Option<String>,
```

### Implementation Plan

1. **Create new module: `stack.rs`**
   - Location: `crates/zjj-core/src/coordination/stack.rs`
   - Contains pure stack operations

2. **Function signature:**
```rust
/// Calculate the depth of a workspace in a stack hierarchy.
///
/// # Arguments
/// * `workspace` - The workspace to calculate depth for
/// * `entries` - Slice of queue entries to search for parents
///
/// # Returns
/// * `Ok(u32)` - The depth (0 = root/no parent, 1 = first child, etc.)
/// * `Err(StackError::CycleDetected)` - If a cycle is found in the chain
pub fn calculate_stack_depth(
    workspace: &str,
    entries: &[QueueEntry],
) -> Result<u32, StackError>
```

3. **Algorithm:**
   - Start at given workspace
   - Look up parent in entries slice
   - If no parent, return 0
   - If parent found, recurse/increment and continue
   - Track visited workspaces to detect cycles
   - Return error if cycle detected

### Files to Create/Modify
1. `crates/zjj-core/src/coordination/stack.rs` - New module with function
2. `crates/zjj-core/src/coordination/mod.rs` - Export new module
