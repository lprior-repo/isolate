# Scout Notes - bd-1i1a: Add parent_workspace column

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
