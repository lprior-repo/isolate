# Queue Workers

Automated agents that process queue items.

## What is a Worker?

A process that:
1. Claims next queue item
2. Creates workspace
3. Does work (via script/agent)
4. Lands changes
5. Repeats

## Starting Workers

Single run:
```bash
zjj queue worker --once
```

Continuous:
```bash
zjj queue worker --loop
```

With agent ID:
```bash
zjj queue worker --loop --agent agent-001
```

## Worker Lifecycle

```
Start → Claim → Work → Done → (loop)
         ↓       ↓      ↓
      Create  Agent  Land
      Workspace Logic  Changes
```

## Multiple Workers

Run 3 parallel workers:
```bash
# Terminal 1
zjj queue worker --loop --agent agent-001

# Terminal 2
zjj queue worker --loop --agent agent-002

# Terminal 3
zjj queue worker --loop --agent agent-003
```

## Monitoring

Watch progress:
```bash
zjj queue list
# ID  BEAD    STATUS       AGENT
# 1   BD-101  in_progress  agent-001
# 2   BD-102  in_progress  agent-002
# 3   BD-103  pending      -
```

## Worker Script

Workers can run custom logic:

```bash
# ~/.zjj/worker.sh
#!/bin/bash
set -e

# Get bead ID from env
BEAD_ID=$ZJJ_BEAD_ID

# Do work based on bead
case $BEAD_ID in
  BD-101) ./fix-login.sh ;;
  BD-102) ./add-feature.sh ;;
  *) echo "Unknown: $BEAD_ID"; exit 1 ;;
esac
```

## Stopping Workers

Graceful shutdown:
```bash
Ctrl+C
```

Or kill specific agent:
```bash
kill <agent-pid>
```

## Troubleshooting

**Worker stuck:**
```bash
# Check what it's doing
zjj queue list --status in_progress

# Reclaim if crashed
zjj queue reclaim --agent agent-001
```

**No work available:**
```bash
zjj queue worker --once
# No queue items - exits cleanly
```

## See Also

- [Queue Coordination](./queue.md)
- [Multi-Agent Workflows](./multi-agent.md)
