# Database Migration Strategy

## Overview

ZJJ uses a robust, type-safe database migration system built on functional programming principles. The migration system ensures:

- **Zero panics**: All errors are handled through `Result<T, E>`
- **Transactional safety**: Each migration runs in a transaction (atomic apply or rollback)
- **Bidirectional migrations**: Every migration has both up (upgrade) and down (downgrade) paths
- **Version tracking**: Schema version is persisted in a dedicated `schema_version` table
- **Type safety**: Migration definitions use strongly-typed structs with validation

## Architecture

### Components

```
┌─────────────────────────────────────┐
│   MigrationEngine                   │
│   - Manages schema versioning       │
│   - Applies migrations atomically   │
│   - Tracks migration history        │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│   Migration Registry                │
│   - get_migrations() function       │
│   - Sequential version numbers      │
│   - Up/down SQL definitions         │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│   schema_version Table              │
│   - version INTEGER PRIMARY KEY     │
│   - description TEXT                │
│   - applied_at INTEGER              │
└─────────────────────────────────────┘
```

### Migration Flow

```
Start
  ↓
Check current schema version
  ↓
Compare with target version
  ↓
┌─────────────┐     ┌─────────────┐
│  Upgrade?   │ Yes │ Apply UP    │
│  target > current │ migrations  │
└─────────────┘     └─────────────┘
       ↓ No
┌─────────────┐     ┌─────────────┐
│ Downgrade?  │ Yes │ Apply DOWN  │
│ target < current  │ migrations  │
└─────────────┘     └─────────────┘
       ↓ No
Already at target version
  ↓
Done
```

## Database Schema Versioning

### Version Table Schema

```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
```

### Version Semantics

- **Version 0**: Empty database (no migrations applied)
- **Version N**: N migrations have been applied
- Versions are **sequential** and **monotonic** (1, 2, 3, ...)
- **No gaps**: Migrations must be applied in order

## Creating a Migration

### Migration Structure

```rust
use zjj::migrations::Migration;

let migration = Migration::new(
    version,        // u32: Sequential version number
    description,    // String: Human-readable description
    up_sql,         // String: SQL to apply migration
    down_sql,       // String: SQL to reverse migration
)?;
```

### Example: Adding a Column

```rust
Migration::new(
    2,  // Version 2
    "Add priority column to sessions table",
    // UP: Add column
    "ALTER TABLE sessions ADD COLUMN priority INTEGER DEFAULT 0 NOT NULL",
    // DOWN: Remove column (SQLite doesn't support DROP COLUMN easily, so recreate)
    r#"
        CREATE TABLE sessions_backup AS SELECT
            id, name, status, workspace_path, branch,
            created_at, updated_at, last_synced, metadata
        FROM sessions;
        DROP TABLE sessions;
        ALTER TABLE sessions_backup RENAME TO sessions;
    "#,
)
```

### Example: Adding a Table

```rust
Migration::new(
    3,
    "Create tags table for session categorization",
    // UP: Create table with indexes
    r#"
        CREATE TABLE tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            tag TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );
        CREATE INDEX idx_tags_session_id ON tags(session_id);
        CREATE INDEX idx_tags_tag ON tags(tag);
    "#,
    // DOWN: Drop table
    r#"
        DROP INDEX IF EXISTS idx_tags_tag;
        DROP INDEX IF EXISTS idx_tags_session_id;
        DROP TABLE IF EXISTS tags;
    "#,
)
```

### Migration Registry

All migrations are registered in `/home/lewis/src/zjj/crates/zjj/src/migrations.rs`:

```rust
pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration::new(1, "Initial schema", up_sql, down_sql).ok(),
        Migration::new(2, "Add feature X", up_sql, down_sql).ok(),
        Migration::new(3, "Add feature Y", up_sql, down_sql).ok(),
    ]
    .into_iter()
    .filter_map(|m| m)
    .collect()
}
```

**To add a new migration:**
1. Add to the end of the `vec!` in `get_migrations()`
2. Increment version number sequentially
3. Provide both up and down SQL
4. Test upgrade and downgrade paths

## Using the Migration Engine

### Automatic Migration (Recommended)

ZJJ automatically runs migrations when initializing a new database:

```rust
use zjj::db::SessionDb;

// Automatically runs migrations to latest version
let db = SessionDb::create_or_open(&db_path)?;
```

### Manual Migration

```rust
use zjj::migrations::{MigrationEngine, get_migrations};
use std::sync::{Arc, Mutex};

// Create engine with existing connection
let engine = MigrationEngine::new(conn)?;
let migrations = get_migrations();

// Migrate to latest version
engine.migrate_to_latest(&migrations)?;

// Migrate to specific version
engine.migrate_to_version(&migrations, 2)?;

// Check current version
let version = engine.current_version()?;

// View migration history
let history = engine.migration_history()?;
for (version, description, timestamp) in history {
    println!("v{}: {} (applied at {})", version, description, timestamp);
}
```

### CLI Commands (Future)

```bash
# Show current schema version
zjj migrate status

# Migrate to latest version
zjj migrate up

# Migrate to specific version
zjj migrate to 3

# Downgrade by one version
zjj migrate down

# Show migration history
zjj migrate history
```

## Safety Guarantees

### Transactional Integrity

Every migration runs inside a SQLite transaction:

```rust
fn apply_migration_up(&self, migration: &Migration) -> Result<()> {
    let tx = conn.transaction()?;

    // Apply migration SQL
    migration.apply_up(&tx)?;

    // Record version
    tx.execute("INSERT INTO schema_version ...")?;

    // Commit transaction (or rollback on error)
    tx.commit()?;
    Ok(())
}
```

**Guarantees:**
- If SQL fails, transaction is rolled back
- Database state is never left inconsistent
- Version table always matches actual schema

### Error Handling

All migration operations return `Result<T, MigrationError>`:

```rust
pub enum MigrationError {
    MigrationNotFound { version: u32 },
    NoDowngradePath { from: u32, to: u32 },
    MigrationFailed { version: u32, reason: String },
    CorruptedVersionTable { reason: String },
    TransactionFailed { reason: String },
    InvalidMigrationOrder { version: u32, current: u32 },
}
```

### Idempotency

Migrations are idempotent - running the same migration multiple times is safe:

```rust
// Safe to call multiple times
engine.migrate_to_latest(&migrations)?;
engine.migrate_to_latest(&migrations)?; // No-op
```

### Concurrency Safety

The migration engine uses `Arc<Mutex<Connection>>` for thread-safe access:

```rust
pub struct MigrationEngine {
    conn: Arc<Mutex<Connection>>,
}
```

Multiple threads attempting to migrate will serialize at the lock level.

## Testing Migrations

### Unit Tests

Every migration should have tests:

```rust
#[test]
fn test_migration_v2_up() -> Result<()> {
    let db = setup_test_db()?;
    let engine = MigrationEngine::new(db)?;

    let migrations = get_migrations();
    engine.migrate_to_version(&migrations, 2)?;

    // Verify schema changes
    assert_eq!(engine.current_version()?, 2);
    // ... verify column exists, etc.

    Ok(())
}

#[test]
fn test_migration_v2_down() -> Result<()> {
    let db = setup_test_db()?;
    let engine = MigrationEngine::new(db)?;

    let migrations = get_migrations();

    // Go up then down
    engine.migrate_to_version(&migrations, 2)?;
    engine.migrate_to_version(&migrations, 1)?;

    // Verify reverted
    assert_eq!(engine.current_version()?, 1);
    // ... verify column removed, etc.

    Ok(())
}
```

### Integration Tests

See `/home/lewis/src/zjj/crates/zjj/tests/migration_tests.rs` for comprehensive integration tests covering:

- Fresh database migration to latest
- Idempotent migration application
- Upgrade/downgrade roundtrips
- Partial upgrades and downgrades
- Migration history accuracy
- Production schema validation
- Concurrent migration safety
- Failed migration rollback

### Running Tests

```bash
# Run all migration tests
moon run :test -- --test migration_tests

# Run specific test
moon run :test -- --test migration_tests::test_production_schema_creates_sessions_table

# Run with output
moon run :test -- --test migration_tests --nocapture
```

## Best Practices

### 1. Always Provide Down Migrations

Even if you don't plan to downgrade, always provide a `down_sql`:

```rust
// GOOD
Migration::new(
    2,
    "Add column",
    "ALTER TABLE sessions ADD COLUMN foo TEXT",
    "ALTER TABLE sessions DROP COLUMN foo",  // Even if SQLite makes this hard
)

// BAD
Migration::new(
    2,
    "Add column",
    "ALTER TABLE sessions ADD COLUMN foo TEXT",
    "",  // Empty down migration - INVALID
)
```

### 2. Use Descriptive Names

```rust
// GOOD
"Add priority column to sessions for task ordering"

// BAD
"Add column"
```

### 3. Test Both Directions

Always test:
- Applying migration (up)
- Reversing migration (down)
- Roundtrip (up → down → up)

### 4. SQLite Limitations

SQLite doesn't support `ALTER TABLE DROP COLUMN` (before 3.35.0). Workarounds:

```rust
// Option 1: Recreate table
r#"
    CREATE TABLE sessions_new AS SELECT id, name, status FROM sessions;
    DROP TABLE sessions;
    ALTER TABLE sessions_new RENAME TO sessions;
"#

// Option 2: Accept limitation (mark as irreversible)
r#"
    -- WARNING: Cannot fully reverse this migration
    -- Column will remain but be unused
"#
```

### 5. Foreign Key Considerations

If using foreign keys:

```rust
// Disable FK constraints during migration
r#"
    PRAGMA foreign_keys = OFF;
    -- migration SQL
    PRAGMA foreign_keys = ON;
"#
```

### 6. Data Migrations

For data transformations:

```rust
Migration::new(
    4,
    "Convert status values to lowercase",
    r#"
        UPDATE sessions SET status = LOWER(status);
    "#,
    r#"
        UPDATE sessions SET status = UPPER(status);
    "#,
)
```

### 7. Index Creation

Always create indexes in migrations, never in application code:

```rust
Migration::new(
    5,
    "Add index on created_at for performance",
    "CREATE INDEX idx_sessions_created_at ON sessions(created_at)",
    "DROP INDEX idx_sessions_created_at",
)
```

## Troubleshooting

### Problem: "migration X not found"

**Cause**: Migration version gap or missing migration

**Solution**: Ensure migrations are sequential (1, 2, 3, ...) with no gaps

### Problem: "cannot downgrade from X to Y: no path exists"

**Cause**: Trying to downgrade but down migrations are missing

**Solution**: Add down SQL to all migrations between X and Y

### Problem: "schema version table is corrupted"

**Cause**: Manual schema modification or database corruption

**Solution**:
1. Check `schema_version` table: `SELECT * FROM schema_version`
2. Restore from backup if available
3. Manually fix version table or reinitialize with `zjj init --force`

### Problem: Migration fails midway

**Cause**: Invalid SQL or constraint violation

**Solution**:
1. Check error message for SQL syntax issues
2. Verify data constraints (NOT NULL, UNIQUE, etc.)
3. Transaction ensures rollback - no manual cleanup needed
4. Fix migration SQL and retry

## Migration Workflow Example

### Scenario: Adding Session Tags

**Step 1: Define Migration**

```rust
// In src/migrations.rs, add to get_migrations():
Migration::new(
    2,
    "Add tags support for session categorization",
    r#"
        CREATE TABLE session_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            tag TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, tag)
        );
        CREATE INDEX idx_session_tags_session ON session_tags(session_id);
        CREATE INDEX idx_session_tags_tag ON session_tags(tag);
    "#,
    r#"
        DROP INDEX IF EXISTS idx_session_tags_tag;
        DROP INDEX IF EXISTS idx_session_tags_session;
        DROP TABLE IF EXISTS session_tags;
    "#,
)?
```

**Step 2: Write Tests**

```rust
#[test]
fn test_migration_v2_creates_tags_table() -> Result<()> {
    let db = setup_test_db()?;
    let engine = MigrationEngine::new(db)?;

    engine.migrate_to_version(&get_migrations(), 2)?;

    // Verify table exists
    let guard = db.lock()?;
    let exists: bool = guard.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='session_tags'",
        [],
        |row| row.get(0),
    )?;
    assert!(exists);

    Ok(())
}
```

**Step 3: Run Tests**

```bash
moon run :test -- --test migration_tests
```

**Step 4: Apply Migration**

```rust
// Happens automatically on next db.create_or_open()
let db = SessionDb::create_or_open(&db_path)?;
```

**Step 5: Verify**

```bash
sqlite3 .zjj/sessions.db "SELECT * FROM schema_version"
# Should show version 2 with timestamp
```

## Future Enhancements

### Planned Features

1. **CLI Commands**: `zjj migrate` subcommands for manual control
2. **Migration Dry-Run**: Preview migration without applying
3. **Migration Squashing**: Combine old migrations for performance
4. **Migration Checksums**: Verify migration integrity
5. **Custom Migration Scripts**: Support for Rust-based migrations (not just SQL)

### Migration Format Evolution

Future versions may support:

```rust
// Rust-based migrations for complex logic
Migration::with_rust_fn(
    6,
    "Complex data transformation",
    |tx| {
        // Custom Rust logic
        migrate_sessions(tx)?;
        Ok(())
    },
    |tx| {
        // Reverse logic
        revert_sessions(tx)?;
        Ok(())
    },
)
```

## References

- **Implementation**: `/home/lewis/src/zjj/crates/zjj/src/migrations.rs`
- **Integration Tests**: `/home/lewis/src/zjj/crates/zjj/tests/migration_tests.rs`
- **Database Module**: `/home/lewis/src/zjj/crates/zjj/src/db.rs`
- **Error Types**: `/home/lewis/src/zjj/crates/zjj-core/src/lib.rs`

## Related Documentation

- [11_ARCHITECTURE.md](./11_ARCHITECTURE.md) - Overall system architecture
- [12_AI_GUIDE.md](./12_AI_GUIDE.md) - AI-assisted development patterns
- SQLite Documentation: https://www.sqlite.org/lang_altertable.html
- Railway-Oriented Programming: https://fsharpforfunandprofit.com/rop/
