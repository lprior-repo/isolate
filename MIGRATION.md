# Migration Guide

## v0.2.0: Binary and Directory Rename

ZJJ v0.2.0 introduces a complete rename for consistency:

- **Binary**: `jjz` → `zjj`
- **Directory**: `.jjz/` → `.zjj/`
- **Session Prefix**: `jjz:` → `zjj:` (configurable)

### What Changed

This is a simple, direct rename with no backwards compatibility layer since there are no production users yet.

| Component | Old Name | New Name |
|-----------|----------|----------|
| Binary | `jjz` | `zjj` |
| Config Directory | `.jjz/` | `.zjj/` |
| Config File | `.jjz/config.toml` | `.zjj/config.toml` |
| State Database | `.jjz/state.db` | `.zjj/state.db` |
| Layouts Directory | `.jjz/layouts/` | `.zjj/layouts/` |
| Session Prefix | `jjz:` | `zjj:` |
| Status Pane Command | `jjz status` | `zjj status` |

### Fresh Installation

If you're installing v0.2.0 or later for the first time, everything is automatically configured with the new naming.

### Upgrading from Earlier Versions

If you have an existing installation using the old naming:

**Manual Migration:**
```bash
# In your repository
mv .jjz .zjj

# Update your shell configuration if you have aliases
# Remove or update any references to 'jjz' command
```

**Update Scripts:**
If you have shell scripts or automation that references the old names, update them:
```bash
# Before
jjz init
jjz add my-session

# After
zjj init
zjj add my-session
```

**Shell Alias (Optional):**
If you want to keep using the old name for muscle memory:
```bash
# Add to ~/.bashrc, ~/.zshrc, or equivalent
alias jjz='zjj'
```

### Configuration Updates

If you have custom configuration in `.jjz/config.toml`, move it to `.zjj/config.toml`. The format is unchanged - only the directory path is different.

### All Commands Renamed

All commands use the new `zjj` binary:

```bash
# Initialization
zjj init                    # (was: jjz init)
zjj init --repair          # (was: jjz init --repair)
zjj init --force           # (was: jjz init --force)

# Session Management
zjj add <name>             # (was: jjz add)
zjj list                   # (was: jjz list)
zjj remove <name>          # (was: jjz remove)
zjj focus <name>           # (was: jjz focus)
zjj status [name]          # (was: jjz status)

# Version Control
zjj sync <name>            # (was: jjz sync)
zjj diff [name]            # (was: jjz diff)

# Administration
zjj doctor                 # (was: jjz doctor)
zjj config                 # (was: jjz config)
zjj backup                 # (was: jjz backup)
zjj restore                # (was: jjz restore)

# Utilities
zjj completions <shell>    # (was: jjz completions)
```

### Questions?

- Check `.zjj/config.toml` for configuration details
- Run `zjj --help` for command help
- Run `zjj doctor` to check system health
- See README.md for usage examples
