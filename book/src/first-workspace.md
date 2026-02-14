# Your First Workspace

Let's create your first isolated workspace and do some work.

---

## Prerequisites

Make sure you have:
- ZJJ installed and verified (`zjj --version`)
- A JJ repository initialized (`jj status` works)
- ZJJ initialized in the repo (`zjj init`)

---

## Step 1: Check Your Location

Always start by checking where you are:

```bash
zjj whereami
```

Expected output:
```
Location: main branch
```

---

## Step 2: Create a Session

```bash
zjj add my-first-feature
```

This creates:
- A new JJ workspace named `my-first-feature`
- A Zellij tab for this workspace
- An entry in the ZJJ database

---

## Step 3: Switch to Your Workspace

```bash
zjj focus my-first-feature
```

This switches your Zellij tab to the workspace. Verify:

```bash
zjj whereami
```

Expected output:
```
Location: workspace 'my-first-feature'
```

---

## Step 4: Make Changes

Now make some changes to your code:

```bash
# Edit a file
echo "# My First Feature" > feature.md

# Check JJ status
jj status
```

You'll see your changes in the workspace.

---

## Step 5: Check Session Status

```bash
zjj status my-first-feature
```

This shows:
- Branch name
- Changes (modified, added, deleted)
- Diff statistics

---

## Step 6: Sync with Main (Optional)

If main has moved forward:

```bash
zjj sync my-first-feature
```

This rebases your workspace onto the latest main.

---

## Step 7: Complete Your Work

When you're done:

```bash
zjj done
```

This:
- Commits your changes
- Merges to main
- Pushes to remote (if configured)

---

## Step 8: Clean Up

```bash
zjj remove my-first-feature
```

This removes the session and workspace.

---

## Common Workflow

Here's the typical daily workflow:

```bash
# Morning: create session for today's work
zjj add fix-bug-123 --bead BD-123

# Focus on it
zjj focus fix-bug-123

# Work...
# ... make changes ...

# Sync with main periodically
zjj sync fix-bug-123

# End of day: finish up
zjj done

# Clean up
zjj remove fix-bug-123
```

---

## Multiple Workspaces

You can have multiple workspaces active:

```bash
# Create multiple sessions
zjj add feature-a --bead BD-101
zjj add feature-b --bead BD-102
zjj add bugfix-c --bead BD-103

# List them all
zjj list

# Switch between them
zjj focus feature-a
zjj focus feature-b
zjj focus bugfix-c

# Or use interactive switch
zjj switch
```

---

## Next Steps

- **[User Guide - Workspaces](./guide/workspaces.md)** - Detailed workspace management
- **[User Guide - Queue](./guide/queue.md)** - Multi-worker coordination
- **[Command Reference](./reference/commands.md)** - All available commands
