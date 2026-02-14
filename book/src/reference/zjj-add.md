# zjj add

Create an isolated workspace.

```bash
zjj add <name> [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `-b, --bead <id>` | Associate with bead/issue |
| `-n, --no-tab` | Skip Zellij tab creation |
| `-d, --description <text>` | Add description |
| `-j, --json` | JSON output |

## Examples

Create workspace:
```bash
zjj add feature-auth
```

With issue tracking:
```bash
zjj add fix-login --bead BD-123
```

JSON output:
```bash
zjj add api --json
# {"ok":true,"workspace":{"name":"api","bead_id":null}}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Invalid arguments |
| 3 | Already exists |
| 4 | Not a JJ repo |

## Common Errors

**Already exists:**
```bash
zjj add feature-auth
# Error: Workspace exists
# Fix: zjj remove feature-auth && zjj add feature-auth
```

**Not in JJ repo:**
```bash
# Fix: jj init && zjj init
```
