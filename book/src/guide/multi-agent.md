# Multi-Agent Workflows

Run multiple AI agents safely in parallel.

## The Problem

Without coordination:
```
Agent 1 ─→ edits file A
Agent 2 ─→ edits file A (conflict!)
Agent 3 ─→ duplicates Agent 1's work
```

With ZJJ:
```
Agent 1 ─→ claims BD-123 ─→ isolated workspace
Agent 2 ─→ claims BD-124 ─→ isolated workspace
Agent 3 ─→ claims BD-125 ─→ isolated workspace
```

## Architecture

```
┌─────────────┐
│   Queue     │ SQLite-backed
├─────────────┤
│  Workers    │ Claim → Work → Done
├─────────────┤
│ Workspaces  │ Isolated JJ workspaces
└─────────────┘
```

## Quick Setup

1. **Initialize:**
```bash
zjj init
```

2. **Add work:**
```bash
zjj queue add --bead BD-101 --priority 5
zjj queue add --bead BD-102 --priority 5
zjj queue add --bead BD-103 --priority 5
```

3. **Start workers:**
```bash
# Terminal 1
zjj queue worker --loop --agent agent-001

# Terminal 2
zjj queue worker --loop --agent agent-002

# Terminal 3
zjj queue worker --loop --agent agent-003
```

Each agent:
- Claims highest priority item
- Creates isolated workspace
- Does work
- Lands changes
- Repeats

## Agent Configuration

Agents are identified by ID:
```bash
zjj work --agent coder-001
zjj queue worker --loop --agent reviewer-002
```

## Monitoring

Watch all agents:
```bash
zjj queue list
# ID  BEAD    STATUS       AGENT
# 1   BD-101  in_progress  agent-001
# 2   BD-102  in_progress  agent-002
# 3   BD-103  pending      -
```

## Safety Guarantees

- **One claim** - Only one agent per item
- **Isolation** - No workspace conflicts
- **Atomic landing** - All-or-nothing commits
- **Recovery** - Stale claims auto-expire

## Scaling

Run 6-12 agents:
```bash
for i in {001..006}; do
  zjj queue worker --loop --agent agent-$i &
done
```

## Best Practices

**Granular work items:**
```bash
# Good: Small, independent
zjj queue add --bead BD-123 --priority 5  # Fix login
zjj queue add --bead BD-124 --priority 5  # Update CSS

# Bad: Large, dependent
zjj queue add --bead BD-999 --priority 5  # Rewrite everything
```

**Priority by impact:**
```bash
zjj queue add --bead BD-100 --priority 9  # Critical bug
zjj queue add --bead BD-101 --priority 5  # Feature
zjj queue add --bead BD-102 --priority 2  # Cleanup
```

**Monitor and adjust:**
```bash
# Check queue health
zjj queue list

# Reclaim stale items
zjj queue reclaim --stale
```

## Troubleshooting

**Agent stuck:**
```bash
# Check agent status
zjj queue list --status in_progress

# Release stale claim
zjj queue reclaim --agent agent-001
```

**Too many conflicts:**
```bash
# Agents working on related code - add dependencies
zjj queue add --bead BD-102 --priority 5 --after BD-101
```
