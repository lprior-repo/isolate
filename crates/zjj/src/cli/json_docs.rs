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
  "intent": "Create workspace and register agent for automated task execution",
  "prerequisites": [
    "zjj init must have been run",
    "Beads database must contain specified bead_id",
    "Agent must be registered (or --no-agent)"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Agent session", "Database session record", "Agent lock"],
    "modifies": ["Bead status (open → in_progress)"],
    "state_transition": "none → active"
  },
  "inputs": {
    "bead_id": {
      "type": "string",
      "required": true,
      "position": 1,
      "validation": "Must exist in beads database",
      "examples": ["zjj-abc123", "zjj-def456"]
    },
    "agent": {
      "type": "string",
      "required": false,
      "flag": "--agent",
      "description": "Agent command to run"
    },
    "no_agent": {
      "type": "boolean",
      "required": false,
      "flag": "--no-agent",
      "description": "Don't spawn agent, just create workspace"
    }
  },
  "outputs": {
    "success": {
      "session_name": "string",
      "workspace_path": "string",
      "bead_id": "string",
      "agent_id": "string",
      "status": "active"
    },
    "errors": [
      "BeadNotFound",
      "BeadAlreadyInProgress",
      "AgentNotRegistered",
      "WorkspaceCreationFailed"
    ]
  },
  "examples": [
    "zjj work zjj-abc123",
    "zjj work zjj-abc123 --agent claude",
    "zjj work zjj-abc123 --no-agent"
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
    },
    "idempotent": {
      "type": "boolean",
      "required": false,
      "flag": "--idempotent",
      "description": "Safe retry mode when workspace already exists"
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
    "zjj spawn zjj-abc123 --idempotent",
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
      "zjj work bead-id --agent claude",
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

    /// Machine-readable contract for zjj query command
    pub const fn query() -> &'static str {
        r#"AI CONTRACT for zjj query:
{
  "command": "zjj query",
  "intent": "Programmatically query system state without side effects",
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
      "options": ["session-exists", "session-count", "can-run", "suggest-name"],
      "position": 1
    },
    "args": {
      "type": "string",
      "required": false,
      "position": 2,
      "description": "Arguments specific to the query type"
    }
  },
  "outputs": {
    "success": {
      "query_type": "string",
      "result": "any"
    }
  },
  "examples": [
    "zjj query session-exists feature-x",
    "zjj query session-count",
    "zjj query suggest-name feature"
  ]
}"#
    }

    /// Machine-readable contract for zjj can-i command
    pub const fn can_i() -> &'static str {
        r#"AI CONTRACT for zjj can-i:
{
  "command": "zjj can-i",
  "intent": "Check permission or capability to perform an action",
  "prerequisites": [],
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
      "description": "The action to check (e.g., 'create-session', 'push')"
    },
    "resource": {
      "type": "string",
      "required": false,
      "position": 2,
      "description": "The resource to check against"
    }
  },
  "outputs": {
    "success": {
      "allowed": "boolean",
      "reason": "string"
    }
  },
  "examples": [
    "zjj can-i create-session",
    "zjj can-i push feature-branch"
  ]
}"#
    }

    /// Machine-readable contract for zjj integrity validate command
    pub const fn validate() -> &'static str {
        r#"AI CONTRACT for zjj integrity validate:
{
  "command": "zjj integrity validate",
  "intent": "Validate the integrity of a workspace or session",
  "prerequisites": [
    "Workspace must exist"
  ],
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
      "description": "Command to validate inputs for"
    },
    "args": {
      "type": "array",
      "required": false,
      "description": "Arguments to validate"
    }
  },
  "outputs": {
    "success": {
      "valid": "boolean",
      "issues": ["string"]
    }
  },
  "examples": [
    "zjj validate add feature-x"
  ]
}"#
    }

    /// Machine-readable contract for zjj introspect command
    pub const fn introspect() -> &'static str {
        r#"AI CONTRACT for zjj introspect:
{
  "command": "zjj introspect",
  "intent": "Discover capabilities, commands, and environment state",
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
      "description": "Specific command to introspect"
    },
    "ai": {
      "type": "boolean",
      "flag": "--ai",
      "description": "AI-optimized output"
    }
  },
  "outputs": {
    "success": {
      "commands": ["object"],
      "dependencies": "object",
      "system_state": "object"
    }
  },
  "examples": [
    "zjj introspect",
    "zjj introspect focus",
    "zjj introspect --ai"
  ]
}"#
    }

    /// Machine-readable contract for zjj whereami command
    pub const fn whereami() -> &'static str {
        r#"AI CONTRACT for zjj whereami:
{
  "command": "zjj whereami",
  "intent": "Determine current location (main branch vs workspace)",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {},
  "outputs": {
    "success": {
      "location_type": "main|workspace",
      "workspace_name": "string|null",
      "workspace_path": "string|null",
      "simple": "string"
    }
  },
  "examples": [
    "zjj whereami",
    "zjj whereami --json"
  ]
}"#
    }

    /// Machine-readable contract for zjj whoami command
    pub const fn whoami() -> &'static str {
        r#"AI CONTRACT for zjj whoami:
{
  "command": "zjj whoami",
  "intent": "Identify current agent and session context",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {},
  "outputs": {
    "success": {
      "agent_id": "string|null",
      "session_name": "string|null",
      "bead_id": "string|null",
      "registered": "boolean"
    }
  },
  "examples": [
    "zjj whoami",
    "zjj whoami --json"
  ]
}"#
    }

    /// Machine-readable contract for zjj list command
    pub const fn list() -> &'static str {
        r#"AI CONTRACT for zjj list:
{
  "command": "zjj list",
  "intent": "List and filter active sessions",
  "prerequisites": ["zjj init must have been run"],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "all": {
      "type": "boolean",
      "flag": "--all",
      "description": "Include completed/failed sessions"
    },
    "bead": {
      "type": "string",
      "flag": "--bead",
      "description": "Filter by bead ID"
    },
    "state": {
      "type": "string",
      "flag": "--state",
      "description": "Filter by state (active, working, ready, etc)"
    }
  },
  "outputs": {
    "success": {
      "data": ["session_object"]
    }
  },
  "examples": [
    "zjj list",
    "zjj list --all",
    "zjj list --bead zjj-123"
  ]
}"#
    }

    /// Machine-readable contract for zjj focus command
    pub const fn focus() -> &'static str {
        r#"AI CONTRACT for zjj focus:
{
  "command": "zjj focus",
  "intent": "Switch focus to a specific session (Zellij tab)",
  "prerequisites": ["Zellij must be running"],
  "side_effects": {
    "creates": [],
    "modifies": ["Zellij active tab"],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "position": 1,
      "description": "Session name (interactive if omitted)"
    },
    "no_zellij": {
      "type": "boolean",
      "flag": "--no-zellij",
      "description": "Skip Zellij integration (dry run equivalent)"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "zellij_tab": "string",
      "message": "string"
    },
    "errors": ["SessionNotFound", "ZellijNotRunning"]
  },
  "examples": [
    "zjj focus feature-auth",
    "zjj focus"
  ]
}"#
    }

    /// Machine-readable contract for zjj remove command
    pub const fn remove() -> &'static str {
        r#"AI CONTRACT for zjj remove:
{
  "command": "zjj remove",
  "intent": "Remove a session and its associated workspace",
  "prerequisites": ["Session must exist"],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Zellij tab"],
    "modifies": [],
    "state_transition": "any → none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Name of session to remove"
    },
    "force": {
      "type": "boolean",
      "flag": "-f|--force",
      "description": "Skip confirmation prompt"
    },
    "merge": {
      "type": "boolean",
      "flag": "-m|--merge",
      "description": "Merge to main before removing"
    },
    "keep_branch": {
      "type": "boolean",
      "flag": "-k|--keep-branch",
      "description": "Preserve git/jj branch after removal"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "message": "string"
    },
    "errors": ["SessionNotFound", "WorkspaceDirty", "MergeFailed"]
  },
  "examples": [
    "zjj remove feature-x",
    "zjj remove -f feature-y"
  ]
}"#
    }

    /// Machine-readable contract for zjj abort command
    pub const fn abort() -> &'static str {
        r#"AI CONTRACT for zjj abort:
{
  "command": "zjj abort",
  "intent": "Abandon work in a session without merging",
  "prerequisites": ["Session must exist"],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Zellij tab"],
    "modifies": ["Bead status (in_progress → open)"],
    "state_transition": "any → none"
  },
  "inputs": {
    "workspace": {
      "type": "string",
      "required": false,
      "position": 1,
      "default": "current",
      "description": "Workspace/session to abort"
    },
    "keep_workspace": {
      "type": "boolean",
      "flag": "--keep-workspace",
      "description": "Keep files on disk (just stop tracking)"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "message": "string"
    },
    "errors": ["SessionNotFound", "BeadUpdateFailed"]
  },
  "examples": [
    "zjj abort",
    "zjj abort feature-x"
  ]
}"#
    }

    /// Machine-readable contract for zjj sync command
    pub const fn sync() -> &'static str {
        r#"AI CONTRACT for zjj sync:
{
  "command": "zjj sync",
  "intent": "Synchronize workspace with main branch (rebase)",
  "prerequisites": ["Session must exist", "Main branch must be available"],
  "side_effects": {
    "creates": ["New commits (rebasing)"],
    "modifies": ["Workspace state", "JJ operation log"],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "position": 1,
      "default": "current",
      "description": "Session name to sync"
    },
    "all": {
      "type": "boolean",
      "flag": "--all",
      "description": "Sync all active sessions"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "synced_count": "number",
      "failed_count": "number"
    },
    "errors": ["SessionNotFound", "MergeConflict", "RebaseFailed"]
  },
  "examples": [
    "zjj sync",
    "zjj sync --all"
  ]
}"#
    }

    /// Machine-readable contract for zjj diff command
    pub const fn diff() -> &'static str {
        r#"AI CONTRACT for zjj diff:
{
  "command": "zjj diff",
  "intent": "Show changes between workspace and main branch",
  "prerequisites": ["Session must exist"],
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
      "default": "current",
      "description": "Session name to diff"
    },
    "stat": {
      "type": "boolean",
      "flag": "--stat",
      "description": "Show summary only (diffstat)"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "diff_stat": "object",
      "diff_content": "string"
    },
    "errors": ["SessionNotFound", "JJCommandFailed"]
  },
  "examples": [
    "zjj diff",
    "zjj diff --stat"
  ]
}"#
    }

    /// Machine-readable contract for zjj context command
    pub const fn context() -> &'static str {
        r#"AI CONTRACT for zjj context:
{
  "command": "zjj context",
  "intent": "Show complete environment context (AI agent query)",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "field": {
      "type": "string",
      "required": false,
      "flag": "--field",
      "description": "Extract a single field (e.g., repository.branch)"
    },
    "no_beads": {
      "type": "boolean",
      "flag": "--no-beads",
      "description": "Skip beads database query (faster)"
    },
    "no_health": {
      "type": "boolean",
      "flag": "--no-health",
      "description": "Skip health checks (faster)"
    }
  },
  "outputs": {
    "success": {
      "repository": "object",
      "sessions": ["session_object"],
      "beads": "object",
      "health": "object",
      "environment": "object"
    }
  },
  "examples": [
    "zjj context",
    "zjj context --field=repository.branch"
  ]
}"#
    }

    /// Machine-readable contract for zjj whatif command
    pub const fn whatif() -> &'static str {
        r#"AI CONTRACT for zjj whatif:
{
  "command": "zjj whatif",
  "intent": "Preview command effects without executing (impact analysis)",
  "prerequisites": ["Command must be valid"],
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
      "description": "Command to preview"
    },
    "args": {
      "type": "array",
      "required": false,
      "position": 2,
      "description": "Command arguments"
    }
  },
  "outputs": {
    "success": {
      "command": "string",
      "steps": ["string"],
      "resource_changes": {
        "files": ["string"],
        "sessions": ["string"],
        "db": ["string"]
      },
      "prerequisites_checked": ["string"],
      "reversible": "boolean"
    }
  },
  "examples": [
    "zjj whatif done",
    "zjj whatif add feature-x"
  ]
}"#
    }

    /// Machine-readable contract for zjj ai command
    pub const fn ai() -> &'static str {
        r#"AI CONTRACT for zjj ai:
{
  "command": "zjj ai",
  "intent": "AI-first entry point - start here for AI agents",
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
      "options": ["status", "workflow", "quick-start", "next"],
      "description": "AI action to perform"
    }
  },
  "outputs": {
    "success": {
      "status": "object",
      "workflow": "object",
      "next_action": {
        "action": "string",
        "command": "string",
        "reason": "string",
        "priority": "string"
      }
    }
  },
  "examples": [
    "zjj ai status",
    "zjj ai next"
  ]
}"#
    }

    /// Machine-readable contract for zjj contract command
    pub const fn contract() -> &'static str {
        r#"AI CONTRACT for zjj contract:
{
  "command": "zjj contract",
  "intent": "Show command contracts for AI integration (meta-discovery)",
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
      "description": "Command to show contract for"
    }
  },
  "outputs": {
    "success": {
      "contracts": ["contract_object"]
    }
  },
  "examples": [
    "zjj contract",
    "zjj contract add"
  ]
}"#
    }

    /// Machine-readable contract for zjj examples command
    pub const fn examples() -> &'static str {
        r#"AI CONTRACT for zjj examples:
{
  "command": "zjj examples",
  "intent": "Show usage examples for commands",
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
      "description": "Filter examples for specific command"
    },
    "use_case": {
      "type": "string",
      "required": false,
      "flag": "--use-case",
      "description": "Filter by use case category"
    }
  },
  "outputs": {
    "success": {
      "examples": [
        {
          "command": "string",
          "description": "string",
          "category": "string"
        }
      ]
    }
  },
  "examples": [
    "zjj examples",
    "zjj examples add"
  ]
}"#
    }
}
