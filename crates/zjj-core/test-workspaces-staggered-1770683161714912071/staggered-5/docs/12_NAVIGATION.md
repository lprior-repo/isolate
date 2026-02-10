# Navigation and Workspace Management

ZJJ provides powerful navigation and workspace management commands to help you move between sessions and understand your current location.

## Commands

### zjj whereami

Shows your current location (main branch or workspace name).

**Usage:**
```bash
zjj whereami
```

**Examples:**
```bash
$ zjj whereami
Location: main branch

$ zjj whereami
Location: workspace 'feature-auth'
```

**Use cases:**
- Quick check before starting work
- Verify you're in the right workspace
- Scripts that need to know current location

### zjj switch [name]

Navigate between workspaces. Interactive selector if name omitted.

**Usage:**
```bash
zjj switch [NAME]           # Switch to specific workspace
zjj switch                  # Interactive selector
```

**Examples:**
```bash
$ zjj switch feature-auth
Switched to workspace 'feature-auth'

$ zjj switch
? Select workspace:
  ▸ feature-auth
    bugfix-123
    experiment
```

**Options:**
- Omit name for interactive fuzzy-search selector
- Tab completion available for workspace names

### zjj status

Show all sessions with legend and current workspace indicator.

**Usage:**
```bash
zjj status [NAME]           # Show specific session status
zjj status                  # Show all sessions
zjj status --watch          # Continuous updates
```

**Output:**
```
╭─ SESSIONS ──────────────────────────────────────────────────────────────────╮
│     NAME             STATUS     BRANCH        CHANGES          DIFF          │
├─────────────────────────────────────────────────────────────────────────────┤
│ ▶   feature-auth     active     feature-auth  M:3 A:1 D:0 R:0  +120 -45     │
│     bugfix-123       paused     main          clean            +0 -0        │
╰─────────────────────────────────────────────────────────────────────────────╯

Legend:
  Changes: M=Modified  A=Added  D=Deleted  R=Renamed
  Beads:   O=Open  P=in_Progress  B=Blocked  C=Closed
  BEAD:    Associated bead ID and title
  ▶ = Current workspace
```

**Features:**
- Shows all active sessions with detailed status
- Current workspace marked with ▶ indicator
- File changes breakdown (modified, added, deleted, renamed)
- Diff statistics (insertions/deletions)
- Associated bead information (if any)

## Legend

### Changes
- **M** = Modified
- **A** = Added
- **D** = Deleted
- **R** = Renamed

### Beads
- **O** = Open
- **P** = in_Progress
- **B** = Blocked
- **C** = Closed

### Current Workspace Indicator
- **▶** = Current workspace (shown when you're in a workspace)

## Best Practices

1. **Check location before work**: Run `zjj whereami` to confirm you're in the right place
2. **Use status regularly**: Run `zjj status` to see all active sessions and their states
3. **Switch efficiently**: Use `zjj switch` without arguments for quick interactive selection
4. **Watch mode for monitoring**: Use `zjj status --watch` when waiting for background processes

## Related Commands

- `zjj list` - List all sessions (simpler output than status)
- `zjj list --verbose` - Show workspace paths and bead titles
- `zjj add <name>` - Create new session
- `zjj remove <name>` - Remove session
- `zjj sync` - Sync workspace with main branch
