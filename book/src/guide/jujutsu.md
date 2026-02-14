# Jujutsu (JJ)

ZJJ uses JJ for version control.

## What is JJ?

[Jujutsu](https://github.com/martinvonz/jj) is a version control system with:
- **Immutable history** - Safe rebase/rewrite
- **Automatic rebase** - Stays up to date
- **Conflicts as first-class** - Commit with conflicts, resolve later
- **Git compatibility** - Works with Git repos

## JJ vs Git

| Concept | Git | JJ |
|---------|-----|-----|
| Commit | `git commit` | `jj commit` |
| Branch | `git checkout` | `jj edit` |
| Rebase | `git rebase` | Automatic |
| Staging | `git add` | Automatic |
| Conflicts | Must resolve | Can commit |

## Essential Commands

**Check status:**
```bash
jj status
```

**Create commit:**
```bash
jj commit -m "Add feature"
```

**View log:**
```bash
jj log
```

**Switch revision:**
```bash
jj edit <commit-id>
```

**Resolve conflicts:**
```bash
jj resolve
```

## In ZJJ Workspaces

ZJJ manages JJ workspaces for you:

```bash
zjj add my-feature
# Creates JJ workspace automatically

zjj focus my-feature
# Switches to that workspace

# Inside, use JJ normally:
jj status
jj commit -m "WIP"

zjj done
# Lands to main
```

## Conflicts

JJ handles conflicts differently:

```bash
# Sync causes conflicts
zjj sync
# Warning: Conflicts in src/auth.rs

# View conflicts
jj status

# Resolve
jj resolve

# Commit the resolution
jj commit -m "Resolve conflicts"
```

## Best Practices

**Commit often:**
```bash
jj commit -m "WIP: login form"
# Later squash:
jj squash --into main
```

**Describe changes:**
```bash
jj describe -m "Clear commit message"
```

**Use ZJJ for coordination:**
```bash
# Don't manually rebase
zjj sync  # Does it safely
```

## Learning More

- [JJ Documentation](https://martinvonz.github.io/jj/)
- [JJ GitHub](https://github.com/martinvonz/jj)
- `jj help` - Built-in help

## See Also

- [JJ Workflow](./jj-workflow.md) - ZJJ-specific patterns
- [Resolving Conflicts](./jj-conflicts.md) - Conflict handling
