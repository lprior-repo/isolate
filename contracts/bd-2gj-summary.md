# Contract Summary: bd-2gj - conflict_resolutions Table

**Bead ID:** bd-2gj
**Issue ID:** zjj-20260217-014-db-conflict-resolutions-table
**Title:** CREATE TABLE conflict_resolutions for tracking AI/human decisions
**Status:** Contract Design Complete
**Version:** 1.0

## Deliverables

### 1. Contract Specification
**File:** `/home/lewis/src/zjj/contracts/bd-2gj-contract-spec.md`

Complete contract specification including:
- Table schema with 8 columns
- 4 indexes for query optimization
- 5 public API functions
- Comprehensive error taxonomy
- Performance requirements
- Security considerations

### 2. Martin Fowler Test Plan
**File:** `/home/lewis/src/zjj/contracts/bd-2gj-martin-fowler-tests.md`

Comprehensive test plan with:
- **46 tests** across 6 test suites
- Schema tests (10 tests)
- Insert tests (15 tests)
- Query tests (10 tests)
- Invariant tests (8 tests)
- Error tests (3 tests)
- Performance tests (3 tests)

## Table Structure

```sql
CREATE TABLE IF NOT EXISTS conflict_resolutions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    session TEXT NOT NULL,
    file TEXT NOT NULL,
    strategy TEXT NOT NULL,
    reason TEXT,
    confidence TEXT,
    decider TEXT NOT NULL CHECK(decider IN ('ai', 'human'))
);
```

## Key Design Decisions

### 1. Append-Only Audit Trail
- No UPDATE operations allowed
- No DELETE operations allowed
- Records are immutable once inserted
- Enables full transparency and accountability

### 2. AI vs Human Tracking
- `decider` field with CHECK constraint: only "ai" or "human"
- Explicit tracking of who made decisions
- Optional `confidence` field for AI decisions
- Optional `reason` field for human/AI explanations

### 3. Performance Optimization
- 4 indexes for common query patterns
- Composite index on (session, timestamp)
- Target: < 10ms insert, < 100ms query
- Optimized for high insert workload

### 4. Flexible Schema
- `strategy` is TEXT (not enum) for extensibility
- Optional fields: `reason`, `confidence`
- Supports future enhancements without schema migration

## API Functions

### Schema Initialization
```rust
pub async fn init_conflict_resolutions_schema(
    pool: &SqlitePool,
) -> Result<()>;
```

### Insert Operation
```rust
pub async fn insert_conflict_resolution(
    pool: &SqlitePool,
    resolution: &ConflictResolution,
) -> Result<i64>;
```

### Query Operations
```rust
pub async fn get_conflict_resolutions(
    pool: &SqlitePool,
    session: &str,
) -> Result<Vec<ConflictResolution>>;

pub async fn get_resolutions_by_decider(
    pool: &SqlitePool,
    decider: &str,
) -> Result<Vec<ConflictResolution>>;

pub async fn get_resolutions_by_time_range(
    pool: &SqlitePool,
    start_time: &str,
    end_time: &str,
) -> Result<Vec<ConflictResolution>>;
```

## Implementation Files

### New Files to Create
```
crates/zjj-core/src/coordination/conflict_resolutions.rs
crates/zjj-core/src/coordination/conflict_resolutions_entities.rs
sql_schemas/03_conflict_resolutions.sql
crates/zjj-core/tests/conflict_resolutions_tests.rs
```

### Files to Update
```
crates/zjj-core/src/coordination/mod.rs  # Add module
sql_schemas/README.md                     # Document new table
```

## Dependencies

### Requires
- `zjj-20260217-012-db-merge-queue-tables` (must complete first)
- SQLite with CHECK constraint support
- SQLx for async database operations

### Blocks
- `zjj-20260217-027-conflict-analyze-cmd`
- `zjj-20260217-028-conflict-resolve-cmd`
- `zjj-20260217-029-conflict-quality-signals`

## Acceptance Criteria

### Must Have (P0 - Critical)
- [x] Contract specification complete
- [x] Test plan complete
- [ ] Table schema implemented
- [ ] All 4 indexes created
- [ ] ConflictResolution struct defined
- [ ] init_conflict_resolutions_schema() works
- [ ] insert_conflict_resolution() works
- [ ] All query functions work
- [ ] CHECK constraint enforced
- [ ] No UPDATE/DELETE in implementation
- [ ] All functions return Result<>
- [ ] Unit tests pass

### Should Have (P1 - High)
- [ ] Comprehensive error messages
- [ ] Module documentation
- [ ] Integration tests
- [ ] Performance benchmarks pass

## Performance Targets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Insert    | < 10ms | Per insert operation |
| Query     | < 100ms | For up to 1000 records |
| Concurrent Insert | < 5s | For 50 concurrent inserts |

## Security Considerations

1. **Append-Only Design**
   - No tampering with historical records
   - Immutable audit trail

2. **Decider Tracking**
   - Every decision explicitly attributed
   - Accountability enforced

3. **Input Validation**
   - CHECK constraint on decider field
   - No SQL injection (parameterized queries)

## Testing Coverage

| Suite | Tests | Priority |
|-------|-------|----------|
| Schema Tests | 10 | P0 |
| Insert Tests | 15 | P0 |
| Query Tests | 10 | P0 |
| Invariant Tests | 8 | P0 |
| Error Tests | 3 | P0 |
| Performance Tests | 3 | P1 |
| **Total** | **46** | **43 P0, 3 P1** |

## Next Steps

1. **Implementation**
   - Create `sql_schemas/03_conflict_resolutions.sql`
   - Implement `conflict_resolutions_entities.rs` with struct
   - Implement `conflict_resolutions.rs` with functions
   - Add module to `coordination/mod.rs`

2. **Testing**
   - Implement tests in `conflict_resolutions_tests.rs`
   - Run all 46 tests
   - Verify performance targets

3. **Documentation**
   - Update `sql_schemas/README.md`
   - Add module documentation
   - Update API documentation

## Migration Notes

- Schema initialization is idempotent (safe to run multiple times)
- No foreign key constraint in v1 (may add in future)
- No data migration needed (new table)
- Compatible with existing database schema

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Table growth | High | Implement retention policy later |
| Performance degradation | Medium | Indexes optimize queries |
| Constraint violations | Low | CHECK constraint enforced |
| Concurrent insert conflicts | Low | SQLite handles serialization |

---

**Contract Version:** 1.0
**Created:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for Implementation
**Estimated Effort:** 4-6 hours (implementation + tests)
