// jjz Configuration Schema
// Defines all configuration options with defaults and validation
package jjz

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

#Config: {
    workspace_dir:    string | *"../{repo}__workspaces"
    main_branch:      string | *""  // auto-detected if empty
    default_template: string | *"standard"
    state_db:         string | *".jjz/state.db"

    watch:   #WatchConfig
    hooks:   #HooksConfig
    zellij:  #ZellijConfig
    dashboard: #DashboardConfig
    agent:   #AgentConfig
    session: #SessionConfig
}

#WatchConfig: {
    enabled:     bool | *true
    debounce_ms: int & >=10 & <=5000 | *100
    paths:       [...string] | *[".beads/beads.db"]
}

#HooksConfig: {
    post_create: [...string] | *[]
    pre_remove:  [...string] | *[]
    post_merge:  [...string] | *[]
}

#ZellijConfig: {
    session_prefix: string | *"jjz"
    use_tabs:       bool | *true
    layout_dir:     string | *".jjz/layouts"
    panes:          #PanesConfig
}

#PanesConfig: {
    main:  #PaneConfig & {command: string | *"claude", size: string | *"70%"}
    beads: #PaneConfig & {command: string | *"bv", size: string | *"50%"}
    status: #PaneConfig & {command: string | *"jjz", args: [...string] | *["status", "--watch"], size: string | *"50%"}
    float: #FloatPaneConfig
}

#PaneConfig: {
    command: string
    args:    [...string] | *[]
    size:    string
}

#FloatPaneConfig: {
    enabled: bool | *true
    command: string | *""
    width:   string | *"80%"
    height:  string | *"60%"
}

#DashboardConfig: {
    refresh_ms: int & >=100 & <=10000 | *1000
    theme:      string | *"default"
    columns:    [...string] | *["name", "status", "branch", "changes", "beads"]
    vim_keys:   bool | *true
}

#AgentConfig: {
    command: string | *"claude"
    env:     {[string]: string} | *{}
}

#SessionConfig: {
    auto_commit:   bool | *false
    commit_prefix: string | *"wip:"
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG LOADING ORDER
// ═══════════════════════════════════════════════════════════════════════════

#ConfigSource: {
    path:     string & !=""
    priority: int & >=1 & <=5
    required: bool
}

config_sources: [...#ConfigSource] & [
    {path: "<builtin>", priority: 1, required: true},
    {path: "~/.config/jjz/config.toml", priority: 2, required: false},
    {path: ".jjz/config.toml", priority: 3, required: false},
    {path: "JJZ_* environment variables", priority: 4, required: false},
    {path: "CLI flags", priority: 5, required: false},
]

// ═══════════════════════════════════════════════════════════════════════════
// ENVIRONMENT VARIABLE MAPPING
// ═══════════════════════════════════════════════════════════════════════════

#EnvVar: {
    env_name:    string & =~"^JJZ_"
    config_path: string & !=""
    value_type:  "string" | "bool" | "int" | "list"
}

env_vars: [...#EnvVar] & [
    {env_name: "JJZ_WORKSPACE_DIR", config_path: "workspace_dir", value_type: "string"},
    {env_name: "JJZ_MAIN_BRANCH", config_path: "main_branch", value_type: "string"},
    {env_name: "JJZ_DEFAULT_TEMPLATE", config_path: "default_template", value_type: "string"},
    {env_name: "JJZ_WATCH_ENABLED", config_path: "watch.enabled", value_type: "bool"},
    {env_name: "JJZ_WATCH_DEBOUNCE_MS", config_path: "watch.debounce_ms", value_type: "int"},
    {env_name: "JJZ_ZELLIJ_USE_TABS", config_path: "zellij.use_tabs", value_type: "bool"},
    {env_name: "JJZ_DASHBOARD_REFRESH_MS", config_path: "dashboard.refresh_ms", value_type: "int"},
    {env_name: "JJZ_DASHBOARD_VIM_KEYS", config_path: "dashboard.vim_keys", value_type: "bool"},
    {env_name: "JJZ_AGENT_COMMAND", config_path: "agent.command", value_type: "string"},
]

// ═══════════════════════════════════════════════════════════════════════════
// TEMPLATE VARIABLES
// ═══════════════════════════════════════════════════════════════════════════

#TemplateVar: {
    name:        string & =~"^\\{.*\\}$"
    description: string & !=""
    example:     string
}

template_vars: [...#TemplateVar] & [
    {name: "{session_name}", description: "Name of the session", example: "feature-auth"},
    {name: "{workspace_path}", description: "Full path to workspace directory", example: "/home/user/project__workspaces/feature-auth"},
    {name: "{repo_name}", description: "Repository directory name", example: "my-project"},
    {name: "{branch}", description: "Branch name for the session", example: "feature-auth"},
    {name: "{main_branch}", description: "Main branch name", example: "main"},
]

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT CONFIG INSTANCE
// ═══════════════════════════════════════════════════════════════════════════

default_config: #Config & {
    workspace_dir:    "../{repo}__workspaces"
    main_branch:      ""
    default_template: "standard"
    state_db:         ".jjz/state.db"

    watch: {
        enabled:     true
        debounce_ms: 100
        paths:       [".beads/beads.db"]
    }

    hooks: {
        post_create: []
        pre_remove:  []
        post_merge:  []
    }

    zellij: {
        session_prefix: "jjz"
        use_tabs:       true
        layout_dir:     ".jjz/layouts"
        panes: {
            main:  {command: "claude", args: [], size: "70%"}
            beads: {command: "bv", args: [], size: "50%"}
            status: {command: "jjz", args: ["status", "--watch"], size: "50%"}
            float: {enabled: true, command: "", width: "80%", height: "60%"}
        }
    }

    dashboard: {
        refresh_ms: 1000
        theme:      "default"
        columns:    ["name", "status", "branch", "changes", "beads"]
        vim_keys:   true
    }

    agent: {
        command: "claude"
        env:     {}
    }

    session: {
        auto_commit:   false
        commit_prefix: "wip:"
    }
}
