# Basic Workflow

Daily JJ workflow within ZJJ.

## Typical Session

```bash
# 1. Create workspace
zjj add feature-auth --bead BD-123

# 2. Switch to it
zjj focus feature-auth

# 3. Check status
jj status

# 4. Make changes
vim src/auth.rs

# 5. Check status
jj status
# M src/auth.rs

# 6. Commit
jj commit -m "WIP: auth refactor"

# 7. Sync with main
zjj sync

# 8. Land work
zjj done --message "Add OAuth support"
```

## JJ in Workspaces

Inside a ZJJ workspace, use JJ normally:

```bash
jj status       # Check changes
jj diff         # View diff
jj log          # View history
jj commit       # Create commit
jj describe     # Edit message
jj undo         # Undo last operation
```

## Automatic Operations

ZJJ handles:
- Workspace creation
- Rebasing on sync
- Landing to main

You handle:
- Making changes
- Committing
- Conflict resolution

## Common Patterns

**Commit early:**
```bash
jj commit -m "WIP: login form"
jj commit -m "WIP: validation"
jj commit -m "WIP: error handling"
```

**Squash before landing:**
```bash
jj squash --into main  # Combine commits
zjj done
```

**Describe commits:**
```bash
jj describe -m "Clear, descriptive message"
```

## ZJJ vs JJ Commands

Use ZJJ for:
- `zjj add` - Create workspace
- `zjj sync` - Sync with main
- `zjj done` - Land changes

Use JJ for:
- `jj commit` - Commit changes
- `jj status` - Check status
- `jj log` - View history
- `jj diff` - View changes
