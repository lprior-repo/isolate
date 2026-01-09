// jjz Architecture Specification
// Defines system components, data flow, and technology stack
package jjz

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════

#Component: {
    name:        string & !=""
    description: string & !=""
    depends_on:  [...#ComponentName]
    provides:    [...string]
}

#ComponentName: "cli" | "jj_manager" | "zellij_manager" | "state_store" | "config_loader" | "hook_runner" | "file_watcher" | "tui_dashboard"

components: {
    cli: #Component & {
        name:        "CLI"
        description: "Command-line interface using clap for argument parsing"
        depends_on:  ["jj_manager", "zellij_manager", "state_store", "config_loader", "hook_runner"]
        provides:    ["add", "list", "remove", "status", "dashboard", "focus", "sync", "init"]
    }

    jj_manager: #Component & {
        name:        "JJ Workspace Manager"
        description: "Manages JJ workspace lifecycle via jj CLI commands"
        depends_on:  ["config_loader"]
        provides:    ["workspace_create", "workspace_forget", "workspace_list", "workspace_status", "workspace_diff"]
    }

    zellij_manager: #Component & {
        name:        "Zellij Layout Manager"
        description: "Generates KDL layouts and spawns Zellij tabs via zellij action CLI"
        depends_on:  ["config_loader"]
        provides:    ["layout_generate", "tab_open", "tab_close", "tab_focus"]
    }

    state_store: #Component & {
        name:        "State Store"
        description: "SQLite-backed session state persistence"
        depends_on:  []
        provides:    ["session_create", "session_update", "session_delete", "session_query", "session_list"]
    }

    config_loader: #Component & {
        name:        "Configuration Loader"
        description: "Loads and merges global, project, and environment configuration"
        depends_on:  []
        provides:    ["config_load", "config_get", "config_defaults"]
    }

    hook_runner: #Component & {
        name:        "Hook Runner"
        description: "Executes shell commands for lifecycle hooks"
        depends_on:  ["config_loader"]
        provides:    ["hook_post_create", "hook_pre_remove", "hook_post_merge"]
    }

    file_watcher: #Component & {
        name:        "File Watcher"
        description: "Watches beads.db for changes using notify-rs with debouncing"
        depends_on:  ["state_store"]
        provides:    ["watch_start", "watch_stop", "event_stream"]
    }

    tui_dashboard: #Component & {
        name:        "TUI Dashboard"
        description: "Ratatui-based terminal UI with kanban layout"
        depends_on:  ["state_store", "jj_manager", "file_watcher", "zellij_manager"]
        provides:    ["dashboard_run", "session_view", "session_actions"]
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA FLOW
// ═══════════════════════════════════════════════════════════════════════════

#DataFlow: {
    trigger:     string & !=""
    source:      #ComponentName
    destination: #ComponentName
    data:        string & !=""
}

data_flows: [...#DataFlow] & [
    {trigger: "jjz add <name>", source: "cli", destination: "config_loader", data: "config request"},
    {trigger: "jjz add <name>", source: "cli", destination: "jj_manager", data: "workspace name + path"},
    {trigger: "jjz add <name>", source: "jj_manager", destination: "state_store", data: "session record"},
    {trigger: "jjz add <name>", source: "cli", destination: "hook_runner", data: "post_create hooks"},
    {trigger: "jjz add <name>", source: "cli", destination: "zellij_manager", data: "layout config"},
    {trigger: "jjz add <name>", source: "zellij_manager", destination: "state_store", data: "tab info update"},

    {trigger: "jjz remove <name>", source: "cli", destination: "hook_runner", data: "pre_remove hooks"},
    {trigger: "jjz remove <name>", source: "cli", destination: "zellij_manager", data: "tab close request"},
    {trigger: "jjz remove <name>", source: "cli", destination: "jj_manager", data: "workspace forget"},
    {trigger: "jjz remove <name>", source: "cli", destination: "state_store", data: "session delete"},

    {trigger: "jjz dashboard", source: "tui_dashboard", destination: "state_store", data: "session list query"},
    {trigger: "jjz dashboard", source: "tui_dashboard", destination: "jj_manager", data: "status queries"},
    {trigger: "jjz dashboard", source: "file_watcher", destination: "tui_dashboard", data: "beads change events"},
]

// ═══════════════════════════════════════════════════════════════════════════
// DIRECTORY STRUCTURE
// ═══════════════════════════════════════════════════════════════════════════

#DirectoryLayout: {
    path:        string & !=""
    description: string & !=""
    contents:    [...string]
}

directories: {
    project: #DirectoryLayout & {
        path:        ".jjz/"
        description: "Project-specific jjz data"
        contents:    ["config.toml", "state.db", "layouts/"]
    }

    global: #DirectoryLayout & {
        path:        "~/.config/jjz/"
        description: "Global jjz configuration"
        contents:    ["config.toml", "templates/"]
    }

    workspaces: #DirectoryLayout & {
        path:        "../{repo}__workspaces/"
        description: "Sibling directory for JJ workspaces"
        contents:    ["<session-name>/"]
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TECHNOLOGY STACK
// ═══════════════════════════════════════════════════════════════════════════

#Dependency: {
    name:    string & !=""
    version: string
    purpose: string & !=""
}

rust_dependencies: [...#Dependency] & [
    {name: "clap", version: "4", purpose: "CLI argument parsing with derive macros"},
    {name: "ratatui", version: "0.28", purpose: "Terminal UI framework"},
    {name: "crossterm", version: "0.28", purpose: "Terminal manipulation backend"},
    {name: "rusqlite", version: "0.32", purpose: "SQLite database access"},
    {name: "notify", version: "6", purpose: "File system event watching"},
    {name: "tokio", version: "1", purpose: "Async runtime for watchers"},
    {name: "serde", version: "1", purpose: "Serialization/deserialization"},
    {name: "toml", version: "0.8", purpose: "TOML config file parsing"},
    {name: "kdl", version: "4", purpose: "KDL layout file generation"},
    {name: "thiserror", version: "1", purpose: "Error type derivation"},
    {name: "directories", version: "5", purpose: "Platform-specific config paths"},
]

external_tools: [...#Dependency] & [
    {name: "jj", version: ">=0.20", purpose: "Jujutsu VCS for workspace management"},
    {name: "zellij", version: ">=0.40", purpose: "Terminal multiplexer for session layout"},
    {name: "claude", version: "*", purpose: "Claude Code AI agent"},
    {name: "bv", version: "*", purpose: "Beads viewer TUI"},
]
