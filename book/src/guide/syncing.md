# Syncing with Main

Keep workspaces up to date.

## Why Sync?

- Get latest changes from main
- Reduce merge conflicts
- Stay current with team

## Basic Sync

Current workspace:
```bash
zjj sync
```

Specific workspace:
```bash
zjj sync feature-auth
```

## When to Sync

**Must sync:**
- Before starting work
- Before landing (`zjj done`)
- When main has significant changes

**Good to sync:**
- Morning start
- After meetings
- Before major refactoring

## What It Does

1. Fetches latest main
2. Rebases workspace onto main
3. Updates sync timestamp

## Sync Conflicts

If conflicts occur:
```bash
zjj sync feature-auth
# Error: Conflicts in src/auth.rs

# Resolve:
zjj focus feature-auth
jj resolve

# Retry sync:
zjj sync
```

## Sync Status

Check last sync:
```bash
zjj list
# NAME         SYNCED
# feature-auth 2h ago  ⚠️ Stale
```

Stale = haven't synced recently (risk of conflicts)

## Bulk Sync

Sync all workspaces:
```bash
for ws in $(zjj list --json | jq -r '.[].name'); do
  echo "Syncing $ws..."
  zjj sync "$ws"
done
```

## Auto-Sync

Configure in `.zjj/config.toml`:
```toml
[core]
auto_sync = true  # Sync before landing
```

## Best Practices

**Sync early:**
```bash
zjj sync  # Start of day
# ... work ...
```

**Sync before big changes:**
```bash
zjj sync
# Now safe to refactor
```

**Handle conflicts immediately:**
```bash
zjj sync
# Has conflicts? Fix now, don't wait
```
