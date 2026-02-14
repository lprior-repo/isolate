# Workspace Management

Complete guide to managing isolated workspaces with ZJJ.

---

## What is a Workspace?

A workspace is an isolated JJ working copy where you can make changes without affecting main. In ZJJ, workspaces are:

- **Isolated** - Changes don't affect other workspaces or main
- **Persistent** - They exist until explicitly removed
- **Synchronized** - Can be kept in sync with main
- **Integrated** - Connected to Zellij tabs for easy switching

---

## Creating Workspaces

### Basic Creation

```bash
zjj add <name>
```

This creates a minimal workspace.

### With Bead Association

```bash
zjj add <name> --bead <BEAD_ID>
```

Associates the workspace with an issue/bead for tracking.

### Example

```bash
# Simple workspace
zjj add feature-auth

# With issue tracking
zjj add fix-login --bead BD-123

# With custom settings
zjj add experiment --no-tab
```

---

## Listing Workspaces

```bash
# Simple list
zjj list

# Verbose (shows paths and bead titles)
zjj list --verbose

# JSON output
zjj list --json
```

---

## Switching Workspaces

```bash
# Direct switch
zjj focus <name>

# Interactive picker
zjj switch

# Via whereami check
zjj whereami  # See where you are
zjj focus other-workspace  # Switch
```

---

## Syncing Workspaces

Keep your workspace up to date with main:

```bash
# Sync specific workspace
zjj sync <name>

# Sync current workspace
zjj sync

# Sync all workspaces
zjj sync --all
```

This rebases your workspace onto the latest main.

---

## Completing Work

When you're done:

```bash
zjj done
```

This:
1. Commits your changes
2. Merges to main  
3. Pushes to remote (if configured)
4. Updates bead status (if associated)

---

## Removing Workspaces

```bash
zjj remove <name>
```

This deletes the workspace and cleans up the session.

---

## Best Practices

### Naming Conventions

- Use descriptive names: `feature-auth-refactor` not `temp`
- Include issue ID: `fix-BD-123-login-bug`
- Use lowercase with hyphens: `my-feature` not `My_Feature`

### Sync Regularly

```bash
# Good: sync before major work
zjj sync my-feature

# Better: sync at start of day
zjj sync --all
```

### Clean Up

Remove workspaces when done:

```bash
zjj remove old-feature
```

Or clean all stale sessions:

```bash
zjj clean
```

---

## Advanced Usage

### Multiple Workspaces

Work on multiple features simultaneously:

```bash
zjj add feature-a --bead BD-101
zjj add feature-b --bead BD-102  
zjj add hotfix-c --bead BD-103

# Switch between them
zjj focus feature-a
# ... work ...
zjj focus hotfix-c
# ... work ...
zjj focus feature-b
```

### Workspace Status

Check detailed status:

```bash
zjj status <name>
```

Shows:
- Branch name
- Changes (M/A/D/R counts)
- Diff statistics (+/- lines)
- Associated bead info

---

## Troubleshooting

### "Workspace not found"

```bash
zjj list  # Check it exists
zjj add my-workspace  # Create if missing
```

### "Conflicts during sync"

```bash
zjj sync my-workspace
# If conflicts occur, resolve in JJ:
jj resolve
# Then try sync again
```

### "Can't remove workspace"

```bash
zjj status my-workspace  # Check if in use
zjj remove my-workspace --force  # Force removal
```

---

## See Also

- **[Creating Sessions](./creating-sessions.md)** - Detailed session creation
- **[Switching Workspaces](./switching.md)** - Advanced switching
- **[Syncing](./syncing.md)** - Sync strategies
- **[Completing Work](./completing.md)** - Finishing up
