# Contract Specification: bd-2gj - CREATE TABLE conflict_resolutions for tracking AI/human decisions

**Bead ID:** bd-2gj
**Title:** CREATE TABLE conflict_resolutions for tracking AI/human decisions
**Status:** Design Contract
**Version:** 1.0

## Overview

This contract defines the creation of the `conflict_resolutions` table to track conflict resolution decisions in the zjj workspace management system. The table provides an **append-only audit trail** for recording who resolved conflicts (AI or human), what strategy was used, and the reasoning behind the decision.

### Key Design Principles

1. **Append-Only Audit Log:** No UPDATE or DELETE operations allowed
2. **AI vs Human Tracking:** Every resolution records the decider type
3. **Transparency:** Full audit trail for debugging and accountability
4. **Performance:** Optimized for inserts and queries with indexes

## Function Signatures

### Core Database Initialization

```rust
/// Initialize conflict_resolutions table schema
///
/// This function is called during database initialization to create
/// the conflict_resolutions table and its indexes.
///
/// # Arguments
/// * `pool` - SQLite database connection pool
///
/// # Returns
/// * `Ok(())` - Schema initialized successfully
/// * `Err(Error::DatabaseError)` - Schema creation failed
pub async fn init_conflict_resolutions_schema(
    pool: &SqlitePool,
) -> Result<()>;
```

### Insert Operations

```rust
/// Insert a conflict resolution record
///
/// # Arguments
/// * `pool` - SQLite database connection pool
/// * `resolution` - ConflictResolution record to insert
///
/// # Returns
/// * `Ok(id)` - ID of inserted record
/// * `Err(Error::DatabaseError)` - Insert failed
pub async fn insert_conflict_resolution(
    pool: &SqlitePool,
    resolution: &ConflictResolution,
) -> Result<i64>;
```

### Query Operations

```rust
/// Get all conflict resolutions for a session
///
/// # Arguments
/// * `pool` - SQLite database connection pool
/// * `session` - Session name to query
///
/// # Returns
/// * `Ok(resolutions)` - Vector of ConflictResolution records
/// * `Err(Error::DatabaseError)` - Query failed
pub async fn get_conflict_resolutions(
    pool: &SqlitePool,
    session: &str,
) -> Result<Vec<ConflictResolution>>;

/// Get conflict resolutions by decider type
///
/// # Arguments
/// * `pool` - SQLite database connection pool
/// * `decider` - Decider type ("ai" or "human")
///
/// # Returns
/// * `Ok(resolutions)` - Vector of ConflictResolution records
/// * `Err(Error::DatabaseError)` - Query failed
pub async fn get_resolutions_by_decider(
    pool: &SqlitePool,
    decider: &str,
) -> Result<Vec<ConflictResolution>>;

/// Get conflict resolutions within time range
///
/// # Arguments
/// * `pool` - SQLite database connection pool
/// * `start_time` - ISO 8601 timestamp (inclusive)
/// * `end_time` - ISO 8601 timestamp (exclusive)
///
/// # Returns
/// * `Ok(resolutions)` - Vector of ConflictResolution records
/// * `Err(Error::DatabaseError)` - Query failed
pub async fn get_resolutions_by_time_range(
    pool: &SqlitePool,
    start_time: &str,
    end_time: &str,
) -> Result<Vec<ConflictResolution>>;
```

## Data Types

### ConflictResolution Entity

```rust
/// Conflict resolution record
///
/// Represents a single conflict resolution event in the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConflictResolution {
    /// Primary key (auto-increment)
    pub id: i64,

    /// ISO 8601 timestamp of resolution
    pub timestamp: String,

    /// Session name where conflict occurred
    pub session: String,

    /// File path with conflict
    pub file: String,

    /// Resolution strategy used
    /// Examples: "accept_theirs", "accept_ours", "manual_merge", "skip"
    pub strategy: String,

    /// Human-readable reason for resolution (optional)
    pub reason: Option<String>,

    /// Confidence score for AI decisions (optional)
    /// Examples: "high", "medium", "low", "0.95"
    pub confidence: Option<String>,

    /// Who made the decision
    /// Must be "ai" or "human"
    pub decider: String,
}
```

### Resolution Strategy Enum (Future Enhancement)

```rust
/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    /// Accept incoming changes
    AcceptTheirs,

    /// Accept current changes
    AcceptOurs,

    /// Manual merge by human
    ManualMerge,

    /// Skip file (defer resolution)
    Skip,

    /// AI-suggested merge (future)
    AiSuggested,
}
```

## Database Schema

### Table Schema

```sql
CREATE TABLE IF NOT EXISTS conflict_resolutions (
    -- Primary key (auto-increment)
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- ISO 8601 timestamp of resolution
    timestamp TEXT NOT NULL,

    -- Session name where conflict occurred
    session TEXT NOT NULL,

    -- File path with conflict
    file TEXT NOT NULL,

    -- Resolution strategy used
    strategy TEXT NOT NULL,

    -- Human-readable reason (optional)
    reason TEXT,

    -- Confidence for AI decisions (optional)
    confidence TEXT,

    -- Decider type: 'ai' or 'human'
    decider TEXT NOT NULL CHECK(decider IN ('ai', 'human'))
);
```

### Indexes

```sql
-- Index for session-based queries
-- Used by: get_conflict_resolutions()
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session
ON conflict_resolutions(session);

-- Index for time-based queries
-- Used by: get_resolutions_by_time_range()
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_timestamp
ON conflict_resolutions(timestamp);

-- Index for decider-based queries
-- Used by: get_resolutions_by_decider()
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_decider
ON conflict_resolutions(decider);

-- Composite index for session+time queries (common pattern)
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session_timestamp
ON conflict_resolutions(session, timestamp);
```

## Preconditions

### Global Preconditions (MUST hold before execution)

1. **Database Pool Initialized**
   - `SqlitePool` must be connected and valid
   - Database file must be writable
   - **Violation:** Returns `Error::DatabaseError` with message "Failed to initialize schema"

2. **Sessions Table Exists**
   - The `sessions` table must exist (created by `init_sessions_schema`)
   - Foreign key relationships may be validated
   - **Violation:** Schema initialization fails with error indicating missing dependency

### Insert-Specific Preconditions

3. **Valid ConflictResolution Record**
   - `session` must reference an existing session (optional validation)
   - `file` must be non-empty
   - `strategy` must be non-empty
   - `decider` must be "ai" or "human"
   - `timestamp` must be valid ISO 8601 format
   - **Violation:** Returns `Error::DatabaseError` with constraint violation details

### Query-Specific Preconditions

4. **Valid Session Name**
   - `session` parameter must be non-empty for `get_conflict_resolutions`
   - **Violation:** Returns `Error::InvalidInput` or empty result set

5. **Valid Decider Type**
   - `decider` must be "ai" or "human"
   - **Violation:** Returns `Error::InvalidInput` or empty result set

6. **Valid Time Range**
   - `start_time` and `end_time` must be valid ISO 8601 timestamps
   - `start_time` < `end_time`
   - **Violation:** Returns `Error::InvalidInput`

## Postconditions

### Schema Initialization Success Postconditions

1. **Table Created**
   - `conflict_resolutions` table exists in database
   - All columns defined with correct types
   - CHECK constraint on `decider` column enforced

2. **Indexes Created**
   - `idx_conflict_resolutions_session` exists
   - `idx_conflict_resolutions_timestamp` exists
   - `idx_conflict_resolutions_decider` exists
   - `idx_conflict_resolutions_session_timestamp` exists

3. **No Data Loss**
   - Schema initialization is idempotent
   - Existing records (if any) are preserved
   - `CREATE TABLE IF NOT EXISTS` ensures safety

### Insert Success Postconditions

4. **Record Persisted**
   - New row inserted with auto-generated `id`
   - All fields match input `ConflictResolution` record
   - `id` is monotonically increasing

5. **Timestamp Immutable**
   - `timestamp` field is set at insert time
   - No UPDATE operations allowed (append-only)

6. **Return Value Correct**
   - Returns the `id` of inserted record
   - `id > 0` and `id` matches `SELECT last_insert_rowid()`

### Query Success Postconditions

7. **Results Match Criteria**
   - All returned records match query filter
   - Results ordered by `id` (insertion order) unless otherwise specified
   - Empty vector returned if no matches found (not an error)

8. **Session Query Results**
   - `get_conflict_resolutions(session)` returns only records for that session
   - Session name comparison is case-sensitive

9. **Decider Query Results**
   - `get_resolutions_by_decider(decider)` returns only records with matching decider
   - Decider comparison is case-sensitive

10. **Time Range Query Results**
    - `get_resolutions_by_time_range(start, end)` returns only records with timestamps in range
    - Range is inclusive of start, exclusive of end: `[start, end)`

## Invariants

### Always True (during and after execution)

1. **Append-Only Table**
   - **Always:** No UPDATE operations on `conflict_resolutions` table
   - **Always:** No DELETE operations on `conflict_resolutions` table
   - Records are immutable once inserted

2. **Decider Constraint**
   - **Always:** `decider` field is either "ai" or "human"
   - **Always:** CHECK constraint enforced at database level
   - No other values allowed

3. **Timestamp Format**
   - **Always:** `timestamp` is valid ISO 8601 format
   - **Always:** Timestamp represents UTC time
   - No NULL timestamps allowed

4. **Primary Key Uniqueness**
   - **Always:** `id` is unique across all records
   - **Always:** `id` is monotonically increasing
   - No gaps in sequence (except after database vacuum)

5. **Index Consistency**
   - **Always:** All indexes are maintained automatically by SQLite
   - **Always:** Query results consistent with index definitions
   - No manual index rebuilding required

6. **Non-Null Fields**
   - **Always:** `id`, `timestamp`, `session`, `file`, `strategy`, `decider` are NOT NULL
   - **Always:** Only `reason` and `confidence` may be NULL

## Error Taxonomy

### Exhaustive Error Variants

```rust
pub enum ConflictResolutionError {
    // === Database Errors (Exit Code 3) ===

    /// Schema initialization failed
    SchemaInitializationError {
        operation: String,  // "CREATE TABLE", "CREATE INDEX"
        source: sqlx::Error,
        recovery: String,  // "Check database permissions"
    },

    /// Insert operation failed
    InsertError {
        record: ConflictResolution,
        source: sqlx::Error,
        constraint: Option<String>,  // "CHECK constraint failed: decider"
        recovery: String,
    },

    /// Query operation failed
    QueryError {
        operation: String,  // "get_conflict_resolutions"
        source: sqlx::Error,
        recovery: String,
    },

    // === Validation Errors (Exit Code 1) ===

    /// Invalid decider type
    InvalidDeciderError {
        decider: String,
        expected: Vec<String>,  // ["ai", "human"]
    },

    /// Invalid timestamp format
    InvalidTimestampError {
        timestamp: String,
        expected_format: String,  // "ISO 8601"
    },

    /// Empty required field
    EmptyFieldError {
        field: String,  // "session", "file", "strategy"
    },

    /// Invalid time range
    InvalidTimeRangeError {
        start_time: String,
        end_time: String,
        reason: String,  // "start_time >= end_time"
    },
}
```

### Error Propagation Mapping

```rust
impl From<sqlx::Error> for ConflictResolutionError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                if db_err.code().as_deref() == Some("206") {
                    // CHECK constraint failed
                    ConflictResolutionError::InsertError {
                        record: /* unknown */,
                        source: err,
                        constraint: db_err.message().into(),
                        recovery: "Ensure decider is 'ai' or 'human'".into(),
                    }
                } else {
                    ConflictResolutionError::DatabaseError {
                        operation: "unknown".into(),
                        source: err,
                        recovery: "Check database state".into(),
                    }
                }
            }
            _ => ConflictResolutionError::DatabaseError {
                operation: "unknown".into(),
                source: err,
                recovery: "Check database connectivity".into(),
            },
        }
    }
}
```

## API Contracts

### init_conflict_resolutions_schema

```rust
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `sessions` table exists (dependency)
///
/// ## Postconditions
/// - `conflict_resolutions` table exists
/// - All indexes created
/// - Function is idempotent (safe to call multiple times)
///
/// ## Errors
/// - `Error::DatabaseError` if table creation fails
///
/// ## Example
/// ```rust
/// init_conflict_resolutions_schema(&pool).await?;
/// // Table now exists and ready for inserts
/// ```
pub async fn init_conflict_resolutions_schema(
    pool: &SqlitePool,
) -> Result<()>;
```

### insert_conflict_resolution

```rust
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `resolution.session` is valid (may check existence)
/// - `resolution.decider` is "ai" or "human"
/// - `resolution.timestamp` is valid ISO 8601
/// - `resolution.file` and `resolution.strategy` are non-empty
///
/// ## Postconditions
/// - Record inserted with auto-generated ID
/// - Returned ID matches inserted record
/// - `SELECT * FROM conflict_resolutions WHERE id = ?` returns record
///
/// ## Errors
/// - `Error::DatabaseError` if insert fails (constraint violation, I/O error)
/// - `Error::InvalidInput` if validation fails
///
/// ## Example
/// ```rust
/// let resolution = ConflictResolution {
///     id: 0,  // Auto-generated
///     timestamp: "2025-02-18T12:34:56Z".to_string(),
///     session: "my-session".to_string(),
///     file: "src/main.rs".to_string(),
///     strategy: "accept_theirs".to_string(),
///     reason: Some("Automatic resolution".to_string()),
///     confidence: Some("high".to_string()),
///     decider: "ai".to_string(),
/// };
/// let id = insert_conflict_resolution(&pool, &resolution).await?;
/// assert!(id > 0);
/// ```
pub async fn insert_conflict_resolution(
    pool: &SqlitePool,
    resolution: &ConflictResolution,
) -> Result<i64>;
```

### get_conflict_resolutions

```rust
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `session` is non-empty
///
/// ## Postconditions
/// - Returns all records for given session
/// - Results ordered by `id` ascending
/// - Returns empty Vec if no matches (not an error)
///
/// ## Errors
/// - `Error::DatabaseError` if query fails
/// - `Error::InvalidInput` if session is empty
///
/// ## Example
/// ```rust
/// let resolutions = get_conflict_resolutions(&pool, "my-session").await?;
/// for resolution in resolutions {
///     println!("{}: {} resolved by {}", resolution.file, resolution.strategy, resolution.decider);
/// }
/// ```
pub async fn get_conflict_resolutions(
    pool: &SqlitePool,
    session: &str,
) -> Result<Vec<ConflictResolution>>;
```

### get_resolutions_by_decider

```rust
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `decider` is "ai" or "human"
///
/// ## Postconditions
/// - Returns all records with matching decider
/// - Results ordered by `id` ascending
/// - Returns empty Vec if no matches (not an error)
///
/// ## Errors
/// - `Error::DatabaseError` if query fails
/// - `Error::InvalidInput` if decider is invalid
///
/// ## Example
/// ```rust
/// let ai_resolutions = get_resolutions_by_decider(&pool, "ai").await?;
/// println!("AI resolved {} conflicts", ai_resolutions.len());
/// ```
pub async fn get_resolutions_by_decider(
    pool: &SqlitePool,
    decider: &str,
) -> Result<Vec<ConflictResolution>>;
```

### get_resolutions_by_time_range

```rust
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `start_time` and `end_time` are valid ISO 8601 timestamps
/// - `start_time` < `end_time`
///
/// ## Postconditions
/// - Returns all records with timestamps in [start_time, end_time)
/// - Results ordered by `timestamp` ascending
/// - Returns empty Vec if no matches (not an error)
///
/// ## Errors
/// - `Error::DatabaseError` if query fails
/// - `Error::InvalidInput` if timestamps are invalid or range invalid
///
/// ## Example
/// ```rust
/// let resolutions = get_resolutions_by_time_range(
///     &pool,
///     "2025-02-18T00:00:00Z",
///     "2025-02-18T23:59:59Z"
/// ).await?;
/// println!("Resolved {} conflicts today", resolutions.len());
/// ```
pub async fn get_resolutions_by_time_range(
    pool: &SqlitePool,
    start_time: &str,
    end_time: &str,
) -> Result<Vec<ConflictResolution>>;
```

## Migration Notes

### Schema Versioning

This table is part of schema version 1 (if incrementing from current schema).

### Dependencies

- Requires `sessions` table to exist first
- May add foreign key in future: `FOREIGN KEY (session) REFERENCES sessions(name)`

### Future Enhancements

1. **Resolution Strategy Enum**
   - Convert `strategy` from TEXT to enum type
   - Add validation in Rust layer

2. **Foreign Key Constraints**
   - Add `FOREIGN KEY (session) REFERENCES sessions(name)` once referential integrity validated

3. **Audit Trail Compression**
   - Consider archiving old records for performance
   - Implement retention policy (e.g., keep 90 days)

4. **Metrics Integration**
   - Add counters for AI vs human decisions
   - Track strategy usage patterns

## Performance Requirements

### Insert Performance

- **Target:** < 10ms per insert
- **Method:** Single-row INSERT with auto-increment ID
- **Optimization:** Indexes optimized for INSERT workload

### Query Performance

- **Target:** < 100ms for queries returning up to 1000 records
- **Method:** Index-based queries
- **Optimization:** Composite indexes for common query patterns

### Storage Growth

- **Estimate:** ~200 bytes per record
- **Capacity:** 1 million records ≈ 200 MB
- **Mitigation:** Implement retention policy if needed

## Security Considerations

### Append-Only Audit Trail

1. **No Tampering**
   - No UPDATE operations allowed
   - No DELETE operations allowed
   - Records are immutable once inserted

2. **Decider Tracking**
   - Every record explicitly identifies decider (AI or human)
   - Enables accountability for decisions

3. **Transparency**
   - Full history accessible via queries
   - No hidden decisions or shadow operations

### Input Validation

1. **Decider Field**
   - CHECK constraint at database level
   - Only "ai" or "human" allowed
   - No injection possible

2. **File Paths**
   - Stored as TEXT (no filesystem access)
   - No path traversal concerns (append-only)

3. **Session Names**
   - Must match existing sessions (optional validation)
   - No privilege escalation concerns

## Code Organization

### File Structure

```
crates/zjj-core/src/coordination/
├── conflict_resolutions.rs     # Main module (NEW)
│   ├── init_conflict_resolutions_schema()
│   ├── insert_conflict_resolution()
│   ├── get_conflict_resolutions()
│   ├── get_resolutions_by_decider()
│   └── get_resolutions_by_time_range()
├── conflict_resolutions_entities.rs  # Entity types (NEW)
│   └── struct ConflictResolution
└── queue.rs                     # Existing (for reference)

sql_schemas/
├── 01_sessions.sql              # Existing
├── 02_session_locks.sql         # Existing
├── 03_conflict_resolutions.sql  # NEW
└── README.md                    # Update with new table
```

### Module Documentation

```rust
//! Conflict resolution audit trail
//!
//! This module provides an append-only audit log for tracking conflict
//! resolution decisions in zjj workspaces. Each record captures:
//!
//! - **Who** resolved the conflict (AI or human)
//! - **What** strategy was used
//! - **Why** the decision was made (optional reason)
//! - **When** the resolution occurred
//!
//! # Design Principles
//!
//! 1. **Append-Only**: No UPDATE or DELETE operations
//! 2. **Transparent**: Full audit trail for debugging
//! 3. **Performant**: Optimized for inserts and queries
//!
//! # Example
//!
//! ```rust
//! use zjj_core::coordination::conflict_resolutions::*;
//!
//! // Initialize schema (called during db init)
//! init_conflict_resolutions_schema(&pool).await?;
//!
//! // Record a conflict resolution
//! let resolution = ConflictResolution {
//!     id: 0,
//!     timestamp: "2025-02-18T12:34:56Z".to_string(),
//!     session: "my-session".to_string(),
//!     file: "src/main.rs".to_string(),
//!     strategy: "accept_theirs".to_string(),
//!     reason: Some("Incoming changes are more recent".to_string()),
//!     confidence: Some("high".to_string()),
//!     decider: "ai".to_string(),
//! };
//! let id = insert_conflict_resolution(&pool, &resolution).await?;
//!
//! // Query resolutions for a session
//! let resolutions = get_conflict_resolutions(&pool, "my-session").await?;
//! for r in resolutions {
//!     println!("{}: {} by {}", r.file, r.strategy, r.decider);
//! }
//! ```
```

## Testing Strategy

See `/home/lewis/src/zjj/contracts/bd-2gj-martin-fowler-tests.md` for comprehensive test plan covering:

- **Schema Initialization:** Table and indexes created correctly
- **Insert Operations:** Valid and invalid records
- **Query Operations:** All query functions tested
- **Append-Only Invariant:** No UPDATE/DELETE operations
- **Decider Constraint:** Only "ai" or "human" allowed
- **Performance:** Insert and query latency targets
- **Concurrency:** Multiple concurrent inserts
- **Error Handling:** All error variants tested

## Acceptance Criteria

### Must Have (P0 - Critical)

1. [ ] `conflict_resolutions` table created with correct schema
2. [ ] All 4 indexes created
3. [ ] `ConflictResolution` struct defined with correct fields
4. [ ] `init_conflict_resolutions_schema()` function works
5. [ ] `insert_conflict_resolution()` function works
6. [ ] `get_conflict_resolutions()` function works
7. [ ] `get_resolutions_by_decider()` function works
8. [ ] `get_resolutions_by_time_range()` function works
9. [ ] CHECK constraint on `decider` enforced
10. [ ] No UPDATE or DELETE operations in implementation
11. [ ] All functions return `Result<>` (no unwraps)
12. [ ] Unit tests pass (see test plan)

### Should Have (P1 - High)

1. [ ] Foreign key to sessions table (optional validation)
2. [ ] Comprehensive error messages
3. [ ] Module documentation with examples
4. [ ] Integration tests with real database
5. [ ] Performance benchmarks (insert < 10ms, query < 100ms)

### Could Have (P2 - Nice to Have)

1. [ ] Migration script for existing databases
2. [ ] Retention policy for old records
3. [ ] Metrics/observability integration
4. [ ] Resolution strategy enum (future enhancement)

---

**Contract Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
