# zjj focus

Switch to a workspace's Zellij tab.

```bash
zjj focus <name>
```

## Examples

Switch workspace:
```bash
zjj focus feature-auth
```

## Behavior

- Switches to the workspace's Zellij tab
- Creates tab if missing (unless `--no-tab` used)
- Changes working directory to workspace

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 3 | Workspace not found |
| 5 | Zellij not running |

## See Also

- [`zjj whereami`](./zjj-whereami.html) - Check current location
