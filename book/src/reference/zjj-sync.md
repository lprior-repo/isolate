# zjj sync

Sync workspace with main.

```bash
zjj sync [name] [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `-j, --json` | JSON output |

## Examples

Sync current workspace:
```bash
zjj sync
```

Sync specific workspace:
```bash
zjj sync feature-auth
```

## What It Does

1. Rebases workspace onto main
2. Resolves conflicts if possible
3. Updates sync timestamp

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 3 | Conflicts detected |
| 4 | Workspace not found |

## Conflicts

If conflicts occur:
```bash
zjj sync feature-auth
# Error: Conflicts in src/auth.rs

# Resolve:
zjj focus feature-auth
jj resolve
```
