# ZJJ AI Agent Quickstart

This guide is designed for AI agents. It provides the minimum information needed to start using zjj productively.

## TL;DR - 3 Commands

```bash
zjj whereami        # Check: "main" or "workspace:<name>"
zjj work my-task    # Start: Create workspace
zjj done            # Finish: Merge and cleanup
```

## AI Entry Point

Start here:
```bash
zjj ai status       # Full status with guided next action
zjj ai workflow     # 7-step parallel agent workflow
zjj ai quick-start  # Minimum commands reference
```

## Essential Commands

| Command | Purpose | Output |
|---------|---------|--------|
| `zjj whereami` | Location check | `main` or `workspace:<name>` |
| `zjj whoami` | Agent identity | `<agent-id>` or `unregistered` |
| `zjj work <name>` | Create workspace | Workspace info + enter command |
| `zjj done` | Complete work | Merge confirmation |
| `zjj abort` | Abandon work | Cleanup confirmation |

## Minimal Workflow

```bash
# 1. Check location
zjj whereami

# 2. Start work (safe to retry with --idempotent)
zjj work my-task --idempotent

# 3. Enter workspace
cd $(zjj context --json | jq -r '.location.path // empty')

# 4. Do work...

# 5. Complete
zjj done
```

## Safe Flags (Always Use These)

| Flag | Effect |
|------|--------|
| `--idempotent` | Succeed even if already exists |
| `--dry-run` | Preview without executing |
| `--json` | Machine-readable output |

## Quick Queries

```bash
zjj query location              # Where am I?
zjj query can-spawn             # Can I start work?
zjj query lock-status <name>    # Is session locked?
zjj query pending-merges        # What needs merging?
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZJJ_AGENT_ID` | Your agent ID (set by register) |
| `ZJJ_SESSION` | Current session name |
| `ZJJ_WORKSPACE` | Current workspace path |
| `ZJJ_BEAD_ID` | Associated bead ID |
| `ZJJ_ACTIVE` | "1" when in workspace |

## JSON Output Pattern

All commands support `--json` and return:
```json
{
  "$schema": "zjj://<command>-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  ...
}
```

## Error Handling

Exit codes:
- 0: Success
- 1: Validation error (user input)
- 2: Not found
- 3: System error
- 4: External command error
- 5: Lock contention

Errors include suggestions:
```json
{
  "success": false,
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "...",
    "suggestion": "Use 'zjj list' to see available sessions"
  }
}
```

## Introspection

```bash
zjj introspect              # All capabilities
zjj introspect <cmd>        # Command details
zjj introspect --env-vars   # Environment variables
zjj introspect --workflows  # Workflow patterns
```

## Agent Lifecycle

```bash
# Register (optional but recommended)
zjj agent register

# Send heartbeat while working
zjj agent heartbeat --command "implementing"

# Check your status
zjj agent status

# Unregister when done
zjj agent unregister
```

## Parallel Agent Workflow

1. **Orient**: `zjj whereami` - Check location
2. **Register**: `zjj agent register` - Identify yourself
3. **Isolate**: `zjj work <name> --idempotent` - Create workspace
4. **Enter**: `cd $(zjj context --json | jq -r '.location.path')` - Go to workspace
5. **Implement**: Do the work
6. **Heartbeat**: `zjj agent heartbeat` - Signal liveness
7. **Complete**: `zjj done` - Merge and cleanup

## Common Patterns

### Start Fresh
```bash
zjj whereami                        # Should return "main"
zjj work feature-auth --idempotent
```

### Continue Existing Work
```bash
zjj whereami                        # Returns "workspace:feature-auth"
# Already in workspace, continue working
```

### Abandon and Start Over
```bash
zjj abort --dry-run                 # Preview
zjj abort                           # Execute
zjj work feature-auth-v2            # Start fresh
```

### Multiple Sessions
```bash
zjj list --json                     # List all sessions
zjj sync --all                      # Sync all with main
```

## What NOT to Do

- Don't use `zjj spawn` for simple workflows (use `zjj work`)
- Don't forget `--idempotent` when retrying
- Don't skip `zjj whereami` before operations
- Don't modify files outside your workspace

## Reference

Full documentation: `zjj --help`
Command details: `zjj introspect <command>`
AI status: `zjj ai status`
