# zjj done

Complete work and land to main.

```bash
zjj done [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `-m, --message <text>` | Commit message |
| `-p, --push` | Push to remote |
| `-r, --remove` | Remove workspace after |
| `-j, --json` | JSON output |

## Examples

Basic completion:
```bash
zjj done
```

With commit message:
```bash
zjj done --message "Add OAuth authentication"
```

Complete workflow:
```bash
zjj done --message "Fix login" --push --remove
```

## What It Does

1. Commits changes (if uncommitted)
2. Rebases onto main
3. Fast-forwards main to workspace
4. Optionally pushes to remote
5. Optionally removes workspace

## Requirements

- Must be in a workspace
- Workspace must be conflict-free
- Main must be at expected commit

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 3 | Conflicts detected |
| 4 | Push failed |
