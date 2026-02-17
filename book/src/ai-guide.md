# AI Agent Guide

Complete guide for AI agents using ZJJ. Start here.

---

## 7 Mandatory Rules

```
1. Never use cargo — use moon run :check|:test|:build|:quick|:ci
2. Never use unwrap/expect/panic — Result<T,E> everywhere
3. Never modify lint config — fix code, not lint
4. Never skip git push — not done until pushed
5. After br sync — git add .beads/ && git commit
6. Load functional-rust-generator skill for all Rust
7. Manual testing mandatory — test actual behavior
```

---

## Quick Reference

| What | Command |
|------|---------|
| Where am I? | `zjj whereami` |
| Who am I? | `zjj whoami` |
| System state | `zjj context` |
| Can I do X? | `zjj can-i <action>` |
| What commands? | `zjj introspect` |
| Command contract | `zjj <cmd> --contract` |
| Execution hints | `zjj <cmd> --ai-hints` |

---

## The 7-Step Workflow

```
1. ORIENT     → zjj whereami
2. REGISTER   → zjj agents register
3. ISOLATE    → zjj work <name> --bead <id>
4. ENTER      → cd $(zjj context --field=workspace.path)
5. IMPLEMENT  → moon run :check && moon run :test
6. HEARTBEAT  → zjj agents heartbeat -c "building"
7. COMPLETE   → zjj done -m "message" && git push
```

---

## Starting Work

### Register as Agent

```bash
zjj agents register
# Sets ZJJ_AGENT_ID env var
```

### Create Workspace

```bash
# Manual session
zjj add <name> --bead <id> --no-zellij --idempotent

# Or unified work command
zjj work <name> --bead <id> --no-agent --idempotent
```

### Check Context

```bash
zjj context --json
```

Returns:
```json
{
  "ok": true,
  "repository": { "branch": "main", "root": "/path" },
  "workspace": { "name": "feature-auth", "path": "/path/.jj/workspaces/feature-auth" },
  "bead": { "id": "BD-123", "status": "in_progress" }
}
```

---

## During Work

### Heartbeats

```bash
zjj agents heartbeat -c "running tests"
```

Send every 30-60 seconds to indicate liveness.

### Sync with Main

```bash
zjj sync
```

Do before major changes and before done.

### Check Status

```bash
zjj status --json
```

---

## Completing Work

### Pre-flight Checks

```bash
moon run :check
moon run :test
zjj sync
```

### Merge to Main

```bash
zjj done -m "Implement feature X" --squash
```

### Push

```bash
git push
# Or with done:
zjj done -m "msg" --push
```

### Update Bead

```bash
br update <bead-id> --status done
br sync --flush-only
git add .beads/ && git commit -m "sync beads"
git push
```

---

## Queue Operations

### For Multi-Agent Coordination

```bash
# Add work to queue
zjj queue --add feature-a --bead BD-101 --priority 5

# Claim next item
zjj queue --next

# Process continuously
zjj queue worker --loop --interval 30

# Check status
zjj queue --list --json
zjj queue --stats
```

### Queue States

| State | Meaning |
|-------|---------|
| `pending` | Waiting to be processed |
| `processing` | Claimed by a worker |
| `done` | Successfully completed |
| `failed_retryable` | Failed, can retry |
| `failed_terminal` | Failed, needs human |

---

## Error Handling

### Exit Codes

| Code | Name | Action |
|------|------|--------|
| 0 | SUCCESS | Continue |
| 1 | GENERAL_ERROR | Check message |
| 2 | INVALID_ARGS | Fix arguments |
| 3 | ALREADY_EXISTS | Use --idempotent |
| 4 | NOT_FOUND | Check name |
| 7 | LOCK_CONFLICT | Wait or force |
| 8 | CONFLICT | Resolve manually |

### Recovery

```bash
zjj doctor --fix
zjj integrity repair <workspace>
```

---

## JSON Output

All commands support `--json`:

```bash
zjj list --json
zjj status --json
zjj done --json
```

Standard response format:

```json
{
  "ok": true,
  "data": { ... },
  "error": null,
  "links": [...]
}
```

---

## Environment Variables

| Variable | Set By | Purpose |
|----------|--------|---------|
| `ZJJ_AGENT_ID` | `agents register` | Agent identity |
| `ZJJ_SESSION` | `work` | Current session |
| `ZJJ_BEAD_ID` | `work --bead` | Current bead |

---

## Introspection Commands

```bash
# All capabilities
zjj introspect --ai --json

# Specific command contract
zjj done --contract

# Execution hints
zjj sync --ai-hints

# Query system state
zjj query session-exists feature-x
zjj query session-count
zjj query can-run

# Permission check
zjj can-i create workspace
```

---

## Complete Workflow Example

```bash
# 1. Register
zjj agents register --id agent-001

# 2. Check state
zjj context --json

# 3. Create workspace
zjj work feature-auth --bead BD-123 --no-zellij --idempotent

# 4. Enter workspace
cd $(zjj context --field=workspace.path)

# 5. Do work
moon run :check
moon run :test

# 6. Heartbeat
zjj agents heartbeat -c "testing"

# 7. Sync
zjj sync

# 8. Complete
zjj done -m "Add authentication" --push --squash

# 9. Update bead
br update BD-123 --status done
br sync --flush-only
git add .beads/ && git commit -m "sync"
git push
```

---

## Parallel Agents

Run multiple agents safely:

```bash
# Agent 1
zjj work feature-a --bead BD-101 --agent-id agent-001

# Agent 2 (separate process)
zjj work feature-b --bead BD-102 --agent-id agent-002

# Each gets isolated workspace
# Queue coordinates merge order
```

---

## See Also

- [User Guide](./user-guide.md) — Human workflow details
- [Command Reference](./commands.md) — All commands
- [Troubleshooting](./troubleshooting.md) — Error resolution
