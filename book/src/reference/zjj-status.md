# zjj status

Show detailed workspace status.

```bash
zjj status [name] [flags]
```

## Flags

| Flag | Description |
|------|-------------|
| `-j, --json` | JSON output |

## Examples

Current workspace:
```bash
zjj status
```

Specific workspace:
```bash
zjj status feature-auth
```

## Output

```
Workspace: feature-auth
Bead: BD-123
Status: active
Path: workspaces/feature-auth

Changes:
  M src/auth.rs
  A tests/auth_test.rs

Sync: 2 minutes ago
Branch: feature-auth@abc123
```

## JSON Output

```bash
zjj status --json
```

```json
{
  "name": "feature-auth",
  "bead_id": "BD-123",
  "status": "active",
  "path": "workspaces/feature-auth",
  "changes": {
    "modified": ["src/auth.rs"],
    "added": ["tests/auth_test.rs"],
    "deleted": []
  },
  "last_sync": "2026-02-14T10:30:00Z"
}
```
