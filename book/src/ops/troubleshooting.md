# Troubleshooting

Fix common ZJJ issues.

## "Not in a JJ repository"

**Cause:** Not in a JJ repo
**Fix:**
```bash
jj init
zjj init
```

## "Workspace already exists"

**Cause:** Name taken
**Fix:**
```bash
zjj list
zjj remove <name> --force
zjj add <name>
```

## "Conflicts during sync"

**Cause:** Main changed same files
**Fix:**
```bash
zjj focus <workspace>
jj resolve  # Edit conflict markers
jj commit -m "Resolve conflicts"
zjj sync
```

## "Database corruption detected"

**Cause:** Crash during write
**Fix:**
```bash
zjj doctor  # Auto-repair
# Check .zjj/recovery.log
```

## "Zellij not found"

**Cause:** Zellij not running
**Fix:**
```bash
zellij  # Start Zellij
# Or disable:
zjj add <name> --no-tab
```

## Queue Issues

**Worker stuck:**
```bash
zjj queue list --status in_progress
zjj queue reclaim --agent <id>
```

**Can't claim:**
```bash
# Check if already claimed
zjj queue list --bead <id>
```

## Debug Mode

Verbose output:
```bash
zjj --verbose <command>
RUST_LOG=debug zjj <command>
```

## Get Help

- `zjj <command> --help`
- [GitHub Issues](https://github.com/lprior-repo/zjj/issues)
