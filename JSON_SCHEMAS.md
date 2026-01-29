# JSON Output Schemas for ZJJ Commands

This document describes the JSON output format for all zjj commands that support `--json`.

## SchemaEnvelope Structure

All JSON outputs use a standard `SchemaEnvelope` wrapper:

```json
{
  "$schema": "zjj://<command>-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single|array",
  "success": true,
  ... // command-specific fields
}
```

## Core Commands

### `zjj add <name> --json`

Creates a new session and returns session information.

**Schema**: `zjj://add-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://add-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "name": "feature-auth",
  "workspace_path": "/path/to/repo/.zjj/workspaces/feature-auth",
  "zellij_tab": "zjj:feature-auth",
  "message": "Created session 'feature-auth' (with Zellij tab)"
}
```

### `zjj list --json`

Lists all sessions with their status.

**Schema**: `zjj://list-response/v1`
**Type**: `array`

```json
{
  "$schema": "zjj://list-response/v1",
  "_schema_version": "1.0",
  "schema_type": "array",
  "success": true,
  "data": [
    {
      "name": "feature-auth",
      "status": "active",
      "branch": "feature-auth",
      "changes": "3",
      "beads": "5/2/1",
      "id": 1,
      "workspace_path": "/path/to/.zjj/workspaces/feature-auth",
      "zellij_tab": "zjj:feature-auth",
      "created_at": 1704067200,
      "updated_at": 1704067200
    }
  ]
}
```

### `zjj remove <name> --json`

Removes a session and its workspace.

**Schema**: `zjj://remove-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://remove-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "name": "old-feature",
  "message": "Removed session 'old-feature'"
}
```

### `zjj focus <name> --json`

Switches to a session's Zellij tab.

**Schema**: `zjj://focus-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://focus-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "name": "feature-auth",
  "zellij_tab": "zjj:feature-auth",
  "message": "Switched to session 'feature-auth'"
}
```

### `zjj status [--name] --json`

Shows detailed session status.

**Schema**: `zjj://status-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://status-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "sessions": [
    {
      "name": "feature-auth",
      "status": "active",
      "workspace_path": "/path/to/.zjj/workspaces/feature-auth",
      "branch": "feature-auth",
      "changes": {
        "modified": 2,
        "added": 1,
        "deleted": 0,
        "renamed": 0,
        "unknown": 0
      },
      "diff_stats": {
        "insertions": 50,
        "deletions": 10
      },
      "beads": {
        "open": 5,
        "in_progress": 2,
        "blocked": 1,
        "closed": 10
      },
      "id": 1,
      "zellij_tab": "zjj:feature-auth",
      "created_at": 1704067200,
      "updated_at": 1704067200
    }
  ]
}
```

### `zjj sync [name] --json`

Syncs session workspace with main branch.

**Schema**: `zjj://sync-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://sync-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "name": "feature-auth",
  "synced_count": 1,
  "failed_count": 0,
  "errors": []
}
```

When syncing all sessions, `name` is `null`:

```json
{
  "$schema": "zjj://sync-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "name": null,
  "synced_count": 3,
  "failed_count": 0,
  "errors": []
}
```

## Advanced Commands

### `zjj init --json`

Initializes zjj in a JJ repository.

**Schema**: `zjj://init-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://init-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "message": "Initialized zjj in /path/to/repo",
  "zjj_dir": "/path/to/repo/.zjj",
  "config_file": "/path/to/repo/.zjj/config.toml",
  "state_db": "/path/to/repo/.zjj/state.db",
  "layouts_dir": "/path/to/repo/.zjj/layouts"
}
```

### `zjj spawn <bead_id> --json`

Creates an agent workspace for a bead.

**Schema**: `zjj://spawn-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://spawn-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "bead_id": "zjj-abc12",
  "session_name": "agent-zjj-abc12",
  "workspace_path": "/path/to/.zjj/workspaces/agent-zjj-abc12",
  "agent": "claude",
  "status": "started",
  "message": "Spawned workspace for zjj-abc12 with agent claude"
}
```

### `zjj done --json`

Completes work and merges to main.

**Schema**: `zjj://done-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://done-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "session_name": "feature-auth",
  "merged": true,
  "commit_id": "abc123def456",
  "message": "Merged and cleaned up 'feature-auth'"
}
```

### `zjj diff <name> --json`

Shows diff between session and main.

**Schema**: `zjj://diff-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://diff-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "name": "feature-auth",
  "base": "main",
  "head": "feature-auth",
  "diff_stat": {
    "files_changed": 3,
    "insertions": 50,
    "deletions": 10,
    "files": [
      {
        "path": "src/auth.rs",
        "insertions": 30,
        "deletions": 5,
        "status": "modified"
      }
    ]
  },
  "diff_content": "diff --git a/src/auth.rs b/src/auth.rs\n..."
}
```

## Utility Commands

### `zjj config [key] [value] --json`

Views or modifies configuration.

**Schema**: `zjj://config-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://config-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "key": "workspace_dir",
  "value": "../zjj__workspaces",
  "config": {
    "workspace_dir": "../zjj__workspaces",
    "zellij": {
      "use_tabs": true
    }
  }
}
```

### `zjj clean --json`

Removes stale sessions.

**Schema**: `zjj://clean-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://clean-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "removed_count": 2,
  "sessions": ["old-feature-1", "old-feature-2"]
}
```

### `zjj introspect [command] --json`

Discovers zjj capabilities.

**Schema**: `zjj://introspect-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://introspect-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "commands": [...],
  "dependencies": {
    "jj": { "required": true, "installed": true, "version": "0.15.1" },
    "zellij": { "required": true, "installed": true, "version": "0.39.2" }
  },
  "system_state": {
    "jj_repo": true,
    "zjj_initialized": true,
    "sessions_count": 3
  }
}
```

### `zjj doctor --json`

Runs system health checks.

**Schema**: `zjj://doctor-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://doctor-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "checks": [
    {
      "name": "JJ Installed",
      "status": "pass",
      "message": "JJ 0.15.1 is installed",
      "suggestion": null
    },
    {
      "name": "Zellij Running",
      "status": "warn",
      "message": "Zellij is not running",
      "suggestion": "Start Zellij with: zellij attach -c new-session"
    }
  ],
  "summary": {
    "passed": 8,
    "warnings": 1,
    "failed": 0
  }
}
```

### `zjj query <query_type> --json`

Queries system state programmatically.

**Schema**: `zjj://query-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://query-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "query_type": "session-exists",
  "result": true
}
```

### `zjj context --json`

Shows complete environment context.

**Schema**: `zjj://context-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://context-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "repository": {
    "root": "/path/to/repo",
    "branch": "main"
  },
  "sessions": [...],
  "beads": {
    "open": 10,
    "in_progress": 3
  },
  "health": {...},
  "environment": {...}
}
```

### `zjj checkpoint <action> --json`

Saves and restores session state snapshots.

**Schema**: `zjj://checkpoint-response/v1`
**Type**: `single`

```json
{
  "$schema": "zjj://checkpoint-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  "action": "create",
  "checkpoint_id": "ckpt-20240101-120000",
  "checkpoints": [
    {
      "id": "ckpt-20240101-120000",
      "timestamp": 1704067200,
      "description": "Before big refactor"
    }
  ]
}
```

## Error Response Format

All errors use the `JsonError` structure:

```json
{
  "success": false,
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "Session 'feature-x' not found",
    "exit_code": 2,
    "details": {
      "session_name": "feature-x",
      "available_sessions": ["feature-auth", "bugfix-123"]
    },
    "suggestion": "Use 'zjj list' to see available sessions"
  }
}
```

## Exit Codes

- **0**: Success (all checks passed, operation completed)
- **1**: Validation errors (user input issues)
- **2**: Not found errors (missing resources)
- **3**: System errors (IO, database issues)
- **4**: External command errors (JJ, Zellij failures)
- **5**: Lock contention errors

## Schema Versioning

All schemas currently use version `v1`. The `$schema` field follows the pattern:

```
zjj://<command>-response/v1
```

Schema versions will be incremented when breaking changes are introduced to the JSON structure.
