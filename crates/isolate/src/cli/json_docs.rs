//! JSON OUTPUT documentation for command help
//! These strings document the `SchemaEnvelope` structure used in JSON output

pub const fn add() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://add-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "message": "Created session '<name>'"
  }"#
}

pub const fn list() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelopeArray:
  {
    "$schema": "isolate://list-response/v1",
    "_schema_version": "1.0",
    "schema_type": "array",
    "success": true,
    "data": [
      {
        "display_branch": "<branch_name or null>",
        "changes": "<modified_count>",
        "beads": "<open/in_progress/blocked>",
        "id": <db_id>,
        "name": "<session_name>",
        "status": "<creating|active|paused|completed|failed>",
        "state": "<created|working|ready|merged|abandoned|conflict>",
        "workspace_path": "<absolute_path>",
        "branch": "<branch_name or null>",
        "created_at": <unix_timestamp>,
        "updated_at": <unix_timestamp>,
        "last_synced": <unix_timestamp or null>,
        "metadata": { ... } or null
      }
    ]
  }
  
  NOTE: display_branch is a convenience field for display (null shown as "-").
  Session fields are included via serde(flatten) - no duplicate keys (RFC 8259 compliant)."#
}

pub const fn remove() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://remove-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "message": "Removed session '<name>' | Session '<name>' already removed"
  }"#
}

pub const fn focus() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://focus-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "message": "Switched to session '<name>'"
  }"#
}

pub const fn status() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelope:
  {
    "$schema": "isolate://status-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "sessions": [
      {
        "name": "<session_name>",
        "status": "<active|paused|completed|failed>",
        "workspace_path": "<absolute_path>",
        "branch": "<branch_name>",
        "changes": {
          "modified": <count>,
          "added": <count>,
          "deleted": <count>,
          "renamed": <count>
        },
        "diff_stats": {
          "insertions": <count>,
          "deletions": <count>
        },
        "beads": {
          "open": <count>,
          "in_progress": <count>,
          "blocked": <count>,
          "closed": <count>
        },
        ...
      }
    ]
  }"#
}

pub const fn sync() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://sync-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name_or_null>",
    "synced_count": <count>,
    "failed_count": <count>,
    "errors": []
  }"#
}

pub const fn init() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://init-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "message": "<message>",
    "isolate_dir": "<absolute_path>",
    "config_file": "<absolute_path>",
    "state_db": "<absolute_path>",
    "layouts_dir": "<absolute_path>"
  }"#
}

pub const fn spawn() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://spawn-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "bead_id": "<bead_id>",
    "session_name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "agent": "<agent_command>",
    "status": "<started|running|completed|failed>",
    "message": "<status_message>"
  }"#
}

pub const fn done() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://done-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "session_name": "<session_name>",
    "merged": true,
    "commit_id": "<commit_hash>",
    "message": "Merged and cleaned up '<name>'"
  }"#
}

pub const fn diff() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://diff-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "base": "<base_commit>",
    "head": "<head_commit>",
    "diff_stat": {
      "files_changed": <count>,
      "insertions": <count>,
      "deletions": <count>,
      "files": [...]
    },
    "diff_content": "<full_diff_or_null>"
  }"#
}

pub const fn config() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://config-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "key": "<config_key_or_null>",
    "value": "<config_value_or_null>",
    "config": {...}
  }"#
}

pub const fn clean() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://clean-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "removed_count": <count>,
    "sessions": ["<session_name>", ...]
  }"#
}

pub const fn introspect() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://introspect-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "commands": [...],
    "dependencies": {...},
    "system_state": {...}
  }"#
}

pub const fn doctor() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://doctor-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "checks": [
      {
        "name": "<check_name>",
        "status": "<pass|warn|fail>",
        "message": "<message>",
        "suggestion": "<suggestion_or_null>"
      },
      ...
    ],
    "summary": {
      "passed": <count>,
      "warnings": <count>,
      "failed": <count>
    }
  }"#
}

pub const fn query() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used (default), output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://query-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "query_type": "<query_type>",
    "result": <query_specific_result>
  }"#
}

pub const fn context() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used (default when not TTY), output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://context-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "repository": {...},
    "sessions": [...],
    "beads": {...},
    "health": {...},
    "environment": {...}
  }"#
}

pub const fn checkpoint() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://checkpoint-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "action": "<create|restore|list>",
    "checkpoint_id": "<id_or_null>",
    "checkpoints": [...]
  }"#
}

pub const fn queue() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps queue responses in a SchemaEnvelope:
  {
    "$schema": "isolate://queue-<action>-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    ...action-specific fields...
  }

  Queue action envelopes include:
  - queue-add-response
  - queue-list-response
  - queue-next-response
  - queue-process-response
  - queue-remove-response
  - queue-status-response
  - queue-status-id-response
  - queue-stats-response
  - queue-cancel-response
  - queue-retry-response
  - queue-reclaim-stale-response

  Flag constraints:
  - --bead, --priority, and --agent require --add.
  - Invalid flag combinations return non-zero exit and an error envelope in --json mode."#
}

pub const fn export() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "isolate://export-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "version": "<format_version>",
    "exported_at": "<RFC3339_timestamp>",
    "count": <session_count>,
    "sessions": [...]
  }"#
}

/// AI-Native contract documentation for commands
pub mod ai_contracts {
    /// Machine-readable contract for isolate add command
    pub const fn add() -> &'static str {
        r#"AI CONTRACT for isolate add:
{
  "command": "isolate add",
  "intent": "Create isolated workspace for manual interactive development",
  "prerequisites": [
    "isolate init must have been run",
    "JJ repository must be initialized"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Database session record"],
    "modifies": [],
    "state_transition": "none → active"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "validation": "Must be valid session name (alphanumeric, hyphens, underscores)",
      "examples": ["feature-auth", "bugfix-123", "experiment-alpha"]
    },
    "template": {
      "type": "string",
      "required": false,
      "flag": "-t|--template",
      "examples": ["default", "minimal", "full"]
    },
    "no_open": {
      "type": "boolean",
      "required": false,
      "flag": "--no-open",
      "description": "Skip opening workspace after creation"
    },
    "no_hooks": {
      "type": "boolean",
      "required": false,
      "flag": "--no-hooks",
      "description": "Skip post-create hooks"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "workspace_path": "string",
      "status": "active"
    },
    "errors": [
      "SessionAlreadyExists",
      "InvalidSessionName",
      "JJInitFailed",
      "DatabaseError"
    ]
  },
  "examples": [
    "isolate add feature-auth",
    "isolate add bugfix-123 --no-open",
    "isolate add experiment -t minimal"
  ],
  "next_commands": [
    "isolate focus <name>",
    "isolate status <name>",
    "isolate work <bead_id>"
  ]
}"#
    }

    /// Machine-readable contract for isolate work command
    pub const fn work() -> &'static str {
        r#"AI CONTRACT for isolate work:
{
  "command": "isolate work",
  "intent": "Create or reuse a named workspace and optionally register an agent",
  "prerequisites": [
    "isolate init must have been run",
    "Must run inside a JJ repository"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Database session record"],
    "modifies": ["Session metadata"],
    "state_transition": "none → active"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "position": 1,
      "validation": "Must pass session name validation",
      "examples": ["feature-auth", "bug-fix-123"]
    },
    "bead": {
      "type": "string",
      "required": false,
      "flag": "-b|--bead",
      "description": "Optional bead ID to associate"
    },
    "agent_id": {
      "type": "string",
      "required": false,
      "flag": "--agent-id",
      "description": "Optional agent identifier"
    },
    "no_agent": {
      "type": "boolean",
      "required": false,
      "flag": "--no-agent",
      "description": "Skip agent registration"
    },
    "idempotent": {
      "type": "boolean",
      "required": false,
      "flag": "--idempotent",
      "description": "Reuse existing session if present"
    },
    "dry_run": {
      "type": "boolean",
      "required": false,
      "flag": "--dry-run",
      "description": "Preview without creating"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "workspace_path": "string",
      "created": "boolean",
      "agent_id": "string|null",
      "bead_id": "string|null",
      "env_vars": "array",
      "enter_command": "string"
    },
    "errors": [
      "InvalidSessionName",
      "SessionAlreadyExists",
      "NotInJjRepository",
      "WorkspaceCreationFailed"
    ]
  },
  "examples": [
    "isolate work feature-auth",
    "isolate work bug-fix --bead isolate-123",
    "isolate work feature-auth --agent-id agent-1 --idempotent",
    "isolate work feature-auth --dry-run"
  ],
  "next_commands": [
    "isolate done",
    "isolate checkpoint create",
    "isolate status"
  ]
}"#
    }

    /// Machine-readable contract for isolate queue command
    pub const fn queue() -> &'static str {
        r#"AI CONTRACT for isolate queue:
{
  "command": "isolate queue",
  "intent": "Manage merge queue entries and worker processing",
  "inputs": {
    "add": {
      "flag": "--add <WORKSPACE>",
      "description": "Add a workspace to queue"
    },
    "bead": {
      "flag": "--bead <BEAD_ID>",
      "requires": "--add"
    },
    "priority": {
      "flag": "--priority <PRIORITY>",
      "requires": "--add",
      "type": "integer",
      "default_when_adding": 5
    },
    "agent": {
      "flag": "--agent <AGENT_ID>",
      "requires": "--add"
    },
    "actions": [
      "--list",
      "--next",
      "--process",
      "--remove <WORKSPACE>",
      "--status <WORKSPACE>",
      "--status-id <ID>",
      "--cancel <ID>",
      "--retry <ID>",
      "--stats",
      "--reclaim-stale [SECS]",
      "worker --once|--loop"
    ]
  },
  "outputs": {
    "json_envelope": {
      "$schema": "isolate://queue-<action>-response/v1",
      "schema_type": "single",
      "success": "boolean"
    },
    "errors": {
      "invalid_input": "Non-zero exit with error envelope in --json mode"
    }
  }
}"#
    }

    /// Machine-readable contract for isolate spawn command
    pub const fn spawn() -> &'static str {
        r#"AI CONTRACT for isolate spawn:
{
  "command": "isolate spawn",
  "intent": "Create workspace and spawn automated agent with isolation",
  "prerequisites": [
    "isolate init must have been run",
    "Beads database must be available",
    "Agent system must be configured"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Agent process", "Database records"],
    "modifies": ["Bead status", "Agent registry"],
    "state_transition": "open → in_progress"
  },
  "inputs": {
    "bead_id": {
      "type": "string",
      "required": true,
      "position": 1,
      "validation": "Must be open bead in database"
    },
    "agent": {
      "type": "string",
      "required": false,
      "flag": "-a|--agent",
      "default": "claude"
    }
  },
  "outputs": {
    "success": {
      "bead_id": "string",
      "session_name": "string",
      "workspace_path": "string",
      "agent": "string",
      "status": "started|running|completed|failed"
    }
  },
  "examples": [
    "isolate spawn isolate-abc123",
    "isolate spawn isolate-abc123 --agent claude-opus"
  ]
}"#
    }

    /// Machine-readable contract for isolate done command
    pub const fn done() -> &'static str {
        r#"AI CONTRACT for isolate done:
{
  "command": "isolate done",
  "intent": "Complete work, merge changes to main, and cleanup workspace",
  "prerequisites": [
    "Session must be active",
    "Workspace must have committed changes",
    "No merge conflicts should exist"
  ],
  "side_effects": {
    "creates": ["Merge commit on main"],
    "deletes": ["JJ workspace", "Session record"],
    "modifies": ["Main branch", "Bead status"],
    "state_transition": "active → completed"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "default": "current session"
    },
    "force": {
      "type": "boolean",
      "flag": "--force",
      "description": "Force merge even with conflicts"
    }
  },
  "outputs": {
    "success": {
      "session_name": "string",
      "merged": true,
      "commit_id": "string"
    },
    "errors": [
      "NoActiveSession",
      "MergeConflict",
      "WorkspaceDirty",
      "SessionNotFound"
    ]
  },
  "examples": [
    "isolate done",
    "isolate done feature-auth",
    "isolate done --force"
  ]
}"#
    }

    /// Machine-readable contract for isolate sync command
    pub const fn sync() -> &'static str {
        r#"AI CONTRACT for isolate sync:
{
  "command": "isolate sync",
  "intent": "Sync session workspace with main branch by rebasing onto latest main",
  "prerequisites": [
    "Session must exist in database",
    "Workspace directory must exist",
    "JJ repository must be accessible",
    "No uncommitted changes with conflicts"
  ],
  "side_effects": {
    "creates": [],
    "modifies": ["Session workspace (rebases onto main)", "last_synced timestamp"],
    "state_transition": "workspace -> workspace (updated)"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "default": "current workspace (detected from context)",
      "description": "Session name to sync",
      "examples": ["feature-auth", "bugfix-123"]
    },
    "all": {
      "type": "boolean",
      "flag": "--all",
      "required": false,
      "description": "Sync all active sessions"
    },
    "dry_run": {
      "type": "boolean",
      "flag": "--dry-run",
      "required": false,
      "description": "Preview sync without executing"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "name": "string|null",
      "synced_count": "number",
      "failed_count": "number",
      "errors": "array of error objects"
    },
    "errors": [
      "SessionNotFound",
      "WorkspaceNotFound",
      "RebaseConflict",
      "JjCommandError"
    ]
  },
  "examples": [
    "isolate sync",
    "isolate sync feature-auth",
    "isolate sync --all",
    "isolate sync --dry-run",
    "isolate sync --json"
  ],
  "next_commands": [
    "isolate done",
    "isolate diff",
    "isolate status"
  ]
}"#
    }

    /// Machine-readable contract for isolate abort command
    pub const fn abort() -> &'static str {
        r#"AI CONTRACT for isolate abort:
{
  "command": "isolate abort",
  "intent": "Abandon workspace without merging, discarding all changes",
  "prerequisites": [
    "Must be in a workspace or specify --workspace",
    "Workspace should exist in session database"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Workspace files (unless --keep-workspace)"],
    "modifies": ["Bead status (set back to ready unless --no-bead-update)"],
    "state_transition": "active → abandoned"
  },
  "inputs": {
    "workspace": {
      "type": "string",
      "flag": "-w|--workspace",
      "required": false,
      "default": "current workspace",
      "description": "Workspace/session to abort"
    },
    "keep_workspace": {
      "type": "boolean",
      "flag": "--keep-workspace",
      "required": false,
      "description": "Keep workspace files, just remove from isolate tracking"
    },
    "no_bead_update": {
      "type": "boolean",
      "flag": "--no-bead-update",
      "required": false,
      "description": "Don't update bead status back to ready"
    },
    "dry_run": {
      "type": "boolean",
      "flag": "--dry-run",
      "required": false,
      "description": "Preview abort without executing"
    }
  },
  "outputs": {
    "success": {
      "session_name": "string",
      "workspace_removed": "boolean",
      "bead_updated": "boolean",
      "message": "string"
    },
    "errors": [
      "NotInWorkspace",
      "SessionNotFound",
      "WorkspaceRemovalFailed"
    ]
  },
  "examples": [
    "isolate abort",
    "isolate abort --workspace feature-x",
    "isolate abort --keep-workspace",
    "isolate abort --dry-run"
  ]
}"#
    }

    /// Machine-readable contract for isolate remove command
    pub const fn remove() -> &'static str {
        r#"AI CONTRACT for isolate remove:
{
  "command": "isolate remove",
  "intent": "Remove a session and its workspace, optionally merging changes first",
  "prerequisites": [
    "isolate init must have been run",
    "Session must exist in database (unless --idempotent)"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Workspace directory"],
    "modifies": ["Session database", "Main branch (if --merge)"],
    "state_transition": "active → removed"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Name of the session to remove",
      "examples": ["old-feature", "test-session", "experiment"]
    },
    "force": {
      "type": "boolean",
      "flag": "-f, --force",
      "required": false,
      "description": "Skip pre_remove hooks (non-interactive, no confirmation)"
    },
    "merge": {
      "type": "boolean",
      "flag": "-m, --merge",
      "required": false,
      "description": "Squash-merge changes to main before removal"
    },
    "keep_branch": {
      "type": "boolean",
      "flag": "-k, --keep-branch",
      "required": false,
      "description": "Preserve branch after removal"
    },
    "idempotent": {
      "type": "boolean",
      "flag": "--idempotent",
      "required": false,
      "description": "Succeed if session doesn't exist (safe for retries)"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "message": "string (e.g., 'Removed session <name>' or 'Session <name> already removed')"
    },
    "errors": [
      "SessionNotFound",
      "WorkspaceRemovalFailed",
      "MergeFailed",
      "DatabaseError"
    ]
  },
  "exit_codes": {
    "0": "Success",
    "1": "Validation error",
    "2": "Not found error",
    "3": "IO error"
  },
  "examples": [
    "isolate remove old-feature",
    "isolate remove test-session -f",
    "isolate remove feature-x --merge",
    "isolate remove stale-session --idempotent",
    "isolate remove experiment --json"
  ],
  "next_commands": [
    "isolate list",
    "isolate add <name>",
    "isolate clean"
  ]
}"#
    }

    /// Machine-readable contract for isolate status command
    pub const fn status() -> &'static str {
        r#"AI CONTRACT for isolate status:
{
  "command": "isolate status",
  "intent": "Query current state of sessions and workspaces",
  "prerequisites": [
    "isolate init must have been run"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "description": "Specific session name, or all if omitted"
    }
  },
  "outputs": {
    "success": {
      "sessions": [
        {
          "name": "string",
          "status": "active|paused|completed|failed",
          "workspace_path": "string",
          "branch": "string",
          "changes": {
            "modified": "number",
            "added": "number",
            "deleted": "number"
          },
          "beads": {
            "open": "number",
            "in_progress": "number",
            "blocked": "number"
          }
        }
      ]
    }
  },
  "examples": [
    "isolate status",
    "isolate status feature-auth"
  ]
}"#
    }

    /// Machine-readable contract for isolate ai command
    pub const fn ai() -> &'static str {
        r#"AI CONTRACT for isolate ai:
{
  "command": "isolate ai",
  "intent": "AI-first entry point providing status, workflows, and guidance for AI agents",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "subcommand": {
      "type": "string",
      "required": false,
      "default": "default overview",
      "options": ["status", "workflow", "quick-start", "next"],
      "description": "AI subcommand to execute"
    }
  },
  "outputs": {
    "status": {
      "location": "string",
      "workspace": "string|null",
      "agent_id": "string|null",
      "initialized": "boolean",
      "active_sessions": "number",
      "ready": "boolean",
      "suggestion": "string",
      "next_command": "string"
    },
    "workflow": {
      "name": "string",
      "steps": [
        {
          "step": "number",
          "command": "string",
          "description": "string"
        }
      ]
    },
    "quick-start": {
      "essential_commands": "array",
      "orientation": "array",
      "workflow": "array"
    },
    "next": {
      "action": "string",
      "command": "string",
      "reason": "string",
      "priority": "high|medium|low"
    }
  },
  "examples": [
    "isolate ai",
    "isolate ai status",
    "isolate ai workflow",
    "isolate ai quick-start",
    "isolate ai next",
    "isolate ai --json"
  ]
}"#
    }

    /// Machine-readable contract for isolate contract command
    pub const fn contract() -> &'static str {
        r#"AI CONTRACT for isolate contract:
{
  "command": "isolate contract",
  "intent": "Query machine-readable contracts for isolate commands to understand inputs, outputs, and side effects",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": false,
      "position": 1,
      "description": "Specific command to show contract for (shows all if omitted)",
      "examples": ["add", "done", "spawn", "work"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON format"
    }
  },
  "outputs": {
    "success": {
      "commands": [
        {
          "name": "string",
          "description": "string",
          "required_args": "array",
          "optional_args": "array",
          "flags": "array",
          "output_schema": "string",
          "side_effects": "array",
          "examples": "array",
          "reversible": "boolean",
          "undo_command": "string|null",
          "prerequisites": "array"
        }
      ],
      "global_flags": "array",
      "version": "string"
    },
    "errors": [
      "UnknownCommand"
    ]
  },
  "examples": [
    "isolate contract                    Show all command contracts",
    "isolate contract add                Show contract for 'add' command",
    "isolate contract --json             Output all contracts as JSON",
    "isolate contract done --json        Show 'done' contract as JSON"
  ]
}"#
    }

    /// Machine-readable contract for isolate can-i command
    pub const fn can_i() -> &'static str {
        r#"AI CONTRACT for isolate can-i:
{
  "command": "isolate can-i",
  "intent": "Check if an action is permitted in the current context",
  "prerequisites": [
    "isolate must be initialized"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "action": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Action to check permission for",
      "examples": ["add", "done", "merge", "abort"]
    },
    "resource": {
      "type": "string",
      "required": false,
      "position": 2,
      "description": "Resource to check permission on",
      "examples": ["session-name", "workspace-name"]
    }
  },
  "outputs": {
    "success": {
      "action": "string",
      "resource": "string|null",
      "permitted": "boolean",
      "reason": "string"
    },
    "errors": [
      "InvalidAction",
      "ResourceNotFound"
    ]
  },
  "examples": [
    "isolate can-i add",
    "isolate can-i done feature-x",
    "isolate can-i merge"
  ]
}"#
    }

    /// AI hints for command sequencing
    pub const fn command_flow() -> &'static str {
        r#"AI COMMAND FLOW:
{
  "typical_workflows": {
    "manual_feature_development": [
      "isolate init",
      "isolate add feature-name",
      "isolate focus feature-name",
      "... work ...",
      "isolate checkpoint create",
      "isolate done"
    ],
    "automated_agent_task": [
      "isolate init",
      "isolate work feature-name --agent-id agent-1",
      "isolate focus session-name",
      "... agent works ...",
      "isolate done"
    ],
    "parallel_agent_tasks": [
      "isolate init",
      "isolate spawn bead-1",
      "isolate spawn bead-2",
      "isolate spawn bead-3",
      "... agents work in parallel ...",
      "isolate sync --all",
      "isolate done --all"
    ]
  },
  "command_preconditions": {
    "isolate add": ["isolate init"],
    "isolate work": ["isolate init"],
    "isolate spawn": ["isolate init"],
    "isolate done": ["active session"],
    "isolate focus": ["Zellij running"],
    "isolate sync": ["active session"]
  },
  "error_recovery": {
    "MergeConflict": ["isolate resolve", "isolate done --force"],
    "WorkspaceDirty": ["isolate checkpoint create", "jj commit"],
    "SessionNotFound": ["isolate list", "isolate add"],
    "AgentCrash": ["isolate attach", "isolate status"]
  }
}"#
    }

    /// Machine-readable contract for isolate diff command
    pub const fn diff() -> &'static str {
        r#"AI CONTRACT for isolate diff:
{
  "command": "isolate diff",
  "intent": "Show changes between session workspace and main branch",
  "prerequisites": [
    "Session must exist in database",
    "Workspace directory must exist",
    "JJ repository must be accessible"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "default": "auto-detected from current workspace",
      "description": "Session name to show diff for",
      "examples": ["feature-auth", "bugfix-123"]
    },
    "stat": {
      "type": "boolean",
      "flag": "--stat",
      "required": false,
      "description": "Show diffstat summary instead of full diff"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "session": "string",
      "diff_type": "full|stat",
      "content": "string (diff output)",
      "stats": {
        "files_changed": "number",
        "insertions": "number",
        "deletions": "number"
      }
    },
    "errors": [
      "SessionNotFound",
      "WorkspaceNotFound",
      "JjCommandError"
    ]
  },
  "examples": [
    "isolate diff",
    "isolate diff feature-auth",
    "isolate diff --stat",
    "isolate diff feature-auth --json"
  ],
  "next_commands": [
    "isolate done",
    "isolate status",
    "isolate sync"
  ]
}"#
    }

    /// Machine-readable contract for isolate list command
    pub const fn list() -> &'static str {
        r#"AI CONTRACT for isolate list:
{
  "command": "isolate list",
  "intent": "Query all sessions in the repository to see status and metadata",
  "prerequisites": [
    "isolate init must have been run"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "all": {
      "type": "boolean",
      "flag": "--all",
      "required": false,
      "description": "Include completed and failed sessions (default: active only)"
    },
    "verbose": {
      "type": "boolean",
      "flag": "-v, --verbose",
      "required": false,
      "description": "Show workspace paths and bead titles"
    },
    "bead": {
      "type": "string",
      "flag": "--bead",
      "required": false,
      "description": "Filter sessions by bead ID",
      "examples": ["isolate-abc123", "feat-456"]
    },
    "agent": {
      "type": "string",
      "flag": "--agent",
      "required": false,
      "description": "Filter sessions by agent owner"
    },
    "state": {
      "type": "string",
      "flag": "--state",
      "required": false,
      "description": "Filter by workspace state (created, working, ready, merged, abandoned, conflict, active, complete, terminal, non-terminal)"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelopeArray"
    }
  },
  "outputs": {
    "success": {
      "schema_type": "array",
      "data": [
        {
          "name": "string",
          "status": "active|paused|completed|failed",
          "branch": "string",
          "changes": "string (count)",
          "beads": "string (open/in_progress/blocked)",
          "workspace_path": "string",
          "metadata": "object|null"
        }
      ]
    },
    "errors": [
      "DatabaseError"
    ]
  },
  "examples": [
    "isolate list",
    "isolate list --all",
    "isolate list --verbose",
    "isolate list --bead isolate-abc123",
    "isolate list --agent agent-001",
    "isolate list --state active",
    "isolate list --json"
  ],
  "next_commands": [
    "isolate status <name>",
    "isolate focus <name>",
    "isolate add <name>",
    "isolate work <bead_id>"
  ]
}"#
    }

    /// Machine-readable contract for isolate focus command
    pub const fn focus() -> &'static str {
        r#"AI CONTRACT for isolate focus:
{
  "command": "isolate focus",
  "intent": "Switch to a session to work on it",
  "prerequisites": [
    "Session must exist in database"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "position": 1,
      "default": "interactive selection",
      "description": "Name of the session to focus",
      "examples": ["feature-auth", "bugfix-123"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "message": "string"
    },
    "errors": [
      "SessionNotFound",
      "NoSessionsAvailable"
    ]
  },
  "examples": [
    "isolate focus feature-auth",
    "isolate focus",
    "isolate focus bugfix-123 --json"
  ],
  "next_commands": [
    "isolate status",
    "isolate done",
    "isolate diff"
  ]
}"#
    }

    /// Machine-readable contract for isolate context command
    pub const fn context() -> &'static str {
        r#"AI CONTRACT for isolate context:
{
  "command": "isolate context",
  "intent": "Show complete environment context for AI agents and programmatic access",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "field": {
      "type": "string",
      "flag": "--field",
      "required": false,
      "description": "Extract single field using JSON pointer path",
      "examples": ["repository.branch", "session.name", "location.path"]
    },
    "no_beads": {
      "type": "boolean",
      "flag": "--no-beads",
      "required": false,
      "description": "Skip beads database query (faster)"
    },
    "no_health": {
      "type": "boolean",
      "flag": "--no-health",
      "required": false,
      "description": "Skip health checks (faster)"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "default": "true when not TTY",
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "location": {
        "type": "string (main) or object (workspace)",
        "description": "Current location in repository"
      },
      "session": {
        "type": "object|null",
        "description": "Session context if in workspace",
        "fields": ["name", "status", "bead_id", "agent", "created_at", "last_synced"]
      },
      "repository": {
        "type": "object",
        "description": "Repository state information",
        "fields": ["root", "branch", "uncommitted_files", "commits_ahead", "has_conflicts"]
      },
      "beads": {
        "type": "object|null",
        "description": "Beads tracking information",
        "fields": ["active", "blocked_by", "ready_count", "in_progress_count"]
      },
      "health": {
        "type": "object",
        "description": "Health status of the system",
        "status_values": ["good", "warn", "error"]
      },
      "suggestions": {
        "type": "array of strings",
        "description": "Actionable suggestions based on context"
      }
    },
    "errors": [
      "NotInJjRepo",
      "SessionNotFound",
      "BeadsDatabaseError"
    ]
  },
  "examples": [
    "isolate context",
    "isolate context --json",
    "isolate context --field=repository.branch",
    "isolate context --no-beads --no-health",
    "isolate context --field=location.path"
  ],
  "next_commands": [
    "isolate whereami",
    "isolate status",
    "isolate work"
  ]
}"#
    }

    /// Machine-readable contract for isolate introspect command
    #[allow(clippy::too_many_lines)]
    pub const fn introspect() -> &'static str {
        r#"AI CONTRACT for isolate introspect:
{
  "command": "isolate introspect",
  "intent": "Discover isolate capabilities, command details, and system state for AI agent understanding",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": false,
      "position": 1,
      "description": "Specific command to introspect (shows all if omitted)",
      "examples": ["add", "done", "focus", "sync"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    },
    "ai": {
      "type": "boolean",
      "flag": "--ai",
      "required": false,
      "description": "AI-optimized output: combines capabilities, state, and recommendations"
    },
    "env-vars": {
      "type": "boolean",
      "flag": "--env-vars",
      "required": false,
      "description": "Show environment variables isolate reads and sets"
    },
    "workflows": {
      "type": "boolean",
      "flag": "--workflows",
      "required": false,
      "description": "Show common workflow patterns for AI agents"
    },
    "session-states": {
      "type": "boolean",
      "flag": "--session-states",
      "required": false,
      "description": "Show valid session state transitions"
    }
  },
  "outputs": {
    "success": {
      "version": "string",
      "commands": [
        {
          "name": "string",
          "description": "string",
          "arguments": "array",
          "flags": "array",
          "examples": "array",
          "prerequisites": "object",
          "side_effects": "array",
          "error_conditions": "array"
        }
      ],
      "dependencies": {
        "jj": "object|null"
      },
      "system_state": {
        "initialized": "boolean",
        "jj_repo": "boolean",
        "active_sessions": "number"
      }
    },
    "env_vars_mode": {
      "env_vars": [
        {
          "name": "string",
          "description": "string",
          "direction": "read|write|both",
          "default": "string|null",
          "example": "string"
        }
      ]
    },
    "workflows_mode": {
      "workflows": [
        {
          "name": "string",
          "description": "string",
          "steps": [
            {
              "step": "number",
              "command": "string",
              "description": "string"
            }
          ]
        }
      ]
    },
    "session_states_mode": {
      "states": ["creating", "active", "syncing", "merging", "completed", "failed"],
      "transitions": [
        {
          "from": "string",
          "to": "string",
          "trigger": "string"
        }
      ]
    },
    "errors": [
      "UnknownCommand"
    ]
  },
  "examples": [
    "isolate introspect",
    "isolate introspect add",
    "isolate introspect --json",
    "isolate introspect --env-vars",
    "isolate introspect --workflows",
    "isolate introspect --session-states",
    "isolate introspect --ai"
  ],
  "next_commands": [
    "isolate contract",
    "isolate context",
    "isolate ai"
  ]
}"#
    }

    /// Machine-readable contract for isolate examples command
    pub const fn examples() -> &'static str {
        r#"AI CONTRACT for isolate examples:
{
  "command": "isolate examples",
  "intent": "Show copy-pastable usage examples for commands, useful for AI agents and users",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "position": 1,
      "required": false,
      "description": "Filter examples for a specific command",
      "examples": ["add", "done", "work", "spawn"]
    },
    "use_case": {
      "type": "string",
      "flag": "--use-case",
      "required": false,
      "description": "Filter by use case category",
      "options": ["workflow", "single-command", "error-handling", "maintenance", "automation", "ai-agent", "multi-agent", "safety"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "examples": [
        {
          "name": "string",
          "description": "string",
          "commands": ["array of command strings"],
          "expected_output": "string or null",
          "use_case": "string",
          "prerequisites": ["array of strings"],
          "notes": "string or null"
        }
      ],
      "use_cases": ["array of available use case categories"]
    }
  },
  "examples": [
    "isolate examples",
    "isolate examples add",
    "isolate examples --use-case workflow",
    "isolate examples --json",
    "isolate examples done --json"
  ],
  "next_commands": [
    "isolate contract",
    "isolate ai quick-start",
    "isolate context"
  ]
}"#
    }

    /// Machine-readable contract for isolate validate command
    pub const fn validate() -> &'static str {
        r#"AI CONTRACT for isolate validate:
{
  "command": "isolate validate",
  "intent": "Validate command arguments before execution",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Command to validate arguments for"
    },
    "args": {
      "type": "array of strings",
      "required": false,
      "description": "Arguments to validate"
    },
    "dry_run": {
      "type": "boolean",
      "flag": "--dry-run",
      "required": false,
      "description": "Preview validation without executing"
    }
  },
  "outputs": {
    "success": {
      "valid": "boolean",
      "command": "string",
      "errors": "array of strings"
    }
  },
  "examples": [
    "isolate validate add feature-auth",
    "isolate validate remove old-session"
  ]
}"#
    }

    /// Machine-readable contract for isolate whatif command
    pub const fn whatif() -> &'static str {
        r#"AI CONTRACT for isolate whatif:
{
  "command": "isolate whatif",
  "intent": "Preview what a command would do without executing it, showing steps, resources, and reversibility",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none (preview only)"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Command to preview",
      "examples": ["add", "done", "remove", "spawn", "sync"]
    },
    "args": {
      "type": "array of strings",
      "required": false,
      "position": "2..",
      "description": "Arguments for the command being previewed",
      "examples": ["feature-auth", "--workspace", "my-session"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output preview as JSON with SchemaEnvelope"
    },
    "on_success": {
      "type": "string",
      "flag": "--on-success",
      "required": false,
      "description": "Command to run after successful execution"
    },
    "on_failure": {
      "type": "string",
      "flag": "--on-failure",
      "required": false,
      "description": "Command to run after failed execution"
    }
  },
  "outputs": {
    "success": {
      "command": "string",
      "args": "array of strings",
      "steps": [
        {
          "order": "number",
          "description": "string",
          "action": "string",
          "can_fail": "boolean",
          "on_failure": "string or null"
        }
      ],
      "creates": [
        {
          "resource_type": "string",
          "resource": "string",
          "description": "string"
        }
      ],
      "modifies": "array of resource changes",
      "deletes": "array of resource changes",
      "side_effects": "array of strings",
      "reversible": "boolean",
      "undo_command": "string or null",
      "warnings": "array of strings",
      "prerequisites": [
        {
          "check": "string",
          "status": "met|notmet|unknown",
          "description": "string"
        }
      ]
    },
    "errors": [
      "InvalidSessionName"
    ]
  },
  "examples": [
    "isolate whatif add feature-x",
    "isolate whatif done --workspace feature-x",
    "isolate whatif remove old-session",
    "isolate whatif spawn isolate-abc123",
    "isolate whatif sync --all --json"
  ],
  "next_commands": [
    "isolate add",
    "isolate done",
    "isolate remove",
    "isolate spawn"
  ]
}"#
    }

    /// Machine-readable contract for isolate whereami command
    pub const fn whereami() -> &'static str {
        r#"AI CONTRACT for isolate whereami:
{
  "command": "isolate whereami",
  "intent": "Quick location query returning simple location identifier for AI agent orientation",
  "prerequisites": [
    "Must be in a JJ repository"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    },
    "contract": {
      "type": "boolean",
      "flag": "--contract",
      "required": false,
      "description": "Show machine-readable contract for AI integration"
    }
  },
  "outputs": {
    "success": {
      "location_type": "string (main or workspace)",
      "workspace_name": "string or null",
      "workspace_path": "string or null",
      "simple": "string (main or workspace:<name>)"
    },
    "main_location": {
      "location_type": "main",
      "workspace_name": null,
      "workspace_path": null,
      "simple": "main"
    },
    "workspace_location": {
      "location_type": "workspace",
      "workspace_name": "<workspace_name>",
      "workspace_path": "<absolute_path>",
      "simple": "workspace:<workspace_name>"
    },
    "errors": [
      "NotInJjRepo"
    ]
  },
  "examples": [
    "isolate whereami                    Returns 'main' or 'workspace:<name>'",
    "isolate whereami --json             Full JSON output with SchemaEnvelope",
    "isolate whereami --contract         Show this contract"
  ],
  "next_commands": [
    "isolate context",
    "isolate status",
    "isolate work"
  ]
}"#
    }

    /// Machine-readable contract for isolate query command
    #[allow(clippy::too_many_lines)]
    pub const fn query() -> &'static str {
        r#"AI CONTRACT for isolate query:
{
  "command": "isolate query",
  "intent": "Query system state programmatically for AI agents and automation",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "query_type": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Type of query to execute",
      "options": [
        "session-exists",
        "session-count",
        "can-run",
        "suggest-name",
        "lock-status",
        "can-spawn",
        "pending-merges",
        "location"
      ],
      "examples": ["session-exists", "can-run", "location"]
    },
    "args": {
      "type": "string",
      "required": false,
      "position": 2,
      "description": "Query-specific arguments",
      "examples": ["my-session", "add", "feat{n}"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "default": true,
      "description": "Output as JSON (default for query command)"
    }
  },
  "outputs": {
    "session-exists": {
      "exists": "boolean",
      "session": {
        "name": "string",
        "status": "string"
      },
      "error": "object or null"
    },
    "session-count": {
      "count": "number",
      "filter": "object or null"
    },
    "can-run": {
      "can_run": "boolean",
      "command": "string",
      "blockers": ["array of blocker objects"],
      "prerequisites_met": "number",
      "prerequisites_total": "number"
    },
    "suggest-name": {
      "pattern": "string",
      "suggested": "string",
      "next_available_n": "number",
      "existing_matches": ["array of strings"]
    },
    "lock-status": {
      "session": "string",
      "locked": "boolean",
      "holder": "string or null",
      "expires_at": "string or null",
      "error": "object or null"
    },
    "can-spawn": {
      "can_spawn": "boolean",
      "bead_id": "string or null",
      "reason": "string or null",
      "blockers": ["array of strings"]
    },
    "pending-merges": {
      "sessions": ["array of session objects"],
      "count": "number",
      "error": "object or null"
    },
    "location": {
      "type": "string (main or workspace)",
      "name": "string or null",
      "path": "string or null",
      "simple": "string",
      "error": "object or null"
    },
    "errors": [
      "UnknownQueryType",
      "MissingRequiredArgument",
      "DatabaseError",
      "InvalidPattern"
    ]
  },
  "examples": [
    "isolate query session-exists my-feature",
    "isolate query session-count",
    "isolate query session-count --status=active",
    "isolate query can-run add",
    "isolate query suggest-name 'feature-{n}'",
    "isolate query lock-status my-session",
    "isolate query can-spawn",
    "isolate query pending-merges",
    "isolate query location"
  ],
  "next_commands": [
    "isolate context",
    "isolate status",
    "isolate introspect"
  ]
}"#
    }

    /// Machine-readable contract for isolate whoami command
    pub const fn whoami() -> &'static str {
        r#"AI CONTRACT for isolate whoami:
{
  "command": "isolate whoami",
  "intent": "Query the current agent identity - returns agent ID or 'unregistered'",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "registered": "boolean",
      "agent_id": "string|null",
      "current_session": "string|null",
      "current_bead": "string|null",
      "simple": "string (agent_id or 'unregistered')"
    },
    "environment_sources": {
      "Isolate_AGENT_ID": "Agent identifier",
      "Isolate_BEAD_ID": "Current bead being worked on",
      "Isolate_WORKSPACE": "Current workspace path",
      "Isolate_SESSION": "Current session name"
    }
  },
  "examples": [
    "isolate whoami",
    "isolate whoami --json"
  ],
  "next_commands": [
    "isolate context",
    "isolate status",
    "isolate whereami"
  ]
}"#
    }
}

#[cfg(test)]
mod tests {
    use super::ai_contracts;

    mod martin_fowler_work_contract_behavior {
        use super::*;

        /// GIVEN: The AI contract for `isolate work`
        /// WHEN: We inspect supported agent-related flags
        /// THEN: It should document `--agent-id` and reject stale `--agent`
        #[test]
        fn given_work_contract_when_inspecting_flags_then_documents_real_agent_flag() {
            let contract = ai_contracts::work();

            assert!(contract.contains("--agent-id"));
            assert!(!contract.contains("\"flag\": \"--agent\""));
        }

        /// GIVEN: The AI command flow examples
        /// WHEN: We inspect the automated agent workflow
        /// THEN: It should use current `isolate work` syntax
        #[test]
        fn given_command_flow_when_automated_workflow_then_examples_use_current_syntax() {
            let flow = ai_contracts::command_flow();

            assert!(flow.contains("isolate work feature-name --agent-id agent-1"));
            assert!(!flow.contains("isolate work bead-id --agent claude"));
        }

        /// GIVEN: The AI contract describes `isolate work` inputs
        /// WHEN: We inspect the input schema block
        /// THEN: It should use `name` as positional input and avoid stale `bead_id` positional
        /// input
        #[test]
        fn given_work_contract_inputs_when_reading_then_uses_name_not_stale_bead_id_position() {
            let contract = ai_contracts::work();

            assert!(contract.contains("\"name\""));
            assert!(!contract.contains("\"bead_id\": {"));
        }

        /// GIVEN: The AI contract describes `isolate work` output payload
        /// WHEN: We inspect success fields
        /// THEN: It should include actual runtime fields (`name`, `created`, `enter_command`)
        #[test]
        fn given_work_contract_outputs_when_reading_then_matches_runtime_shape() {
            let contract = ai_contracts::work();

            assert!(contract.contains("\"name\": \"string\""));
            assert!(contract.contains("\"created\": \"boolean\""));
            assert!(contract.contains("\"enter_command\": \"string\""));
        }
    }
}
