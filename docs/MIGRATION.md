# ZJJ CLI Migration Guide

This guide documents the transition from flat command structure to object-based commands.

## Overview

ZJJ is transitioning from flat commands (`zjj add`, `zjj list`) to an object-based structure (`zjj session add`, `zjj session list`). The new structure provides:

- Better organization by domain (session, task, agent, etc.)
- Consistent command patterns (`zjj <object> <action>`)
- Easier discovery through grouped help text
- Better AI agent integration

## Current State

### Deprecated Aliases (Still Work)

The following old commands still work but emit deprecation warnings:

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `zjj add <name>` | `zjj session add <name>` | Create manual session |
| `zjj list` | `zjj session list` | List sessions |
| `zjj claim <resource>` | `zjj task claim <id>` | Claim exclusive access |
| `zjj yield <resource>` | `zjj task yield <id>` | Release exclusive access |
| `zjj sync [name]` | `zjj session sync [name]` | Sync workspace with main |
| `zjj submit` | `zjj session submit` | Submit for review/merge |
| `zjj done` | `zjj session done` | Complete and merge work |

### Object Commands

| Object | Purpose | Current CLI Notes |
|--------|---------|-------------------|
| `task` | Manage work items and beads | `task` subcommands defined, using `claim`/`yield` aliases |
| `session` | Manage workspaces and Zellij | `add`/`list`/`sync`/`done` as flat commands with aliases |
| `agent` | Manage agent coordination | Uses `agents` (plural), lists by default |
| `status` | Query system state | Flat command with `whereami`/`whoami`/`context` separate |
| `config` | Manage configuration | Flat command with positional args |
| `doctor` | Run diagnostics | Flat command with `--fix` flag |

---

## Command Reference

### Task Commands

Manage tasks and work items (beads).

#### `zjj task list`

List all tasks with optional filtering.

```bash
# List open tasks
zjj task list

# List all tasks including completed
zjj task list --all

# Filter by state
zjj task list --state in_progress

# JSON output for automation
zjj task list --json
```

#### `zjj task show <id>`

Show details for a specific task.

```bash
zjj task show zjj-abc123
zjj task show zjj-abc123 --json
```

#### `zjj task claim <id>`

Claim exclusive access to a task. Aliases: `take`.

```bash
# Claim a task
zjj task claim zjj-abc123

# With JSON output
zjj task claim zjj-abc123 --json
```

#### `zjj task yield <id>`

Release a claimed task. Aliases: `release`.

```bash
zjj task yield zjj-abc123
zjj task yield zjj-abc123 --json
```

#### `zjj task start <id>`

Start work on a task (creates a session).

```bash
# Start work with default template
zjj task start zjj-abc123

# Start with specific template
zjj task start zjj-abc123 --template full
```

#### `zjj task done [id]`

Complete a task. Aliases: `complete`.

```bash
# Complete current session's task
zjj task done

# Complete specific task
zjj task done zjj-abc123
```

---

### Session Commands

Manage workspaces and Zellij sessions.

#### `zjj session list`

List all sessions.

```bash
# List active sessions
zjj session list

# Include closed sessions
zjj session list --all

# Verbose output with paths
zjj session list --verbose

# Filter by bead
zjj session list --bead zjj-abc123

# Filter by agent
zjj session list --agent claude

# Filter by state
zjj session list --state working

# JSON output
zjj session list --json
```

#### `zjj session add <name>`

Create a new session for manual work. Aliases: `create`.

```bash
# Basic session creation
zjj session add feature-auth

# With bead association
zjj session add feature-auth --bead zjj-abc123

# With custom template
zjj session add feature-auth --template full

# Without opening Zellij tab
zjj session add feature-auth --no-open

# Skip post-create hooks
zjj session add feature-auth --no-hooks

# Preview without creating
zjj session add feature-auth --dry-run

# JSON output
zjj session add feature-auth --json
```

#### `zjj session remove <name>`

Remove a session and its workspace.

```bash
# Remove session
zjj session remove old-feature

# Force removal (skip hooks)
zjj session remove old-feature --force
```

#### `zjj session focus <name>`

Switch to a session's Zellij tab (must be inside Zellij).

```bash
zjj session focus feature-auth
zjj session focus feature-auth --json
```

#### `zjj session pause [name]`

Pause a session.

```bash
# Pause current session
zjj session pause

# Pause specific session
zjj session pause feature-auth
```

#### `zjj session resume [name]`

Resume a paused session.

```bash
# Resume current session
zjj session resume

# Resume specific session
zjj session resume feature-auth
```

#### `zjj session clone <name>`

Clone a session.

```bash
zjj session clone feature-auth
zjj session clone feature-auth --new-name feature-auth-v2
```

#### `zjj session rename <old> <new>`

Rename a session.

```bash
zjj session rename old-name new-name
```

#### `zjj session attach <name>`

Attach to a session from outside Zellij.

```bash
zjj session attach feature-auth
```

#### `zjj session spawn <bead>`

Spawn a session for automated agent work.

```bash
# Spawn with default agent (Claude)
zjj session spawn zjj-abc123

# With specific agent
zjj session spawn zjj-abc123 --agent opus

# Preview
zjj session spawn zjj-abc123 --dry-run
```

#### `zjj session sync [name]`

Sync session with remote. Aliases: `rebase`.

```bash
# Sync current session
zjj session sync

# Sync specific session
zjj session sync feature-auth

# Sync and push
zjj session sync feature-auth --push

# Sync and pull
zjj session sync feature-auth --pull
```

#### `zjj session init`

Initialize zjj in a JJ repository.

```bash
zjj session init
zjj session init --dry-run
zjj session init --json
```

---

### Agent Commands

Manage agent coordination and tracking. Note: The actual CLI uses `agents` (plural).

#### `zjj agents`

List all active agents (default action).

```bash
# List active agents
zjj agents

# Include stale agents
zjj agents --all

# Filter by session
zjj agents --session feature-auth

# JSON output
zjj agents --json
```

#### `zjj agents register`

Register as an agent.

```bash
# Register with auto-generated ID
zjj agents register

# Register with session
zjj agents register --session feature-auth
```

#### `zjj agents unregister`

Unregister as an agent.

```bash
# Unregister using ZJJ_AGENT_ID env var
zjj agents unregister
```

#### `zjj agents heartbeat`

Send a heartbeat.

```bash
zjj agents heartbeat
zjj agents heartbeat --command "building"
```

#### `zjj agents status`

Show current agent status.

```bash
zjj agents status
zjj agents status --json
```

---

### Status Commands

Query system and session status. Note: In the current CLI, `status` is a flat command and `whereami`/`whoami`/`context` are separate top-level commands.

#### `zjj status [session]`

Show detailed session status.

```bash
# Show all sessions
zjj status

# Show specific session
zjj status feature-auth

# Watch live updates
zjj status --watch

# JSON output
zjj status --json
```

#### `zjj whereami`

Show current location (quick query).

```bash
zjj whereami
# Returns: 'main' or 'workspace:<name>'

zjj whereami --json
```

#### `zjj whoami`

Show current identity (agent query).

```bash
zjj whoami
# Returns: agent ID or 'unregistered'

zjj whoami --json
```

#### `zjj context [session]`

Show complete environment context (AI agent query).

```bash
zjj context
zjj context feature-auth
zjj context --json
```

---

### Config Commands

Manage zjj configuration. Note: In the current CLI, `config` is a flat command that uses positional arguments instead of subcommands.

#### `zjj config [key] [value]`

View or modify configuration.

```bash
# Show all config
zjj config

# Get a specific value
zjj config workspace_dir

# Set a value
zjj config workspace_dir /path/to/workspaces

# JSON output
zjj config --json
zjj config workspace_dir --json
zjj config workspace_dir /new/path --json

# Global config
zjj config --global
```

---

### Doctor Commands

Run diagnostics and health checks. Note: In the current CLI, `doctor` is a flat command that uses flags instead of subcommands.

#### `zjj doctor`

Run system health checks.

```bash
# Run all checks
zjj doctor

# Auto-fix issues
zjj doctor --fix

# Preview fixes
zjj doctor --fix --dry-run

# Verbose output
zjj doctor --fix --verbose

# JSON output
zjj doctor --json
```

#### Related Commands

```bash
# Clean up invalid sessions (separate command)
zjj clean

# Check system integrity (separate command)
zjj integrity

# Prune invalid sessions (deterministic)
zjj prune-invalid
```

---

## Common Workflows

### Starting New Work

```bash
# Current CLI (with deprecated aliases still working)
zjj add feature-auth

# Future object-based way (when fully implemented)
zjj session add feature-auth

# With bead association
zjj add feature-auth --bead zjj-abc123

# For AI agents
zjj work feature-auth --bead zjj-abc123
```

### Checking Status

```bash
# List sessions
zjj list

# Detailed status
zjj status

# Quick location check
zjj whereami

# Agent identity check
zjj whoami
```

### Completing Work

```bash
# Complete and merge
zjj done

# With custom message
zjj done -m "feat: implement feature X"

# Preview without executing
zjj done --dry-run
```

### Agent Coordination

```bash
# List agents
zjj agents

# Register as agent
zjj agents register --session feature-auth

# Send heartbeat
zjj agents heartbeat --command "editing code"

# Check status
zjj agents status

# Unregister when done
zjj agents unregister
```

---

## Migration Checklist

For each script or workflow:

1. [ ] Replace `zjj add` with `zjj session add`
2. [ ] Replace `zjj list` with `zjj session list`
3. [ ] Replace `zjj claim` with `zjj task claim`
4. [ ] Replace `zjj yield` with `zjj task yield`
5. [ ] Replace `zjj sync` with `zjj session sync`
6. [ ] Replace `zjj submit` with `zjj session submit`
7. [ ] Replace `zjj done` with `zjj session done`

---

## Global Options

All commands support these global options:

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON (machine-parseable) |
| `-v, --verbose` | Enable verbose output |
| `--dry-run` | Preview without executing |
| `--on-success <CMD>` | Run command after success |
| `--on-failure <CMD>` | Run command after failure |

---

## JSON Output Format

All commands with `--json` return a SchemaEnvelope:

```json
{
  "$schema": "zjj://<command>-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "...": "command-specific fields"
}
```

---

## Getting Help

```bash
# Top-level help
zjj --help

# Object help
zjj session --help
zjj task --help

# Action help
zjj session add --help
zjj task claim --help
```

---

## Timeline

- **Current**: Deprecated aliases work with warnings
- **Next**: Object commands fully implemented
- **Future**: Deprecated aliases removed

Update your scripts and workflows before deprecated commands are removed.
