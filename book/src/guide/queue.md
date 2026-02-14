# Queue Coordination

SQLite-backed queue for work distribution.

## Why Queue?

- **Coordination** - Multiple agents don't duplicate work
- **Priority** - High priority items claimed first
- **Tracking** - Know what's in progress
- **Recovery** - Resume after crashes

## Basic Workflow

Add work:
```bash
zjj queue add --bead BD-123 --priority 5
zjj queue add --bead BD-124 --priority 3
zjj queue add --bead BD-125 --priority 5
```

Claim and work:
```bash
zjj work
# Claimed BD-125 (highest priority)
# ... do work ...
zjj done
```

## Queue States

```
Pending → Claimed → In Progress → Completed
    ↓          ↓           ↓
  (can be  (assigned  (working
  claimed)   to agent)  on it)
```

## Viewing Queue

List all:
```bash
zjj queue list
# ID   BEAD    PRIORITY  STATUS     AGENT
# 1    BD-125  5         claimed    agent-001
# 2    BD-123  5         pending    -
# 3    BD-124  3         pending    -
```

Filter by status:
```bash
zjj queue list --status pending
```

## Priorities

Higher number = higher priority:

- `10` - Critical hotfixes
- `7-9` - High priority features
- `4-6` - Normal work (default: 5)
- `1-3` - Low priority / backlog

## Manual Claim

```bash
# Claim specific item
zjj queue claim 123 --agent agent-001

# Mark complete
zjj queue complete 123
```

## Queue Workers

Run once:
```bash
zjj queue worker --once
```

Run continuously:
```bash
zjj queue worker --loop --agent agent-001
```

## Common Patterns

**Sprint planning:**
```bash
# Add all sprint items
for bead in BD-{101..110}; do
  zjj queue add --bead $bead --priority 5
done

# Start 3 workers
zjj queue worker --loop --agent agent-001 &
zjj queue worker --loop --agent agent-002 &
zjj queue worker --loop --agent agent-003 &
```

**Priority escalation:**
```bash
# Add critical hotfix at top priority
zjj queue add --bead BD-999 --priority 10
# Next worker will claim it first
```

## See Also

- [Queue Workers](./queue-workers.md) - Worker setup
- [Multi-Agent Workflows](./multi-agent.md) - Advanced patterns
