# Failure Taxonomy and Remediation Table

Complete catalog of all failure modes in the zjj system, classified by retryability with remediation steps.

## Retryability Classification

| Class | Symbol | Description | Retry Action |
|-------|--------|-------------|--------------|
| **Retryable** | 🔄 | Transient failures that may succeed on retry | Automatic retry with backoff |
| **Terminal** | 🛑 | Permanent failures that will not succeed on retry | Manual intervention required |

---

## Category 1: Input Validation Failures

All validation failures are **Terminal** - fixing input is the only solution.

| Error Code | Exit | Retryable | Cause | Remediation |
|------------|------|-----------|-------|-------------|
| `VALIDATION_ERROR` | 1 | 🛑 | Invalid user input (session name, parameters) | Fix input to match format requirements |
| `INVALID_CONFIG` | 1 | 🛑 | Malformed configuration file | Fix TOML syntax or invalid values |
| `PARSE_ERROR` | 1 | 🛑 | JSON/TOML parse failure | Fix syntax errors in the input file |

### Remediation Commands

```bash
# For VALIDATION_ERROR
zjj add valid-session-name    # Must match: ^[a-zA-Z][a-zA-Z0-9_-]*$

# For INVALID_CONFIG
zjj config list              # View current config
zjj config reset            # Reset to defaults
zjj config edit            # Edit config file

# For PARSE_ERROR
jq . <file.json>           # Validate JSON
zjj doctor                 # Check TOML syntax
```

---

## Category 2: Resource Not Found

These are **Terminal** - the resource does not exist.

| Error Code | Exit | Retryable | Cause | Remediation |
|------------|------|-----------|-------|-------------|
| `NOT_FOUND` | 2 | 🛑 | Session/workspace doesn't exist | Create the resource or check name |
| `SESSION_NOT_FOUND` | 2 | 🛑 | Specific session not in database | Verify session exists with `zjj list` |

### Remediation Commands

```bash
zjj list                     # List all sessions
zjj add <session-name>       # Create missing session
zjj context                  # Check current context
```

---

## Category 3: System I/O Failures

Mixed retryability - some are transient, others are permanent.

| Error Code | Exit | Retryable | Cause | Remediation |
|------------|------|-----------|-------|-------------|
| `IO_ERROR` | 3 | 🔄 | Filesystem operation failed | Check permissions, disk space |
| `DATABASE_ERROR` | 3 | 🔄 | SQLite operation failed | Run diagnostics, may need repair |

### Remediation Commands

```bash
# For IO_ERROR
ls -la <path>               # Check file/directory exists
chmod 755 <path>            # Fix permissions
df -h                       # Check disk space

# For DATABASE_ERROR
zjj doctor                  # Check database health
zjj doctor --fix           # Attempt automatic repair
br sync                    # Sync and rebuild
```

### Retry Policy

- **IO_ERROR**: Retry up to 3 times with exponential backoff (1s, 2s, 4s)
- **DATABASE_ERROR**: Retry up to 3 times, then escalate to manual repair

---

## Category 4: External Command Failures

These involve external tools (JJ, Zellij, hooks).

| Error Code | Exit | Retryable | Cause | Remediation |
|------------|------|-----------|-------|-------------|
| `COMMAND_ERROR` | 4 | 🔄 | External command not found | Install command or fix PATH |
| `JJ_COMMAND_ERROR` | 4 | 🔄 | JJ CLI failed | Check JJ installation and status |
| `JJ_WORKSPACE_CONFLICT` | 4 | 🛑 | JJ workspace conflict | Resolve conflict manually |
| `HOOK_FAILED` | 4 | 🛑 | Hook script returned error | Fix hook script or skip with `--no-hooks` |
| `HOOK_EXECUTION_FAILED` | 4 | 🛑 | Hook script not executable | Make hook executable |

### Remediation Commands

```bash
# For COMMAND_ERROR
which <command>            # Check command exists
export PATH=$PATH:<dir>    # Add to PATH

# For JJ_COMMAND_ERROR
jj --version               # Verify JJ installed
jj status                  # Check JJ working
cargo install jj-cli       # Install JJ

# For JJ_WORKSPACE_CONFLICT
jj workspace list          # List workspaces
jj workspace forget <name> # Forget problematic workspace
jj workspace update-stale  # Update stale workspace

# For HOOK_FAILED / HOOK_EXECUTION_FAILED
zjj config list hooks      # List hook configuration
chmod +x ~/.zjj/hooks/*     # Make hooks executable
zjj add <session> --no-hooks  # Skip hooks for debugging
```

### JJ Workspace Conflict Types

| Conflict Type | Cause | Resolution |
|---------------|-------|------------|
| `AlreadyExists` | Workspace name in use | Use different name or forget existing |
| `ConcurrentModification` | Multiple JJ operations | Wait for other operations, then retry |
| `Abandoned` | Workspace was abandoned | Forget and recreate |
| `Stale` | Working copy out of sync | Run `jj workspace update-stale` |

---

## Category 5: Concurrency / Locking

Session locking errors indicate contention for resources.

| Error Code | Exit | Retryable | Cause | Remediation |
|------------|------|-----------|-------|-------------|
| `SESSION_LOCKED` | 5 | 🔄 | Another agent holds lock | Wait or yield the lock |
| `NOT_LOCK_HOLDER` | 5 | 🛑 | You don't hold the lock | Claim lock or wait for holder |
| `LOCK_TIMEOUT` | 5 | 🔄 | Couldn't acquire lock in time | Wait for system to settle |

### Remediation Commands

```bash
zjj agents status           # See all agent locks
zjj claim <session>        # Claim available lock
zjj yield <session>        # Release lock you hold
zjj agent kill <agent-id>  # Force-kill stuck agent
```

### Retry Policy

- **LOCK_TIMEOUT**: Retry up to 5 times with backoff (100ms, 200ms, 400ms, 800ms, 1600ms)
- **SESSION_LOCKED**: Wait for holder to release, then retry

---

## Category 6: Worker Queue Errors

These occur during background task processing.

| Error Type | Retryable | Cause | Remediation |
|------------|-----------|-------|-------------|
| `MergeConflict` | 🛑 | JJ merge conflict | Resolve conflicts manually |
| `WorkspaceNotFound` | 🛑 | Invalid workspace path | Recreate workspace |
| `GitOperationFailed` | 🛑 | Git command failed | Check git status manually |
| `IoError` | 🔄 | Transient I/O issue | Retry operation |
| `DatabaseError` | 🔄 | Database locked/busy | Retry with backoff |
| `LockContention` | 🔄 | Resource locked | Wait and retry |
| `Timeout` | 🔄 | Operation took too long | Retry with longer timeout |
| `ServiceUnavailable` | 🔄 | External service down | Wait for service |
| `ValidationError` | 🛑 | Invalid input to worker | Fix input data |
| `ConfigurationError` | 🛑 | Bad worker config | Fix configuration |
| `PermissionDenied` | 🛑 | Access denied | Fix permissions |

### Retry Policy for Workers

```rust
// Retryable errors: max 3 attempts with exponential backoff
const MAX_RETRY_ATTEMPTS: i32 = 3;
const BASE_BACKOFF_MS: u64 = 1000;

// Terminal errors: no retry, mark as failed immediately
// Examples: MergeConflict, WorkspaceNotFound, ValidationError
```

---

## Category 7: Operation Cancellation

| Error Code | Exit | Retryable | Cause | Remediation |
|------------|------|-----------|-------|-------------|
| `OPERATION_CANCELLED` | 130 | 🛑 | User interrupted (Ctrl+C) | Re-run operation if needed |

---

## Error Pattern Matching

For unknown errors, the system uses pattern matching to classify them:

### Terminal Patterns (default)

```
conflict, not in a workspace, workspace not found,
validation failed, invalid config, permission denied,
access denied, authentication failed, not authorized,
branch diverged, no such file, does not exist,
malformed, corrupt, invalid format, parse error, syntax error
```

### Retryable Patterns

```
timeout, timed out, connection refused, connection reset,
network unreachable, temporarily unavailable, resource temporarily,
would block, try again, database is locked, sqlite_busy,
too many connections, rate limit, throttl, backoff, retry,
transient, interrupted, deadline exceeded
```

---

## Quick Reference: Exit Codes

| Exit Code | Category | Retryable |
|-----------|----------|-----------|
| 1 | Validation | 🛑 |
| 2 | Not Found | 🛑 |
| 3 | System (IO/DB) | 🔄 |
| 4 | External Commands | Mixed |
| 5 | Lock Contention | 🔄 |
| 130 | Cancelled | 🛑 |

---

## See Also

- [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md) - Detailed error code reference
- [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - Error handling patterns
