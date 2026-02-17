# Command Reference

All ZJJ commands in one place.

---

## Session Management

### zjj init

Initialize ZJJ in a JJ repository.

```
zjj init [--json] [--dry-run]
```

---

### zjj add

Create a session for manual work.

```
zjj add <name> [options]
```

| Option | Description |
|--------|-------------|
| `-b, --bead <id>` | Associate with bead/issue |
| `-t, --template <name>` | Zellij layout (minimal, standard, full) |
| `--no-open` | Don't open Zellij tab |
| `--no-zellij` | Skip Zellij entirely |
| `--no-hooks` | Skip post-create hooks |
| `--idempotent` | Succeed if exists |
| `--dry-run` | Preview without creating |
| `-j, --json` | JSON output |

```bash
zjj add feature-auth --bead BD-123
zjj add quick-test --template minimal
zjj add automation --no-zellij --idempotent
```

---

### zjj remove

Remove a session and its workspace.

```
zjj remove <name> [options]
```

| Option | Description |
|--------|-------------|
| `-f, --force` | Skip confirmation |
| `-m, --merge` | Squash-merge to main first |
| `-k, --keep-branch` | Preserve branch |
| `--idempotent` | Succeed if not found |
| `--dry-run` | Preview removal |
| `-j, --json` | JSON output |

```bash
zjj remove old-feature
zjj remove merged-work --merge --force
```

---

### zjj list

List all sessions.

```
zjj list [options]
```

| Option | Description |
|--------|-------------|
| `--all` | Include completed/failed |
| `-v, --verbose` | Show details |
| `--bead <id>` | Filter by bead |
| `--agent <name>` | Filter by agent |
| `--state <state>` | Filter by state |
| `-j, --json` | JSON output |

---

### zjj status

Show session status.

```
zjj status [<name>] [options]
```

| Option | Description |
|--------|-------------|
| `--watch` | Live updates (1s refresh) |
| `-j, --json` | JSON output |

---

### zjj focus

Switch to session's Zellij tab (inside Zellij).

```
zjj focus [<name>] [--no-zellij] [--json]
```

---

### zjj attach

Enter Zellij session from outside.

```
zjj attach <name> [--json]
```

---

### zjj switch

Switch between workspaces.

```
zjj switch [<name>] [--show-context] [--json]
```

---

## Work Completion

### zjj sync

Sync workspace with main (rebase).

```
zjj sync [<name>] [options]
```

| Option | Description |
|--------|-------------|
| `--all` | Sync all active sessions |
| `--dry-run` | Preview sync |
| `-j, --json` | JSON output |

---

### zjj done

Complete work and merge to main.

```
zjj done [options]
```

| Option | Description |
|--------|-------------|
| `-m, --message <msg>` | Commit message |
| `-w, --workspace <name>` | Specific workspace |
| `--squash` | Squash all commits |
| `--keep-workspace` | Don't remove after |
| `--detect-conflicts` | Check before merging |
| `--dry-run` | Preview |
| `-j, --json` | JSON output |

```bash
zjj done -m "Add feature"
zjj done --squash --push
zjj done -w feature-auth -m "Fix bug"
```

---

### zjj undo

Undo last done operation.

```
zjj undo [--list] [--dry-run] [--json]
```

---

### zjj revert

Revert specific session merge.

```
zjj revert <name> [--dry-run] [--json]
```

---

### zjj diff

Show changes vs main.

```
zjj diff [<name>] [--stat] [--json]
```

---

### zjj submit

Submit for review/merge.

```
zjj submit [--auto-commit] [-m <msg>] [--dry-run] [--json]
```

---

## Queue Operations

### zjj queue

Manage merge queue.

```
zjj queue <action> [options]
```

| Action | Description |
|--------|-------------|
| `--add <workspace>` | Add to queue |
| `--list` | List all entries |
| `--next` | Get next pending |
| `--process` | Process next ready |
| `--status <workspace>` | Check status |
| `--cancel <id>` | Cancel entry |
| `--retry <id>` | Retry failed |
| `--stats` | Show statistics |
| `--reclaim-stale [secs]` | Reclaim expired leases |
| `--remove <workspace>` | Remove from queue |

| Option | Description |
|--------|-------------|
| `--bead <id>` | Associate with bead (with --add) |
| `--priority <1-10>` | Priority (with --add, lower = higher) |
| `--agent <id>` | Assign to agent (with --add) |
| `-j, --json` | JSON output |

```bash
zjj queue --add feature-a --bead BD-101 --priority 3
zjj queue --list --json
zjj queue --next
zjj queue --cancel 123
```

---

### zjj queue worker

Process queue entries.

```
zjj queue worker [options]
```

| Option | Description |
|--------|-------------|
| `--once` | Process one item and exit |
| `--loop` | Run continuously |
| `--interval <secs>` | Poll interval (default: 10) |
| `--worker-id <id>` | Worker identifier |
| `-j, --json` | JSON output |

```bash
zjj queue worker --once
zjj queue worker --loop --interval 30
```

---

## Agent Commands

### zjj agents

Manage agents.

```
zjj agents [subcommand] [options]
```

| Subcommand | Description |
|------------|-------------|
| (none) | List active agents |
| `register` | Register as agent |
| `heartbeat` | Send liveness signal |
| `status` | Show current agent |
| `unregister` | Remove agent |

| Option | Description |
|--------|-------------|
| `--all` | Include stale agents |
| `--session <name>` | Filter by session |
| `--id <id>` | Specific agent ID |
| `-c, --command <cmd>` | Current command (heartbeat) |
| `-j, --json` | JSON output |

```bash
zjj agents register --session my-work
zjj agents heartbeat -c "running tests"
zjj agents --all
```

---

### zjj work

Unified workflow start for agents.

```
zjj work <name> [options]
```

| Option | Description |
|--------|-------------|
| `-b, --bead <id>` | Associate with bead |
| `--agent-id <id>` | Agent identifier |
| `--no-zellij` | Skip Zellij |
| `--no-agent` | Skip agent registration |
| `--idempotent` | Succeed if exists |
| `--dry-run` | Preview |
| `-j, --json` | JSON output |

```bash
zjj work feature-auth --bead BD-123
```

---

### zjj spawn

Spawn automated agent work on a bead.

```
zjj spawn <bead-id> [options]
```

| Option | Description |
|--------|-------------|
| `--agent-command <cmd>` | Agent to run (default: claude) |
| `--agent-args <args>` | Additional args |
| `--timeout <secs>` | Timeout (default: 14400) |
| `-b, --background` | Run in background |
| `--no-auto-merge` | Don't merge on success |
| `--no-auto-cleanup` | Don't cleanup on failure |
| `--idempotent` | Succeed if exists |
| `--dry-run` | Preview |
| `-j, --json` | JSON output |

```bash
zjj spawn zjj-abc12
zjj spawn zjj-xyz34 --background
```

---

### zjj whereami

Get current location.

```
zjj whereami [--json]
```

Returns: `main` or `workspace:<name>`

---

### zjj whoami

Get agent identity.

```
zjj whoami [--json]
```

---

### zjj context

Get full environment context.

```
zjj context [options]
```

| Option | Description |
|--------|-------------|
| `--field <path>` | Extract single field |
| `--no-beads` | Skip beads query |
| `--no-health` | Skip health checks |
| `-j, --json` | JSON output |

```bash
zjj context --json
zjj context --field=workspace.path
```

---

### zjj broadcast

Send message to all agents.

```
zjj broadcast <message> [--agent-id <id>] [--json]
```

---

## Diagnostics & Recovery

### zjj doctor

Run health checks.

```
zjj doctor [options]
```

| Option | Description |
|--------|-------------|
| `--fix` | Auto-fix issues |
| `--dry-run` | Preview fixes |
| `-v, --verbose` | Detailed output |
| `-j, --json` | JSON output |

---

### zjj integrity

Workspace integrity management.

```
zjj integrity <subcommand> [options]
```

| Subcommand | Description |
|------------|-------------|
| `validate <workspace>` | Check integrity |
| `repair <workspace>` | Fix corruption |
| `backup list` | List backups |
| `backup restore <id>` | Restore backup |

| Option | Description |
|--------|-------------|
| `--force` | Skip confirmation |
| `--rebind` | Update session location |
| `-j, --json` | JSON output |

---

### zjj checkpoint

Save/restore session snapshots.

```
zjj checkpoint <subcommand> [options]
```

| Subcommand | Description |
|------------|-------------|
| `create` | Create checkpoint |
| `list` | List checkpoints |
| `restore <id>` | Restore checkpoint |

```bash
zjj checkpoint create -d "before major change"
zjj checkpoint list
zjj checkpoint restore ckpt-123
```

---

### zjj clean

Remove stale sessions.

```
zjj clean [options]
```

| Option | Description |
|--------|-------------|
| `-f, --force` | Skip confirmation |
| `--dry-run` | Preview |
| `--periodic` | Run as daemon |
| `--age-threshold <secs>` | Age cutoff |
| `-j, --json` | JSON output |

---

### zjj prune-invalid

Bulk remove invalid sessions.

```
zjj prune-invalid [--yes] [--dry-run] [--json]
```

---

## Introspection

### zjj introspect

Discover capabilities.

```
zjj introspect [<command>] [options]
```

| Option | Description |
|--------|-------------|
| `--ai` | AI-optimized output |
| `--env-vars` | Show environment variables |
| `--workflows` | Show workflow patterns |
| `--session-states` | Show state transitions |
| `-j, --json` | JSON output |

---

### zjj query

Query system state.

```
zjj query <type> [args] [--json]
```

| Query Type | Args | Returns |
|------------|------|---------|
| `session-exists` | `<name>` | true/false |
| `session-count` | — | number |
| `can-run` | — | true/false |
| `suggest-name` | `<pattern>` | suggested name |
| `lock-status` | `<resource>` | lock info |
| `pending-merges` | — | list |

---

### zjj can-i

Check permissions.

```
zjj can-i <action> [--resource <name>]
```

---

### zjj contract

Show command contract (AI use).

```
zjj <command> --contract
```

---

## Configuration

### zjj config

View/modify configuration.

```
zjj config [<key> [<value>]] [-g, --global] [--json]
```

```bash
zjj config                    # Show all
zjj config recovery.policy    # Show key
zjj config recovery.policy warn  # Set key
```

---

### zjj template

Manage Zellij layouts.

```
zjj template <subcommand> [options]
```

| Subcommand | Description |
|------------|-------------|
| `list` | List templates |
| `show <name>` | Show details |
| `create <name>` | Create template |
| `delete <name>` | Delete template |

```bash
zjj template list
zjj template create custom --builtin standard
```

---

### zjj bookmark

Manage JJ bookmarks.

```
zjj bookmark <subcommand> [options]
```

| Subcommand | Description |
|------------|-------------|
| `list` | List bookmarks |
| `create <name>` | Create bookmark |
| `delete <name>` | Delete bookmark |
| `move <name> --to <rev>` | Move bookmark |

| Option | Description |
|--------|-------------|
| `-p, --push` | Push to remote |
| `--all` | Show all including remote |

```bash
zjj bookmark create feature-x --push
zjj bookmark list --all
```

---

## Advanced

### zjj batch

Execute multiple commands.

```
zjj batch --commands <cmd1,cmd2> [--atomic] [--stop-on-error]
```

---

### zjj events

View event stream.

```
zjj events [--follow] [--limit <n>] [--session <name>] [--type <type>]
```

---

### zjj pane

Manage Zellij panes.

```
zjj pane <action> [--direction <dir>] [--session <name>]
```

| Action | Description |
|--------|-------------|
| `list` | List panes |
| `focus` | Focus pane |
| `next` | Next pane |

---

### zjj backup

Database backup management.

```
zjj backup <action> [options]
```

| Action | Description |
|--------|-------------|
| `create` | Create backup |
| `list` | List backups |
| `restore <timestamp>` | Restore backup |
| `status` | Backup status |

---

### zjj export

Export session state.

```
zjj export --session <name> --output <file>
```

---

### zjj import

Import session state.

```
zjj import --file <path> [--force] [--skip-existing]
```

---

### zjj completions

Generate shell completions.

```
zjj completions <shell>
```

Shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`

---

## Global Options

All commands support:

| Option | Description |
|--------|-------------|
| `-j, --json` | JSON output |
| `--contract` | Show AI contract |
| `--ai-hints` | Show execution hints |
| `--dry-run` | Preview without executing |
| `-h, --help` | Show help |
