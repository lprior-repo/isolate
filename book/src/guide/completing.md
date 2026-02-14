# Completing Work

Finish and land your changes.

## Quick Done

```bash
zjj done
```

This commits, merges to main, and pushes.

## Options

With message:
```bash
zjj done --message "Add OAuth support"
```

Full workflow:
```bash
zjj done --message "Fix login" --push --remove
```

## What Happens

1. Commits uncommitted changes
2. Syncs with main (rebase)
3. Fast-forwards main
4. Optionally pushes
5. Optionally removes workspace
6. Updates bead status

## Requirements

- Must be in a workspace (not main)
- Workspace must be conflict-free
- Must have permission to push (if --push)

## Common Patterns

**Simple completion:**
```bash
zjj done
```

**Ready to merge:**
```bash
zjj done --message "Feature complete" --push
```

**Full cleanup:**
```bash
zjj done --message "Done" --push --remove
```

## After Done

Check status:
```bash
zjj whereami
# main

jj log
# Shows your commit on main
```

## If Done Fails

**Conflicts:**
```bash
zjj done
# Error: Conflicts detected

jj resolve
zjj done
```

**Push failed:**
```bash
zjj done --push
# Error: Push rejected

# Pull latest:
jj git fetch
zjj sync
zjj done --push
```

## See Also

- [`zjj done`](../reference/zjj-done.html) - Command reference
- [Workspace Management](./workspaces.md) - Full guide
