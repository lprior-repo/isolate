# zjj list

Show all workspaces.

```bash
zjj list [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `-a, --all` | Include archived |
| `-b, --bead <id>` | Filter by bead |
| `-s, --status <state>` | Filter by status |
| `-j, --json` | JSON output |

## Examples

List all:
```bash
zjj list
# NAME           BEAD    STATUS
# feature-auth   BD-123  active
# api-changes    -       syncing
```

Filter by status:
```bash
zjj list --status conflicted
```

JSON for scripting:
```bash
zjj list --json | jq '.[].name'
```

## Status Values

- `active` - Ready
- `syncing` - Sync in progress
- `conflicted` - Needs resolution
- `stale` - Sync recommended
- `completed` - Ready to remove

## Output Columns

```
NAME    BEAD    STATUS    SYNCED    CHANGES
```

- **SYNCED**: Time since last sync
- **CHANGES**: +added ~modified -deleted
