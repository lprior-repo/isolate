# Beads (Issues)

Track work with bead/issue IDs.

## What are Beads?

Beads are work items tracked in your issue system:
- GitHub Issues
- Jira tickets
- Linear issues
- Custom tracker

## Linking Workspaces

Associate workspace with bead:
```bash
zjj add fix-login --bead BD-123
```

## Benefits

- **Traceability** - Know which code fixes which issue
- **Status sync** - Update bead when done
- **Branch naming** - Auto-named from bead
- **Reporting** - Track time per bead

## Workflow

```bash
# Get issue ID from tracker
# e.g., BD-123 Fix login timeout

# Create linked workspace
zjj add fix-login --bead BD-123

# Do work...

# Complete - updates bead
zjj done --message "Fix login timeout (#BD-123)"
```

## Queue Integration

Add beads to queue:
```bash
zjj queue add --bead BD-123 --priority 5
```

Workers claim by bead:
```bash
zjj work
# Claimed: BD-123
```

## Listing by Bead

Find workspace for bead:
```bash
zjj list --bead BD-123
```

## Best Practices

**Always link issues:**
```bash
zjj add my-work --bead BD-456  # Good
zjj add my-work                 # Bad - untracked
```

**Use bead ID in commit:**
```bash
zjj done --message "Fix timeout (BD-123)"
```

## Bead-First Workflow

```bash
# 1. Create bead in tracker
# 2. Add to queue
zjj queue add --bead BD-123 --priority 5

# 3. Worker picks it up
zjj work

# 4. Complete
zjj done
# Auto-updates bead status
```
