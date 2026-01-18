//! CLI argument definitions and command builders
//!
//! This module contains all clap command builders for the zjj CLI.
//! Each function returns a configured `clap::Command` for a subcommand.

use clap::{Arg, ArgAction, Command};

pub fn cmd_init() -> Command {
    Command::new("init")
        .about("Initialize zjj in a JJ repository (or create one)")
        .long_about(
            "Initialize ZJJ in a Repository\n\
             \n\
             WHAT IT DOES:\n\
             Sets up ZJJ infrastructure:\n  \
             1. Checks for JJ repository (creates one if needed)\n  \
             2. Creates .zjj/ directory for state and layouts\n  \
             3. Initializes SQLite database (state.db)\n  \
             4. Creates default configuration (.zjj/config.toml)\n  \
             5. Sets up workspace directory structure\n  \
             6. Runs health checks on dependencies (jj, zellij)\n\
             \n\
             WHEN TO USE:\n\
             • First time using zjj in a repository (REQUIRED)\n  \
             • After cloning a repository that uses zjj\n  \
             • When database is corrupted (with --repair)\n  \
             • To reset all session data (with --force)\n\
             \n\
             SAFE TO RE-RUN:\n\
             Running init on an already-initialized repo is safe.\n  \
             It will detect existing setup and skip initialization.\n  \
             Use --repair to fix database issues.\n  \
             Use --force to completely reset (creates backup first).\n\
             \n\
             CREATES:\n  \
             • .zjj/state.db          - Session database\n  \
             • .zjj/config.toml       - Project configuration\n  \
             • .zjj/layouts/          - Zellij layout files\n  \
             • <workspace_dir>/       - Session workspaces (default: ../<repo>__workspaces)\n\
             \n\
             PREREQUISITES:\n  \
             • jj installed (https://github.com/martinvonz/jj)\n  \
             • zellij installed (https://zellij.dev)\n  \
             • Write permissions in current directory\n\
             \n\
             WORKFLOW POSITION:\n\
             This is the FIRST command you run:\n  \
             zjj init → zjj add → [work] → zjj sync → zjj remove\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj doctor       - Check system health\n  \
             • zjj config       - View/modify configuration\n  \
             • zjj backup       - Backup session database\n  \
             • zjj restore      - Restore from backup",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON for machine parsing"),
        )
        .arg(
            Arg::new("repair")
                .long("repair")
                .action(clap::ArgAction::SetTrue)
                .help("Attempt to repair corrupted database (preserves sessions)"),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Force reinitialize - destroys ALL session data (creates backup first)"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # First-time setup\n  \
             zjj init\n\
             \n  \
             # Fix database corruption\n  \
             zjj init --repair\n\
             \n  \
             # Complete reset (creates backup)\n  \
             zjj init --force\n\
             \n  \
             # Silent initialization for scripts\n  \
             zjj init --json\n\
             \n\
             COMMON USE CASES:\n  \
             New project setup:        zjj init\n  \
             After git clone:          cd repo && zjj init\n  \
             Database corrupted:       zjj init --repair\n  \
             Start fresh:              zjj init --force\n\
             \n\
             WHAT IF IT FAILS:\n  \
             • JJ not found: Install from https://github.com/martinvonz/jj\n  \
             • Zellij not found: Install from https://zellij.dev\n  \
             • Permission denied: Check directory write permissions\n  \
             • Already initialized: This is fine, initialization is idempotent\n\
             \n\
             AI AGENTS:\n  \
             Run this ONCE per repository.\n  \
             Check exit code: 0 = success, 1 = user error, 2 = system error.\n  \
             Use --json for structured output.\n  \
             Run 'zjj doctor' after init to verify setup.",
        )
}

#[allow(clippy::too_many_lines)]
pub fn cmd_add() -> Command {
    Command::new("add")
        .about("Create a new session with JJ workspace + Zellij tab")
        .long_about(
            "Create a New Development Session\n\
             \n\
             WHAT IT DOES:\n\
             Creates an isolated development environment by:\n  \
             1. Creating a JJ workspace (isolated working directory)\n  \
             2. Generating a Zellij layout configuration\n  \
             3. Opening a Zellij tab with the layout\n  \
             4. Storing session metadata in SQLite database\n  \
             5. Running post_create hooks (if configured)\n\
             \n\
             SESSION NAME RULES:\n  \
             • Must start with a letter (a-z, A-Z)\n  \
             • Can contain letters, numbers, hyphens, underscores\n  \
             • Max 64 characters\n  \
             • Case-sensitive (feature-A and feature-a are different)\n\
             \n\
             LAYOUT TEMPLATES:\n  \
             • minimal:  Single Claude pane (simplest)\n  \
             • standard: 70% Claude + 30% sidebar (beads + jj log) [DEFAULT]\n  \
             • full:     Standard + floating pane for quick commands\n  \
             • split:    Two Claude instances side-by-side\n  \
             • review:   Diff viewer + beads + Claude (for PR review)\n\
             \n\
             PREREQUISITES:\n  \
             • Must be in a JJ repository (run 'zjj init' first)\n  \
             • Must be inside Zellij session (unless using --no-open)\n  \
             • jj and zellij must be installed\n  \
             • Session name must not already exist\n\
             \n\
             WORKFLOW POSITION:\n\
             This is typically the FIRST command in a development workflow:\n  \
             zjj add → [work] → zjj sync → zjj remove\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj list         - See all sessions\n  \
             • zjj status       - Check session details\n  \
             • zjj remove       - Delete session when done\n  \
             • zjj sync         - Rebase on main branch\n  \
             • zjj focus        - Switch to existing session",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name for the new session (must start with a letter)"),
        )
        .arg(
            Arg::new("no-hooks")
                .long("no-hooks")
                .action(clap::ArgAction::SetTrue)
                .help("Skip executing post_create hooks"),
        )
        .arg(
            Arg::new("template")
                .short('t')
                .long("template")
                .value_name("TEMPLATE")
                .help("Zellij layout template: minimal, standard, full, split, review (default: standard)"),
        )
        .arg(
            Arg::new("no-open")
                .long("no-open")
                .action(clap::ArgAction::SetTrue)
                .help("Create workspace without opening Zellij tab (for background sessions)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON for machine parsing"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview what would happen without executing (plan mode)"),
        )
        .arg(
            Arg::new("bead")
                .short('b')
                .long("bead")
                .value_name("BEAD_ID")
                .help("Create session from bead ID (auto-pulls spec and updates status)"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Standard workflow - create and open session\n  \
             zjj add feature-auth\n\
             \n  \
             # Background session (no tab opened)\n  \
             zjj add bugfix-123 --no-open\n\
             \n  \
             # Minimal layout (single Claude pane)\n  \
             zjj add experiment -t minimal\n\
             \n  \
             # Preview mode - see what will be created\n  \
             zjj add feature-test --dry-run\n\
             \n  \
             # Skip hooks for quick testing\n  \
             zjj add quick-test --no-hooks\n\
             \n  \
             # For scripting/automation\n  \
             zjj add api-work --json --no-open\n\
             \n\
             COMMON USE CASES:\n  \
             Start new feature:    zjj add feature-name\n  \
             Quick experiment:     zjj add test -t minimal --no-hooks\n  \
             PR review:            zjj add review-pr-123 -t review\n  \
             Parallel work:        zjj add hotfix --no-open\n\
             \n\
             AI AGENTS:\n  \
             Use --dry-run first to preview operations.\n  \
             Use --json for structured output.\n  \
             Check 'zjj context --json' for environment state before adding.",
        )
}

pub fn cmd_add_batch() -> Command {
    Command::new("add-batch")
        .about("Create multiple sessions from bead IDs (stdin)")
        .long_about(
            "Batch Session Creation from Beads\n\
             \n\
             WHAT IT DOES:\n\
             Creates multiple sessions at once by reading bead IDs from stdin.\n  \
             Validates all beads upfront, then creates sessions sequentially.\n\
             \n\
             USAGE:\n  \
             bd ready | head -5 | zjj add-batch --beads-stdin\n  \
             bd list --status=open | zjj add-batch --beads-stdin --json\n\
             \n\
             WORKFLOW:\n  \
             1. Read bead IDs from stdin (one per line)\n  \
             2. Validate ALL beads exist (fail fast if any invalid)\n  \
             3. Create sessions sequentially (avoid conflicts)\n  \
             4. Report results (text or JSON)\n\
             \n\
             STDIN FORMAT:\n  \
             zjj-1234\n  \
             zjj-5678\n  \
             zjj-9012\n\
             \n\
             Each line should contain one bead ID.\n\
             \n\
             FLAGS:\n  \
             --beads-stdin : Read bead IDs from stdin (REQUIRED)\n  \
             --no-open     : Don't open Zellij tabs (recommended for batch)\n  \
             --no-hooks    : Skip post_create hooks\n  \
             --json        : Output results as JSON\n\
             \n\
             JSON OUTPUT:\n  \
             Returns BatchOperationOutput with per-item results:\n  \
             { \n    \
               \"success\": true,\n    \
               \"total_count\": 3,\n    \
               \"success_count\": 3,\n    \
               \"failure_count\": 0,\n    \
               \"results\": [\n      \
                 { \"success\": true, \"item_id\": \"zjj-1234\", \"index\": 0, ... },\n      \
                 ...\n    \
               ]\n  \
             }\n\
             \n\
             PREREQUISITES:\n  \
             • Must be in a JJ repository (run 'zjj init' first)\n  \
             • Beads must exist in .beads/beads.db\n  \
             • Session names (= bead IDs) must not already exist\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj add       - Create single session\n  \
             • bd ready      - List beads ready to work\n  \
             • bd list       - List all beads\n\
             \n\
             EXIT CODES:\n  \
             0 - All sessions created successfully\n  \
             2 - One or more sessions failed to create",
        )
        .arg(
            Arg::new("beads-stdin")
                .long("beads-stdin")
                .action(ArgAction::SetTrue)
                .help("Read bead IDs from stdin (one per line)"),
        )
        .arg(
            Arg::new("no-open")
                .long("no-open")
                .action(ArgAction::SetTrue)
                .help("Don't open Zellij tabs (recommended for batch operations)"),
        )
        .arg(
            Arg::new("no-hooks")
                .long("no-hooks")
                .action(ArgAction::SetTrue)
                .help("Skip post_create hooks"),
        )
        .arg(
            Arg::new("template")
                .short('t')
                .long("template")
                .value_name("TEMPLATE")
                .help(
                    "Layout template to use for all sessions (minimal|standard|full|split|review)",
                ),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output results as JSON"),
        )
}

pub fn cmd_list() -> Command {
    Command::new("list")
        .about("List all sessions")
        .long_about(
            "List All Development Sessions\n\
             \n\
             WHAT IT SHOWS:\n\
             • Session name and status (creating, active, completed, failed)\n  \
             • Workspace path\n  \
             • Zellij tab name\n  \
             • Creation and update timestamps\n  \
             • Bead information (if attached)\n  \
             • Agent information (if active)\n\
             \n\
             DEFAULT BEHAVIOR:\n\
             Shows only active and creating sessions (filters out completed/failed).\n  \
             Use --all to see everything including historical sessions.\n\
             \n\
             OUTPUT MODES:\n  \
             • Human-readable table (default, when stdout is a TTY)\n  \
             • Silent mode (minimal, when piped or with --silent)\n  \
             • JSON mode (structured data with --json)\n\
             \n\
             FILTERING:\n\
             Filter sessions by bead or agent metadata:\n  \
             • --filter-by-bead <ID>   - Show only sessions with specific bead\n  \
             • --filter-by-agent <ID>  - Show only sessions with specific agent\n  \
             • --with-beads            - Show only sessions that have beads\n  \
             • --with-agents           - Show only sessions with active agents\n\
             \n\
             STATUS VALUES:\n  \
             • creating:  Session being set up (transient state)\n  \
             • active:    Ready to use (normal state)\n  \
             • completed: Successfully finished and removed\n  \
             • failed:    Error occurred during operation\n\
             \n\
             WORKFLOW POSITION:\n\
             Use this to:\n  \
             • See what sessions exist before adding a new one\n  \
             • Find session names for sync/remove/focus commands\n  \
             • Monitor active development work\n  \
             • Check session status after operations\n  \
             • Filter sessions by bead or agent\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj add         - Create new session\n  \
             • zjj status      - Detailed info about one session\n  \
             • zjj focus       - Switch to a session\n  \
             • zjj dashboard   - Interactive view of all sessions",
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include completed and failed sessions (historical data)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON array of session objects"),
        )
        .arg(
            Arg::new("silent")
                .long("silent")
                .action(clap::ArgAction::SetTrue)
                .help("Minimal output for pipes (auto-detected when stdout is not a TTY)"),
        )
        .arg(
            Arg::new("filter-by-bead")
                .long("filter-by-bead")
                .value_name("BEAD_ID")
                .help("Show only sessions with specific bead ID"),
        )
        .arg(
            Arg::new("filter-by-agent")
                .long("filter-by-agent")
                .value_name("AGENT_ID")
                .help("Show only sessions with specific agent ID"),
        )
        .arg(
            Arg::new("with-beads")
                .long("with-beads")
                .action(clap::ArgAction::SetTrue)
                .help("Show only sessions that have beads attached"),
        )
        .arg(
            Arg::new("with-agents")
                .long("with-agents")
                .action(clap::ArgAction::SetTrue)
                .help("Show only sessions with active agents"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # List active sessions\n  \
             zjj list\n\
             \n  \
             # List everything (including completed/failed)\n  \
             zjj list --all\n\
             \n  \
             # Filter by specific bead\n  \
             zjj list --filter-by-bead zjj-1234\n\
             \n  \
             # Show only sessions with beads\n  \
             zjj list --with-beads\n\
             \n  \
             # Show only sessions with agents\n  \
             zjj list --with-agents\n\
             \n  \
             # JSON for scripting/parsing\n  \
             zjj list --json | jq '.[] | select(.status == \"active\")'\n\
             \n  \
             # Silent mode for scripts\n  \
             zjj list --silent | wc -l\n\
             \n\
             COMMON USE CASES:\n  \
             Check what's running:     zjj list\n  \
             Get session names:        zjj list --json | jq '.[].name'\n  \
             Count active sessions:    zjj list --json | jq 'length'\n  \
             Find bead sessions:       zjj list --with-beads\n  \
             Find old sessions:        zjj list --all\n\
             \n\
             AI AGENTS:\n  \
             Always use --json for programmatic access.\n  \
             Parse the JSON array to find sessions by name or status.\n  \
             Use filters to find sessions by bead or agent.\n  \
             Use this before 'add' to check if a session name exists.",
        )
}

#[allow(clippy::too_many_lines)]
pub fn cmd_remove() -> Command {
    Command::new("remove")
        .about("Remove a session and its workspace")
        .long_about(
            "Remove a Development Session\n\
             \n\
             WHAT IT DOES:\n\
             Cleanly removes a session by:\n  \
             1. Optionally merging changes to main (with --merge)\n  \
             2. Closing the Zellij tab\n  \
             3. Removing the JJ workspace\n  \
             4. Deleting session from database\n  \
             5. Running pre_remove hooks (if configured)\n\
             \n\
             SAFE CLEANUP:\n\
             • Default: Prompts for confirmation before removal\n  \
             • Preserves uncommitted changes in workspace\n  \
             • Can merge work to main before cleanup (--merge)\n  \
             • Can preserve branch after workspace removal (--keep-branch)\n\
             \n\
             WORKFLOW POSITION:\n\
             This is typically the LAST command in a development workflow:\n  \
             zjj add → [work] → zjj sync → zjj remove\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj list      - See all sessions before deciding what to remove\n  \
             • zjj status    - Check session state before removal\n  \
             • zjj sync      - Sync with main before removing\n  \
             • zjj backup    - Backup database before bulk removals",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to remove"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt and hooks"),
        )
        .arg(
            Arg::new("merge")
                .short('m')
                .long("merge")
                .action(clap::ArgAction::SetTrue)
                .help("Squash-merge to main before removal"),
        )
        .arg(
            Arg::new("keep-branch")
                .short('k')
                .long("keep-branch")
                .action(clap::ArgAction::SetTrue)
                .help("Preserve branch after removal"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Show what would be done without executing"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Standard removal (with confirmation)\n  \
             zjj remove feature-auth\n\
             \n  \
             # Preview what will be removed\n  \
             zjj remove feature-test --dry-run\n\
             \n  \
             # Merge to main and remove\n  \
             zjj remove feature-done --merge\n\
             \n  \
             # Quick removal without confirmation\n  \
             zjj remove temp-work --force\n\
             \n  \
             # Remove workspace but keep branch\n  \
             zjj remove experiment --keep-branch\n\
             \n  \
             # For scripting/automation\n  \
             zjj remove old-session --force --json\n\
             \n\
             COMMON USE CASES:\n  \
             Finished feature:         zjj remove feature-name --merge\n  \
             Abandoned experiment:     zjj remove test --force\n  \
             Keep branch for later:    zjj remove work --keep-branch\n  \
             Preview first:            zjj remove session --dry-run\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Step 1: Verify session exists\n  \
             zjj list --json | jq '.[] | select(.name == \"feature-x\")'\n\
             \n  \
             # Step 2: Preview removal to see what will be affected\n  \
             zjj remove feature-x --dry-run --json\n\
             \n  \
             # Step 3: Merge work and remove (automated workflow)\n  \
             zjj remove feature-x --merge --force --json\n\
             \n  \
             # Alternative: Verify work is synced before removing\n  \
             zjj status feature-x --json  # Check for uncommitted changes\n  \
             zjj sync feature-x           # Sync if needed\n  \
             zjj remove feature-x --merge --force --json\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Always verify session exists with 'zjj list --json' first\n  \
             • Use --dry-run to preview destructive operations\n  \
             • Check 'zjj status <name> --json' for uncommitted changes\n  \
             • Use --merge when work should be integrated to main\n  \
             • Use --force in automated workflows to skip prompts\n  \
             • Parse JSON output to verify removal success (exit code 0)",
        )
}

pub fn cmd_focus() -> Command {
    Command::new("focus")
        .about("Switch to a session's Zellij tab")
        .long_about(
            "Switch to a Session's Zellij Tab\n\
             \n\
             WHAT IT DOES:\n\
             Switches the active Zellij tab to the specified session.\n  \
             This is useful for quickly moving between different work sessions.\n\
             \n\
             PREREQUISITES:\n\
             • Must be inside Zellij session\n  \
             • Target session must exist and be active\n  \
             • Session must have an associated Zellij tab\n\
             \n\
             WORKFLOW POSITION:\n\
             Use this to switch between parallel work sessions:\n  \
             zjj add feature-a → [work] → zjj focus feature-b → [work] → zjj focus feature-a\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj list      - See all available sessions to focus\n  \
             • zjj add       - Create new session to switch to\n  \
             • zjj status    - Check session details before focusing",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to focus"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Switch to a session\n  \
             zjj focus feature-auth\n\
             \n  \
             # JSON output for scripting\n  \
             zjj focus api-work --json\n\
             \n\
             COMMON USE CASES:\n  \
             Switch contexts:          zjj focus other-feature\n  \
             Return to main work:      zjj focus main-task\n  \
             Jump to review:           zjj focus review-pr-123\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Step 1: List available sessions to focus\n  \
             zjj list --json | jq '.[] | select(.status == \"active\") | .name'\n\
             \n  \
             # Step 2: Focus on specific session\n  \
             zjj focus feature-x --json\n\
             \n  \
             # Full workflow: Find and switch to a session\n  \
             SESSION=$(zjj list --json | jq -r '.[] | select(.name | contains(\"auth\")) | .name' | head -1)\n  \
             zjj focus \"$SESSION\" --json\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Verify session exists with 'zjj list --json' before focusing\n  \
             • Check session status is \"active\" (not \"creating\" or \"failed\")\n  \
             • Use --json for programmatic output parsing\n  \
             • Check exit code: 0 = success, 3 = session not found\n  \
             • Only focus if inside Zellij (check ZELLIJ environment variable)",
        )
}

pub fn cmd_status() -> Command {
    Command::new("status")
        .about("Show detailed session status")
        .long_about(
            "Show Detailed Session Status\n\
             \n\
             WHAT IT SHOWS:\n\
             • Session state (creating, active, completed, failed)\n  \
             • Workspace path and JJ status\n  \
             • Zellij tab information\n  \
             • Creation and last update timestamps\n  \
             • Uncommitted changes count\n  \
             • Sync status with main branch\n\
             \n\
             OUTPUT MODES:\n\
             • Single session: Detailed view of one session\n  \
             • All sessions: Summary view of all sessions (if name omitted)\n  \
             • Watch mode: Live updates every 1 second (--watch)\n  \
             • JSON mode: Machine-readable output (--json)\n\
             \n\
             WORKFLOW POSITION:\n\
             Use this to:\n  \
             • Check session health before working\n  \
             • Verify workspace state before syncing\n  \
             • Monitor session during long operations\n  \
             • Debug issues with session setup\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj list      - Quick overview of all sessions\n  \
             • zjj diff      - See actual code changes\n  \
             • zjj sync      - Update session after checking status\n  \
             • zjj doctor    - Check system health if status shows issues",
        )
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Show status of specific session\n  \
             zjj status feature-auth\n\
             \n  \
             # Show status of all sessions\n  \
             zjj status\n\
             \n  \
             # Watch mode for live updates\n  \
             zjj status feature-x --watch\n\
             \n  \
             # JSON output for scripting\n  \
             zjj status api-work --json\n\
             \n\
             COMMON USE CASES:\n  \
             Check before sync:        zjj status feature-name\n  \
             Monitor all work:         zjj status --json | jq '.'\n  \
             Watch long operation:     zjj status build --watch\n  \
             Verify session health:    zjj status session-name --json\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Check if session has uncommitted changes\n  \
             zjj status feature-x --json | jq '.uncommitted_changes'\n\
             \n  \
             # Verify session is ready for work\n  \
             zjj status feature-x --json | jq 'select(.status == \"active\")'\n\
             \n  \
             # Check sync status before merging\n  \
             zjj status feature-x --json | jq '.sync_status'\n\
             \n  \
             # Get workspace path for direct operations\n  \
             WORKSPACE=$(zjj status feature-x --json | jq -r '.workspace_path')\n  \
             cd \"$WORKSPACE\" && jj log\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Use --json to parse session state programmatically\n  \
             • Check 'status' field for session health (active, failed, etc.)\n  \
             • Verify 'uncommitted_changes' before destructive operations\n  \
             • Check 'sync_status' to determine if rebase needed\n  \
             • Use status before sync/remove to validate state\n  \
             • Parse 'workspace_path' for direct JJ operations",
        )
}

#[allow(clippy::too_many_lines)]
pub fn cmd_sync() -> Command {
    Command::new("sync")
        .about("Sync a session's workspace with main (rebase)")
        .long_about(
            "Sync Session with Main Branch\n\
             \n\
             WHAT IT DOES:\n\
             Rebases the session's workspace onto the latest main branch:\n  \
             1. Fetches latest main branch changes\n  \
             2. Rebases session commits on top of main\n  \
             3. Resolves conflicts if necessary\n  \
             4. Updates session metadata\n\
             \n\
             WHEN TO USE:\n\
             • Before merging work back to main\n  \
             • When main branch has new commits\n  \
             • To ensure session is up-to-date\n  \
             • Before creating pull requests\n\
             \n\
             CONFLICT HANDLING:\n\
             If conflicts occur during rebase:\n  \
             • JJ will mark conflicted files\n  \
             • Resolve conflicts manually in workspace\n  \
             • Run 'jj resolve' to mark as resolved\n  \
             • Sync will complete after conflicts resolved\n\
             \n\
             WORKFLOW POSITION:\n\
             Regular sync during development:\n  \
             zjj add → [work] → zjj sync → [more work] → zjj sync → zjj remove --merge\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj status    - Check if sync is needed\n  \
             • zjj diff      - Preview changes before sync\n  \
             • zjj remove    - Remove after final sync",
        )
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to sync (syncs current workspace if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Show what would be done without executing"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Sync specific session\n  \
             zjj sync feature-auth\n\
             \n  \
             # Sync current workspace (auto-detect)\n  \
             zjj sync\n\
             \n  \
             # Preview sync operation\n  \
             zjj sync feature-x --dry-run\n\
             \n  \
             # JSON output for scripting\n  \
             zjj sync api-work --json\n\
             \n\
             COMMON USE CASES:\n  \
             Daily sync:               zjj sync\n  \
             Before PR:                zjj sync feature-name\n  \
             Check conflicts:          zjj sync --dry-run\n  \
             Automated workflow:       zjj sync session --json\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Step 1: Check if sync is needed\n  \
             zjj status feature-x --json | jq '.sync_status'\n\
             \n  \
             # Step 2: Preview sync to check for potential conflicts\n  \
             zjj sync feature-x --dry-run --json\n\
             \n  \
             # Step 3: Perform actual sync\n  \
             zjj sync feature-x --json\n\
             \n  \
             # Full workflow: Conditional sync based on status\n  \
             NEEDS_SYNC=$(zjj status feature-x --json | jq -r '.sync_status.needs_sync')\n  \
             if [ \"$NEEDS_SYNC\" = \"true\" ]; then\n    \
               zjj sync feature-x --json\n  \
             fi\n\
             \n  \
             # Handle potential conflicts\n  \
             zjj sync feature-x --json > sync_result.json\n  \
             if jq -e '.conflicts' sync_result.json > /dev/null; then\n    \
               echo \"Conflicts detected - manual resolution required\"\n    \
               jq '.conflicts[]' sync_result.json\n  \
             fi\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Check 'zjj status <name> --json' for sync_status before syncing\n  \
             • Use --dry-run first to detect potential conflicts\n  \
             • Parse JSON output for conflict information\n  \
             • Sync regularly to keep sessions up-to-date\n  \
             • Handle exit codes: 0 = success, 2 = conflicts need resolution\n  \
             • Report conflicts to user for manual resolution",
        )
}

pub fn cmd_diff() -> Command {
    Command::new("diff")
        .about("Show diff between session and main branch")
        .long_about(
            "Show Diff Between Session and Main\n\
             \n\
             WHAT IT SHOWS:\n\
             • Full unified diff of changes (default)\n  \
             • Summary statistics with --stat (files changed, insertions, deletions)\n  \
             • JSON-formatted diff data with --json\n\
             \n\
             OUTPUT MODES:\n\
             • Full diff: Complete unified diff output (default)\n  \
             • Stat mode: Concise summary of changes (--stat)\n  \
             • JSON mode: Machine-readable diff metadata (--json)\n\
             \n\
             WORKFLOW POSITION:\n\
             Use this to:\n  \
             • Review changes before syncing\n  \
             • Check work before creating PR\n  \
             • Verify what will be merged\n  \
             • Generate diff reports\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj status    - Check session metadata\n  \
             • zjj sync      - Sync before checking final diff\n  \
             • zjj remove    - Remove after reviewing diff",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Session name to show diff for"),
        )
        .arg(
            Arg::new("stat")
                .long("stat")
                .action(clap::ArgAction::SetTrue)
                .help("Show diffstat only (summary of changes)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Show full diff\n  \
             zjj diff feature-auth\n\
             \n  \
             # Show summary only\n  \
             zjj diff feature-x --stat\n\
             \n  \
             # JSON output for parsing\n  \
             zjj diff api-work --json\n\
             \n  \
             # Pipe to pager for large diffs\n  \
             zjj diff feature-name | less\n\
             \n\
             COMMON USE CASES:\n  \
             Review before PR:         zjj diff feature-name | less\n  \
             Quick summary:            zjj diff feature --stat\n  \
             Count changes:            zjj diff work --json | jq '.stats'\n  \
             Compare to main:          zjj diff session-name\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Get diff statistics\n  \
             zjj diff feature-x --stat --json | jq '{files: .files_changed, insertions, deletions}'\n\
             \n  \
             # Check if session has changes\n  \
             CHANGES=$(zjj diff feature-x --stat --json | jq '.files_changed')\n  \
             if [ \"$CHANGES\" -gt 0 ]; then\n    \
               echo \"Session has $CHANGES file(s) changed\"\n  \
             fi\n\
             \n  \
             # Extract changed file list\n  \
             zjj diff feature-x --json | jq -r '.files[].path'\n\
             \n  \
             # Verify changes before automated merge\n  \
             zjj diff feature-x --stat --json > diff_summary.json\n  \
             INSERTIONS=$(jq '.insertions' diff_summary.json)\n  \
             DELETIONS=$(jq '.deletions' diff_summary.json)\n  \
             echo \"Changes: +$INSERTIONS -$DELETIONS\"\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Use --stat --json for quick change summaries\n  \
             • Parse 'files_changed', 'insertions', 'deletions' from JSON\n  \
             • Check diff before automated sync/merge operations\n  \
             • Use diff to validate expected changes\n  \
             • Extract file lists for targeted operations\n  \
             • Report significant changes to user for review",
        )
}

#[allow(clippy::too_many_lines)]
pub fn cmd_config() -> Command {
    Command::new("config")
        .alias("cfg")
        .about("View or modify configuration")
        .long_about(
            "Configuration Management\n\
             \n\
             USAGE PATTERNS:\n\
             • zjj config                        - View all configuration\n\
             • zjj config KEY                    - Get specific value\n\
             • zjj config KEY VALUE              - Set configuration value\n\
             • zjj config --validate             - Validate configuration\n\
             \n\
             CONFIGURATION SCOPES:\n\
             • Project: .zjj/config.toml (default)\n\
             • Global: ~/.config/zjj/config.toml (with --global)\n\
             \n\
             COMMON SETTINGS:\n\
             • workspace_dir: Where JJ workspaces are created\n\
             • default_template: Default Zellij layout (minimal/standard/full/split/review)\n\
             • hooks.post_create: Commands to run after session creation\n\
             • hooks.pre_remove: Commands to run before session removal\n\
             • zellij.use_tabs: Whether to use tabs or panes\n\
             \n\
             KEY FORMAT:\n\
             Use dot notation to access nested values:\n\
             • 'workspace_dir' - top-level key\n\
             • 'hooks.post_create' - nested key\n\
             • 'zellij.use_tabs' - nested boolean",
        )
        .arg(
            Arg::new("key")
                .required(false)
                .index(1)
                .help("Configuration key (dot notation: 'workspace.dir')"),
        )
        .arg(
            Arg::new("value")
                .required(false)
                .index(2)
                .help("Value to set (only when KEY is provided)"),
        )
        .arg(
            Arg::new("global")
                .long("global")
                .short('g')
                .action(ArgAction::SetTrue)
                .help("Use global config instead of project config"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .action(ArgAction::SetTrue)
                .help("Validate configuration integrity"),
        )
        .after_help(
            "EXAMPLES:\n\
             \n\
             # View all settings\n\
             zjj config\n\
             zjj config --json\n\
             \n\
             # Get specific setting\n\
             zjj config workspace_dir\n\
             zjj config sync_strategy --json\n\
             \n\
             # Set a value\n\
             zjj config workspace_dir /custom/path\n\
             zjj config default_template standard --global\n\
             \n\
             # Validate configuration\n\
             zjj config --validate\n\
             zjj config --validate --json\n\
             \n\
             COMMON USE CASES:\n\
             View all settings:        zjj config --json\n\
             Change workspace dir:     zjj config workspace_dir ~/workspaces\n\
             Set default template:     zjj config default_template minimal\n\
             Add post-create hook:     zjj config hooks.post_create 'make setup'\n\
             \n\
             AI AGENT USAGE:\n\
             # Get all configuration as JSON\n\
             zjj config --json\n\
             \n\
             # Extract specific setting\n\
             zjj config workspace_dir --json\n\
             \n\
             # Validate before modifying\n\
             zjj config --validate --json\n\
             \n\
             # Set configuration programmatically\n\
             zjj config workspace_dir /custom/path --json\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n\
             AI agents should:\n\
             • Use --json to parse configuration programmatically\n\
             • Validate config before making changes (--validate flag)\n\
             • Check workspace_dir to understand session locations\n\
             • Respect user's default_template for session creation\n\
             • Be aware of hooks that may affect operations\n\
             • Use --global for user-wide defaults vs project-specific settings\n\
             \n\
             RELATED COMMANDS:\n\
             • zjj init      - Initialize with default config\n\
             • zjj doctor    - Validate system configuration",
        )
}

pub fn cmd_dashboard() -> Command {
    Command::new("dashboard")
        .about("Launch interactive TUI dashboard with kanban view")
        .alias("dash")
        .long_about(
            "Launch Interactive TUI Dashboard\n\
             \n\
             WHAT IT DOES:\n\
             • Displays all sessions in kanban board layout\n\
             • Shows session status, current branches, and workspace health\n\
             • Provides interactive navigation and actions\n\
             • Real-time updates of session states\n\
             \n\
             PREREQUISITES:\n\
             • Must be inside Zellij session\n\
             • zjj must be initialized (zjj init)\n\
             • Terminal must support TUI rendering\n\
             \n\
             PURPOSE:\n\
             Visual overview of all development sessions with:\n\
             • Quick status assessment across sessions\n\
             • Interactive session management\n\
             • Workspace health monitoring\n\
             • Branch and sync status visualization\n\
             \n\
             USE CASES:\n\
             • Daily standup preparation - see all active work\n\
             • Context switching - find session to focus on\n\
             • Session health monitoring\n\
             • Project status overview\n\
             \n\
             RELATED COMMANDS:\n\
             • zjj list       - Text-based session list\n\
             • zjj status     - Detailed single session status\n\
             • zjj context    - Full environment context for AI",
        )
        .after_help(
            "EXAMPLES:\n\
             \n\
             # Launch dashboard (most common usage)\n\
             zjj dashboard\n\
             zjj dash  # Using alias\n\
             \n\
             COMMON USE CASES:\n\
             \n\
             1. Morning Standup:\n\
                zjj dashboard\n\
                # Visual overview of all active sessions and their states\n\
             \n\
             2. Find Session to Resume:\n\
                zjj dashboard\n\
                # Navigate with arrow keys, press 'f' to focus on session\n\
             \n\
             3. Monitor Multiple Feature Branches:\n\
                zjj dashboard\n\
                # See sync status and commit counts across sessions\n\
             \n\
             AI AGENT EXAMPLES:\n\
             \n\
             # AI agents should use programmatic alternatives:\n\
             zjj list --json          # Get session data programmatically\n\
             zjj context --json       # Get full context as JSON\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n\
             \n\
             Dashboard is for human visual interface. AI agents should:\n\
             • Use 'zjj list --json' for session enumeration\n\
             • Use 'zjj context --json' for comprehensive state\n\
             • Use 'zjj status <session> --json' for specific session details\n\
             \n\
             Dashboard provides visual kanban board that humans use for:\n\
             • Quick visual scanning of session health\n\
             • Interactive keyboard-driven session management\n\
             • Context switching with visual confirmation",
        )
}

pub fn cmd_context() -> Command {
    Command::new("context")
        .about("Show full environment context for AI agents")
        .alias("ctx")
        .long_about(
            "Show Full Environment Context\n\
             \n\
             WHAT IT SHOWS:\n\
             • Current session information (if in a session workspace)\n  \
             • All active sessions\n  \
             • JJ repository status\n  \
             • Zellij session status\n  \
             • Configuration summary\n  \
             • System health indicators\n  \
             • Available commands and capabilities\n\
             \n\
             PURPOSE:\n\
             This command provides AI agents with complete environmental awareness:\n  \
             • What sessions exist and their states\n  \
             • What operations are currently valid\n  \
             • System capabilities and limitations\n  \
             • Current working context\n\
             \n\
             USE CASES:\n\
             • AI agents discovering available operations\n  \
             • Determining current session context\n  \
             • Validating prerequisites before commands\n  \
             • Generating status reports\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj introspect    - Discover command capabilities\n  \
             • zjj doctor        - Check system health\n  \
             • zjj query         - Query specific state information",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Show context in human-readable format\n  \
             zjj context\n\
             \n  \
             # JSON output for AI agents\n  \
             zjj context --json\n\
             \n\
             COMMON USE CASES:\n  \
             Understand environment:   zjj context --json\n  \
             Check current session:    zjj context --json | jq '.current_session'\n  \
             List active sessions:     zjj context --json | jq '.sessions[]'\n  \
             Verify capabilities:      zjj context --json | jq '.capabilities'\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Get complete environment state\n  \
             zjj context --json > environment.json\n\
             \n  \
             # Check if inside a session workspace\n  \
             CURRENT_SESSION=$(zjj context --json | jq -r '.current_session.name // \"none\"')\n  \
             if [ \"$CURRENT_SESSION\" != \"none\" ]; then\n    \
               echo \"Working in session: $CURRENT_SESSION\"\n  \
             fi\n\
             \n  \
             # Determine what operations are valid\n  \
             zjj context --json | jq '.capabilities.can_create_session'\n  \
             zjj context --json | jq '.capabilities.can_focus_session'\n\
             \n  \
             # Get session count and health\n  \
             zjj context --json | jq '{session_count: (.sessions | length), healthy: .system_health.healthy}'\n\
             \n  \
             # Check prerequisites before operation\n  \
             INSIDE_ZELLIJ=$(zjj context --json | jq '.environment.inside_zellij')\n  \
             if [ \"$INSIDE_ZELLIJ\" = \"false\" ]; then\n    \
               echo \"Not inside Zellij - cannot create sessions with UI\"\n  \
             fi\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Run 'zjj context --json' at the start of workflows for full state\n  \
             • Check 'current_session' to understand working context\n  \
             • Verify 'capabilities' before attempting operations\n  \
             • Monitor 'system_health' for issues\n  \
             • Use 'environment' to check Zellij/JJ availability\n  \
             • Parse 'sessions' array for active work items\n  \
             • This is the primary discovery command for AI agents",
        )
}

pub fn cmd_prime() -> Command {
    Command::new("prime")
        .about("AI context recovery - essential workflow information")
        .long_about(
            "AI Context Recovery Command\n\
             \n\
             WHAT IT PROVIDES:\n\
             • Curated context for AI agents after context loss\n  \
             • JJ repository status and current branch\n  \
             • Active ZJJ sessions and their states\n  \
             • Essential command reference by category\n  \
             • Common workflows and patterns\n  \
             • Beads integration status\n\
             \n\
             PURPOSE:\n\
             Similar to 'bd prime' for beads, this provides recovery context\n  \
             when AI agents lose context due to:\n  \
             • Session compaction\n  \
             • New conversation starts\n  \
             • Context window resets\n  \
             • Switching between workspaces\n\
             \n\
             COMPARISON WITH RELATED COMMANDS:\n\
             • 'prime' - Curated essentials (this command)\n  \
             • 'context' - Complete environment state\n  \
             • 'introspect' - CLI metadata and command docs\n\
             \n\
             HOOK INTEGRATION:\n\
             This command is automatically called by startup hooks to inject\n  \
             workflow context into AI agent sessions. See 'zjj hooks install'.",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON for programmatic use"),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .action(clap::ArgAction::SetTrue)
                .help("Suppress output (for use in hooks)"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Human-readable markdown context\n  \
             zjj prime\n\
             \n  \
             # JSON output for parsing\n  \
             zjj prime --json\n\
             \n  \
             # Silent mode for hooks\n  \
             zjj prime --quiet\n\
             \n\
             TYPICAL WORKFLOW:\n  \
             After context loss, run 'zjj prime' to recover:\n  \
             • What repository you're in\n  \
             • What sessions exist\n  \
             • What commands are available\n  \
             • Common workflow patterns\n\
             \n\
             AI AGENTS:\n  \
             Run this at session start or after compaction.\n  \
             Use --json for structured parsing.\n  \
             This is lighter than 'context --json' - only essentials.",
        )
}

pub fn cmd_introspect() -> Command {
    Command::new("introspect")
        .about("Discover zjj capabilities and command details")
        .long_about(
            "Discover Command Capabilities\n\
             \n\
             WHAT IT SHOWS:\n\
             • All available commands and their purposes\n  \
             • Command arguments and options\n  \
             • Expected input/output formats\n  \
             • Usage examples and patterns\n  \
             • Command relationships and workflows\n\
             \n\
             OUTPUT MODES:\n\
             • All commands: Complete command catalog (default)\n  \
             • Single command: Detailed info about one command (with argument)\n  \
             • JSON mode: Machine-readable schema (--json)\n\
             \n\
             PURPOSE:\n\
             Enables AI agents to discover:\n  \
             • What commands are available\n  \
             • How to invoke each command\n  \
             • What flags/options each command supports\n  \
             • Expected JSON schemas for input/output\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj context       - Get current environment state\n  \
             • zjj query         - Query specific state info\n  \
             • zjj --help-json   - Get complete CLI schema",
        )
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to introspect (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # List all commands\n  \
             zjj introspect\n\
             \n  \
             # Get details about specific command\n  \
             zjj introspect add\n\
             \n  \
             # JSON schema for all commands\n  \
             zjj introspect --json\n\
             \n  \
             # JSON schema for specific command\n  \
             zjj introspect sync --json\n\
             \n\
             COMMON USE CASES:\n  \
             Discover commands:        zjj introspect --json\n  \
             Learn command args:       zjj introspect add --json\n  \
             Generate docs:            zjj introspect --json > api.json\n  \
             Validate schemas:         zjj introspect <cmd> --json | jq '.'\n\
             \n\
             AI AGENT EXAMPLES:\n  \
             # Get list of all available commands\n  \
             zjj introspect --json | jq '.commands[].name'\n\
             \n  \
             # Find commands that support --json flag\n  \
             zjj introspect --json | jq '.commands[] | select(.flags[] | contains(\"--json\")) | .name'\n\
             \n  \
             # Get command signature for validation\n  \
             zjj introspect add --json | jq '{name, args: .arguments, flags: .flags}'\n\
             \n  \
             # Discover session management commands\n  \
             zjj introspect --json | jq '.commands[] | select(.category == \"Session Lifecycle\")'\n\
             \n  \
             # Build dynamic help system\n  \
             for cmd in $(zjj introspect --json | jq -r '.commands[].name'); do\n    \
               echo \"Command: $cmd\"\n    \
               zjj introspect \"$cmd\" --json | jq -r '.description'\n  \
             done\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n  \
             AI agents should:\n  \
             • Run 'zjj introspect --json' to discover available commands\n  \
             • Use command schemas to validate inputs before execution\n  \
             • Check 'flags' array to see what options are supported\n  \
             • Parse 'arguments' to understand required vs optional params\n  \
             • Use 'category' to group related commands\n  \
             • This is the primary command discovery mechanism",
        )
}

pub fn cmd_doctor() -> Command {
    Command::new("doctor")
        .about("Run system health checks")
        .alias("check")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible"),
        )
        .after_help(
            "Examples:\n  \
             # Check system health\n  \
             zjj doctor\n\n  \
             # Auto-fix issues\n  \
             zjj doctor --fix\n\n  \
             # JSON output for monitoring\n  \
             zjj doctor --json",
        )
}

pub fn cmd_query() -> Command {
    Command::new("query")
        .about("Query system state programmatically")
        .long_about(
            "Query ZJJ System State Programmatically\n\
             \n\
             WHAT IT DOES:\n\
             Executes system state queries for script and AI agent integration:\n  \
             • Check if session exists\n  \
             • Get total session count\n  \
             • Check if operations are allowed\n  \
             • Get suggested session name\n  \
             • Validate session names\n\
             \n\
             QUERY TYPES:\n\
             • session-exists <name>      - Check if named session exists\n  \
             • session-count              - Get total number of sessions\n  \
             • can-run <operation>        - Check if operation is allowed (add, remove, sync)\n  \
             • suggest-name [prefix]      - Generate unique session name\n  \
             • validate-name <name>       - Validate session name format\n\
             \n\
             OUTPUT:\n\
             Machine-readable responses for conditional shell logic:\n  \
             • Success: \"true\" or \"<value>\"\n  \
             • Failure: \"false\" or error message\n  \
             • JSON: Structured output with --json flag\n\
             \n\
             PREREQUISITES:\n\
             • zjj must be initialized (zjj init)\n  \
             • Database must be accessible\n  \
             • Jujutsu and Zellij installations (for some queries)\n\
             \n\
             USE CASES:\n\
             • Shell scripts: Conditional logic based on session state\n  \
             • CI/CD pipelines: Check before automation\n  \
             • AI agents: Validate names before session creation\n  \
             • Bash completions: Generate dynamic suggestions\n\
             \n\
             RELATED COMMANDS:\n\
             • zjj list       - List all sessions\n  \
             • zjj status     - Get detailed session status\n  \
             • zjj context    - Full environment context for AI",
        )
        .arg(Arg::new("query_type").required(true).help(
            "Type of query: session-exists, session-count, can-run, suggest-name, validate-name",
        ))
        .arg(
            Arg::new("args")
                .required(false)
                .help("Query-specific arguments (e.g., session name for session-exists)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON for machine parsing"),
        )
        .after_help(
            "EXAMPLES:\n\
             \n\
             # Check if session exists\n\
             zjj query session-exists my-session && echo \"Found\" || echo \"Not found\"\n\
             \n\
             # Get current session count\n\
             count=$(zjj query session-count)\n\
             echo \"$count sessions active\"\n\
             \n\
             # Check if you can add sessions\n\
             zjj query can-run add && zjj add my-session\n\
             \n\
             # Get unique session name suggestion\n\
             name=$(zjj query suggest-name feature-)\n\
             zjj add \"$name\"\n\
             \n\
             # Validate session name format\n\
             zjj query validate-name \"my_session-123\"\n\
             \n\
             # JSON output for scripts\n\
             zjj query session-count --json | jq .count\n\
             \n\
             COMMON USE CASES:\n\
             \n\
             1. Conditional Session Creation (Bash Script):\n\
                if zjj query session-exists dev; then\n  \
                  echo \"Using existing dev session\"\n\
                else\n  \
                  zjj add dev\n\
                fi\n\
             \n\
             2. Bash Completion (Generate Suggestions):\n\
                # Suggest names matching prefix\n\
                if [[ \"$word\" == feature-* ]]; then\n  \
                  zjj query suggest-name \"${word%-*}-\" --json\n  \
                fi\n\
             \n\
             3. CI/CD Gate Check:\n\
                if ! zjj query can-run add; then\n  \
                  echo \"Cannot create session (database locked)\"\n  \
                  exit 1\n  \
                fi\n\
             \n\
             AI AGENT EXAMPLES:\n\
             \n\
             # Before creating session, validate name format\n\
             zjj query validate-name suggested_name --json\n\
             \n\
             # Get current session count before operations\n\
             current=$(zjj query session-count --json | jq .count)\n\
             \n\
             # Check if agent can run cleanup operations\n\
             zjj query can-run remove --json\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n\
             \n\
             Query provides programmatic gating:\n  \
             • Prevent invalid session names before creation\n  \
             • Gate operations on system state\n  \
             • Check resource availability\n  \
             • Make informed decisions about session management",
        )
}

pub fn cmd_completions() -> Command {
    Command::new("completions")
        .about("Generate shell completion scripts")
        .long_about(
            "Generate Shell Completion Scripts\n\
             \n\
             WHAT IT DOES:\n\
             Generates shell-specific completion scripts that enable:\n  \
             • Command name autocompletion (zjj <TAB>)\n  \
             • Subcommand autocompletion (zjj add <TAB>)\n  \
             • Session name completion (zjj remove my-s<TAB> → my-session)\n  \
             • Flag autocompletion (zjj add --<TAB>)\n  \
             • Smart suggestions based on system state\n\
             \n\
             SUPPORTED SHELLS:\n\
             • bash      - Bash 3.2+, with completion package\n  \
             • zsh       - Zsh 4.3.11+, with completion support\n  \
             • fish      - Fish 2.3+\n  \
             • elvish    - Elvish 0.13+\n  \
             • powershell - PowerShell 7.0+\n\
             \n\
             WHAT GETS COMPLETED:\n  \
             • All subcommands (add, remove, focus, sync, list, etc.)\n  \
             • All flags (--json, --dry-run, --force, --merge, etc.)\n  \
             • Session names from current database\n  \
             • Shell-specific syntax and options\n\
             \n\
             HOW IT WORKS:\n\
             This command generates completion scripts in the shell's native format.\n  \
             Install the output to your shell's completion directory.\n  \
             Some shells cache completions, requiring shell restart after install.\n\
             \n\
             PREREQUISITES:\n\
             • Shell installed and in PATH\n  \
             • Write permissions to shell completion directory\n  \
             • Shell support for completion scripts\n\
             \n\
             RELATED COMMANDS:\n\
             • zjj help      - View command help\n  \
             • zjj --version - Show version info",
        )
        .arg(
            Arg::new("shell")
                .required(true)
                .help("Shell type: bash, zsh, fish, elvish, powershell"),
        )
        .arg(
            Arg::new("instructions")
                .long("instructions")
                .short('i')
                .action(clap::ArgAction::SetTrue)
                .help("Print installation instructions for the specified shell"),
        )
        .after_help(
            "EXAMPLES:\n\
             \n\
             # Generate bash completions\n\
             zjj completions bash > ~/.local/share/bash-completion/completions/zjj\n\
             source ~/.bashrc  # Reload shell\n\
             \n\
             # Generate with installation instructions\n\
             zjj completions zsh --instructions\n\
             \n\
             # Fish completions (auto-installs to correct location)\n\
             zjj completions fish > ~/.config/fish/completions/zjj.fish\n\
             \n\
             # PowerShell completions\n\
             zjj completions powershell | Out-File $PROFILE\n\
             \n\
             COMMON USE CASES:\n\
             \n\
             1. Bash User First-Time Setup:\n\
                zjj completions bash --instructions\n\
                # Follow the printed instructions\n\
                source ~/.bashrc  # Apply completions\n\
             \n\
             2. Enable Completions in Zsh:\n\
                mkdir -p $HOME/.zsh/completions\n\
                zjj completions zsh > $HOME/.zsh/completions/_zjj\n\
                # Add to .zshrc: fpath=($HOME/.zsh/completions $fpath)\n\
             \n\
             3. Update Completions After Upgrade:\n\
                # Regenerate if new commands added\n\
                zjj completions fish > ~/.config/fish/completions/zjj.fish\n\
             \n\
             SHELL-SPECIFIC TIPS:\n\
             \n\
             Bash:\n  \
             • Install to: ~/.local/share/bash-completion/completions/\n  \
             • Or system-wide: /etc/bash_completion.d/\n  \
             • Requires 'bash-completion' package\n\
             \n\
             Zsh:\n  \
             • Add $fpath to .zshrc before calling compinit\n  \
             • May require 'autoload -Uz compinit && compinit'\n\
             \n\
             Fish:\n  \
             • Completions auto-load from ~/.config/fish/completions/\n  \
             • Most compatible shell for dynamic completion\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n\
             \n\
             Completions enable human shell productivity:\n  \
             • Reduce typos in session names\n  \
             • Discover available subcommands\n  \
             • Avoid remembering all flags\n  \
             • AI agents don't need completions (use --json for parsing)",
        )
}

pub fn cmd_backup() -> Command {
    Command::new("backup")
        .about("Create a backup of the session database")
        .arg(
            Arg::new("path")
                .help("Backup file path (default: .zjj/backups/zjj-backup-<timestamp>.json)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "Examples:\n  \
             # Create backup with auto-generated filename\n  \
             zjj backup\n\n  \
             # Create backup at specific path\n  \
             zjj backup ~/backups/zjj-sessions.json\n\n  \
             # JSON output for scripting\n  \
             zjj backup --json",
        )
}

pub fn cmd_restore() -> Command {
    Command::new("restore")
        .about("Restore session database from a backup file")
        .arg(
            Arg::new("path")
                .required(true)
                .help("Path to backup file to restore from"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt (DANGEROUS)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "Examples:\n  \
             # Restore from backup (with confirmation)\n  \
             zjj restore ~/backups/zjj-sessions.json\n\n  \
             # Restore without confirmation (DANGEROUS)\n  \
             zjj restore backup.json --force\n\n\
             WARNING: This command will REPLACE ALL existing session data!",
        )
}

pub fn cmd_verify_backup() -> Command {
    Command::new("verify-backup")
        .about("Verify integrity of a backup file")
        .long_about(
            "Verify Backup File Integrity\n\
             \n\
             WHAT IT DOES:\n\
             Performs comprehensive checks on a backup file to ensure it's:\n  \
             • Well-formed JSON (valid structure)\n  \
             • Contains all required fields\n  \
             • Has correct schema version\n  \
             • Has valid session data\n  \
             • Not corrupted or truncated\n\
             \n\
             INTEGRITY CHECKS PERFORMED:\n\
             • JSON syntax validation\n  \
             • Schema version compatibility\n  \
             • Required fields presence\n  \
             • Session data structure\n  \
             • Session count accuracy\n  \
             • File not corrupted/truncated\n  \
             • Permissions readable\n\
             \n\
             WHAT FAILURES MEAN:\n  \
             • \"Invalid JSON\" - File is corrupted, not valid JSON\n  \
             • \"Schema mismatch\" - Backup from incompatible version\n  \
             • \"Missing fields\" - Backup file incomplete or damaged\n  \
             • \"File truncated\" - Backup copy incomplete\n  \
             • \"Permission denied\" - Cannot read backup file\n  \
             • \"File not found\" - Backup path invalid\n\
             \n\
             TYPICAL USAGE:\n\
             Run before restore to ensure backup is safe to restore from.\n  \
             Regular verification catches corruption early.\n\
             \n\
             EXIT CODES:\n\
             • 0 - Verification successful (backup is valid)\n  \
             • 1 - Verification failed (backup is invalid)\n  \
             • 2 - System error (permission denied, file not found)\n\
             \n\
             RELATED COMMANDS:\n\
             • zjj backup         - Create a backup\n  \
             • zjj restore        - Restore from backup\n  \
             • zjj backup-list    - List all backups",
        )
        .arg(
            Arg::new("path")
                .required(true)
                .help("Path to backup file to verify"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON for machine parsing"),
        )
        .after_help(
            "EXAMPLES:\n\
             \n\
             # Verify backup file\n\
             zjj verify-backup backup.json\n\
             \n\
             # Verify and get JSON output\n\
             zjj verify-backup backup.json --json\n\
             \n\
             # Check exit code for scripting\n\
             if zjj verify-backup backup.json; then\n  \
               echo \"Backup is valid\"\n  \
               zjj restore backup.json\n\
             else\n  \
               echo \"Backup verification failed\"\n  \
               exit 1\n\
             fi\n\
             \n\
             COMMON USE CASES:\n\
             \n\
             1. Before Critical Restore (Recommended):\n\
                # Always verify before restore\n  \
                zjj verify-backup ~/.backups/sessions-2025.json\n  \
                # If successful, proceed with restore\n  \
                zjj restore ~/.backups/sessions-2025.json\n\
             \n\
             2. Regular Backup Maintenance:\n\
                # Verify all backups monthly\n  \
                for backup in ~/.zjj/backups/*.json; do\n  \
                  echo \"Checking $backup...\"\n  \
                  zjj verify-backup \"$backup\" && echo \"OK\" || echo \"FAILED\"\n  \
                done\n\
             \n\
             3. After Copying Backup to New Machine:\n\
                # Ensure backup transferred correctly\n  \
                scp user@oldmachine:backup.json .\n  \
                zjj verify-backup backup.json\n  \
                # If valid, safe to restore\n\
             \n\
             WORKFLOW CONTEXT FOR AI:\n\
             \n\
             • Always verify before restore operations\n  \
             • Check exit code: 0 = valid, non-zero = invalid\n  \
             • Invalid backups should not be restored\n  \
             • Use --json for programmatic validation\n  \
             • Report verification failures before attempting restore",
        )
}

pub fn cmd_essentials() -> Command {
    Command::new("essentials")
        .about("Show essential commands for daily use (human-friendly quick reference)")
        .long_about(
            "Essential Commands Quick Reference\n\
             \n\
             WHAT IT SHOWS:\n\
             A curated subset of the most important zjj commands for daily use.\n  \
             This is designed to be human-friendly and help you get started quickly.\n\
             \n\
             COMPARED TO OTHER HELP:\n  \
             • zjj essentials    - Human-friendly quick reference (you are here)\n  \
             • zjj --help        - Complete command list with details\n  \
             • zjj context       - AI agent context (environment state)\n  \
             • zjj introspect    - AI agent introspection (command metadata)\n\
             \n\
             WHEN TO USE:\n  \
             • Learning zjj for the first time\n  \
             • Quick reminder of common commands\n  \
             • Share with team members getting started\n  \
             • Forget the exact command name\n\
             \n\
             SHOWS THESE CATEGORIES:\n  \
             • Getting Started: init\n  \
             • Session Management: add, list, focus, remove\n  \
             • Working in Sessions: status, sync, diff\n  \
             • Tools: dashboard, doctor\n\
             \n\
             OUTPUT MODES:\n  \
             • Human-readable (default): Clean, organized display\n  \
             • JSON (--json): Structured data for scripts",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Show essential commands\n  \
             zjj essentials\n\
             \n  \
             # JSON output for scripts\n  \
             zjj essentials --json\n\
             \n\
             TYPICAL USE CASE:\n  \
             New to zjj? Run this first to see the core workflow.\n  \
             Forgot a command? Run this for a quick reminder.\n\
             \n\
             FOR AI AGENTS:\n  \
             This is optimized for humans. AI agents should use:\n  \
             • zjj context --json       - Environment state\n  \
             • zjj introspect --json    - Command metadata\n  \
             • zjj --help-json          - Complete documentation",
        )
}

pub fn cmd_version() -> Command {
    Command::new("version")
        .about("Show detailed version information")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(
            "Examples:\n  \
             # Show version\n  \
             zjj version\n\n  \
             # JSON output for AI agents\n  \
             zjj version --json",
        )
}

pub fn cmd_onboard() -> Command {
    Command::new("onboard")
        .about("Output AGENTS.md template snippet for AI agent integration")
        .long_about(
            "Generate AI Agent Onboarding Snippet\n\
             \n\
             WHAT IT DOES:\n\
             Outputs a ready-to-paste markdown snippet for your project's AGENTS.md file.\n  \
             This helps AI agents quickly understand how to use ZJJ in your project.\n\
             \n\
             OUTPUT INCLUDES:\n  \
             • Essential commands (context, add, list, sync, remove, etc.)\n  \
             • Workflow patterns (create → work → sync → cleanup)\n  \
             • Exit code semantics (0-4 meaning)\n  \
             • JSON output examples for AI consumption\n  \
             • Link to full AI guide (docs/12_AI_GUIDE.md)\n\
             \n\
             WHEN TO USE:\n  \
             • Setting up AI agent documentation for your project\n  \
             • Creating AGENTS.md file for the first time\n  \
             • Updating AI agent instructions after ZJJ changes\n  \
             • Sharing ZJJ patterns with new AI collaborators\n\
             \n\
             WORKFLOW POSITION:\n\
             Run this ONCE during project setup or when updating AI docs:\n  \
             zjj onboard >> AGENTS.md\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj context      - Get current environment state\n  \
             • zjj introspect   - Explore all CLI commands\n  \
             • zjj doctor       - Check system health",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON with structured command list"),
        )
        .after_help(
            "EXAMPLES:\n  \
             # Output snippet to terminal\n  \
             zjj onboard\n\
             \n  \
             # Append to AGENTS.md\n  \
             zjj onboard >> AGENTS.md\n\
             \n  \
             # JSON format for programmatic use\n  \
             zjj onboard --json\n\
             \n  \
             # Create new AGENTS.md with header\n  \
             echo '# AI Agent Guide' > AGENTS.md\n  \
             zjj onboard >> AGENTS.md\n\
             \n\
             COMMON USE CASES:\n  \
             First-time setup:     zjj onboard >> AGENTS.md\n  \
             Update docs:          zjj onboard > /tmp/snippet.md\n  \
             JSON for tooling:     zjj onboard --json | jq '.snippet'\n\
             \n\
             AI AGENTS:\n  \
             This command is specifically designed for you.\n  \
             Use the output to document ZJJ patterns in your project's AGENTS.md.\n  \
             The snippet includes all essential commands and workflow patterns.\n  \
             Exit code 0 always (this command cannot fail).",
        )
}

pub fn cmd_hooks() -> Command {
    Command::new("hooks")
        .about("Manage git hooks for AI workflow integration")
        .subcommand(
            Command::new("install")
                .about("Install git hooks for AI integration")
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview what would be installed without making changes"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON for machine parsing"),
                ),
        )
}

pub fn cmd_agent() -> Command {
    Command::new("agent")
        .about("Track and query AI agents working in sessions")
        .long_about(
            "Agent Tracking\n\
             \n\
             WHAT IT DOES:\n\
             Tracks AI agents working in sessions and provides commands to:\n  \
             • List all agents working across sessions\n  \
             • Query agent metadata (task, PID, artifacts, etc.)\n  \
             • Show agent activity and status\n\
             \n\
             AGENT METADATA:\n\
             Agents are identified by metadata stored in session.metadata:\n  \
             • agent_id: Agent identifier (e.g., \"claude-code-1234\")\n  \
             • task_id: Task/bead ID being worked on (e.g., \"zjj-1fei\")\n  \
             • spawned_at: Unix timestamp of spawn time\n  \
             • pid: Agent process ID\n  \
             • exit_code: Agent exit code after completion\n  \
             • artifacts_path: Path to agent outputs\n\
             \n\
             WORKFLOW POSITION:\n\
             Use this to monitor AI agents working on tasks:\n  \
             zjj add task → [agent spawns] → zjj agent list → [work] → zjj agent list\n\
             \n\
             RELATED COMMANDS:\n  \
             • zjj list         - List all sessions\n  \
             • zjj status       - Show detailed session status\n  \
             • zjj context      - Get environment context",
        )
        .subcommand(
            Command::new("list")
                .about("List agents working in sessions")
                .long_about(
                    "List AI Agents\n\
                     \n\
                     WHAT IT SHOWS:\n\
                     • Agent ID and session name\n  \
                     • Task ID being worked on\n  \
                     • When agent was spawned\n  \
                     • Agent process ID (if available)\n  \
                     • Exit code (if completed)\n  \
                     • Artifacts path (if set)\n\
                     \n\
                     OUTPUT MODES:\n\
                     • Table: Human-readable table (default)\n  \
                     • JSON: Machine-readable output (--json)\n\
                     \n\
                     FILTERING:\n\
                     • All sessions: Shows agents in all active sessions (default)\n  \
                     • Specific session: Use --session <name> to filter",
                )
                .arg(
                    Arg::new("session")
                        .long("session")
                        .short('s')
                        .value_name("NAME")
                        .help("Filter by session name"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                )
                .after_help(
                    "EXAMPLES:\n  \
                     # List all agents\n  \
                     zjj agent list\n\
                     \n  \
                     # List agent for specific session\n  \
                     zjj agent list --session feature-x\n\
                     \n  \
                     # JSON output for scripting\n  \
                     zjj agent list --json\n\
                     \n\
                     COMMON USE CASES:\n  \
                     Monitor agents:           zjj agent list\n  \
                     Check specific session:   zjj agent list -s session-name\n  \
                     Parse programmatically:   zjj agent list --json | jq '.agents[]'\n\
                     \n\
                     AI AGENT EXAMPLES:\n  \
                     # Get all agents as JSON\n  \
                     zjj agent list --json\n\
                     \n  \
                     # Find agent for specific task\n  \
                     zjj agent list --json | jq '.agents[] | select(.task_id == \"zjj-1fei\")'\n\
                     \n  \
                     # Check if agent is still running\n  \
                     zjj agent list --json | jq '.agents[] | select(.exit_code == null)'",
                ),
        )
}

/// Build the root CLI command with all subcommands
#[allow(clippy::too_many_lines)]
pub fn build_cli() -> Command {
    Command::new("zjj")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZJJ Contributors")
        .about("ZJJ - Manage JJ workspaces with Zellij sessions")
        .long_about(
            "ZJJ - Manage JJ Workspaces with Zellij Sessions\n\
             \n\
             WHAT IS ZJJ?\n\
             A workflow tool that manages isolated development sessions by combining:\n  \
             • JJ (Jujutsu) workspaces for parallel Git branches\n  \
             • Zellij terminal multiplexer for organized UI layouts\n  \
             • SQLite database for session state tracking\n\
             \n\
             CORE CONCEPTS:\n  \
             • Session: A named development task with its own workspace and Zellij tab\n  \
             • Workspace: Isolated JJ workspace (similar to Git worktree)\n  \
             • Layout: Zellij tab configuration (templates: minimal, standard, full, split, review)\n\
             \n\
             TYPICAL WORKFLOW:\n  \
             1. zjj init              # Initialize in JJ repository (once)\n  \
             2. zjj add feature-x     # Create session with workspace + Zellij tab\n  \
             3. [work in session]     # Develop in isolated environment\n  \
             4. zjj sync feature-x    # Rebase on main branch\n  \
             5. zjj remove feature-x  # Cleanup when done\n\
             \n\
             COMMAND CATEGORIES:\n  \
             Session Lifecycle:  add, remove, list, status, focus\n  \
             Workspace Sync:     sync, diff\n  \
             System:             init, config, doctor\n  \
             Introspection:      context, introspect, dashboard\n  \
             Utilities:          backup, restore, verify-backup, completions, query\n\
             \n\
             PREREQUISITES:\n  \
             • jj (Jujutsu VCS) - https://github.com/martinvonz/jj\n  \
             • zellij (terminal multiplexer) - https://zellij.dev\n  \
             • Must be in a JJ repository (or use 'zjj init' to create one)\n  \
             • Must be inside Zellij session (for commands that open tabs)\n\
             \n\
             AI AGENT FEATURES:\n  \
             • All commands support --json for structured output\n  \
             • Semantic exit codes (0=success, 1=user error, 2=system, 3=not found, 4=invalid state)\n  \
             • Use 'zjj context --json' for complete environment information\n  \
             • Use 'zjj introspect --json' for machine-readable command documentation\n  \
             • Use 'zjj query <type>' for programmatic state queries\n\
             \n\
             COMMON PATTERNS:\n  \
             Create session:          zjj add feature-name\n  \
             Create background:       zjj add task --no-open\n  \
             Preview operation:       zjj add test --dry-run\n  \
             Sync with main:          zjj sync\n  \
             Check what changed:      zjj diff feature-name\n  \
             List all sessions:       zjj list --json\n  \
             Interactive dashboard:   zjj dashboard\n  \
             System health:           zjj doctor\n\
             \n\
             For detailed command help: zjj <command> --help",
        )
        .after_help(
            "EXIT CODES:\n  \
             0   Success\n  \
             1   User error (invalid input, validation failure, bad configuration)\n  \
             2   System error (IO failure, external command error, hook failure)\n  \
             3   Not found (session not found, resource missing, JJ not installed)\n  \
             4   Invalid state (database corruption, unhealthy system)\n\
             \n\
             QUICK START:\n  \
             zjj init                    # Setup (first time)\n  \
             zjj add my-feature          # Create session\n  \
             zjj list                    # See all sessions\n  \
             zjj sync my-feature         # Sync with main\n  \
             zjj remove my-feature       # Cleanup\n\
             \n\
             AI AGENTS:\n  \
             All commands support --json for structured output with semantic exit codes.\n  \
             Use 'zjj context --json' for complete environment state.\n  \
             Use 'zjj introspect --json' for machine-readable docs.",
        )
        .arg(
            Arg::new("help-json")
                .long("help-json")
                .action(clap::ArgAction::SetTrue)
                .global(true)
                .help("Output complete CLI documentation as JSON (for AI agents)"),
        )
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(cmd_init())
        .subcommand(cmd_add())
        .subcommand(cmd_add_batch())
        .subcommand(cmd_list())
        .subcommand(cmd_remove())
        .subcommand(cmd_focus())
        .subcommand(cmd_status())
        .subcommand(cmd_sync())
        .subcommand(cmd_diff())
        .subcommand(cmd_config())
        .subcommand(cmd_context())
        .subcommand(cmd_prime())
        .subcommand(cmd_dashboard())
        .subcommand(cmd_introspect())
        .subcommand(cmd_doctor())
        .subcommand(cmd_query())
        .subcommand(cmd_completions())
        .subcommand(cmd_backup())
        .subcommand(cmd_restore())
        .subcommand(cmd_verify_backup())
        .subcommand(cmd_essentials())
        .subcommand(cmd_version())
        .subcommand(cmd_onboard())
        .subcommand(cmd_hooks())
        .subcommand(cmd_agent())
}
