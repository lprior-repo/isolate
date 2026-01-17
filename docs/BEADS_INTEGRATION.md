# Beads Integration Documentation

## Overview

This project uses [Beads](https://github.com/steveyegge/beads) for AI-native issue tracking. Beads is a git-native, CLI-first issue tracker that lives in your repository alongside your code.

## Requirements

### Software Requirements

- **Beads CLI**: Version 0.1.0 or later
  - Install: `curl -sSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash`
  - Verify: `bd --version`

- **SQLite3**: Required for database operations
  - Most systems have this pre-installed
  - Verify: `sqlite3 --version`

### Repository Setup

Beads must be initialized in your repository:

```bash
# Initialize Beads in the repository
bd init

# Verify initialization
ls -la .beads/
```

Expected `.beads/` structure:
```
.beads/
├── beads.db          # SQLite database (gitignored)
├── issues.jsonl      # JSONL export (committed to git)
├── config.yaml       # Beads configuration
├── bd.sock          # Daemon socket (optional)
├── daemon.pid       # Daemon process ID (optional)
└── README.md        # Beads documentation
```

## Integration Architecture

### Database Schema

The zjj project integrates with Beads via the SQLite database at `.beads/beads.db`.

#### Core Tables

**issues** - Main issue tracking table
```sql
CREATE TABLE issues (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'open',
    priority INTEGER NOT NULL DEFAULT 2,
    issue_type TEXT NOT NULL DEFAULT 'task',
    assignee TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    closed_at DATETIME,
    -- Additional fields omitted for brevity
);
```

**labels** - Issue labels (many-to-many)
```sql
CREATE TABLE labels (
    issue_id TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (issue_id, label),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
```

**dependencies** - Issue relationships (blocks, depends_on, parent)
```sql
CREATE TABLE dependencies (
    issue_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'blocks',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL,
    PRIMARY KEY (issue_id, depends_on_id, type),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
```

### Schema Compatibility

The `zjj-core` Rust library provides read-only access to the Beads database through the `beads` module.

#### Field Mapping

| BeadIssue Field | DB Table | DB Column | Type | Notes |
|----------------|----------|-----------|------|-------|
| `id` | issues | id | TEXT | Primary key |
| `title` | issues | title | TEXT | Required |
| `status` | issues | status | TEXT | Enum: open, in_progress, blocked, deferred, closed |
| `priority` | issues | priority | INTEGER | 0-4 (P0-P4) |
| `issue_type` | issues | issue_type | TEXT | Enum: bug, feature, task, epic, chore, merge-request |
| `description` | issues | description | TEXT | Optional |
| `labels` | labels | label | TEXT[] | Joined from labels table |
| `assignee` | issues | assignee | TEXT | Optional |
| `parent` | dependencies | depends_on_id | TEXT | Where type='parent' |
| `depends_on` | dependencies | depends_on_id | TEXT[] | Where type='blocks' |
| `blocked_by` | dependencies | depends_on_id | TEXT[] | Where type='blocked_by' |
| `created_at` | issues | created_at | DATETIME | ISO 8601 |
| `updated_at` | issues | updated_at | DATETIME | ISO 8601 |
| `closed_at` | issues | closed_at | DATETIME | Optional |

**Note**: The current implementation in `crates/zjj-core/src/beads.rs` uses a simplified query that doesn't join the `labels` and `dependencies` tables. This means:
- `labels` will always be `None`
- `parent`, `depends_on`, and `blocked_by` will always be `None`

This is acceptable for the MVP use case (displaying issue summaries) but should be enhanced for full functionality.

## Usage in Code

### Query Beads Issues

```rust
use zjj_core::beads::{query_beads, BeadFilter, IssueStatus};
use std::path::Path;

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Path::new(".");

    // Query all issues
    let issues = query_beads(workspace)?;
    println!("Found {} issues", issues.len());

    // Filter open issues
    let filter = BeadFilter::new()
        .with_status(IssueStatus::Open);
    let open_issues = filter_issues(&issues, &filter);

    // Get summary
    let summary = summarize(&issues);
    println!("Active: {}, Blocked: {}", summary.active(), summary.blocked);

    Ok(())
}
```

### Available Functions

The `zjj_core::beads` module provides:

- `query_beads(path: &Path) -> Result<Vec<BeadIssue>, BeadsError>`
  - Read all issues from the database

- `filter_issues(issues: &[BeadIssue], filter: &BeadFilter) -> Vec<BeadIssue>`
  - Filter issues by status, type, priority, labels, etc.

- `sort_issues(issues: &[BeadIssue], sort: BeadSort, direction: SortDirection) -> Vec<BeadIssue>`
  - Sort by priority, created_at, updated_at, status, title, id

- `summarize(issues: &[BeadIssue]) -> BeadsSummary`
  - Get counts by status

- `find_ready(issues: &[BeadIssue]) -> Vec<BeadIssue>`
  - Find issues that are open and not blocked

- `find_blocked(issues: &[BeadIssue]) -> Vec<BeadIssue>`
  - Find issues marked as blocked

- `find_blockers(issues: &[BeadIssue]) -> Vec<BeadIssue>`
  - Find issues blocking other issues

- `find_stale(issues: &[BeadIssue], days: u64) -> Vec<BeadIssue>`
  - Find issues not updated in N days

## Common Operations

### Create an Issue

```bash
bd create "Add new feature" --type=feature --priority=P1
```

### Update Status

```bash
bd update zjj-abc --status=in_progress
bd close zjj-abc
```

### Query Issues

```bash
# List all open issues
bd list --status=open

# Show specific issue
bd show zjj-abc

# List with priority filter
bd list --priority=P0,P1

# JSON output for scripting
bd list --json
```

### Sync with Git

```bash
# Sync to remote
bd sync

# Pull changes from remote
bd sync --pull
```

## Testing

### Unit Tests

The beads module includes comprehensive unit tests:

```bash
# Run all beads tests
cargo test --lib beads

# Run with output
cargo test --lib beads -- --nocapture
```

### Integration Tests

Verify Beads integration:

```bash
# Check database exists and is accessible
test -f .beads/beads.db && echo "OK" || echo "FAILED"

# Query issues
sqlite3 .beads/beads.db "SELECT COUNT(*) FROM issues WHERE status != 'closed';"

# Test bd CLI
bd list --status=open
```

### Manual Testing Checklist

- [ ] Database exists at `.beads/beads.db`
- [ ] Can query issues with `bd list`
- [ ] Can create new issues with `bd create`
- [ ] Can update issue status with `bd update`
- [ ] Can close issues with `bd close`
- [ ] Database daemon is running (check `ps aux | grep bd`)
- [ ] Can sync with `bd sync`

## Troubleshooting

### Issue: "Database error: Failed to open beads.db"

**Symptoms**: Code fails with database connection errors

**Solutions**:
1. Verify beads.db exists: `ls -la .beads/beads.db`
2. Check permissions: `stat .beads/beads.db`
3. Ensure Beads is initialized: `bd init`
4. Check if database is locked: `lsof .beads/beads.db`

### Issue: "No issues returned but bd list shows issues"

**Symptoms**: `query_beads()` returns empty array but CLI shows issues

**Solutions**:
1. Check if querying correct workspace path
2. Verify database has non-tombstone issues:
   ```bash
   sqlite3 .beads/beads.db "SELECT COUNT(*) FROM issues WHERE status != 'tombstone';"
   ```
3. Check for database permissions issues

### Issue: "Beads daemon not running"

**Symptoms**: Socket file missing, daemon.pid stale

**Solutions**:
1. Start daemon: `bd daemon start`
2. Check daemon status: `bd daemon status`
3. Check logs: `cat .beads/daemon.log`
4. Kill stale daemon: `bd daemon kill` then restart

### Issue: "Labels and dependencies are always None"

**Symptoms**: `issue.labels` and `issue.depends_on` are always `None`

**Expected Behavior**: This is current limitation of the MVP implementation.

**Solutions**:
1. For MVP use case, this is acceptable
2. For full functionality, enhance `query_beads()` to join labels and dependencies tables:
   ```sql
   SELECT i.*,
          GROUP_CONCAT(DISTINCT l.label) as labels,
          GROUP_CONCAT(DISTINCT d.depends_on_id) as depends_on
   FROM issues i
   LEFT JOIN labels l ON i.id = l.issue_id
   LEFT JOIN dependencies d ON i.id = d.issue_id AND d.type = 'blocks'
   GROUP BY i.id
   ```

### Issue: "Schema version mismatch"

**Symptoms**: Queries fail with unknown column errors

**Solutions**:
1. Update Beads to latest version: `bd upgrade`
2. Check Beads version: `bd --version`
3. Verify schema: `sqlite3 .beads/beads.db ".schema issues"`
4. Re-initialize if needed (backup first): `mv .beads .beads.backup && bd init`

### Issue: "Performance problems with large issue counts"

**Symptoms**: Slow query performance with >1000 issues

**Solutions**:
1. Use filtering at SQL level instead of in-memory
2. Add appropriate indexes to beads.db
3. Use pagination with `limit` and `offset`
4. Consider caching query results

## Best Practices

### Read-Only Access

The zjj integration should **only read** from the Beads database. All writes should go through the `bd` CLI:

✓ **Correct**:
```rust
// Read issues
let issues = query_beads(workspace)?;

// Modify via CLI
Command::new("bd")
    .args(["update", "zjj-abc", "--status=in_progress"])
    .status()?;
```

✗ **Incorrect**:
```rust
// DON'T: Write directly to database
let conn = Connection::open(".beads/beads.db")?;
conn.execute("UPDATE issues SET status='in_progress' WHERE id=?", [id])?;
```

### Error Handling

Always handle database errors gracefully:

```rust
match query_beads(workspace) {
    Ok(issues) => {
        // Process issues
    }
    Err(BeadsError::DatabaseError(_)) => {
        // Database not accessible - maybe not initialized
        eprintln!("Beads not initialized. Run: bd init");
    }
    Err(e) => {
        eprintln!("Failed to query beads: {}", e);
    }
}
```

### Performance

- Cache query results when possible
- Use filters to reduce in-memory processing
- Consider lazy loading for large result sets
- Profile with `EXPLAIN QUERY PLAN` for complex queries

## Future Enhancements

### Planned Improvements

1. **Full Relationship Support**
   - Join labels table for accurate label data
   - Join dependencies table for parent/blocking relationships
   - Add relationship filtering (e.g., "show issues blocking X")

2. **Write Support**
   - Direct database writes (optional)
   - Transaction support
   - Conflict resolution with JSONL

3. **Real-time Updates**
   - Watch for database changes
   - Trigger UI updates on issue changes
   - Integrate with Beads daemon events

4. **Advanced Queries**
   - Full-text search on title/description
   - Complex boolean filters
   - Custom sorting expressions

## References

- [Beads Repository](https://github.com/steveyegge/beads)
- [Beads Documentation](https://github.com/steveyegge/beads/tree/main/docs)
- [zjj-core beads module](../../crates/zjj-core/src/beads.rs)

## Version History

- **2026-01-11**: Initial documentation
  - Documented MVP integration with read-only access
  - Identified schema compatibility requirements
  - Created troubleshooting guide

---

**Last Updated**: 2026-01-11
**Verified With**: Beads v0.1.0, zjj v0.1.0
