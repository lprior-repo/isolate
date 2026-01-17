# ZJJ Uninstall and Cleanup Guide

Complete guide for removing ZJJ and cleaning up all associated files, state, and dependencies.

## Table of Contents

- [Quick Uninstall](#quick-uninstall)
- [Complete Cleanup](#complete-cleanup)
  - [1. Remove ZJJ Sessions](#1-remove-zjj-sessions)
  - [2. Remove ZJJ Binary](#2-remove-zjj-binary)
  - [3. Remove Repository Data](#3-remove-repository-data)
  - [4. Remove JJ Workspaces](#4-remove-jj-workspaces)
  - [5. Remove Global Configuration](#5-remove-global-configuration)
  - [6. Clean Zellij Integration](#6-clean-zellij-integration)
- [Remove Dependencies](#remove-dependencies)
- [Verification](#verification)
- [Selective Cleanup](#selective-cleanup)
- [Troubleshooting](#troubleshooting)

## Quick Uninstall

For a fast uninstall (removes binary only, preserves data):

```bash
# If installed via cargo
cargo uninstall zjj

# If installed manually
sudo rm /usr/local/bin/jjz

# Verify removal
which jjz  # Should return nothing
```

**Note**: This leaves repository data intact. See [Complete Cleanup](#complete-cleanup) for full removal.

## Complete Cleanup

### 1. Remove ZJJ Sessions

Before uninstalling, clean up active sessions to prevent orphaned resources.

```bash
# List all sessions
jjz list

# Remove each session individually (preserves branches)
jjz remove <session-name>

# Or remove all sessions with cleanup
for session in $(jjz list --json | jq -r '.[].name'); do
    jjz remove "$session" --force
done
```

**What this does**:
- Removes session records from database
- Cleans up Zellij tabs (if inside Zellij)
- Optionally merges or abandons JJ workspaces

**Note**: Use `--merge` to preserve work, or `--force` to discard.

### 2. Remove ZJJ Binary

Remove the `jjz` executable from your system.

**If installed via cargo**:
```bash
cargo uninstall zjj
```

**If installed via pre-built binary**:
```bash
# Find the binary
which jjz

# Remove it (typically in /usr/local/bin)
sudo rm /usr/local/bin/jjz

# Or if in ~/.cargo/bin
rm ~/.cargo/bin/jjz
```

**If installed from source**:
```bash
# Navigate to the zjj repository
cd /path/to/zjj

# Uninstall
cargo uninstall --path crates/zjj

# Or manually remove
sudo rm /usr/local/bin/jjz
```

### 3. Remove Repository Data

Clean up ZJJ-specific files from your repositories.

**Per-repository cleanup** (run in each repository where you used ZJJ):

```bash
# Navigate to repository
cd /path/to/your/repo

# Remove ZJJ directory and all contents
rm -rf .jjz/

# Verify removal
ls -la .jjz  # Should show "No such file or directory"
```

**What gets removed**:
- `.jjz/sessions.db` - SQLite database with session state
- `.jjz/state.db` - Additional state database (if using custom config)
- `.jjz/config.toml` - Repository-specific configuration
- `.jjz/layouts/` - Custom Zellij layouts (if created)
- `.jjz/hooks/` - Custom hook scripts (if created)
- `.jjz/workspaces/` - Workspace directory (if configured to be inside `.jjz/`)

**Automated cleanup for multiple repositories**:
```bash
# Find all repositories with ZJJ initialized
find ~ -type d -name ".jjz" 2>/dev/null

# Remove all (use with caution!)
find ~ -type d -name ".jjz" -exec rm -rf {} + 2>/dev/null
```

### 4. Remove JJ Workspaces

Remove JJ workspaces created by ZJJ sessions.

**Identify ZJJ-created workspaces**:
```bash
# List all JJ workspaces
jj workspace list

# ZJJ workspaces typically start with "workspace_" or match session names
```

**Remove workspaces**:
```bash
# Remove individual workspace
jj workspace forget workspace_<session-name>

# Remove all workspaces matching pattern (bash)
jj workspace list | grep '^workspace_' | while read -r ws; do
    jj workspace forget "$ws"
done

# Remove all workspaces matching pattern (nushell)
jj workspace list | lines | where ($it =~ '^workspace_') | each { |ws| jj workspace forget $ws }
```

**What this does**:
- Removes workspace metadata from JJ
- Does NOT delete the actual workspace directory on disk
- The workspace directory remains but is no longer tracked

**Remove workspace directories**:
```bash
# If workspaces were created in default location
rm -rf ./workspaces/

# If custom workspace_dir was configured, find and remove
# Check config first to see where workspaces were stored
cat .jjz/config.toml | grep workspace_dir

# Then remove that directory
rm -rf /path/to/custom/workspace_dir/
```

### 5. Remove Global Configuration

Remove ZJJ global configuration files.

```bash
# Remove global config directory
rm -rf ~/.config/zjj/

# Verify removal
ls ~/.config/zjj  # Should show "No such file or directory"
```

**What gets removed**:
- `~/.config/zjj/config.toml` - Global ZJJ configuration
- `~/.config/zjj/` - Any other global ZJJ data

### 6. Clean Zellij Integration

Remove Zellij tabs created by ZJJ.

**Manual cleanup** (if Zellij is running):
```bash
# Inside Zellij, press: Ctrl+T
# Navigate to tabs named "jjz:*"
# Press: 'x' to close each tab

# Or use Zellij actions to list and close
zellij action list-tabs | grep 'jjz:' | while read -r tab; do
    zellij action close-tab --tab-name "$tab"
done
```

**Note**: Zellij tabs are ephemeral. Restarting Zellij will remove them automatically.

## Remove Dependencies

Only remove these if you no longer need them for other projects.

### Remove Beads

```bash
# If installed via cargo
cargo uninstall beads

# Verify
bd --version  # Should fail
```

### Remove Jujutsu (JJ)

```bash
# If installed via cargo
cargo uninstall jj-cli

# If installed via package manager
# macOS
brew uninstall jj

# Arch Linux
sudo pacman -R jujutsu

# Verify
jj --version  # Should fail
```

### Remove Zellij

```bash
# If installed via cargo
cargo uninstall zellij

# If installed via package manager
# macOS
brew uninstall zellij

# Arch Linux
sudo pacman -R zellij

# If installed manually
sudo rm /usr/local/bin/zellij

# Verify
zellij --version  # Should fail
```

**Remove Zellij configuration** (optional):
```bash
# Remove Zellij config directory
rm -rf ~/.config/zellij/
```

## Verification

Verify complete removal:

```bash
# 1. Binary removed
which jjz
# Expected: nothing (or "jjz not found")

# 2. Repository data removed
ls -la .jjz
# Expected: "No such file or directory"

# 3. Global config removed
ls ~/.config/zjj
# Expected: "No such file or directory"

# 4. JJ workspaces removed
jj workspace list
# Expected: Only "default" workspace (or error if JJ also removed)

# 5. Dependencies removed (if you chose to remove them)
which bd zellij jj
# Expected: nothing if removed
```

## Selective Cleanup

### Keep Data, Remove Binary Only

```bash
# Just remove the executable
cargo uninstall zjj

# Keeps:
# - .jjz/ directories (can reinstall and resume)
# - JJ workspaces (can manage manually)
# - Global configuration (will be used if you reinstall)
```

**Use case**: Temporarily removing ZJJ but planning to reinstall later.

### Remove Binary and Repository Data, Keep Dependencies

```bash
# Remove binary
cargo uninstall zjj

# Remove repository data
find ~ -type d -name ".jjz" -exec rm -rf {} + 2>/dev/null

# Remove global config
rm -rf ~/.config/zjj/

# Keep: JJ, Zellij, Beads (they work independently)
```

**Use case**: Done with ZJJ but still using JJ, Zellij, or Beads for other work.

### Archive Data Before Removal

```bash
# Create backup archive
cd /path/to/repo
tar -czf zjj-backup-$(date +%Y%m%d).tar.gz .jjz/

# Move backup to safe location
mv zjj-backup-*.tar.gz ~/backups/

# Then remove
rm -rf .jjz/
```

**Use case**: Want to preserve session history before cleanup.

### Restore from Archive

```bash
# Reinstall ZJJ
cargo install zjj

# Extract backup
cd /path/to/repo
tar -xzf ~/backups/zjj-backup-YYYYMMDD.tar.gz

# Verify restoration
jjz list
```

## Troubleshooting

### Issue: "Permission denied" when removing files

```bash
# Check ownership
ls -la .jjz/

# Fix permissions if needed
chmod -R u+w .jjz/

# Then remove
rm -rf .jjz/
```

### Issue: Database locked during cleanup

```bash
# Find processes using the database
lsof .jjz/sessions.db

# Or
fuser .jjz/sessions.db

# Kill hung processes
pkill jjz

# Then remove
rm -rf .jjz/
```

### Issue: Cannot remove JJ workspace

**Error**: `jj workspace forget` fails

**Solution**:
```bash
# Force forget (use with caution)
jj workspace forget --ignore-working-copy workspace_name

# Or manually edit JJ metadata (advanced)
# This is not recommended; contact JJ documentation
```

### Issue: Zellij tabs persist after removal

**Solution**:
```bash
# Close Zellij session completely
zellij kill-session

# Or restart Zellij
zellij attach  # Will start fresh without old tabs
```

### Issue: `cargo uninstall` fails

**Error**: "package zjj not found in registry"

**Solution**:
```bash
# Binary was likely installed manually
# Find and remove manually
which jjz
sudo rm $(which jjz)

# Or check cargo installation records
ls ~/.cargo/.crates.toml | grep zjj
```

### Issue: Files remain after cleanup

```bash
# Use find to locate all ZJJ-related files
find ~ -name "*jjz*" -o -name "*zjj*" 2>/dev/null

# Review and remove manually
rm /path/to/remaining/file
```

### Issue: Want to remove only failed sessions

```bash
# List failed sessions
jjz list --json | jq -r '.[] | select(.status == "failed") | .name'

# Remove each
jjz list --json | jq -r '.[] | select(.status == "failed") | .name' | while read -r name; do
    jjz remove "$name" --force
done
```

## Database Details

ZJJ stores data in SQLite databases. Understanding the schema helps with manual cleanup if needed.

### Session Database Schema

**Location**: `.jjz/sessions.db`

**Tables**:
```sql
-- Sessions table
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_synced INTEGER,
    metadata TEXT
);

-- Migrations table (for schema versioning)
CREATE TABLE migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);
```

### Manual Database Inspection

```bash
# Install sqlite3 if not already installed
# Ubuntu/Debian: sudo apt-get install sqlite3
# macOS: brew install sqlite3
# Arch Linux: sudo pacman -S sqlite

# Inspect database
sqlite3 .jjz/sessions.db

# Run queries
sqlite> .tables
sqlite> SELECT name, status FROM sessions;
sqlite> .quit
```

### Manual Database Cleanup

**WARNING**: Only do this if `jjz remove` fails and you understand the risks.

```bash
# Remove specific session from database
sqlite3 .jjz/sessions.db "DELETE FROM sessions WHERE name='session-name';"

# Remove all sessions
sqlite3 .jjz/sessions.db "DELETE FROM sessions;"

# Vacuum to reclaim space
sqlite3 .jjz/sessions.db "VACUUM;"
```

## Files Created by ZJJ

Complete list of files and directories created by ZJJ:

### Per-Repository Files
```
.jjz/
├── sessions.db          # Primary session database (always created)
├── config.toml          # Repository configuration (optional)
├── state.db             # Additional state database (if configured)
├── layouts/             # Custom Zellij layouts (if created)
│   └── *.kdl            # Zellij layout files
├── hooks/               # Custom hook scripts (if created)
│   ├── post_create      # Hook: after session creation
│   ├── pre_remove       # Hook: before session removal
│   ├── post_sync        # Hook: after sync
│   └── on_focus         # Hook: after focus
└── workspaces/          # Workspace directory (if configured here)
    └── <session-name>/  # Per-session workspace

workspaces/              # Default workspace location (outside .jjz/)
└── <session-name>/      # Per-session workspace directories
```

### Global Files
```
~/.config/zjj/
└── config.toml          # Global configuration

~/.cargo/bin/
└── jjz                  # Binary (if installed via cargo)

/usr/local/bin/
└── jjz                  # Binary (if installed manually)
```

### Temporary/Ephemeral Resources
- **Zellij tabs**: Named `jjz:<session-name>` (removed on Zellij restart)
- **JJ workspaces**: Metadata in `.jj/` (forgotten via `jj workspace forget`)

## Post-Uninstall

After complete uninstall, your system will be restored to:
- No `jjz` binary
- No `.jjz/` directories in repositories
- No global ZJJ configuration
- JJ, Zellij, and Beads remain (if not explicitly removed)
- JJ repositories remain functional (without ZJJ sessions)

### What Remains
- **JJ repository**: Fully functional, all commits preserved
- **Git repository**: Unchanged (if using JJ as a Git client)
- **Beads database**: `.beads/beads.db` (not managed by ZJJ)
- **Code and history**: Completely intact

### Next Steps After Uninstall

**If you want to use JJ without ZJJ**:
```bash
# Continue using JJ normally
jj status
jj diff
jj describe -m "message"
```

**If you want to reinstall ZJJ later**:
```bash
# Reinstall
cargo install zjj

# Re-initialize in repository
cd /path/to/repo
jjz init

# Sessions are gone, but JJ workspaces may still exist
jj workspace list
```

## Support

If you encounter issues during uninstall:

1. **Check this guide**: Most issues are covered in [Troubleshooting](#troubleshooting)
2. **File an issue**: https://github.com/lprior-repo/zjj/issues
   - Label: "uninstall" or "cleanup"
   - Include error messages and system details
3. **Manual cleanup**: Use `find` and `rm` as shown above

## Related Documentation

- [INSTALL.md](../INSTALL.md) - Installation guide (if you want to reinstall)
- [11_ARCHITECTURE.md](11_ARCHITECTURE.md) - Understanding ZJJ's file structure
- [00_START_HERE.md](00_START_HERE.md) - Quick start (if reinstalling)

---

**Uninstall completed successfully? We're sorry to see you go!**

If you have feedback on why you uninstalled, please share it:
https://github.com/lprior-repo/zjj/discussions
