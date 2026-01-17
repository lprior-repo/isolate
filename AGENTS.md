# Agent Instructions

This project uses **bd** (beads) for issue tracking and **jjz** for development session management.

## Quick Start

### First Time Setup
```bash
bd onboard            # Get started with beads
jjz doctor            # Check system health
jjz context --json    # Get complete environment state
```

### Discovery Commands

AI agents have powerful introspection capabilities:

```bash
# Environment Discovery
jjz context --json       # Complete environment state (repo, sessions, dependencies)
jjz introspect --json    # CLI structure and capabilities metadata
jjz doctor --json        # System health checks with suggestions

# State Queries
jjz query session-exists <name>     # Check if session exists
jjz query session-count             # Count sessions (with optional filters)
jjz query can-run <command>         # Check if command can run (prerequisites)
jjz query suggest-name <pattern>    # Get next available name (pattern: "feature-{n}")

# Session Management
jjz list --json          # All sessions with metadata
jjz status <name> --json # Detailed session status
```

### Beads (Issue Tracking)

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Discovery Patterns for AI Agents

### Pattern 1: Environment Assessment

When starting work or encountering issues, get complete context:

```bash
# Step 1: Check system health
jjz doctor --json

# Step 2: Get environment context
jjz context --json

# Step 3: Understand available capabilities
jjz introspect --json
```

**What you'll learn:**
- System health status and auto-fixable issues
- Current working directory and repo state
- JJ repository root and current branch
- Active sessions and workspace locations
- Dependency versions (jj, zellij, beads)
- Whether running inside Zellij
- Available commands categorized by function

### Pattern 2: Session Discovery

Before creating or modifying sessions:

```bash
# List all sessions
jjz list --json

# Check if specific session exists
jjz query session-exists <name>

# Get suggested name following pattern
jjz query suggest-name "feature-{n}"

# Count sessions
jjz query session-count
```

**Use case:**
- Avoid duplicate session names
- Find next available sequential name
- Understand current session state before changes
- List sessions for status updates

### Pattern 3: Command Validation

Before running commands, check prerequisites:

```bash
# Check if command can run
jjz query can-run add
jjz query can-run sync
jjz query can-run focus

# Example response (can-run):
{
  "can_run": false,
  "command": "add",
  "blockers": [
    {
      "check": "zellij_running",
      "status": false,
      "message": "Not running inside Zellij"
    }
  ],
  "prerequisites_met": 2,
  "prerequisites_total": 3
}
```

**Use case:**
- Prevent command failures due to missing prerequisites
- Guide user to fix environment before proceeding
- Provide actionable error messages

### Pattern 4: JSON-First Workflow

All commands support `--json` for machine-readable output:

```bash
# Session operations
jjz add feature-x --json
jjz list --json
jjz status feature-x --json
jjz remove feature-x --json

# Diagnostics
jjz doctor --json
jjz context --json
jjz introspect --json

# Version control
jjz diff feature-x --json

# Queries
jjz query session-exists feature-x
jjz query can-run add
```

**JSON output structure:**
- Every response includes `success: bool` field
- Errors include structured error information
- Consistent schema across all commands
- Exit codes match semantic categories (0-4)

### Pattern 5: Exit Code Handling

All commands use semantic exit codes:

```bash
# Exit codes:
0 - Success
1 - User error (invalid input, validation failure, bad configuration)
2 - System error (IO failure, external command error, hook failure)
3 - Not found (session not found, resource missing, JJ not installed)
4 - Invalid state (database corruption, unhealthy system)
```

**Usage pattern:**
```bash
jjz add feature-x --json
EXIT_CODE=$?

case $EXIT_CODE in
  0) echo "Success" ;;
  1) echo "User error - check input" ;;
  2) echo "System error - check logs" ;;
  3) echo "Not found - resource missing" ;;
  4) echo "Invalid state - run jjz doctor" ;;
esac
```

### Pattern 6: Health Check and Auto-Fix

Use `jjz doctor` to diagnose and fix issues:

```bash
# Check system health
jjz doctor --json

# Auto-fix fixable issues
jjz doctor --fix --json

# Example doctor output:
{
  "success": false,
  "checks": [
    {
      "name": "Orphaned Workspaces",
      "status": "warn",
      "message": "Found 2 workspace(s) without session records",
      "suggestion": "Run 'jjz doctor --fix' to clean up",
      "auto_fixable": true,
      "details": {
        "orphaned_workspaces": ["old-feature", "abandoned-work"]
      }
    }
  ],
  "warnings": 1,
  "errors": 0,
  "auto_fixable_issues": 1
}
```

### Pattern 7: Session Workflow

Complete session lifecycle with discovery at each step:

```bash
# 1. Check prerequisites
jjz query can-run add

# 2. Get suggested name
jjz query suggest-name "feature-{n}"

# 3. Create session
jjz add feature-3 --json

# 4. Work in session...

# 5. Check session status
jjz status feature-3 --json

# 6. Sync with main
jjz sync feature-3 --json

# 7. Check diff
jjz diff feature-3 --json

# 8. Remove when done
jjz remove feature-3 --json
```

### Pattern 8: Introspection for New Features

When encountering unfamiliar commands:

```bash
# Get all command metadata
jjz introspect --json

# Get command-specific help
jjz <command> --help

# Get JSON schema for help
jjz --help-json

# Query what you can do
jjz query can-run <command>
```

**Introspect output structure:**
```json
{
  "jjz_version": "0.1.0",
  "capabilities": {
    "session_management": {
      "commands": ["init", "add", "remove", "list", "status", "focus", "sync"],
      "features": ["parallel_workspaces", "zellij_integration", "hook_lifecycle"]
    },
    "introspection": {
      "commands": ["introspect", "doctor", "query"],
      "features": ["capability_discovery", "health_checks", "auto_fix", "state_queries"]
    }
  },
  "dependencies": {
    "jj": {"required": true, "installed": true, "version": "jj 0.36.0"},
    "zellij": {"required": true, "installed": true, "version": "zellij 0.43.1"}
  },
  "system_state": {
    "initialized": true,
    "jj_repo": true,
    "sessions_count": 3,
    "active_sessions": 2
  }
}
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
