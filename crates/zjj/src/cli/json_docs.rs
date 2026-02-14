//! JSON OUTPUT documentation for command help
//! These strings document the `SchemaEnvelope` structure used in JSON output

pub const fn add() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://add-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "zellij_tab": "zjj:<session_name>",
    "message": "Created session '<name>'"
  }"#
}

pub const fn list() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelopeArray:
  {
    "$schema": "zjj://list-response/v1",
    "_schema_version": "1.0",
    "schema_type": "array",
    "success": true,
    "data": [
      {
        "name": "<session_name>",
        "status": "<active|paused|completed|failed>",
        "branch": "<branch_name>",
        "changes": "<modified_count>",
        "beads": "<open/in_progress/blocked>",
        ...
      }
    ]
  }"#
}

pub const fn remove() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://remove-response/v1",
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
    "$schema": "zjj://focus-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "zellij_tab": "zjj:<session_name>",
    "message": "Switched to session '<name>'"
  }"#
}

pub const fn status() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelope:
  {
    "$schema": "zjj://status-response/v1",
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
    "$schema": "zjj://sync-response/v1",
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
    "$schema": "zjj://init-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "message": "<message>",
    "zjj_dir": "<absolute_path>",
    "config_file": "<absolute_path>",
    "state_db": "<absolute_path>",
    "layouts_dir": "<absolute_path>"
  }"#
}

pub const fn spawn() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://spawn-response/v1",
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
    "$schema": "zjj://done-response/v1",
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
    "$schema": "zjj://diff-response/v1",
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
    "$schema": "zjj://config-response/v1",
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
    "$schema": "zjj://clean-response/v1",
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
    "$schema": "zjj://introspect-response/v1",
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
    "$schema": "zjj://doctor-response/v1",
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
    "$schema": "zjj://query-response/v1",
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
    "$schema": "zjj://context-response/v1",
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
    "$schema": "zjj://checkpoint-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "action": "<create|restore|list>",
    "checkpoint_id": "<id_or_null>",
    "checkpoints": [...]
  }"#
}

/// AI-Native contract documentation for commands
pub mod ai_contracts {
    /// Machine-readable contract for zjj add command
    pub const fn add() -> &'static str {
        r#"AI CONTRACT for zjj add:
{
  "command": "zjj add",
  "intent": "Create isolated workspace for manual interactive development",
  "prerequisites": [
    "zjj init must have been run",
    "JJ repository must be initialized",
    "Zellij must be available (unless --no-open)"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Zellij tab", "Database session record"],
    "modifies": ["Zellij session layout"],
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
      "description": "Skip opening Zellij tab"
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
      "zellij_tab": "string",
      "status": "active"
    },
    "errors": [
      "SessionAlreadyExists",
      "InvalidSessionName",
      "JJInitFailed",
      "ZellijNotRunning",
      "DatabaseError"
    ]
  },
  "examples": [
    "zjj add feature-auth",
    "zjj add bugfix-123 --no-open",
    "zjj add experiment -t minimal"
  ],
  "next_commands": [
    "zjj focus <name>",
    "zjj status <name>",
    "zjj work <bead_id>"
  ]
}"#
    }

    /// Machine-readable contract for zjj work command
    pub const fn work() -> &'static str {
        r#"AI CONTRACT for zjj work:
{
  "command": "zjj work",
  "intent": "Create or reuse a named workspace and optionally register an agent",
  "prerequisites": [
    "zjj init must have been run",
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
    "no_zellij": {
      "type": "boolean",
      "required": false,
      "flag": "--no-zellij",
      "description": "Skip opening a Zellij tab"
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
      "zellij_tab": "string",
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
    "zjj work feature-auth",
    "zjj work bug-fix --bead zjj-123",
    "zjj work feature-auth --agent-id agent-1 --idempotent",
    "zjj work quick --no-agent --no-zellij",
    "zjj work feature-auth --dry-run"
  ],
  "next_commands": [
    "zjj done",
    "zjj checkpoint create",
    "zjj status"
  ]
}"#
    }

    /// Machine-readable contract for zjj spawn command
    pub const fn spawn() -> &'static str {
        r#"AI CONTRACT for zjj spawn:
{
  "command": "zjj spawn",
  "intent": "Create workspace and spawn automated agent with isolation",
  "prerequisites": [
    "zjj init must have been run",
    "Beads database must be available",
    "Agent system must be configured"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Zellij tab", "Agent process", "Database records"],
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
    "zjj spawn zjj-abc123",
    "zjj spawn zjj-abc123 --agent claude-opus"
  ]
}"#
    }

    /// Machine-readable contract for zjj done command
    pub const fn done() -> &'static str {
        r#"AI CONTRACT for zjj done:
{
  "command": "zjj done",
  "intent": "Complete work, merge changes to main, and cleanup workspace",
  "prerequisites": [
    "Session must be active",
    "Workspace must have committed changes",
    "No merge conflicts should exist"
  ],
  "side_effects": {
    "creates": ["Merge commit on main"],
    "deletes": ["JJ workspace", "Session record", "Zellij tab"],
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
    "zjj done",
    "zjj done feature-auth",
    "zjj done --force"
  ]
}"#
    }

    /// Machine-readable contract for zjj sync command
    pub const fn sync() -> &'static str {
        r#"AI CONTRACT for zjj sync:
{
  "command": "zjj sync",
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
    "zjj sync",
    "zjj sync feature-auth",
    "zjj sync --all",
    "zjj sync --dry-run",
    "zjj sync --json"
  ],
  "next_commands": [
    "zjj done",
    "zjj diff",
    "zjj status"
  ]
}"#
    }

    /// Machine-readable contract for zjj abort command
    pub const fn abort() -> &'static str {
        r#"AI CONTRACT for zjj abort:
{
  "command": "zjj abort",
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
      "description": "Keep workspace files, just remove from zjj tracking"
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
    "zjj abort",
    "zjj abort --workspace feature-x",
    "zjj abort --keep-workspace",
    "zjj abort --dry-run"
  ]
}"#
    }

    /// Machine-readable contract for zjj remove command
    pub const fn remove() -> &'static str {
        r#"AI CONTRACT for zjj remove:
{
  "command": "zjj remove",
  "intent": "Remove a session and its workspace, optionally merging changes first",
  "prerequisites": [
    "zjj init must have been run",
    "Session must exist in database (unless --idempotent)"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Workspace directory", "Zellij tab (if exists)"],
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
      "description": "Skip confirmation prompt and pre_remove hooks"
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
    "zjj remove old-feature",
    "zjj remove test-session -f",
    "zjj remove feature-x --merge",
    "zjj remove stale-session --idempotent",
    "zjj remove experiment --json"
  ],
  "next_commands": [
    "zjj list",
    "zjj add <name>",
    "zjj clean"
  ]
}"#
    }

    /// Machine-readable contract for zjj status command
    pub const fn status() -> &'static str {
        r#"AI CONTRACT for zjj status:
{
  "command": "zjj status",
  "intent": "Query current state of sessions and workspaces",
  "prerequisites": [
    "zjj init must have been run"
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
    "zjj status",
    "zjj status feature-auth"
  ]
}"#
    }

    /// Machine-readable contract for zjj ai command
    pub const fn ai() -> &'static str {
        r#"AI CONTRACT for zjj ai:
{
  "command": "zjj ai",
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
    "zjj ai",
    "zjj ai status",
    "zjj ai workflow",
    "zjj ai quick-start",
    "zjj ai next",
    "zjj ai --json"
  ]
}"#
    }

    /// Machine-readable contract for zjj contract command
    pub const fn contract() -> &'static str {
        r#"AI CONTRACT for zjj contract:
{
  "command": "zjj contract",
  "intent": "Query machine-readable contracts for zjj commands to understand inputs, outputs, and side effects",
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
    "zjj contract                    Show all command contracts",
    "zjj contract add                Show contract for 'add' command",
    "zjj contract --json             Output all contracts as JSON",
    "zjj contract done --json        Show 'done' contract as JSON"
  ]
}"#
    }

    /// Machine-readable contract for zjj can-i command
    pub const fn can_i() -> &'static str {
        r#"AI CONTRACT for zjj can-i:
{
  "command": "zjj can-i",
  "intent": "Check if an action is permitted in the current context",
  "prerequisites": [
    "zjj must be initialized"
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
    "zjj can-i add",
    "zjj can-i done feature-x",
    "zjj can-i merge"
  ]
}"#
    }

    /// AI hints for command sequencing
    pub const fn command_flow() -> &'static str {
        r#"AI COMMAND FLOW:
{
  "typical_workflows": {
    "manual_feature_development": [
      "zjj init",
      "zjj add feature-name",
      "zjj focus feature-name",
      "... work ...",
      "zjj checkpoint create",
      "zjj done"
    ],
    "automated_agent_task": [
      "zjj init",
      "zjj work feature-name --agent-id agent-1",
      "zjj focus session-name",
      "... agent works ...",
      "zjj done"
    ],
    "parallel_agent_tasks": [
      "zjj init",
      "zjj spawn bead-1",
      "zjj spawn bead-2",
      "zjj spawn bead-3",
      "... agents work in parallel ...",
      "zjj sync --all",
      "zjj done --all"
    ]
  },
  "command_preconditions": {
    "zjj add": ["zjj init"],
    "zjj work": ["zjj init"],
    "zjj spawn": ["zjj init"],
    "zjj done": ["active session"],
    "zjj focus": ["Zellij running"],
    "zjj sync": ["active session"]
  },
  "error_recovery": {
    "MergeConflict": ["zjj resolve", "zjj done --force"],
    "WorkspaceDirty": ["zjj checkpoint create", "jj commit"],
    "SessionNotFound": ["zjj list", "zjj add"],
    "AgentCrash": ["zjj attach", "zjj status"]
  }
}"#
    }

    /// Machine-readable contract for zjj diff command
    pub const fn diff() -> &'static str {
        r#"AI CONTRACT for zjj diff:
{
  "command": "zjj diff",
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
    "zjj diff",
    "zjj diff feature-auth",
    "zjj diff --stat",
    "zjj diff feature-auth --json"
  ],
  "next_commands": [
    "zjj done",
    "zjj status",
    "zjj sync"
  ]
}"#
    }

    /// Machine-readable contract for zjj list command
    pub const fn list() -> &'static str {
        r#"AI CONTRACT for zjj list:
{
  "command": "zjj list",
  "intent": "Query all sessions in the repository to see status and metadata",
  "prerequisites": [
    "zjj init must have been run"
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
      "examples": ["zjj-abc123", "feat-456"]
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
          "zellij_tab": "string",
          "metadata": "object|null"
        }
      ]
    },
    "errors": [
      "DatabaseError"
    ]
  },
  "examples": [
    "zjj list",
    "zjj list --all",
    "zjj list --verbose",
    "zjj list --bead zjj-abc123",
    "zjj list --agent agent-001",
    "zjj list --state active",
    "zjj list --json"
  ],
  "next_commands": [
    "zjj status <name>",
    "zjj focus <name>",
    "zjj add <name>",
    "zjj work <bead_id>"
  ]
}"#
    }

    /// Machine-readable contract for zjj focus command
    pub const fn focus() -> &'static str {
        r#"AI CONTRACT for zjj focus:
{
  "command": "zjj focus",
  "intent": "Switch to a session's Zellij tab to work on that session",
  "prerequisites": [
    "Session must exist in database",
    "Zellij must be running (unless --no-zellij)"
  ],
  "side_effects": {
    "creates": [],
    "modifies": ["Active Zellij tab"],
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
    "no_zellij": {
      "type": "boolean",
      "flag": "--no-zellij",
      "required": false,
      "description": "Skip Zellij integration (for non-TTY environments)"
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
      "zellij_tab": "string",
      "message": "string"
    },
    "errors": [
      "SessionNotFound",
      "ZellijNotRunning",
      "NoSessionsAvailable"
    ]
  },
  "examples": [
    "zjj focus feature-auth",
    "zjj focus",
    "zjj focus bugfix-123 --json",
    "zjj focus --no-zellij"
  ],
  "next_commands": [
    "zjj status",
    "zjj done",
    "zjj diff"
  ]
}"#
    }

    /// Machine-readable contract for zjj context command
    pub const fn context() -> &'static str {
        r#"AI CONTRACT for zjj context:
{
  "command": "zjj context",
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
    "zjj context",
    "zjj context --json",
    "zjj context --field=repository.branch",
    "zjj context --no-beads --no-health",
    "zjj context --field=location.path"
  ],
  "next_commands": [
    "zjj whereami",
    "zjj status",
    "zjj work"
  ]
}"#
    }

    /// Machine-readable contract for zjj introspect command
    #[allow(clippy::too_many_lines)]
    pub const fn introspect() -> &'static str {
        r#"AI CONTRACT for zjj introspect:
{
  "command": "zjj introspect",
  "intent": "Discover zjj capabilities, command details, and system state for AI agent understanding",
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
      "description": "Show environment variables zjj reads and sets"
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
        "jj": "object|null",
        "zellij": "object|null"
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
    "zjj introspect",
    "zjj introspect add",
    "zjj introspect --json",
    "zjj introspect --env-vars",
    "zjj introspect --workflows",
    "zjj introspect --session-states",
    "zjj introspect --ai"
  ],
  "next_commands": [
    "zjj contract",
    "zjj context",
    "zjj ai"
  ]
}"#
    }

    /// Machine-readable contract for zjj examples command
    pub const fn examples() -> &'static str {
        r#"AI CONTRACT for zjj examples:
{
  "command": "zjj examples",
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
    "zjj examples",
    "zjj examples add",
    "zjj examples --use-case workflow",
    "zjj examples --json",
    "zjj examples done --json"
  ],
  "next_commands": [
    "zjj contract",
    "zjj ai quick-start",
    "zjj context"
  ]
}"#
    }

    /// Machine-readable contract for zjj validate command
    pub const fn validate() -> &'static str {
        r#"AI CONTRACT for zjj validate:
{
  "command": "zjj validate",
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
    "zjj validate add feature-auth",
    "zjj validate remove old-session"
  ]
}"#
    }

    /// Machine-readable contract for zjj query command
    pub const fn query() -> &'static str {
        r#"AI CONTRACT for zjj query:
{
  "command": "zjj query",
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
    "zjj query session-exists my-feature",
    "zjj query session-count",
    "zjj query session-count --status=active",
    "zjj query can-run add",
    "zjj query suggest-name 'feature-{n}'",
    "zjj query lock-status my-session",
    "zjj query can-spawn",
    "zjj query pending-merges",
    "zjj query location"
  ],
  "next_commands": [
    "zjj context",
    "zjj status",
    "zjj introspect"
  ]
}"#
    }
}

#[cfg(test)]
mod tests {
    use super::ai_contracts;

    mod martin_fowler_work_contract_behavior {
        use super::*;

        /// GIVEN: The AI contract for `zjj work`
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
        /// THEN: It should use current `zjj work` syntax
        #[test]
        fn given_command_flow_when_automated_workflow_then_examples_use_current_syntax() {
            let flow = ai_contracts::command_flow();

            assert!(flow.contains("zjj work feature-name --agent-id agent-1"));
            assert!(!flow.contains("zjj work bead-id --agent claude"));
        }

        /// GIVEN: The AI contract describes `zjj work` inputs
        /// WHEN: We inspect the input schema block
        /// THEN: It should use `name` as positional input and avoid stale `bead_id` positional
        /// input
        #[test]
        fn given_work_contract_inputs_when_reading_then_uses_name_not_stale_bead_id_position() {
            let contract = ai_contracts::work();

            assert!(contract.contains("\"name\""));
            assert!(!contract.contains("\"bead_id\": {"));
        }

        /// GIVEN: The AI contract describes `zjj work` output payload
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
