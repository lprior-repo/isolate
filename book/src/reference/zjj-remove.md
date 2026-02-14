# zjj remove

Delete a workspace.

```bash
zjj remove <name> [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `-f, --force` | Skip confirmation |
| `-k, --keep-tab` | Keep Zellij tab |
| `-j, --json` | JSON output |

## Examples

Interactive (default):
```bash
zjj remove feature-auth
# Proceed? [y/N]:
```

Skip confirmation:
```bash
zjj remove feature-auth --force
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 3 | Not found |
| 4 | Uncommitted changes (without --force) |

## Warnings

- **Cannot be undone**
- Uncommitted changes are lost without `--force`

## Bulk Remove

```bash
# Remove all completed
zjj list --json | jq -r '.[] | select(.status == "completed") | .name' | \
  xargs -I {} zjj remove {} --force
```
