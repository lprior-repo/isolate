# Martin Fowler Test Plan: zjj-1840

## Title
Add data migration layer

## Test Strategy
Database migrations are critical infrastructure. We need comprehensive tests covering the full migration lifecycle: creation, application, rollback, and error handling.

## Test Catalog

### TM-1: Fresh Database Receives All Migrations
**Scenario**: New user starts zjj for the first time
**Given**: No database file exists
**When**: Application starts
**Then**:
- Database is created with `schema_migrations` table
- All available migrations are applied
- Migration history shows all migrations as applied
- Application functions normally

### TM-2: Existing Database Gets Migration Table
**Scenario**: User upgrades from version without migrations
**Given**: Database exists but has no `schema_migrations` table
**When**: Application starts with migration system
**Then**:
- `schema_migrations` table is created
- Existing schema is marked as baseline (version 0)
- No existing data is lost
- New migrations can be applied

### TM-3: Pending Migrations Run Automatically
**Scenario**: User has database at version 2, new code has migrations 3, 4, 5
**Given**:
- Database has migrations 1, 2 applied
- Migrations directory has files 001-005
**When**: Application starts
**Then**:
- Only migrations 3, 4, 5 are executed
- Migrations run in version order
- All migrations complete successfully
- Migration history shows 5 migrations applied

### TM-4: Failed Migration Rolls Back
**Scenario**: Migration file contains invalid SQL
**Given**:
- Database at version 2
- Migration 3 has `CREATE TABEL typo` (syntax error)
**When**: Application tries to run migration 3
**Then**:
- Migration 3 does not apply
- Database remains at version 2
- Transaction is rolled back
- Error message indicates which migration failed
- Application can retry after fix

### TM-5: Rollback Reverts Migration
**Scenario**: User needs to undo a migration
**Given**:
- Migration 4 has been applied
- Migration 4 has `@rollback` clause
**When**: User runs `zjj migrate rollback 4`
**Then**:
- Migration 4's down SQL is executed
- `schema_migrations` entry for version 4 is removed
- Database is now at version 3
- Data matches pre-migration state

### TM-6: Migrations Are Idempotent
**Scenario**: Migration runner is called twice
**Given**: Database at version 2, migrations 3-5 pending
**When**:
- First call runs migrations 3-5
- Second call runs migrations (no new files)
**Then**:
- First call applies 3, 4, 5
- Second call applies nothing
- No duplicate migration entries
- No errors on second call

### TM-7: Migration Creation Generates Valid File
**Scenario**: Developer creates new migration
**Given**: Developer runs `zjj migrate create add_user_preferences`
**When**: Command executes
**Then**:
- New file created in `migrations/` directory
- Filename has timestamp prefix: `YYYYMMDD_HHMMSS_add_user_preferences.sql`
- File contains `-- UP:` and `-- DOWN` sections
- File is valid SQL syntax

### TM-8: Concurrent Migration Runs Are Safe
**Scenario**: Multiple zjj processes start simultaneously
**Given**: Database at version 2, two processes start
**When**: Both processes try to run migrations
**Then**:
- Only one process applies migrations
- Other process sees migrations already applied
- No duplicate migration entries
- No "database locked" errors

### TM-9: Migration Checksum Validation
**Scenario**: Migration file is modified after being applied
**Given**:
- Migration 3 applied with checksum ABC123
- Migration file is edited (new checksum DEF456)
**When**: Migration runner checks status
**Then**:
- Warning about checksum mismatch
- Migration not re-run (already applied)
- User can force re-run if needed

### TM-10: Irreversible Migration Cannot Roll Back
**Scenario**: Migration drops a column without rollback
**Given**:
- Migration 4 marked as irreversible (no `@rollback`)
- Migration 4 was applied
**When**: User runs `zjj migrate rollback 4`
**Then**:
- Error: "Migration 4 cannot be rolled back"
- Migration remains applied
- Database unchanged

### TM-11: Complex Migration with Multiple Statements
**Scenario**: Migration has multiple SQL statements
**Given**: Migration file with:
```sql
-- UP
CREATE TABLE new_table (...);
ALTER TABLE old_table ADD COLUMN new_id INTEGER;
-- DOWN
DROP TABLE new_table;
ALTER TABLE old_table DROP COLUMN new_id;
```
**When**: Migration runs
**Then**:
- All statements execute in order
- All statements succeed or all fail (atomic)
- Rollback reverses all statements

### TM-12: Migration Status Report
**Scenario**: User checks migration status
**Given**: Database at version 3, 5 migrations exist
**When**: User runs `zjj migrate status`
**Then**:
- Shows current version: 3
- Lists applied migrations: 1, 2, 3
- Lists pending migrations: 4, 5
- Shows timestamps of when migrations were applied

## Test Implementation Structure

### Test File
```rust
// crates/zjj-core/tests/test_migrations.rs

mod migrator {
    #[tokio::test]
    async fn fresh_database_receives_all_migrations() { /* TM-1 */ }

    #[tokio::test]
    async fn existing_database_gets_migration_table() { /* TM-2 */ }

    #[tokio::test]
    async fn pending_migrations_run_automatically() { /* TM-3 */ }

    #[tokio::test]
    async fn failed_migration_rolls_back() { /* TM-4 */ }

    #[tokio::test]
    async fn rollback_reverts_migration() { /* TM-5 */ }

    #[tokio::test]
    async fn migrations_are_idempotent() { /* TM-6 */ }
}
```

### Test Helpers
```rust
async fn with_fresh_db<F>(test: F) -> Result<()>
where F: FnOnce(SqlitePool) -> Result<()>;

async fn with_migrations<F>(migrations: Vec<&str>, test: F) -> Result<()>
where F: FnOnce(SqlitePool) -> Result<()>;

fn create_migration(name: &str, up: &str, down: &str) -> String;
```

## Integration Test Commands
```bash
# Run all migration tests
moon run :test test_migrations

# Run specific test
moon run :test test_migrations::migrator::failed_migration_rolls_back

# Test with real database
cargo test --test test_migrations -- --nocapture
```

## Success Criteria
- All 12 test scenarios pass
- Migrations run automatically on application start
- `zjj migrate status` shows correct information
- `zjj migrate create` generates valid migration files
- No data loss during migrations
- Failed migrations roll back cleanly
