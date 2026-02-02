// zjj CLI Command Specification
// Defines all commands, subcommands, arguments, and options
package zjj

// ═══════════════════════════════════════════════════════════════════════════
// COMMAND SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

#Command: {
    name:        string & !=""
    aliases:     [...string]
    description: string & !=""
    args:        [...#Argument]
    flags:       [...#Flag]
    examples:    [...#Example]
}

#Argument: {
    name:        string & !=""
    description: string & !=""
    required:    bool | *true
    validation:  string | *""
}

#Flag: {
    long:        string & !=""
    short:       string | *""
    description: string & !=""
    takes_value: bool | *false
    default:     string | *""
}

#Example: {
    command: string & !=""
    description: string & !=""
}

// ═══════════════════════════════════════════════════════════════════════════
// COMMAND DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════

commands: [...#Command] & [
    {
        name:        "add"
        aliases:     ["a", "new"]
        description: "Create new parallel development session"
        args: [
            {name: "name", description: "Session name", required: true, validation: "^[a-zA-Z0-9_-]+$"},
        ]
        flags: [
            {long: "no-hooks", description: "Skip post_create hooks"},
            {long: "template", short: "t", description: "Layout template name", takes_value: true, default: "standard"},
            {long: "no-open", description: "Create workspace but don't open Zellij tab"},
        ]
        examples: [
            {command: "zjj add feature-auth", description: "Create session with default template"},
            {command: "zjj add bugfix-123 --no-hooks", description: "Create without running hooks"},
            {command: "zjj add experiment -t minimal", description: "Create with minimal layout"},
        ]
    },
    {
        name:        "list"
        aliases:     ["ls", "l"]
        description: "Show all sessions with status"
        args: []
        flags: [
            {long: "all", short: "a", description: "Include completed and failed sessions"},
            {long: "json", description: "Output as JSON"},
        ]
        examples: [
            {command: "zjj list", description: "Show active sessions"},
            {command: "zjj list --all", description: "Show all sessions including completed"},
            {command: "zjj list --json", description: "JSON output for scripting"},
        ]
    },
    {
        name:        "remove"
        aliases:     ["rm", "delete"]
        description: "Remove session and cleanup workspace"
        args: [
            {name: "name", description: "Session name to remove", required: true},
        ]
        flags: [
            {long: "merge", short: "m", description: "Squash merge to main before removing"},
            {long: "force", short: "f", description: "Skip confirmation and hooks"},
            {long: "keep-branch", description: "Don't delete the branch after removal"},
        ]
        examples: [
            {command: "zjj remove feature-auth", description: "Remove session with confirmation"},
            {command: "zjj remove feature-auth --merge", description: "Merge changes then remove"},
            {command: "zjj remove experiment --force", description: "Force remove without prompts"},
        ]
    },
    {
        name:        "status"
        aliases:     ["st", "s"]
        description: "Show detailed session status"
        args: [
            {name: "name", description: "Session name (optional, shows all if omitted)", required: false},
        ]
        flags: [
            {long: "json", description: "Output as JSON"},
            {long: "watch", short: "w", description: "Continuously watch for changes"},
        ]
        examples: [
            {command: "zjj status", description: "Show status of all sessions"},
            {command: "zjj status feature-auth", description: "Show specific session status"},
            {command: "zjj status --watch", description: "Live status updates"},
        ]
    },
    {
        name:        "dashboard"
        aliases:     ["dash", "d"]
        description: "Open TUI dashboard with kanban view"
        args: []
        flags: []
        examples: [
            {command: "zjj dashboard", description: "Open interactive dashboard"},
            {command: "zjj dash", description: "Short alias"},
        ]
    },
    {
        name:        "focus"
        aliases:     ["f", "goto"]
        description: "Switch to session's Zellij tab"
        args: [
            {name: "name", description: "Session name to focus", required: true},
        ]
        flags: []
        examples: [
            {command: "zjj focus feature-auth", description: "Switch to feature-auth tab"},
        ]
    },
    {
        name:        "sync"
        aliases:     []
        description: "Sync workspaces with main repository"
        args: [
            {name: "name", description: "Session name (optional, syncs all if omitted)", required: false},
        ]
        flags: []
        examples: [
            {command: "zjj sync", description: "Sync all workspaces"},
            {command: "zjj sync feature-auth", description: "Sync specific workspace"},
        ]
    },
    {
        name:        "diff"
        aliases:     []
        description: "Show diff between session and main"
        args: [
            {name: "name", description: "Session name", required: true},
        ]
        flags: [
            {long: "stat", description: "Show diffstat only"},
        ]
        examples: [
            {command: "zjj diff feature-auth", description: "Show full diff"},
            {command: "zjj diff feature-auth --stat", description: "Show summary stats"},
        ]
    },
    {
        name:        "init"
        aliases:     []
        description: "Initialize zjj in current repository"
        args: []
        flags: [
            {long: "global", description: "Create global config only"},
        ]
        examples: [
            {command: "zjj init", description: "Initialize project config"},
            {command: "zjj init --global", description: "Initialize global config"},
        ]
    },
    {
        name:        "config"
        aliases:     ["cfg"]
        description: "View or set configuration"
        args: [
            {name: "key", description: "Config key to view/set", required: false},
            {name: "value", description: "Value to set (omit to view)", required: false},
        ]
        flags: [
            {long: "global", short: "g", description: "Operate on global config"},
        ]
        examples: [
            {command: "zjj config", description: "Show all config"},
            {command: "zjj config workspace_dir", description: "Show specific key"},
            {command: "zjj config workspace_dir ../ws", description: "Set value"},
        ]
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT FORMATS
// ═══════════════════════════════════════════════════════════════════════════

#ListOutput: {
    name:    string & !=""
    status:  #SessionStatus
    branch:  string
    changes: string
    beads:   string
}

#StatusOutput: {
    name:       string & !=""
    status:     #SessionStatus
    workspace:  string & !=""
    branch:     string
    jj_status:  [...#FileChange]
    beads:      [...#BeadSummary]
}

#FileChange: {
    path:   string & !=""
    status: "M" | "A" | "D" | "R" | "?"
}

#BeadSummary: {
    id:     string & !=""
    title:  string & !=""
    status: "open" | "in_progress" | "blocked" | "closed"
}

#SessionStatus: "creating" | "active" | "paused" | "completed" | "failed"
