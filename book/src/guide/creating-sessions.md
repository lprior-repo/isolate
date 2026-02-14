# Creating Sessions

Sessions are isolated workspaces with Zellij integration.

## What Gets Created

Running `zjj add my-feature` creates:

1. **JJ Workspace** - `workspaces/my-feature/`
2. **Zellij Tab** - Named `my-feature`
3. **Database Entry** - Tracks session state

## Quick Examples

Minimal:
```bash
zjj add quick-fix
```

With tracking:
```bash
zjj add oauth-impl --bead BD-456
```

For automation:
```bash
zjj add agent-work --no-tab
```

## Session Lifecycle

```
Create → Work → Sync → Done → Remove
   ↓       ↓      ↓      ↓       ↓
  add   focus   sync  done  remove
```

## Best Practices

**One task per session:**
```bash
zjj add fix-login-timeout          # Good
zjj add february-work              # Bad - too broad
```

**Link to issues:**
```bash
zjj add api-refactor --bead BD-789
```

**Name descriptively:**
```bash
zjj add add-oauth-support          # Good
zjj add tmp                        # Bad
```

## Common Flags

| Flag | Use Case |
|------|----------|
| `--bead` | Link to issue tracker |
| `--no-tab` | Agent/automation workflows |
| `--description` | Add context for others |

## See Also

- [Workspace Management](./workspaces.md) - Full guide
- [zjj add](../reference/zjj-add.html) - Command reference
