# Workspace Management

Isolated workspaces for parallel development.

## Quick Start

```bash
# Create
zjj add feature-auth --bead BD-123

# Switch
zjj focus feature-auth

# Sync
zjj sync

# Finish
zjj done --message "Add auth" --push --remove
```

## Core Workflow

| Action | Command | Purpose |
|--------|---------|---------|
| Create | `zjj add <name>` | New isolated workspace |
| List | `zjj list` | See all workspaces |
| Switch | `zjj focus <name>` | Jump to workspace |
| Check | `zjj whereami` | Confirm location |
| Sync | `zjj sync` | Update from main |
| Complete | `zjj done` | Land to main |
| Remove | `zjj remove <name>` | Clean up |

## Creating Workspaces

Basic:
```bash
zjj add feature-name
```

With tracking:
```bash
zjj add fix-login --bead BD-123
```

For automation (no tab):
```bash
zjj add backend-api --no-tab
```

## Switching Between Workspaces

Direct switch:
```bash
zjj focus feature-auth
```

Check current:
```bash
zjj whereami
# workspace:feature-auth
```

## Keeping Synced

Before major work:
```bash
zjj sync
```

Sync specific workspace:
```bash
zjj sync feature-auth
```

## Completing Work

Simple:
```bash
zjj done
```

Full workflow:
```bash
zjj done --message "Fix auth" --push --remove
```

## Listing & Monitoring

Quick overview:
```bash
zjj list
# NAME         BEAD    STATUS    SYNCED
# feature-auth BD-123  active    2m ago
```

Filter by status:
```bash
zjj list --status conflicted
zjj list --status stale
```

## Best Practices

**Naming:**
```bash
zjj add oauth-implementation      # Good
zjj add temp                      # Bad - too vague
```

**Sync early and often:**
```bash
zjj sync  # Start of day
# ... work ...
zjj sync  # Before major changes
```

**Clean up completed work:**
```bash
zjj list --status completed
zjj remove old-feature --force
```

## Multiple Workspaces

Work on 5 features simultaneously:
```bash
zjj add feature-a --bead BD-101
zjj add feature-b --bead BD-102
zjj add hotfix --bead BD-103

# Context switch in 1 second:
zjj focus feature-a
zjj focus hotfix
zjj focus feature-b
```

## Troubleshooting

**Workspace not found:**
```bash
zjj list
zjj add missing-workspace
```

**Conflicts on sync:**
```bash
zjj sync my-feature
# Error: conflicts

zjj focus my-feature
jj resolve
zjj sync
```

**Can't remove (in use):**
```bash
zjj status my-workspace
zjj remove my-workspace --force
```

## Common Patterns

**Daily workflow:**
```bash
zjj sync --all                    # Morning sync
zjj focus current-feature         # Start work
# ... edit files ...
zjj sync                          # Mid-day sync
# ... more work ...
zjj done --message "WIP"          # End of day
```

**Code review cycle:**
```bash
zjj sync feature                  # Get latest
# ... address feedback ...
zjj done --message "Address review" --push
```

**Hotfix while mid-feature:**
```bash
zjj add hotfix --bead BD-999
zjj focus hotfix
# ... fix ...
zjj done --push --remove
zjj focus my-feature              # Back to work
```
