# zjj queue

Manage the work queue.

```bash
zjj queue [subcommand] [flags]
```

## Subcommands

| Command | Description |
|---------|-------------|
| `list` | Show queue items |
| `add` | Add item to queue |
| `claim` | Claim an item |
| `complete` | Mark item done |
| `worker` | Start queue worker |

## zjj queue list

```bash
zjj queue list [flags]
```

Flags:
- `--status <state>` - Filter by status
- `-j, --json` - JSON output

Example:
```bash
zjj queue list
# ID     BEAD    PRIORITY  STATUS    AGENT
# 101    BD-123  5         claimed   agent-001
# 102    BD-124  3         pending   -
```

## zjj queue add

```bash
zjj queue add --bead <id> [flags]
```

Flags:
- `-b, --bead <id>` - **Required.** Bead/issue ID
- `-p, --priority <n>` - Priority (1-10, default: 5)
- `-a, --agent <id>` - Assign to agent
- `-d, --description <text>` - Description

Example:
```bash
zjj queue add --bead BD-123 --priority 5
```

## zjj queue claim

```bash
zjj queue claim <id> [flags]
```

Example:
```bash
zjj queue claim 101 --agent agent-001
```

## zjj queue complete

```bash
zjj queue complete <id>
```

Mark a claimed item as complete.

## zjj queue worker

```bash
zjj queue worker [flags]
```

Flags:
- `--once` - Process one item and exit
- `--loop` - Run continuously
- `--agent <id>` - Worker agent ID

Example:
```bash
# Process one item
zjj queue worker --once

# Continuous worker
zjj queue worker --loop --agent agent-001
```

## Queue States

- `pending` - Waiting to be claimed
- `claimed` - Assigned to agent
- `in_progress` - Agent working
- `completed` - Done
- `failed` - Error occurred
