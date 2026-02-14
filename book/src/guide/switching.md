# Switching Workspaces

Move between isolated workspaces instantly.

## Quick Switch

```bash
zjj focus <name>
```

## Examples

Direct switch:
```bash
zjj focus feature-auth
```

Check first:
```bash
zjj whereami
# workspace:feature-auth

zjj focus bugfix-login
```

## What Happens

1. Switches Zellij tab
2. Changes working directory
3. Updates shell context

## Switching Workflow

```bash
# Morning - feature work
zjj focus feature-a
# ... work ...

# Urgent bug
zjj focus hotfix
# ... fix ...
zjj done --push --remove

# Back to feature
zjj focus feature-a
```

## Tab vs Focus

ZJJ manages tabs automatically:

```bash
zjj add my-work
# Creates tab automatically

zjj focus my-work
# Switches to that tab
```

## Troubleshooting

**Tab doesn't exist:**
```bash
zjj focus my-work
# Creates tab if missing
```

**Already in workspace:**
```bash
zjj focus feature-auth
# Already in feature-auth - no change
```

## Keyboard Shortcuts

Inside Zellij:
- `Ctrl+p t` - New tab
- `Ctrl+p n` - Next tab
- `Ctrl+p p` - Previous tab
- `Ctrl+p <number>` - Tab by number

Or use ZJJ:
```bash
zjj focus <name>
```
