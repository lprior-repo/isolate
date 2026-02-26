// isolate Requirements Specification - EARS Format
// Easy Approach to Requirements Syntax with CUE validation
// Patterns: Ubiquitous, State-Driven (While), Event-Driven (When),
//           Optional (Where), Unwanted Behavior (If/Then)
package isolate

// ═══════════════════════════════════════════════════════════════════════════
// EARS REQUIREMENT SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

#Requirement: {
    id:       =~"^REQ-[A-Z]+-[0-9]+$"  // e.g., REQ-CLI-001
    pattern:  #EARSPattern
    text:     string & !=""
    priority: "must" | "should" | "may"
    status:   "draft" | "approved" | "implemented" | "verified"
    trace?: [...string]  // traceability to design/test
}

#EARSPattern: "ubiquitous" | "state" | "event" | "optional" | "unwanted"

// ═══════════════════════════════════════════════════════════════════════════
// REQUIREMENT CATEGORIES
// ═══════════════════════════════════════════════════════════════════════════

#RequirementCategory: "cli" | "jj" | "zellij" | "state" | "config" | "hooks" | "tui" | "watch" | "error"

// ═══════════════════════════════════════════════════════════════════════════
// CLI REQUIREMENTS (REQ-CLI-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: cli: [
    {
        id:       "REQ-CLI-001"
        pattern:  "event"
        text:     "When the user invokes 'isolate add <name>', isolate shall create a new JJ workspace at the configured workspace directory with the given name."
        priority: "must"
        status:   "draft"
        trace:    ["DES-JJ-001", "TST-CLI-001"]
    },
    {
        id:       "REQ-CLI-002"
        pattern:  "event"
        text:     "When the user invokes 'isolate add <name>', isolate shall generate a Zellij KDL layout file for the session."
        priority: "must"
        status:   "draft"
        trace:    ["DES-ZELLIJ-001"]
    },
    {
        id:       "REQ-CLI-003"
        pattern:  "event"
        text:     "When the user invokes 'isolate add <name>', isolate shall open a new Zellij tab using the generated layout."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-004"
        pattern:  "event"
        text:     "When the user invokes 'isolate add <name>' and hooks are configured, isolate shall execute post_create hooks in the new workspace before opening Zellij."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-005"
        pattern:  "event"
        text:     "When the user invokes 'isolate add <name> --no-hooks', isolate shall skip all hook execution."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-006"
        pattern:  "event"
        text:     "When the user invokes 'isolate list', isolate shall display all sessions with their name, status, branch, and change summary."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-007"
        pattern:  "event"
        text:     "When the user invokes 'isolate remove <name>', isolate shall close the Zellij tab, run pre_remove hooks, and delete the JJ workspace."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-008"
        pattern:  "event"
        text:     "When the user invokes 'isolate remove <name> --merge', isolate shall squash-merge changes to main branch before removing the workspace."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-009"
        pattern:  "event"
        text:     "When the user invokes 'isolate status', isolate shall display detailed status of all sessions including JJ diff summary and beads status."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-010"
        pattern:  "event"
        text:     "When the user invokes 'isolate status <name>', isolate shall display detailed status of only the named session."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-011"
        pattern:  "event"
        text:     "When the user invokes 'isolate dashboard', isolate shall open the TUI dashboard showing all sessions in kanban view."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-012"
        pattern:  "event"
        text:     "When the user invokes 'isolate focus <name>', isolate shall switch to the named session's Zellij tab."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-013"
        pattern:  "event"
        text:     "When the user invokes 'isolate sync', isolate shall update all workspaces with changes from the main repository."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-014"
        pattern:  "event"
        text:     "When the user invokes 'isolate init', isolate shall create a .isolate directory with default config.toml."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-015"
        pattern:  "ubiquitous"
        text:     "isolate shall validate session names to contain only alphanumeric characters, hyphens, and underscores."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-016"
        pattern:  "ubiquitous"
        text:     "isolate shall support --json flag on list and status commands for machine-readable output."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CLI-017"
        pattern:  "unwanted"
        text:     "If isolate add is invoked while another add operation is in progress for the same name, isolate shall detect the lock and abort with an error."
        priority: "must"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// JJ INTEGRATION REQUIREMENTS (REQ-JJ-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: jj: [
    {
        id:       "REQ-JJ-001"
        pattern:  "ubiquitous"
        text:     "isolate shall use JJ workspaces for isolation rather than git worktrees or full clones."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-JJ-002"
        pattern:  "ubiquitous"
        text:     "isolate shall create workspaces in a sibling directory named '{repo}__workspaces' by default."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-JJ-003"
        pattern:  "event"
        text:     "When creating a workspace, isolate shall execute 'jj workspace add <path>' with the session name."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-JJ-004"
        pattern:  "event"
        text:     "When removing a workspace, isolate shall execute 'jj workspace forget <name>' to clean up."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-JJ-005"
        pattern:  "event"
        text:     "When syncing workspaces, isolate shall detect and report stale workspaces via 'jj workspace list'."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-JJ-006"
        pattern:  "state"
        text:     "While a workspace exists, isolate shall be able to query its JJ status and diff."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-JJ-007"
        pattern:  "event"
        text:     "When creating a workspace and the workspace directory does not exist, isolate shall create it."
        priority: "must"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// ZELLIJ INTEGRATION REQUIREMENTS (REQ-ZELLIJ-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: zellij: [
    {
        id:       "REQ-ZELLIJ-001"
        pattern:  "ubiquitous"
        text:     "isolate shall generate valid KDL layout files for Zellij session configuration."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-002"
        pattern:  "ubiquitous"
        text:     "isolate shall use tabs within the current Zellij session by default, not separate sessions."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-003"
        pattern:  "ubiquitous"
        text:     "isolate shall configure each session tab with a main pane for Claude Code at 70% width."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-004"
        pattern:  "ubiquitous"
        text:     "isolate shall configure each session tab with a side pane split between beads viewer and status."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-005"
        pattern:  "optional"
        text:     "Where floating panes are enabled in config, isolate shall include a floating pane in the layout."
        priority: "may"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-006"
        pattern:  "event"
        text:     "When opening a session tab, isolate shall execute 'zellij action new-tab --layout <path>'."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-007"
        pattern:  "event"
        text:     "When closing a session tab, isolate shall execute 'zellij action close-tab' for the session's tab."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-008"
        pattern:  "event"
        text:     "When focusing a session, isolate shall execute 'zellij action go-to-tab-name <name>'."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-009"
        pattern:  "ubiquitous"
        text:     "isolate shall set the cwd of all panes to the workspace directory."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-010"
        pattern:  "ubiquitous"
        text:     "isolate shall support layout templates with variable substitution for session_name and workspace_path."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-011"
        pattern:  "ubiquitous"
        text:     "isolate shall name Zellij tabs with the session name for easy identification."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-012"
        pattern:  "ubiquitous"
        text:     "isolate shall spawn the main pane with 'claude' command by default, configurable via agent.command in config."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ZELLIJ-013"
        pattern:  "ubiquitous"
        text:     "isolate shall spawn the beads pane with 'bv' command by default, configurable via zellij.panes.beads.command in config."
        priority: "should"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// STATE MANAGEMENT REQUIREMENTS (REQ-STATE-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: state: [
    {
        id:       "REQ-STATE-001"
        pattern:  "ubiquitous"
        text:     "isolate shall persist session state in a SQLite database at .isolate/state.db."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-STATE-002"
        pattern:  "ubiquitous"
        text:     "isolate shall track session name, status, workspace path, branch, and timestamps for each session."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-STATE-003"
        pattern:  "ubiquitous"
        text:     "isolate shall support session states: creating, active, paused, completed, failed."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-STATE-004"
        pattern:  "event"
        text:     "When a session is created, isolate shall record it with status 'creating' then update to 'active'."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-STATE-005"
        pattern:  "event"
        text:     "When a session is removed, isolate shall delete its record from the database."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-STATE-006"
        pattern:  "unwanted"
        text:     "If the database is corrupted or missing, isolate shall recreate it from discovered workspaces."
        priority: "should"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION REQUIREMENTS (REQ-CONFIG-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: config: [
    {
        id:       "REQ-CONFIG-001"
        pattern:  "ubiquitous"
        text:     "isolate shall load configuration from global (~/.config/isolate/config.toml) then project (.isolate/config.toml)."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CONFIG-002"
        pattern:  "ubiquitous"
        text:     "isolate shall allow project config to override global config values."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CONFIG-003"
        pattern:  "ubiquitous"
        text:     "isolate shall support environment variables with ISOLATE_ prefix to override config values."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-CONFIG-004"
        pattern:  "ubiquitous"
        text:     "isolate shall provide sensible defaults for all configuration values."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-CONFIG-005"
        pattern:  "ubiquitous"
        text:     "isolate shall support {repo} placeholder in workspace_dir config for repository name substitution."
        priority: "should"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// HOOKS REQUIREMENTS (REQ-HOOKS-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: hooks: [
    {
        id:       "REQ-HOOKS-001"
        pattern:  "optional"
        text:     "Where post_create hooks are configured, isolate shall execute them sequentially in the workspace after creation."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-HOOKS-002"
        pattern:  "optional"
        text:     "Where pre_remove hooks are configured, isolate shall execute them before removing the workspace."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-HOOKS-003"
        pattern:  "unwanted"
        text:     "If a post_create hook fails, isolate shall set session status to 'failed' and report the error."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-HOOKS-004"
        pattern:  "unwanted"
        text:     "If a pre_remove hook fails, isolate shall abort removal unless --force is specified."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-HOOKS-005"
        pattern:  "ubiquitous"
        text:     "isolate shall execute hooks as shell commands via the user's default shell."
        priority: "must"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// TUI DASHBOARD REQUIREMENTS (REQ-TUI-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: tui: [
    {
        id:       "REQ-TUI-001"
        pattern:  "ubiquitous"
        text:     "isolate dashboard shall display sessions in a kanban-style layout with columns for each status."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-002"
        pattern:  "ubiquitous"
        text:     "isolate dashboard shall support vim-style navigation (h/j/k/l) between sessions and columns."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-003"
        pattern:  "ubiquitous"
        text:     "isolate dashboard shall display JJ change summary for each session."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-004"
        pattern:  "ubiquitous"
        text:     "isolate dashboard shall display beads status counts for each session."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-005"
        pattern:  "ubiquitous"
        text:     "isolate dashboard shall refresh automatically at a configurable interval (default 1s)."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-006"
        pattern:  "event"
        text:     "When the user presses Enter on a session in the dashboard, isolate shall focus that session's Zellij tab."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-007"
        pattern:  "event"
        text:     "When the user presses 'd' on a session in the dashboard, isolate shall prompt for removal confirmation."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-008"
        pattern:  "ubiquitous"
        text:     "isolate dashboard shall adapt layout based on terminal width (responsive columns)."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-009"
        pattern:  "event"
        text:     "When the user presses 'q' in the dashboard, isolate shall exit the dashboard."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-TUI-010"
        pattern:  "event"
        text:     "When the user presses 'a' in the dashboard, isolate shall prompt for a new session name and create it."
        priority: "should"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// FILE WATCHING REQUIREMENTS (REQ-WATCH-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: watch: [
    {
        id:       "REQ-WATCH-001"
        pattern:  "optional"
        text:     "Where beads integration is enabled, isolate shall watch .beads/beads.db for changes."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-WATCH-002"
        pattern:  "ubiquitous"
        text:     "isolate shall debounce file watch events with a 100ms delay to prevent thrashing."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-WATCH-003"
        pattern:  "event"
        text:     "When beads.db changes are detected, isolate shall update beads status in the dashboard."
        priority: "should"
        status:   "draft"
    },
    {
        id:       "REQ-WATCH-004"
        pattern:  "state"
        text:     "While the dashboard is running, isolate shall monitor all session workspaces for beads changes."
        priority: "should"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// UNWANTED BEHAVIOR / ERROR HANDLING (REQ-ERR-*)
// ═══════════════════════════════════════════════════════════════════════════

requirements: error: [
    {
        id:       "REQ-ERR-001"
        pattern:  "unwanted"
        text:     "If JJ is not installed or not in PATH, isolate shall display an error message and exit with code 1."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ERR-002"
        pattern:  "unwanted"
        text:     "If Zellij is not running, isolate shall display an error message suggesting 'zellij' be started first."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ERR-003"
        pattern:  "unwanted"
        text:     "If the current directory is not a JJ repository, isolate shall display an error and exit."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ERR-004"
        pattern:  "unwanted"
        text:     "If a session name already exists, isolate add shall display an error and not overwrite."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ERR-005"
        pattern:  "unwanted"
        text:     "If workspace creation fails, isolate shall clean up any partial state and report the error."
        priority: "must"
        status:   "draft"
    },
    {
        id:       "REQ-ERR-006"
        pattern:  "unwanted"
        text:     "If a session does not exist, isolate remove/status/focus shall display 'session not found' error."
        priority: "must"
        status:   "draft"
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION - Ensure all requirements conform to schema
// ═══════════════════════════════════════════════════════════════════════════

// Flatten all requirements for validation
_allRequirements: [
    for _, reqs in requirements
    for req in reqs { req }
]

// Validate each requirement matches #Requirement schema
_validated: [ for req in _allRequirements { req & #Requirement } ]

// Summary statistics
summary: {
    total:       len(_allRequirements)
    byPriority: {
        must:   len([ for r in _allRequirements if r.priority == "must" { r } ])
        should: len([ for r in _allRequirements if r.priority == "should" { r } ])
        may:    len([ for r in _allRequirements if r.priority == "may" { r } ])
    }
    byPattern: {
        ubiquitous: len([ for r in _allRequirements if r.pattern == "ubiquitous" { r } ])
        event:      len([ for r in _allRequirements if r.pattern == "event" { r } ])
        state:      len([ for r in _allRequirements if r.pattern == "state" { r } ])
        optional:   len([ for r in _allRequirements if r.pattern == "optional" { r } ])
        unwanted:   len([ for r in _allRequirements if r.pattern == "unwanted" { r } ])
    }
    byStatus: {
        draft:       len([ for r in _allRequirements if r.status == "draft" { r } ])
        approved:    len([ for r in _allRequirements if r.status == "approved" { r } ])
        implemented: len([ for r in _allRequirements if r.status == "implemented" { r } ])
        verified:    len([ for r in _allRequirements if r.status == "verified" { r } ])
    }
}
