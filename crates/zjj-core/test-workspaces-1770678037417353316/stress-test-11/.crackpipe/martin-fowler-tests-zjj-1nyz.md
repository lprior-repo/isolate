# Martin Fowler Test Plan: zjj-1nyz

## Title
database: Ensure consistent session counting

## Test Strategy
Session counting is broken across the codebase. We need to identify all counting locations, consolidate to a single source of truth, and add tests to prevent future inconsistencies.

## Test Catalog

### TC-1: List Command Count Matches Displayed Rows
**Scenario**: User runs `zjj list` and sees session count
**Given**: Database has 5 sessions
**When**: User runs `zjj list`
**Then**:
- Output shows exactly 5 session rows
- Footer/splash shows "5 sessions"
- Count matches row count

### TC-2: Status Command Count Matches Database
**Scenario**: User runs `zjj status` to see overview
**Given**: Database has 3 active, 2 closed sessions
**When**: User runs `zjj status`
**Then**:
- Output shows "3 active sessions"
- Database query returns 3 active sessions
- No mismatch

### TC-3: Active Count Excludes Closed Sessions
**Scenario**: Closed sessions shouldn't count as active
**Given**:
- 10 total sessions in database
- 3 sessions have state="closed"
**When**: User queries active session count
**Then**:
- Active count = 7
- Total count = 10
- Closed count = 3
- 7 + 3 = 10

### TC-4: Empty Database Shows Zero
**Scenario**: No sessions exist
**Given**: Empty sessions table
**When**: User runs `zjj list` or `zjj status`
**Then**:
- `zjj list` shows "0 sessions" or "No sessions"
- `zjj status` shows "0 active sessions"
- No negative counts or null values

### TC-5: Count Updates After Add/Remove
**Scenario**: Counts should reflect real-time changes
**Given**: Empty database
**When**:
- Add session A → count should be 1
- Add session B → count should be 2
- Add session C → count should be 3
- Remove session B → count should be 2
**Then**: Each operation updates count correctly

### TC-6: JSON Output Has Consistent Count
**Scenario**: JSON output should match text output
**Given**: Database has 4 sessions
**When**: User runs `zjj list --json`
**Then**:
- JSON has `count: 4` field or array length is 4
- Text output also shows 4 sessions
- Counts match

### TC-7: Filtered List Shows Correct Count
**Scenario**: Filtering sessions should update count
**Given**: 10 sessions total, 3 active
**When**: User runs `zjj list --filter active`
**Then**:
- Only 3 sessions shown
- Count shows "3 sessions" (not 10)
- Filter is reflected in count

### TC-8: Concurrent Session Changes Don't Corrupt Count
**Scenario**: Multiple processes modifying sessions
**Given**: Database with 5 sessions
**When**:
- Process A adds 2 sessions
- Process B removes 1 session
- Process C queries count
**Then**:
- Count is accurate (6 total)
- No race condition corruption
- Count reflects final state

### TC-9: Database Query Returns Same Count as Display
**Scenario**: Direct database query matches command output
**Given**: Database with N sessions
**When**:
- Run `SELECT COUNT(*) FROM sessions` → count1
- Run `zjj list` → parse count2
**Then**: count1 = count2

### TC-10: Orphaned Sessions Are Counted Correctly
**Scenario**: Sessions without workspaces
**Given**:
- 10 sessions with workspaces
- 2 sessions without workspaces (orphaned)
**When**: User runs `zjj list`
**Then**:
- Total count = 12
- Orphaned sessions are marked
- No sessions are "lost"

## Test Implementation Structure

### Test File
```rust
// crates/zjj/tests/test_session_counting.rs

mod list_count {
    #[tokio::test]
    async fn list_count_matches_rows() { /* TC-1 */ }

    #[tokio::test]
    async fn empty_database_shows_zero() { /* TC-4 */ }

    #[tokio::test]
    async fn json_count_matches_rows() { /* TC-6 */ }
}

mod status_count {
    #[tokio::test]
    async fn status_count_matches_database() { /* TC-2 */ }
}

mod active_count {
    #[tokio::test]
    async fn active_excludes_closed() { /* TC-3 */ }
}

mod count_updates {
    #[tokio::test]
    async fn count_updates_after_operations() { /* TC-5 */ }
}
```

### Test Helpers
```rust
async fn create_n_sessions(n: usize) -> Result<Vec<String>>;
async fn count_sessions_from_list_output() -> Result<usize>;
async fn count_sessions_from_status_output() -> Result<usize>;
async fn count_sessions_directly() -> Result<usize>;
```

## Integration Test Commands
```bash
# Run all counting tests
moon run :test test_session_counting

# Run specific test
moon run :test test_session_counting::list_count::list_count_matches_rows

# Manual verification
zjj list | grep "sessions"
zjj status | grep "sessions"
sqlite3 .zjj/state.db "SELECT COUNT(*) FROM sessions;"
```

## Success Criteria
- All 10 test scenarios pass
- `zjj list` count matches rows
- `zjj status` count matches database
- Single count function used throughout codebase
- Active vs total counts clearly defined

## Estimated Effort
1 hour
