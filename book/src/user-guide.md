# User Guide

Everything you need to use ZJJ effectively.

---

## Core Concepts

### Session

A named, isolated workspace with optional Zellij tab and bead association.

```
Session = JJ Workspace + Optional Zellij Tab + Optional Bead
```

### Workspace

A complete isolated copy of your repository where you can work without affecting main.

### Bead

A task/issue ID that tracks work through the system (e.g., `BD-123`, `zjj-abc45`).

---

## The Workflow

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│  init   │ -> │   add   │ -> │  work   │ -> │   done  │
└─────────┘    └─────────┘    └─────────┘    └─────────┘
   Setup        Create         Edit           Merge
```

---

## Session Lifecycle

### Create

```bash
zjj add <name> [--bead <id>]
```

Creates an isolated workspace. Optional `--bead` links to a task.

```bash
zjj add feature-auth --bead BD-123
```

### List

```bash
zjj list [--all] [--verbose]
```

Shows all sessions. Use `--all` for completed/failed, `--verbose` for details.

### Focus

```bash
zjj focus <name>      # Inside Zellij - switch tabs
zjj attach <name>     # Outside Zellij - enter session
```

### Status

```bash
zjj status [<name>] [--watch]
```

Shows workspace state. `--watch` updates live.

### Remove

```bash
zjj remove <name> [--force] [--merge]
```

Removes session. `--merge` squashes to main first.

---

## Working in a Session

### Sync with Main

```bash
zjj sync [<name>] [--all]
```

Rebases your workspace onto latest main. Do this frequently.

### Check Location

```bash
zjj whereami
# Returns: main | workspace:<name>
```

### View Changes

```bash
zjj diff [<name>] [--stat]
```

Shows differences between your workspace and main.

### View Bookmarks

```bash
zjj bookmark list
zjj bookmark create <name> [--push]
zjj bookmark delete <name>
```

---

## Completing Work

### done

```bash
zjj done [--message <msg>] [--squash] [--keep-workspace]
```

Merges your workspace to main.

### Undo (if needed)

```bash
zjj undo              # Undo last done
zjj undo --list       # Show undo history
zjj revert <name>     # Revert specific session
```

---

## Queue Coordination

For multi-agent or coordinated workflows.

### Add to Queue

```bash
zjj queue --add <workspace> --bead <id> --priority <1-10>
```

### View Queue

```bash
zjj queue --list
zjj queue --stats
zjj queue --status <workspace>
```

### Process Queue

```bash
zjj queue --next              # Get next item
zjj queue worker --once       # Process one item
zjj queue worker --loop       # Process continuously
```

### Manage Entries

```bash
zjj queue --cancel <id>       # Cancel entry
zjj queue --retry <id>        # Retry failed entry
zjj queue --reclaim-stale     # Reclaim expired leases
```

---

## Recovery & Diagnostics

### Health Check

```bash
zjj doctor [--fix] [--dry-run]
```

Runs diagnostics. `--fix` attempts automatic repair.

### Integrity

```bash
zjj integrity validate <workspace>
zjj integrity repair <workspace> [--force]
```

### Checkpoints

```bash
zjj checkpoint create --description "before major change"
zjj checkpoint list
zjj checkpoint restore <id>
```

### Clean Stale Sessions

```bash
zjj clean [--dry-run] [--force]
zjj prune-invalid [--yes]
```

---

## Templates

Zellij layout templates for different work styles.

```bash
zjj template list
zjj template show <name>
zjj template create <name> --builtin <minimal|standard|full>
```

Built-in templates:
- `minimal` — Single pane
- `standard` — Two panes (editor + terminal)
- `full` — Three panes (editor + terminal + output)
- `split` — Vertical split
- `review` — Code review layout

---

## Configuration

```bash
zjj config                    # View config
zjj config <key>              # View specific key
zjj config <key> <value>      # Set value
```

Key settings:

```toml
[recovery]
policy = "warn"               # silent | warn | fail-fast

[queue]
default_priority = 5
stale_timeout_seconds = 3600

[zellij]
use_tabs = true
```

---

## Common Patterns

### Daily Workflow

```bash
zjj sync --all                # Morning: sync everything
zjj focus current-feature     # Start working
# ... edit files ...
zjj sync                      # Mid-day: sync with main
zjj done -m "WIP: progress"   # End of day: checkpoint
```

### Hotfix While Mid-Feature

```bash
zjj add hotfix-123 --bead BD-999
zjj focus hotfix-123
# ... fix ...
zjj done --push --squash
zjj remove hotfix-123
zjj focus my-feature          # Back to original
```

### Multiple Features in Parallel

```bash
zjj add feature-a --bead BD-101
zjj add feature-b --bead BD-102
zjj add feature-c --bead BD-103

# Switch between them instantly
zjj focus feature-a
zjj focus feature-b
zjj focus feature-c
```

---

## Zellij Integration

ZJJ creates Zellij tabs automatically. Key shortcuts:

| Shortcut | Action |
|----------|--------|
| `Ctrl+p t` | New tab |
| `Ctrl+p n` | Next tab |
| `Ctrl+p h/l` | Navigate tabs |
| `Ctrl+p w` | Close tab |

Use `zjj focus <name>` for direct tab switching.

---

## What to Read Next

- [AI Agent Guide](./ai-guide.md) — For automated workflows
- [Command Reference](./commands.md) — All commands with options
- [Troubleshooting](./troubleshooting.md) — Error codes and fixes
