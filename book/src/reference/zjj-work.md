# zjj work

Start working on the next queue item.

```bash
zjj work [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `--agent <id>` | Claim as specific agent |
| `-j, --json` | JSON output |

## Examples

Claim next item:
```bash
zjj work
# Claimed: BD-123 - Fix authentication bug
# Created workspace: fix-auth-bug
# Switched to Zellij tab
```

As specific agent:
```bash
zjj work --agent agent-001
```

## What It Does

1. Finds highest priority unclaimed queue item
2. Claims it for this agent
3. Creates workspace with bead ID
4. Switches to workspace tab

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | No work available |
| 2 | Claim conflict |

## No Work Available

```bash
zjj work
# No queue items available
```

Add items with `zjj queue --add`.

## See Also

- [`zjj queue`](./zjj-queue.html) - Queue management
- [`zjj done`](./zjj-done.html) - Complete work
