# Queue Operations Runbook

Operational guide for managing the zjj merge queue. Use this when you need to coordinate sequential processing, debug queue issues, or recover from failures.

## Quick Reference

| Situation | Command | Exit Code |
|-----------|---------|-----------|
| View queue | `zjj queue --list` | 0 |
| Add to queue | `zjj queue --add <workspace> --bead <id>` | 0 |
| Get next entry | `zjj queue --next` | 0 (or 2 if empty) |
| Check status | `zjj queue --status <workspace>` | 0 |
| Remove entry | `zjj queue --remove <workspace>` | 0 |
| Cancel entry | `zjj queue --cancel <id>` | 0 |
| Retry failed | `zjj queue --retry <id>` | 0 |
| View stats | `zjj queue --stats` | 0 |
| Reclaim stale | `zjj queue --reclaim-stale [secs]` | 0 |
| Run worker | `zjj queue worker --once` | 0 |

---

## Common Operations

### 1. Monitoring Queue Health

**Check queue status:**
```bash
zjj queue --stats --json
```

**Expected output:**
```json
{
  "total": 12,
  "pending": 5,
  "processing": 2,
  "completed": 3,
  "failed": 2
}
```

**What to watch for:**
- High `failed` count - indicates systemic issues
- Stuck `processing` entries - workers may have crashed
- Growing `pending` queue - may need more workers

**Remediation:**
```bash
# View all entries with status
zjj queue --list --json | jq '.data.entries[] | {id, workspace, status}'

# Check for stuck entries (processing for >30 min)
zjj queue --list --json | jq '.data.entries[] | select(.status == "processing")'
```

---

### 2. Adding Work to Queue

**Standard addition:**
```bash
zjj queue --add feature-branch --bead BD-123 --priority 3
```

**With agent assignment:**
```bash
zjj queue --add feature-branch --bead BD-123 --agent agent-001
```

**Verify addition:**
```bash
zjj queue --status feature-branch
```

**Priority guidelines:**
| Priority | Use Case |
|----------|----------|
| 1 | Critical hotfixes |
| 2 | High priority features |
| 3-5 | Normal work (default: 5) |
| 6-10 | Low priority/backfill |

---

### 3. Processing Queue Entries

**Manual processing (single entry):**
```bash
# Get next pending entry
zjj queue --next --json

# Process it...
# ... do the work ...

# Mark as done (via done command)
zjj done feature-branch
```

**Running a worker:**
```bash
# Process one entry and exit
zjj queue worker --once

# Continuous processing
zjj queue worker --loop

# With callbacks
zjj queue worker --once --on-success "notify-success.sh" --on-failure "notify-failure.sh"
```

**Worker states:**
- `idle` - Waiting for work
- `claiming` - Attempting to claim entry
- `processing` - Actively working
- `completed` - Finished successfully
- `failed` - Error occurred

---

### 4. Handling Failed Entries

**View failed entries:**
```bash
zjj queue --list --json | jq '.data.entries[] | select(.status == "failed")'
```

**Retry a failed entry:**
```bash
# Check if entry is retryable
zjj queue --status-id <entry-id> --json

# Retry if attempts < max_attempts
zjj queue --retry <entry-id>
```

**Manual intervention:**
```bash
# If retry not possible, remove and re-add
zjj queue --remove <workspace>
# Fix the issue
zjj queue --add <workspace> --bead <id>
```

---

### 5. Canceling Entries

**Cancel non-terminal entry:**
```bash
zjj queue --cancel <entry-id>
```

**When to cancel:**
- Work is no longer needed
- Entry was added in error
- Need to stop processing immediately

**Note:** Canceling releases the worker lease immediately. The entry must be in `pending` or `processing` state.

---

### 6. Recovering From Stale Leases

**Detect stale entries:**
```bash
zjj queue --list --json | jq '.data.entries[] | select(.status == "processing") | {id, workspace, updated_at}'
```

**Reclaim entries with expired leases:**
```bash
# Default: 300 seconds (5 minutes)
zjj queue --reclaim-stale

# Custom threshold
zjj queue --reclaim-stale 600  # 10 minutes
```

**When to reclaim:**
- Worker process crashed
- Worker lost network connection
- Entry stuck in `processing` for too long

---

## Troubleshooting Scenarios

### Scenario 1: Queue Not Processing

**Symptoms:**
- `pending` count increasing
- No entries moving to `processing`
- Workers appear idle

**Diagnostic steps:**
```bash
# 1. Check for stuck processing entries
zjj queue --list --json | jq '.data.entries[] | select(.status == "processing")'

# 2. Reclaim stale if found
zjj queue --reclaim-stale

# 3. Verify workers are running
ps aux | grep "zjj queue worker"

# 4. Check worker logs
zjj queue --list --json | jq '.data.entries[] | {id, attempts, last_error}'
```

**Resolution:**
- Reclaim stale entries blocking the queue
- Restart workers if crashed
- Check database connectivity

---

### Scenario 2: All Entries Failing

**Symptoms:**
- `failed` count equals total
- Same error across all entries

**Diagnostic steps:**
```bash
# 1. Check recent error pattern
zjj queue --list --json | jq '.data.entries[] | select(.status == "failed") | .last_error' | sort | uniq -c

# 2. Test a single entry manually
zjj queue --add test-entry --bead TEST-001
zjj queue worker --once
```

**Common causes:**
- Infrastructure failure (database, network)
- Broken deployment (code bug)
- Resource exhaustion (disk full, memory)
- External dependency down

**Resolution:**
```bash
# Fix the root cause first
# Then retry failed entries:
zjj queue --list --json | jq -r '.data.entries[] | select(.status == "failed") | .id' | while read id; do
  zjj queue --retry "$id"
done
```

---

### Scenario 3: Duplicate Entries

**Symptoms:**
- Same workspace appears multiple times
- Work being done twice

**Check for duplicates:**
```bash
zjj queue --list --json | jq -r '.data.entries[].workspace' | sort | uniq -d
```

**Prevention:**
- Check status before adding:
```bash
zjj queue --status <workspace> || zjj queue --add <workspace> --bead <id>
```

**Cleanup:**
```bash
# Remove duplicates (keep the oldest)
zjj queue --list --json | jq '.data.entries[] | select(.workspace == "dup-ws") | {id, created_at}' | jq -s 'sort_by(.created_at) | .[1:] | .[].id' -r | while read id; do
  zjj queue --cancel "$id" 2>/dev/null || zjj queue --remove "$id"
done
```

---

### Scenario 4: Priority Inversion

**Symptoms:**
- Low priority work being processed before high priority
- FIFO order not respected

**Verify ordering:**
```bash
zjj queue --list --json | jq '.data.entries[] | {workspace, priority, created_at}'
```

**Expected behavior:**
- Entries sorted by priority (lower number first)
- Within same priority, sorted by created_at (FIFO)

**If ordering is wrong:**
- Check for database clock skew
- Verify no manual priority updates bypassed logic
- Review worker claim algorithm

---

## Best Practices

### 1. Always Use --json for Scripts

**Good:**
```bash
result=$(zjj queue --next --json)
workspace=$(echo "$result" | jq -r '.data.entry.workspace')
```

**Bad:**
```bash
workspace=$(zjj queue --next | grep "Workspace" | awk '{print $2}')
```

### 2. Check Before Acting

```bash
# Check status before retry
if zjj queue --status-id "$id" --json | jq -e '.data.entry.status == "failed"' >/dev/null; then
  zjj queue --retry "$id"
fi
```

### 3. Handle Empty Queue Gracefully

```bash
next=$(zjj queue --next --json)
if [ $? -eq 2 ]; then
  echo "Queue is empty, sleeping..."
  sleep 60
fi
```

### 4. Monitor Queue Depth

```bash
# Alert if queue grows too large
count=$(zjj queue --stats --json | jq '.data.pending')
if [ "$count" -gt 50 ]; then
  echo "ALERT: Queue depth is $count"
fi
```

### 5. Clean Up Completed Entries

```bash
# Remove old completed entries (run periodically)
zjj queue --list --json | jq -r '.data.entries[] | select(.status == "completed" and .updated_at < (now - 86400)) | .id' | while read id; do
  zjj queue --remove "$id"
done
```

---

## JSON Output Schema

All queue commands support `--json` for machine-readable output:

### List Response
```json
{
  "$schema": "zjj://schemas/queue/list",
  "_schema_version": "1.0",
  "success": true,
  "data": {
    "entries": [
      {
        "id": 123,
        "workspace": "feature-branch",
        "bead_id": "BD-456",
        "status": "pending",
        "priority": 3,
        "agent_id": null,
        "attempts": 0,
        "created_at": "2026-02-14T04:30:00Z",
        "updated_at": "2026-02-14T04:30:00Z"
      }
    ],
    "total": 1,
    "pending": 1,
    "processing": 0,
    "completed": 0,
    "failed": 0
  }
}
```

### Status Response
```json
{
  "$schema": "zjj://schemas/queue/status",
  "_schema_version": "1.0",
  "success": true,
  "data": {
    "exists": true,
    "entry": {
      "id": 123,
      "workspace": "feature-branch",
      "status": "processing",
      "agent_id": "agent-001",
      "attempts": 1,
      "events": [
        {"type": "created", "at": "2026-02-14T04:30:00Z"},
        {"type": "claimed", "at": "2026-02-14T04:31:00Z", "by": "agent-001"}
      ]
    }
  }
}
```

---

## Exit Codes Reference

| Code | Meaning | When to Retry |
|------|---------|---------------|
| 0 | Success | - |
| 1 | Validation error | Fix input, then retry |
| 2 | Not found | Check if entry exists |
| 3 | System error | Wait, then retry |
| 4 | Command error | Check setup |
| 5 | Lock contention | Wait, then retry |
| 130 | Cancelled | User interrupted |

---

## Emergency Procedures

### Clear Queue (Nuclear Option)

**⚠️ WARNING: Destroys all queue state**

```bash
# List all entries
zjj queue --list --json | jq -r '.data.entries[].id' | while read id; do
  zjj queue --remove "$id" 2>/dev/null || zjj queue --cancel "$id" 2>/dev/null
done
```

### Database Reset

If queue database is corrupted:
```bash
# Backup first
cp ~/.zjj/state.db ~/.zjj/state.db.backup.$(date +%s)

# Reset (will lose queue state)
rm ~/.zjj/state.db
zjj init
```

---

## Related Documentation

- [Error Troubleshooting](ERROR_TROUBLESHOOTING.md) - Debug specific errors
- [Beads Workflow](08_BEADS.md) - Issue tracking and triage
- [Testing](07_TESTING.md) - Queue test patterns

---

**Last Updated:** 2026-02-14
**Version:** 1.0
