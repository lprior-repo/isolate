# Command Reference

Complete reference for all ZJJ commands.

## Workspace Commands

| Command | Description | Common Usage |
|---------|-------------|--------------|
| [`zjj add`](./zjj-add.html) | Create workspace | `zjj add feature --bead BD-123` |
| [`zjj remove`](./zjj-remove.html) | Delete workspace | `zjj remove feature --force` |
| [`zjj list`](./zjj-list.html) | List workspaces | `zjj list --status active` |
| [`zjj status`](./zjj-status.html) | Workspace details | `zjj status [name]` |
| [`zjj focus`](./zjj-focus.html) | Switch to workspace | `zjj focus feature` |
| [`zjj whereami`](./zjj-whereami.html) | Current location | `zjj whereami` |
| [`zjj sync`](./zjj-sync.html) | Sync with main | `zjj sync [name]` |
| [`zjj done`](./zjj-done.html) | Complete work | `zjj done --message "Fix" --push` |

## Queue Commands

| Command | Description | Common Usage |
|---------|-------------|--------------|
| [`zjj work`](./zjj-work.html) | Claim next item | `zjj work --agent id` |
| [`zjj queue`](./zjj-queue.html) | Queue management | `zjj queue add --bead BD-123` |

## Global Flags

Every command supports:

| Flag | Description |
|------|-------------|
| `-j, --json` | JSON output for scripting |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

## Quick Examples

Create and start work:
```bash
zjj add feature-auth --bead BD-123
zjj focus feature-auth
```

Check status:
```bash
zjj list
zjj status
```

Complete and land:
```bash
zjj done --message "Add auth" --push --remove
```

Queue workflow:
```bash
zjj queue add --bead BD-123 --priority 5
zjj work
# ... do work ...
zjj done
```

## Exit Codes

All commands use these exit codes:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Resource not found / already exists |
| 4 | Precondition failed |
| 5 | External dependency missing |

## JSON Output

All commands support `--json`:

```bash
zjj list --json | jq '.[] | select(.status == "active")'
zjj status --json | jq '.changes.modified | length'
```
