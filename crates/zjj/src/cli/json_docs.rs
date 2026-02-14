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
}
