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

---

## bd-mmtr: Add find_stack_root function

### Bead Requirements
- Pure function that finds the root of a stack
- Traverses up the parent chain to find workspace with no parent
- Returns error for cycles (no infinite loop)
- If workspace has no parent, returns itself (it's the root)
- Function is pure, uses Result<T, E>

### Existing Patterns to Follow

1. **calculate_stack_depth function (stack_depth.rs:47-89):**
   - Same traversal pattern but counts depth
   - Uses HashSet to track visited for cycle detection
   - Returns appropriate StackError variants
   - Pure function signature: `fn(workspace: &str, entries: &[QueueEntry]) -> Result<T, StackError>`

2. **StackError enum has relevant variants:**
   - `CycleDetected` - for cycle detection
   - `ParentNotFound` - for missing parents

3. **QueueEntry has parent_workspace field:**
   ```rust
   pub parent_workspace: Option<String>,
   ```

### Implementation Plan

1. **Add function to stack_depth.rs module (or create new stack.rs):**
   - Function name: `find_stack_root`
   - Same parameters as calculate_stack_depth

2. **Function signature:**
   ```rust
   /// Find the root workspace of a stack.
   ///
   /// # Arguments
   /// * `workspace` - The workspace to find root for
   /// * `entries` - Slice of queue entries to search for parents
   ///
   /// # Returns
   /// * `Ok(String)` - The name of the root workspace
   /// * `Err(StackError::CycleDetected)` - If a cycle is found
   /// * `Err(StackError::ParentNotFound)` - If parent doesn't exist
   pub fn find_stack_root(
       workspace: &str,
       entries: &[QueueEntry],
   ) -> Result<String, StackError>
   ```

3. **Algorithm:**
   - Start at given workspace
   - If no parent, return workspace name (it's the root)
   - Track visited workspaces to detect cycles
   - Traverse parent chain until finding workspace with no parent
   - Return that workspace's name

### Files to Modify
1. `crates/zjj-core/src/coordination/stack_depth.rs` - Add function

### Test Cases Needed
1. Root workspace (no parent) returns itself
2. One-level child returns its parent (the root)
3. Multi-level chain returns the actual root
4. Self-referencing workspace returns cycle error
5. Cycle in chain returns cycle error
6. Missing parent returns parent not found error

---

## bd-2ljz: Add get_children method

### Bead Requirements
- Repository method to find direct children of a workspace
- Query entries where `parent_workspace == Some(workspace)`
- Returns `Result<Vec<QueueEntry>>`
- Empty Vec is valid if no children (not an error)
- Only returns direct children, not grandchildren

### Existing Patterns to Follow

1. **list() method pattern (queue.rs:716-743):**
```rust
pub async fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>> {
    let sql = match filter_status {
        Some(_) => "SELECT ... FROM merge_queue WHERE status = ?1 ORDER BY ..."
        None => "SELECT ... FROM merge_queue ORDER BY ..."
    };
    // Build query, bind params, fetch_all
}
```

2. **SQL SELECT column list (includes all stack fields):**
```sql
SELECT id, workspace, bead_id, priority, status, added_at, started_at,
       completed_at, error_message, agent_id, dedupe_key, workspace_state,
       previous_state, state_changed_at, head_sha, tested_against_sha, 
       attempt_count, max_attempts, rebase_count, last_rebase_at, 
       parent_workspace, stack_depth, dependents, stack_root, stack_merge_state
FROM merge_queue WHERE ...
```

3. **QueueRepository trait pattern:**
```rust
// In queue_repository.rs
async fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>>;

// In queue.rs impl QueueRepository for MergeQueue
async fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>> {
    self.list(filter_status).await
}
```

### Implementation Plan

1. **Add to QueueRepository trait (queue_repository.rs):**
```rust
/// Get direct children of a workspace (entries where parent_workspace matches).
async fn get_children(&self, workspace: &str) -> Result<Vec<QueueEntry>>;
```

2. **Add to MergeQueue impl (queue.rs):**
```rust
pub async fn get_children(&self, workspace: &str) -> Result<Vec<QueueEntry>> {
    sqlx::query_as::<_, QueueEntry>(
        "SELECT id, workspace, ... FROM merge_queue WHERE parent_workspace = ?1"
    )
    .bind(workspace)
    .fetch_all(&self.pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to get children: {e}")))
}
```

3. **Add to QueueRepository for MergeQueue impl:**
```rust
async fn get_children(&self, workspace: &str) -> Result<Vec<QueueEntry>> {
    self.get_children(workspace).await
}
```

### Files to Modify
1. `crates/zjj-core/src/coordination/queue_repository.rs` - Add trait method
2. `crates/zjj-core/src/coordination/queue.rs` - Add impl + delegation
