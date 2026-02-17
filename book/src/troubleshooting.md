# Error Codes & Troubleshooting

Everything you need to diagnose and fix issues.

---

## Exit Codes

| Code | Name | Meaning |
|------|------|---------|
| 0 | `SUCCESS` | Command completed successfully |
| 1 | `GENERAL_ERROR` | Unspecified error |
| 2 | `INVALID_ARGS` | Invalid command-line arguments |
| 3 | `ALREADY_EXISTS` | Resource already exists |
| 4 | `NOT_FOUND` | Resource not found |
| 5 | `NOT_JJ_REPO` | Not in a JJ repository |
| 6 | `DATABASE_ERROR` | Database operation failed |
| 7 | `LOCK_CONFLICT` | Resource is locked |
| 8 | `CONFLICT` | Merge conflict detected |
| 9 | `PERMISSION_DENIED` | Insufficient permissions |
| 10 | `WORKSPACE_ERROR` | Workspace operation failed |
| 11 | `ZELLIJ_ERROR` | Zellij integration failed |
| 12 | `QUEUE_ERROR` | Queue operation failed |
| 13 | `VALIDATION_ERROR` | Input validation failed |
| 14 | `RECOVERY_NEEDED` | Corruption detected |

---

## Common Errors

### ERR_NOT_JJ_REPO (5)

**Message:** `Not in a JJ repository`

**Fix:**
```bash
jj init && zjj init
```

Or clone existing:
```bash
jj git clone https://github.com/user/repo.git
cd repo && zjj init
```

---

### ERR_ALREADY_EXISTS (3)

**Message:** `Workspace already exists`

**Fix:**
```bash
# Remove first
zjj remove existing-name

# Or use idempotent flag
zjj add new-name --idempotent
```

---

### ERR_NOT_FOUND (4)

**Message:** `Workspace not found`

**Fix:**
```bash
zjj list  # Check available names
zjj add missing-name
```

---

### ERR_LOCK_CONFLICT (7)

**Message:** `Resource is locked`

**Fix:**
```bash
zjj status <workspace>  # Check who holds lock
# Wait for lock to expire (default: 1 hour)
# Or force release:
zjj unlock <workspace>
```

---

### ERR_CONFLICT (8)

**Message:** `Merge conflict detected`

**Fix:**
```bash
zjj focus <workspace>
jj resolve  # Resolve each conflict
zjj sync    # Verify resolution
```

---

### ERR_DATABASE_ERROR (6)

**Message:** `Database operation failed` or `Database corruption detected`

**Fix:**
```bash
zjj doctor --fix
zjj recover --last
cat .zjj/recovery.log
```

---

### ERR_VALIDATION_ERROR (13)

**Message:** `Invalid input`

**Rules:**
- Workspace names: lowercase, alphanumeric, dashes only
- Must start with a letter
- Max 64 characters

```bash
# Valid
zjj add feature-auth
zjj add fix-123

# Invalid
zjj add Feature-Auth    # Uppercase
zjj add feature_auth    # Underscore
zjj add 123-feature     # Starts with number
```

---

### ERR_ZELLIJ_ERROR (11)

**Message:** `Zellij command failed` or `Zellij not found`

**Fix:**
```bash
# Check installation
zellij --version

# Install if missing
cargo install zellij
# or
brew install zellij

# Or skip Zellij
zjj add <name> --no-zellij
```

---

### ERR_WORKSPACE_ERROR (10)

**Message:** `Workspace operation failed`

**Fix:**
```bash
zjj status <workspace>
jj workspace list

# If corrupted
zjj remove <workspace> --force
zjj add <workspace>
```

---

### ERR_RECOVERY_NEEDED (14)

**Message:** `Corruption detected, recovery required`

**Fix:**
```bash
zjj doctor --fix
zjj recover --diagnose
zjj recover --last
```

---

## JSON Error Format

```json
{
  "ok": false,
  "error": {
    "code": 4,
    "name": "NOT_FOUND",
    "message": "Workspace 'my-feature' not found",
    "details": {
      "workspace": "my-feature",
      "available": ["feature-auth", "feature-api"]
    }
  }
}
```

---

## Troubleshooting by Command

### zjj add

| Error | Fix |
|-------|-----|
| Already exists | Remove first or use `--idempotent` |
| Invalid name | Use lowercase, alphanumeric, dashes |
| Not JJ repo | Run `jj init && zjj init` |

### zjj sync

| Error | Fix |
|-------|-----|
| Conflicts | `jj resolve` in workspace |
| No upstream | Configure remote with `jj git remote add` |
| Stale | Fetch first: `jj git fetch` |

### zjj done

| Error | Fix |
|-------|-----|
| Conflicts | Resolve with `jj resolve` first |
| Not pushed | Use `--push` or push manually |
| No message | Provide `-m "message"` |
| Uncommitted | `jj describe -m "message"` first |

### zjj focus

| Error | Fix |
|-------|-----|
| Not found | Check `zjj list` for exact name |
| No Zellij | Install or use `--no-zellij` |
| Tab exists | Close tab or use existing |

### zjj queue worker

| Error | Fix |
|-------|-----|
| No work | Add items: `zjj queue --add` |
| Claim conflict | Try again, auto-retries next item |
| Lock conflict | Wait or reclaim stale |

---

## Recovery Procedures

### Database Recovery

```bash
# Check health
zjj doctor

# Auto-fix
zjj doctor --fix

# If that fails
zjj recover --diagnose
zjj recover --last

# Check logs
cat .zjj/recovery.log
```

### Workspace Recovery

```bash
# Validate
zjj integrity validate <workspace>

# Repair
zjj integrity repair <workspace> --force

# Restore from backup
zjj integrity backup list
zjj integrity backup restore <id>
```

### Session Recovery

```bash
# List checkpoints
zjj checkpoint list

# Restore checkpoint
zjj checkpoint restore <id>
```

---

## Diagnostics

### Health Check

```bash
zjj doctor --verbose
```

### System State

```bash
zjj context --json
zjj introspect --ai --json
```

### Event Log

```bash
zjj events --follow
zjj events --session <name>
```

### Query State

```bash
zjj query session-exists <name>
zjj query session-count
zjj query can-run
```

---

## Clean Up

### Stale Sessions

```bash
# Preview
zjj clean --dry-run

# Remove
zjj clean --force
```

### Invalid Sessions

```bash
# Preview
zjj prune-invalid --dry-run

# Remove all
zjj prune-invalid --yes
```

### Orphaned Workspaces

```bash
jj workspace list
jj workspace forget <name>
zjj clean --force
```

---

## Getting Help

1. **Diagnostics:** `zjj doctor`
2. **Logs:** `.zjj/recovery.log`
3. **Context:** `zjj context --json`
4. **GitHub:** [Report issues](https://github.com/lprior-repo/zjj/issues)
