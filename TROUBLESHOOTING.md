# ZJJ Troubleshooting Guide

This guide helps you diagnose and resolve common issues when building, testing, and running ZJJ.

## Table of Contents

1. [Common Build Errors](#common-build-errors)
2. [Common Test Failures](#common-test-failures)
3. [Runtime Issues](#runtime-issues)
4. [Database Issues](#database-issues)
5. [JJ Workspace Issues](#jj-workspace-issues)
6. [Getting Help](#getting-help)
7. [Debugging Tips](#debugging-tips)
8. [Known Issues & Workarounds](#known-issues--workarounds)

---

## Common Build Errors

### Error: `unwrap_used` or `expect_used` compiler error

**Error Message:**
```
error: use of deprecated function `unwrap`: using `unwrap()` on `Result` is not allowed
  --> crates/zjj-core/src/example.rs:42:18
   |
42 |     let value = result.unwrap();
   |                    ^^^^^^^^^^^^
```

**Root Cause:**
ZJJ enforces **Zero Unwrap Law** at the compiler level. Direct use of `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `unimplemented!()` is banned.

**Solution:**

Replace with proper error handling:

```rust
// BAD (causes compile error)
let value = result.unwrap();
let value = result.expect("must not fail");

// GOOD - Use Result propagation
fn example() -> Result<String, Error> {
    let value = result?;
    Ok(value)
}

// GOOD - Use match for branching
match result {
    Ok(value) => println!("{}", value),
    Err(e) => return Err(e),
}

// GOOD - Use combinators
let value = result.ok_or_else(|| Error::NotFound("value missing".into()))?;
let value = result.map(|v| v.to_string()).unwrap_or_default(); // Still banned! Use:
let value = result.map(|v| v.to_string()).map_or(String::new(), |v| v);
```

**Prevention:**
- Always return `Result<T, E>` from fallible operations
- Use `?` operator for error propagation
- Use `match` or `if let` for conditional branching
- Use combinators: `map()`, `and_then()`, `or_else()`, `ok_or_else()`

---

### Error: `cargo` command not allowed

**Error Message:**
```
Error: AGENTS.md rule violation: MOON_ONLY
Direct cargo commands are banned. Use moon run instead.
```

**Root Cause:**
ZJJ uses **Moon** for build orchestration. Direct `cargo` commands bypass Moon's caching and violate project rules.

**Solution:**

```bash
# BAD - Direct cargo usage
cargo build
cargo test
cargo fmt

# GOOD - Use Moon tasks
moon run :build
moon run :test
moon run :fmt
moon run :fmt-fix
moon run :check
moon run :ci
moon run :quick
```

**Available Moon Tasks:**
- `:fmt` - Check code formatting
- `:fmt-fix` - Auto-fix formatting issues
- `:check` - Fast type check (no build artifacts)
- `:test` - Run all tests
- `:build` - Release build
- `:ci` - Full CI pipeline
- `:quick` - fmt + check (fastest dev loop)

**See Also:** [CONTRIBUTING.md](CONTRIBUTING.md)

---

### Error: `sqlx` mismatch detected

**Error Message:**
```
error: database schema mismatch detected
sqlx::query!() macro failed to verify against database
```

**Root Cause:**
The `sqlx` macros validate queries against the actual database schema at compile time. The `.sqlx/` metadata is missing or out of sync.

**Solution:**

```bash
# Option 1: Rebuild with offline mode (recommended for CI)
SQLX_OFFLINE=true moon run :build

# Option 2: Regenerate sqlx metadata (requires database)
cargo sqlx prepare --check

# Option 3: Run with database connected
sqlx database create
sqlx migrate run
```

**Prevention:**
- Keep migrations in sync with code
- Use `SQLX_OFFLINE=true` for builds without database
- Run `cargo sqlx prepare` after schema changes

---

### Error: Dependency version conflicts

**Error Message:**
```
error: failed to select a version for `itertools`
... required by multiple packages with incompatible versions
```

**Root Cause:**
Workspace dependency version constraints conflict.

**Solution:**

```bash
# Update workspace Cargo.toml dependencies
# Edit workspace.package.version or dependency versions

# Clean and rebuild
moon run :clean  # if available
rm -rf target/
moon run :build

# Update lockfile
cargo update
```

**Prevention:**
- Pin versions in workspace `[workspace.dependencies]` section
- Keep workspace crate versions in sync with `workspace.package.version`
- Avoid version constraints like `*` or `>=`

---

## Common Test Failures

### Test Failure: Property test shrunk to minimal case

**Error Message:**
```
[proptest] FAILED: test_session_name_validation
    Test case: [0]
    Minimal failing input: [""]
```

**Root Cause:**
Property test found an invariant violation. The test case was automatically shrunk to the minimal failing input.

**Solution:**

```rust
// Identify the failing case from output
// Example: empty string "" failed validation

// Fix validation logic
impl SessionName {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        if name.is_empty() {
            return Err(DomainError::InvalidInput("name cannot be empty".into()));
        }
        // ... rest of validation
        Ok(Self(name))
    }
}

// Add regression test
#[test]
fn test_session_name_rejects_empty() {
    assert!(matches!(
        SessionName::new(""),
        Err(DomainError::InvalidInput(_))
    ));
}
```

**Debugging Property Tests:**

```bash
# Run with verbose output
moon run :test -- --nocapture session_properties

# Run specific test
moon run :test -- session_properties::test_session_name_validation

# Run with more test cases
PROPTEST_CASES=10000 moon run :test -- session_properties
```

**See Also:** `tests/status_properties.rs`, `tests/session_properties.rs`

---

### Test Failure: Database locked

**Error Message:**
```
Error: DatabaseError("database is locked")
  : SqliteError(DatabaseIsLocked)
```

**Root Cause:**
Multiple tests trying to access the same SQLite database file concurrently.

**Solution:**

```bash
# Run tests sequentially
moon run :test -- --test-threads=1

# Or use unique database per test
# In test setup:
let db_path = format!("/tmp/test_{}.db", uuid::Uuid::new_v4());
```

**Prevention:**
- Use `:memory:` databases for unit tests
- Use unique file paths for concurrent integration tests
- Run tests with `--test-threads=1` if using shared database

---

### Test Failure: JJ workspace already exists

**Error Message:**
```
Error: JjWorkspaceConflict {
    conflict_type: AlreadyExists,
    workspace_name: "test-workspace"
}
```

**Root Cause:**
Test didn't clean up JJ workspace, or two tests are using the same workspace name.

**Solution:**

```rust
// Use unique workspace names per test
use uuid::Uuid;

let workspace_name = format!("test-{}", Uuid::new_v4());

// Clean up in test drop
impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = std::process::Command::new("jj")
            .args(["workspace", "forget", &self.workspace_name])
            .output();
    }
}

// Or run tests sequentially
moon run :test -- --test-threads=1
```

**Prevention:**
- Always clean up JJ workspaces in test teardown
- Use unique names with UUID or timestamps
- Consider `serial_test` crate for tests requiring shared state

---

### Test Failure: Async runtime not available

**Error Message:**
```
Error: No reactor is running
must be called from the context of a Tokio runtime
```

**Root Cause:**
Async function called from synchronous test without Tokio runtime.

**Solution:**

```rust
// BAD - Async in sync test
#[test]
fn test_async_function() {
    let result = async_function().await; // Error!
}

// GOOD - Use tokio::test
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await.unwrap(); // Still banned!
    // Use proper error handling
    let result = async_function().await?;
    Ok(())
}

// GOOD - Block on async in sync test
#[test]
fn test_async_function() {
    let result = tokio::runtime::Runtime::new()
        .unwrap() // Still banned! Use match:
        .expect("runtime creation failed")
        .block_on(async_function())
        .expect("function failed");
}
```

---

## Runtime Issues

### Issue: Command exits with code 1-5

**Error:**
```bash
$ zjj add test-session
Error: VALIDATION_ERROR
Exit code: 1
```

**Root Cause:**
ZJJ uses semantic exit codes to indicate error categories.

**Exit Code Meanings:**
- `1` - Validation error (user input issues)
- `2` - Not found (missing resources)
- `3` - System error (IO, database issues)
- `4` - External command error (JJ, hooks)
- `5` - Lock contention (session locked, timeout)
- `130` - Operation cancelled (SIGINT)

**Solution:**

```bash
# Check error details
zjj add test-session --json | jq

# Run diagnostics
zjj doctor

# Check if resource exists
zjj list

# Check lock status
zjj agents status
```

**See Also:** [Error handling in `crates/zjj-core/src/error.rs`](crates/zjj-core/src/error.rs)

---

### Issue: Session locked by another agent

**Error Message:**
```
Error: SESSION_LOCKED
Session 'feature-auth' is locked by agent 'claude-code-001'
```

**Root Cause:**
Another agent or process holds the lock for this session.

**Solution:**

```bash
# Check lock status
zjj agents status

# Wait and retry (if agent is working)
sleep 10 && zjj add feature-auth

# Force yield lock (if agent crashed)
zjj yield feature-auth

# Claim lock (if safe to do so)
zjj claim feature-auth
```

**Prevention:**
- Don't terminate agents abruptly
- Use proper session cleanup
- Configure lock timeouts appropriately

---

### Issue: JJ command not found

**Error Message:**
```
Error: JJ_COMMAND_ERROR
Failed to create workspace: JJ is not installed or not in PATH.

Install JJ:
  cargo install jj-cli
or:
  brew install jj
or visit: https://github.com/martinvonz/jj#installation
```

**Root Cause:**
JJ (Jujutsu) is not installed or not in PATH.

**Solution:**

```bash
# Install JJ
cargo install jj-cli

# Or via homebrew (macOS)
brew install jj

# Verify installation
jj --version
jj status

# Run ZJJ doctor
zjj doctor
```

**Prevention:**
- Add JJ to PATH in shell config (`.bashrc`, `.zshrc`)
- Verify `zjj doctor` passes before starting work

---

### Issue: Permission denied on database

**Error Message:**
```
Error: IO_ERROR
IO error: Permission denied (os error 13)
```

**Root Cause:**
Database file has wrong permissions or is owned by another user.

**Solution:**

```bash
# Check permissions
ls -la .zjj/state.db

# Fix ownership
sudo chown $USER:$USER .zjj/state.db
chmod 600 .zjj/state.db

# Or recreate database
rm .zjj/state.db
zjj init
```

**Prevention:**
- Don't run `sudo zjj` commands
- Avoid running ZJJ as different users
- Keep `.zjj/` in user-controlled directory

---

## Database Issues

### Issue: Database corruption detected

**Error Message:**
```
Error: DATABASE_ERROR
Database corruption detected at: .zjj/state.db
Recovery policy: warn
```

**Root Cause:**
SQLite database file is corrupted (crash, disk failure, concurrent access).

**Solution:**

```bash
# Run diagnostics
zjj doctor

# Check recovery log
cat .zjj/recovery.log

# Recover with warn mode (default)
ZJJ_RECOVERY_POLICY=warn zjj list

# Recreate database (last resort)
rm .zjj/state.db
zjj init

# Export/import if possible (if beads exist)
zjj export > backup.json
rm .zjj/state.db
zjj init
zjj import < backup.json
```

**Prevention:**
- Don't kill ZJJ processes abruptly
- Avoid concurrent writes to database
- Use proper shutdown (SIGTERM, not SIGKILL)
- Keep `.zjj/` on reliable storage

**See Also:** Recovery policy documentation in [README.md](README.md)

---

### Issue: Migration failed

**Error Message:**
```
Error: DATABASE_ERROR
Migration failed: column "status" already exists
```

**Root Cause:**
Database schema is out of sync with code, or migration was partially applied.

**Solution:**

```bash
# Check current migration version
sqlite3 .zjj/state.db "SELECT version FROM _sqlx_migrations;"

# Force fresh database (caution: deletes data!)
rm .zjj/state.db
zjj init

# Or manually rollback (advanced)
sqlite3 .zjj/state.db
> DROP TABLE IF EXISTS sessions;
> DROP TABLE IF EXISTS _sqlx_migrations;
> .quit
zjj init
```

**Prevention:**
- Keep migrations in sync with code
- Don't modify database schema manually
- Test migrations in development before production

---

## JJ Workspace Issues

### Issue: Workspace already exists in JJ

**Error Message:**
```
Error: JJ_WORKSPACE_CONFLICT
Workspace already exists

Workspace: feature-auth

JJ error: Failed to create workspace: there is already a workspace
```

**Root Cause:**
JJ workspace already exists from previous run or manual creation.

**Solution:**

```bash
# List JJ workspaces
jj workspace list

# Forget conflicting workspace
jj workspace forget feature-auth

# Then retry
zjj add feature-auth

# Or use existing workspace
zjj add --reuse-workspace feature-auth
```

**Prevention:**
- Always use `zjj done` to complete work
- Clean up workspaces with `zjj remove`
- Don't manually create JJ workspaces in ZJJ-managed repos

---

### Issue: Working copy is stale

**Error Message:**
```
Error: JJ_WORKSPACE_CONFLICT
Working copy stale

The working copy is stale. Please update it.
```

**Root Cause:**
JJ working copy is out of sync with repository state.

**Solution:**

```bash
# Update working copy
jj workspace update-stale

# Reload repo
jj reload

# Sync with main
zjj sync feature-auth

# Check status
zjj status feature-auth
```

**Prevention:**
- Run `zjj sync` regularly
- Don't modify `.jj/` directory manually
- Avoid concurrent JJ operations on same workspace

---

### Issue: Concurrent modification detected

**Error Message:**
```
Error: JJ_WORKSPACE_CONFLICT
Concurrent modification detected

Multiple JJ operations detected. Check for running processes.
```

**Root Cause:**
Another `jj` or `zjj` process is running concurrently.

**Solution:**

```bash
# Find running JJ processes
pgrep -fl jj

# Wait for completion or terminate (carefully!)
pkill -9 jj  # Last resort!

# Retry after cleanup
zjj add feature-auth
```

**Prevention:**
- Don't run multiple ZJJ commands on same session concurrently
- Use locks/queues for coordination
- Check `zjj agents status` before starting work

---

## Getting Help

### Diagnostic Commands

```bash
# Full system health check
zjj doctor

# Check environment context
zjj context

# Introspect available commands
zjj introspect

# Query system state
zjj query sessions
zjj query queue
zjj query agents

# View configuration
zjj config list
zjj config get <key>
```

### Verbose Logging

```bash
# Enable verbose output
RUST_LOG=debug zjj add feature-auth

# Trace SQL queries
RUST_LOG=sqlx=debug zjj list

# Log to file
RUST_LOG=debug zjj add feature-auth 2>&1 | tee zjj-debug.log

# Backtrace on panic (if test crashes)
RUST_BACKTRACE=1 moon run :test -- failing_test
```

### Getting Help from Community

1. **Check Documentation**
   - [README.md](README.md) - Quick start and overview
   - [docs/INDEX.md](docs/INDEX.md) - Complete documentation index
   - [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines

2. **Search Existing Issues**
   - GitHub Issues: https://github.com/lprior-repo/zjj/issues
   - Search error messages in issues

3. **Create New Issue**
   Include:
   - ZJJ version: `zjj --version`
   - JJ version: `jj --version`
   - Error message (full output)
   - Steps to reproduce
   - `zjj doctor` output
   - `zjj context` output
   - Relevant logs (with sensitive data redacted)

4. **Ask on GitHub Discussions**
   - Questions that aren't bugs
   - Design discussions
   - Usage help

---

## Debugging Tips

### 1. Enable Detailed Error Context

```bash
# Use --json flag for machine-readable errors
zjj add test-session --json | jq

# This includes:
# - Error code
# - Validation hints
# - Context at failure
# - Fix commands
```

### 2. Check Recovery Log

```bash
# View recent recovery events
cat .zjj/recovery.log

# Follow in real-time
tail -f .zjj/recovery.log

# Check for corruption patterns
grep "corruption" .zjj/recovery.log
```

### 3. Database Inspection

```bash
# Open database for inspection
sqlite3 .zjj/state.db

# List tables
.tables

# Inspect sessions
SELECT * FROM sessions;

# Check migrations
SELECT * FROM _sqlx_migrations;

# Quit
.quit
```

### 4. JJ State Inspection

```bash
# List workspaces
jj workspace list

# Check current operation
jj log

# View conflicts
jj resolve --list

# Show diff
jj diff
```

### 5. Test-Specific Debugging

```bash
# Run single test with output
moon run :test -- --nocapture session_feature::test_create_session

# Run tests with logging
RUST_LOG=debug moon run :test -- session_feature

# Show test output
moon run :test -- --show-output

# Run failing test only
moon run :test -- session_feature::test_specific_failure
```

### 6. Build/Cache Issues

```bash
# Check Moon cache status
moon check

# Clean build artifacts
rm -rf target/

# Restart bazel-remote cache
systemctl --user restart bazel-remote

# Check cache stats
curl http://localhost:9090/status | jq
```

---

## Known Issues & Workarounds

### Issue: Proptest regressions appear in CI

**Symptom:**
Property tests fail in CI but pass locally.

**Workaround:**
```bash
# Update regression files
moon run :test -- --persist FailingTest

# Or regenerate all
rm -f tests/*.proptest-regressions
moon run :test
```

**Root Cause:**
Proptest shrinks failing cases and saves them to `.proptest-regressions` files. These may differ across environments.

**Long-term Fix:**
- Commit regression files to git
- Keep them in sync with test changes

---

### Issue: Zellij session not found

**Symptom:**
`zjj focus` fails with "Zellij session not found".

**Workaround:**
```bash
# List Zellij sessions
zellij list-sessions

# Attach manually
zellij attach zjj-session-name

# Or create without Zellij
zjj add feature-session --no-zellij
```

**Root Cause:**
Zellij not running, or session was created outside ZJJ.

---

### Issue: Dedupe key conflicts

**Symptom:**
```
Error: DEDUPE_KEY_CONFLICT
Dedupe key 'BD-123' already used by workspace 'feature-a'
```

**Workaround:**
```bash
# Remove conflicting entry
zjj remove feature-a

# Or use different dedupe key
zjj queue --add feature-b --bead BD-124

# Or wait for existing to complete
zjj queue --status feature-a
```

**Root Cause:**
Queue enforces unique dedupe keys to prevent duplicate work.

**Prevention:**
- Use unique bead IDs per queue entry
- Remove completed entries from queue
- Check queue status before adding

---

### Issue: Lock timeout under heavy load

**Symptom:**
```
Error: LOCK_TIMEOUT
Lock acquisition timeout for 'claim_session' after 5 retries
```

**Workaround:**
```bash
# Increase timeout in config
zjj config set lock.timeout_ms 10000

# Or wait and retry
sleep 5 && zjj claim test-session

# Check for stuck agents
zjj agents status
zjj yield --all-stale
```

**Root Cause:**
Many agents competing for locks, or agent crash holding lock.

**Prevention:**
- Configure appropriate timeouts for your workload
- Use stale lock reclamation: `zjj queue --reclaim-stale 300`
- Monitor agent health

---

### Issue: Moon cache corruption

**Symptom:**
Build fails with "cache checksum mismatch" or "corrupted cache".

**Workaround:**
```bash
# Restart bazel-remote
systemctl --user restart bazel-remote

# Clear Moon cache
rm -rf .moon/cache/

# Clean rebuild
rm -rf target/
moon run :build
```

**Root Cause:**
bazel-remote cache or Moon cache corrupted.

**Prevention:**
- Keep bazel-remote updated
- Monitor disk space on cache volume
- Restart bazel-remote after system updates

---

### Issue: async drop not supported (Rust 1.80 limitation)

**Symptom:**
Test cleanup doesn't run properly in async contexts.

**Workaround:**
```rust
// Use explicit cleanup instead of Drop
async fn cleanup_test_resources() -> Result<()> {
    // cleanup logic
    Ok(())
}

#[tokio::test]
async fn test_with_cleanup() -> Result<()> {
    // test logic
    cleanup_test_resources().await?;
    Ok(())
}
```

**Root Cause:**
Async destructors not stabilized in Rust 1.80.

**Long-term Fix:**
Upgrade Rust version when async drop stabilizes.

---

## Quick Reference Command Summary

```bash
# Diagnostics
zjj doctor              # Full health check
zjj context             # Environment context
zjj introspect          # Discover capabilities

# Session Troubleshooting
zjj list [--verbose]    # List sessions
zjj status [name]       # Detailed status
zjj whereami            # Current location

# Queue Troubleshooting
zjj queue --list        # List queue entries
zjj queue --status ws   # Entry status
zjj queue --reclaim-stale  # Recover stuck entries

# Agent/Lock Troubleshooting
zjj agents status       # Agent status
zjj yield <session>     # Release lock
zjj claim <session>     # Acquire lock

# Database/Config
zjj config list         # View config
zjj config get <key>    # Get value
zjj config reset        # Reset to defaults

# Development
moon run :doctor        # Run diagnostics
moon run :ci           # Full CI pipeline
moon run :quick        # Fast dev loop
```

---

## Additional Resources

- **[AGENTS.md](AGENTS.md)** - Agent workflow and mandatory rules
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development setup and guidelines
- **[README.md](README.md)** - Project overview and quick start
- **[docs/INDEX.md](docs/INDEX.md)** - Complete documentation
- **[Error Types](crates/zjj-core/src/error.rs)** - All error codes and suggestions

---

## Prevention Checklist

Before starting work, verify:

```bash
# 1. System health
zjj doctor

# 2. Environment setup
zjj context

# 3. Build system working
moon run :quick

# 4. No stuck locks
zjj agents status

# 5. Clean workspace state
jj status
jj workspace list
```

Regular maintenance:

```bash
# Weekly
zjj clean              # Remove stale sessions
zjj doctor --fix       # Auto-fix issues

# After crashes
zjj yield --all-stale  # Release stuck locks
zjj doctor             # Check database integrity

# Before releases
moon run :ci          # Full CI pipeline
zjj doctor            # Production readiness
```

---

This guide is continually updated. If you encounter issues not covered here, please create a GitHub Issue with details from the "Getting Help" section.
