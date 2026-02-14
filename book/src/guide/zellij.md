# Zellij Integration

Terminal multiplexer for workspace tabs.

## What is Zellij?

[Zellij](https://zellij.dev) is a terminal workspace with:
- **Tabs** - Multiple workspaces
- **Panes** - Split terminals
- **Sessions** - Persistent layouts

## ZJJ + Zellij

ZJJ creates a Zellij tab for each workspace:

```bash
zjj add my-feature
# Creates tab "my-feature"

zjj focus my-feature
# Switches to that tab
```

## Basic Controls

| Key | Action |
|-----|--------|
| `Ctrl+p t` | New tab |
| `Ctrl+p n` | Next tab |
| `Ctrl+p p` | Previous tab |
| `Ctrl+p w` | Close tab |
| `Ctrl+p q` | Quit |

## Without Zellij

Disable integration:
```bash
zjj add my-work --no-tab
```

Or in config:
```toml
[zellij]
enabled = false
```

## Tab Management

List tabs:
```bash
# In Zellij: Alt+n to show tab bar
```

Rename (in Zellij):
```bash
Ctrl+p c  # Rename current tab
```

## Multiple Panes

Split terminal in workspace:
- `Ctrl+p -` - Horizontal split
- `Ctrl+p |` - Vertical split
- `Ctrl+p <arrow>` - Navigate

## Sessions

Zellij persists sessions. Reattach:
```bash
zellij attach
```

## Troubleshooting

**Zellij not running:**
```bash
zellij
# Then retry zjj commands
```

**Tab already exists:**
```bash
zjj focus my-work
# Uses existing tab
```
