# Rust Contract: zjj-1nyz

## Title
database: Ensure consistent session counting

## Type
bug

## Description
Session counts are wrong in various places. Inconsistent counts, confusing. Found by Agent #5.

## Problem Statement
Session counts are calculated differently in different parts of the codebase:
- `zjj list` shows one count
- `zjj status` shows another count
- Database queries may count differently
- Active vs total sessions are unclear

## Preconditions
- Sessions are stored in database
- Multiple commands show session counts
- No single source of truth for counting

## Postconditions
- All session counts are consistent across commands
- Clear definition of "active" vs "total" sessions
- Single function to count sessions
- Tests verify consistency

## Invariants
- **I1**: `zjj list` row count matches displayed count
- **I2**: `zjj status` session count matches database
- **I3**: All counts use the same counting logic
- **I4**: "Active" sessions are clearly defined

## Investigation Required

### Locations Where Sessions Are Counted
1. `crates/zjj/src/commands/list.rs` - Lists sessions, shows count
2. `crates/zjj/src/commands/status.rs` - Shows session stats
3. `crates/zjj-core/src/store.rs` - Database queries
4. Any other command showing counts

### Counting Methods to Audit
```rust
// Look for patterns like:
SELECT COUNT(*) FROM sessions
.len() on session vectors
.filter().count() on iterators
```

### Definitions to Clarify
- **Total sessions**: All sessions in database
- **Active sessions**: Sessions with state != "closed"
- **Current session**: The session zjj is connected to
- **Orphaned sessions**: Sessions without workspace

## Implementation Strategy

### Step 1: Create Single Count Function
```rust
// In zjj-core/src/store.rs
impl SessionStore {
    /// Count sessions with optional filter
    pub fn count(&self, filter: SessionFilter) -> Result<usize> {
        match filter {
            SessionFilter::All => self.count_all(),
            SessionFilter::Active => self.count_active(),
            SessionFilter::Closed => self.count_closed(),
        }
    }

    fn count_all(&self) -> Result<usize>;
    fn count_active(&self) -> Result<usize>;
    fn count_closed(&self) -> Result<usize>;
}
```

### Step 2: Update All Callers
- Replace `SELECT COUNT(*)` with `store.count()`
- Update `zjj list` to use consistent count
- Update `zjj status` to use consistent count
- Ensure counts match actual rows/items shown

### Step 3: Add Count Display Tests
```rust
#[test]
fn list_count_matches_rows() {
    // Run zjj list, parse output
    // Count number of session rows
    // Verify count matches "N sessions" text
}

#[test]
fn status_count_matches_database() {
    // Run zjj status
    // Parse session count from output
    // Query database directly
    // Verify counts match
}
```

## Files to Audit
- `crates/zjj/src/commands/list.rs`
- `crates/zjj/src/commands/status.rs`
- `crates/zjj-core/src/store.rs`
- Any other command showing counts

## Files to Modify
- `crates/zjj-core/src/store.rs` - Add consistent count methods
- `crates/zjj/src/commands/list.rs` - Use consistent count
- `crates/zjj/src/commands/status.rs` - Use consistent count

## Files to Test
- `crates/zjj/tests/test_session_counting.rs` - New test file

## Test Cases

### TC-1: List Count Matches Rows
- Create 5 sessions in database
- Run `zjj list`
- Count rows in output
- Verify count matches

### TC-2: Status Count Matches Database
- Create 3 active, 2 closed sessions
- Run `zjj status`
- Parse session count
- Query database directly
- Verify counts match

### TC-3: Active vs Total Count
- Create 10 sessions, close 3
- Count active: should be 7
- Count total: should be 10
- Count closed: should be 3
- Verify 7 + 3 = 10

### TC-4: Empty Database
- Delete all sessions
- Run `zjj list`
- Should show "0 sessions"
- Run `zjj status`
- Should show "0 active sessions"

### TC-5: Count After Operations
- Start with 0 sessions
- Add 1 session: count = 1
- Add 2 more: count = 3
- Remove 1: count = 2
- Verify counts update correctly

## Estimated Effort
1 hour
