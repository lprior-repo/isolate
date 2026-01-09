// jjz Research Notes
// Documents analysis of existing tools and technology decisions
package jjz

// ═══════════════════════════════════════════════════════════════════════════
// PROBLEM STATEMENT
// ═══════════════════════════════════════════════════════════════════════════

problem: {
    description: "Running multiple Claude Code instances on the same codebase causes conflicts"
    symptoms: [
        "File overwrites between agents",
        "Context pollution across sessions",
        "Build artifact collisions in _build directory",
        "Test interference when running simultaneously",
    ]
    solution: "Isolated workspaces with unified orchestration layer"
}

// ═══════════════════════════════════════════════════════════════════════════
// ANALYZED TOOLS
// ═══════════════════════════════════════════════════════════════════════════

#ToolAnalysis: {
    name:        string & !=""
    url:         string & !=""
    description: string & !=""
    patterns:    [...string]
    adopted:     [...string]
    rejected:    [...string]
}

analyzed_tools: [...#ToolAnalysis] & [
    {
        name:        "Crystal"
        url:         "https://github.com/stravu/crystal"
        description: "Desktop app for parallel Claude/Codex sessions"
        patterns: [
            "Disposable worktrees as cheap experiments",
            "Commit-per-iteration for automatic undo/redo",
            "Session metadata in ~/.crystal",
            "Agent-agnostic design with pluggable backends",
            "Environment-based config for enterprise deployments",
        ]
        adopted: [
            "Disposable workspace philosophy",
            "Session state persistence in dedicated directory",
        ]
        rejected: [
            "Desktop app approach - prefer CLI/TUI",
        ]
    },
    {
        name:        "workmux"
        url:         "https://github.com/raine/workmux"
        description: "Git worktrees + tmux windows integration"
        patterns: [
            "Two-level config: global + project .workmux.yaml",
            "Pane layout definitions in config",
            "Lifecycle hooks: post_create, pre_merge, pre_remove",
            "Sibling directory pattern: project__worktrees/",
            "Agent status icons in window names",
            "LLM-powered branch auto-naming",
        ]
        adopted: [
            "Config hierarchy (global + project)",
            "Hooks system with lifecycle events",
            "Sibling directory pattern for workspaces",
            "Pane layout configuration",
        ]
        rejected: [
            "tmux dependency - using Zellij instead",
            "LLM branch naming - too complex for MVP",
        ]
    },
    {
        name:        "Vibe Kanban"
        url:         "https://github.com/BloopAI/vibe-kanban"
        description: "Web-based kanban for AI agent orchestration"
        patterns: [
            "Decoupled service architecture",
            "SQLite state persistence",
            "Git worktree isolation built-in",
            "Multi-agent parallel execution",
            "Real-time status tracking via web UI",
        ]
        adopted: [
            "SQLite for state storage",
            "Decoupled component architecture",
        ]
        rejected: [
            "Web UI - prefer TUI for terminal workflow",
            "Node.js stack - using Rust",
        ]
    },
    {
        name:        "bdui"
        url:         "https://github.com/assimelha/bdui"
        description: "TUI for beads issue tracker"
        patterns: [
            "Responsive column layout adapting to terminal width",
            "4-column kanban: Open -> In Progress -> Blocked -> Done",
            "Debounced file watching at 100ms on beads.db",
            "Per-column scroll and selection state",
            "Vim-style navigation with h/j/k/l",
        ]
        adopted: [
            "Responsive layout based on terminal width",
            "File watching with debounce",
            "Vim-style keybindings",
            "Kanban column organization",
        ]
        rejected: [
            "Ink/React stack - using ratatui",
        ]
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// TECHNOLOGY DECISIONS
// ═══════════════════════════════════════════════════════════════════════════

#TechDecision: {
    choice:      string & !=""
    alternative: string & !=""
    rationale:   [...string]
}

tech_decisions: [...#TechDecision] & [
    {
        choice:      "JJ workspaces"
        alternative: "Git worktrees"
        rationale: [
            "First-class workspace concept in JJ",
            "Better conflict handling than git",
            "Simpler mental model without detached HEAD issues",
            "jj workspace update-stale for sync",
            "Anonymous branches by default reduce ceremony",
        ]
    },
    {
        choice:      "Zellij"
        alternative: "tmux"
        rationale: [
            "KDL layout files are declarative and version-controllable",
            "Built-in plugin system for future extensibility",
            "Better floating pane support",
            "Session resume built-in",
            "Modern Rust codebase aligns with jjz",
        ]
    },
    {
        choice:      "Rust"
        alternative: "Go, TypeScript, Python"
        rationale: [
            "Fast startup critical for CLI tools",
            "ratatui ecosystem is mature",
            "Same language as Zellij enables future plugin path",
            "rusqlite is well-supported",
            "Cross-platform without runtime dependency",
        ]
    },
    {
        choice:      "SQLite"
        alternative: "JSON file, YAML, in-memory"
        rationale: [
            "Single file with no server overhead",
            "ACID transactions for state consistency",
            "Easy querying for dashboard views",
            "Survives crashes unlike in-memory",
            "Well-supported by rusqlite",
        ]
    },
    {
        choice:      "TOML config"
        alternative: "YAML, JSON, KDL"
        rationale: [
            "Rust ecosystem standard (Cargo.toml)",
            "Human readable and writable",
            "Good library support with serde",
            "Clear syntax without YAML gotchas",
        ]
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// ZELLIJ KDL REFERENCE
// ═══════════════════════════════════════════════════════════════════════════

zellij_kdl: {
    pane_properties: {
        split_direction: "vertical | horizontal"
        size:            "percentage (50%) or fixed number"
        borderless:      "true | false"
        focus:           "true | false - initial focus"
        name:            "string - pane title"
        cwd:             "path - working directory"
        command:         "executable path"
        args:            "command arguments"
        close_on_exit:   "true | false"
        start_suspended: "true | false - wait for ENTER"
    }

    floating_pane_properties: {
        x:      "position or percentage"
        y:      "position or percentage"
        width:  "size or percentage"
        height: "size or percentage"
    }

    cli_actions: [
        "zellij action new-tab --layout /path/to/layout.kdl",
        "zellij action rename-tab 'name'",
        "zellij action close-tab",
        "zellij action go-to-tab-name 'name'",
        "zellij run -- command args",
        "zellij run -f -- command  # floating",
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// BEADS INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════

beads_integration: {
    description: "Steve Yegge's task management system for coding agents"
    database:    ".beads/beads.db (SQLite)"

    tables: {
        issues:       "id, title, status, priority, type"
        dependencies: "issue_id, depends_on_id"
        comments:     "issue_id, content, timestamp"
    }

    status_values: ["open", "in_progress", "blocked", "closed"]

    watch_pattern: """
        use notify::{Watcher, RecursiveMode};
        let mut watcher = notify::recommended_watcher(|res| { ... })?;
        watcher.watch(".beads/beads.db", RecursiveMode::NonRecursive)?;
        """
}

// ═══════════════════════════════════════════════════════════════════════════
// CLAUDE CODE INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════

claude_integration: {
    session_resume:  "Claude Code stores sessions per project directory. /resume picker shows sessions from same git repo including worktrees."
    spawn_command:   "claude"
    workspace_aware: "Set cwd in Zellij pane to workspace directory"

    commit_pattern: """
        Each agent iteration can create a commit for undo/redo:
        jj commit -m "wip: agent iteration $(date +%s)"
        """
}

// ═══════════════════════════════════════════════════════════════════════════
// PERFORMANCE NOTES
// ═══════════════════════════════════════════════════════════════════════════

performance: {
    workspace_creation: "~1-2s (JJ is fast)"
    hook_execution:     "Variable - deps install can be slow"
    file_watching:      "100ms debounce prevents thrashing"
    sqlite_access:      "Single-file, no server overhead"
    tui_refresh:        "1s interval for dashboard"
}
