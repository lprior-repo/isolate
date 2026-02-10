# Rust Contract: zjj-1840

## Title
Add data migration layer

## Type
chore

## Description
Need schema changes migration system. Database versioning and migrations.

## Problem Statement
The application uses SQLite databases (`.beads/beads.db`, `.zjj/state.db`) but has no migration system. When schema changes are deployed:
- Existing databases become incompatible
- Users get cryptic SQL errors
- No way to upgrade databases safely
- No rollback mechanism

## Preconditions
- SQLite databases exist at `.beads/beads.db` and `.zjj/state.db`
- Current code has hardcoded schema expectations
- No version tracking exists

## Postconditions
- Database version is tracked in a `schema_migrations` table
- New migrations can be added without breaking existing databases
- Migrations run automatically on startup
- Failed migrations can be rolled back
- Migration history is preserved

## Invariants
- **I1**: All migrations must be reversible (or marked as irreversible)
- **I2**: Migrations must be idempotent (running twice is safe)
- **I3**: Migration order is determined by timestamp, not filename
- **I4**: Database is never left in an inconsistent state

## Architecture

### Migration Table Schema
```sql
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL,  -- ISO 8601 timestamp
    rollback_sql TEXT,         -- NULL if irreversible
    checksum TEXT NOT NULL     -- SHA-256 of migration content
);
```

### Migration File Format
Migrations live in `crates/zjj-core/src/migrations/`:

```
migrations/
  001_initial_schema.sql
  002_add_sessions_table.sql
  003_add_bookmarks_table.sql
  ...
```

Each migration file:
```sql
-- UP: Migration description
-- @version: 20240101_120000
-- @name: initial_schema

CREATE TABLE sessions (...);

-- DOWN
-- @rollback: DROP TABLE sessions;
```

### Migration Runner API
```rust
pub struct Migrator {
    db: Arc<SqlitePool>,
    migrations_dir: PathBuf,
}

impl Migrator {
    pub async fn new(db: Arc<SqlitePool>) -> Result<Self>;
    pub async fn run_pending(&self) -> Result<MigrationReport>;
    pub async fn rollback(&self, version: i64) -> Result<()>;
    pub async fn status(&self) -> Result<MigrationStatus>;
    pub async fn create(&self, name: &str) -> Result<PathBuf>;
}
```

## Implementation Tasks

### Phase 1: Migration Table
- [ ] Add `schema_migrations` table creation
- [ ] Add version tracking queries
- [ ] Add migration history queries

### Phase 2: Migration Parser
- [ ] Parse migration files (UP/DOWN sections)
- [ ] Extract metadata (version, name, checksum)
- [ ] Validate migration syntax

### Phase 3: Migration Runner
- [ ] Compare database version vs available migrations
- [ ] Run pending migrations in transaction
- [ ] Record successful migrations
- [ ] Handle migration failures

### Phase 4: CLI Integration
- [ ] `zjj doctor` checks migration status
- [ ] `zjj migrate status` shows pending/ran migrations
- [ ] `zjj migrate create <name>` generates new migration file
- [ ] Automatic migration on startup

## Test Cases

### TM-1: Fresh Database
- No `schema_migrations` table exists
- All migrations run automatically
- All migrations recorded in history

### TM-2: Existing Database (First Run)
- Database exists but no migration table
- Migration table created
- Current schema marked as baseline migration

### TM-3: Pending Migrations
- Database at version 2
- Migrations 3, 4, 5 exist
- Migrations 3, 4, 5 run in order

### TM-4: Failed Migration
- Migration 4 has invalid SQL
- Migration 3 completes successfully
- Migration 4 fails
- Transaction rolled back
- Database state unchanged

### TM-5: Rollback
- Migration 4 applied
- Run `rollback(4)`
- Migration 4 down SQL executed
- Version returns to 3

### TM-6: Idempotent
- Run migrations twice
- Second run is no-op
- No duplicate entries

## Files to Create
- `crates/zjj-core/src/migrations/mod.rs`
- `crates/zjj-core/src/migrations/migrator.rs`
- `crates/zjj-core/src/migrations/parser.rs`
- `crates/zjj-core/tests/test_migrations.rs`

## Files to Modify
- `crates/zjj-core/src/lib.rs` - Export migrations module
- `crates/zjj/src/commands/doctor.rs` - Add migration check
- `crates/zjj/src/main.rs` - Run migrations on startup

## Estimated Effort
4 hours
