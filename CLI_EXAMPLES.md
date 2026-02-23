# CLI Usage Examples

This document provides practical examples of using the ZJJ CLI, demonstrating how domain types ensure type safety and validation at command boundaries.

## Table of Contents

- [Session Management](#session-management)
- [Workspace Operations](#workspace-operations)
- [Queue Commands](#queue-commands)
- [Bead/Task Management](#beadtask-management)
- [Status Reporting](#status-reporting)
- [Error Handling](#error-handling)

---

## Session Management

Sessions are isolated workspaces for parallel development. Each session has a validated `SessionName` that must start with a letter and contain only alphanumeric characters, hyphens, and underscores.

### Creating a Session

```bash
# Create a new session
zjj session add auth-refactor --workspace /home/user/dev/zjj-workspaces/auth

# Output (JSON)
{"type":"action","verb":"create","target":"auth-refactor","status":"in_progress"}
{"type":"session","name":"auth-refactor","status":"creating","state":"initialized","workspace":"/home/user/dev/zjj-workspaces/auth"}
{"type":"result","kind":"command","status":"success","message":"Created session 'auth-refactor'"}
```

**Domain Types Involved:**
- `SessionName`: `"auth-refactor"` - Validated (1-100 chars, starts with letter, alphanumeric/hyphen/underscore)
- `WorkspaceState`: `initialized` - Session state enum
- `SessionStatus`: `creating` - Status following state machine

**Validation Rules:**
- Name must be 1-100 characters
- Must start with a letter (a-z, A-Z)
- Can contain letters, numbers, hyphens, underscores
- Whitespace is trimmed automatically
- Cannot be empty or whitespace-only

### Creating a Stacked Session

```bash
# Create a child session under a parent
zjj session add auth-tests --workspace /home/user/dev/zjj-workspaces/auth-tests --parent auth-refactor

# Output (JSON)
{"type":"action","verb":"create","target":"auth-tests","status":"in_progress"}
{"type":"session","name":"auth-tests","status":"creating","state":"initialized","workspace":"/home/user/dev/zjj-workspaces/auth-tests","parent":"auth-refactor"}
{"type":"result","kind":"command","status":"success","message":"Created session 'auth-tests'"}
```

**Domain Types Involved:**
- `ParentState`: `ChildOf { parent: "auth-refactor" }` - Enum representing parent relationship
- Session name validation applies to both parent and child names

**Validation Rules:**
- Parent session must exist
- Parent cannot be in `Completed` status
- Forms a session hierarchy for nested work

### Listing Sessions

```bash
# List all active sessions
zjj session list

# List sessions with JSON output
zjj session list --format json

# List only active sessions
zjj session list --status active

# List including closed sessions
zjj session list --include-closed
```

**Output (text format):**
```
SESSIONS (3)
------------------------------------------------------------
[*] auth-refactor /home/user/dev/zjj-workspaces/auth
    Status: active | State: initialized | Created: 2025-02-23 14:30
[_] feature-cache /home/user/dev/zjj-workspaces/cache (parent: auth-refactor)
    Status: paused | State: ready | Created: 2025-02-23 15:45
[x] bugfix-login /home/user/dev/zjj-workspaces/login
    Status: completed | State: committed | Created: 2025-02-23 10:15
```

**Domain Types Involved:**
- `SessionStatus`: `active`, `paused`, `completed` - State machine enforced
- `WorkspaceState`: `initialized`, `ready`, `committed` - Workspace lifecycle states

### Pausing and Resuming

```bash
# Pause an active session
zjj session pause auth-refactor

# Output (JSON)
{"type":"action","verb":"pause","target":"auth-refactor","status":"in_progress"}
{"type":"session","name":"auth-refactor","status":"paused","state":"ready","workspace":"/home/user/dev/zjj-workspaces/auth"}
{"type":"result","kind":"command","status":"success","message":"Paused session 'auth-refactor'"}

# Resume a paused session
zjj session resume auth-refactor

# Output (JSON)
{"type":"action","verb":"resume","target":"auth-refactor","status":"in_progress"}
{"type":"session","name":"auth-refactor","status":"active","state":"ready","workspace":"/home/user/dev/zjj-workspaces/auth"}
{"type":"result","kind":"command","status":"success","message":"Resumed session 'auth-refactor'"}
```

**State Machine Validation:**
- `Active` -> `Paused`: Valid transition
- `Paused` -> `Active`: Valid transition
- `Creating` -> `Paused`: **Invalid** (error)
- `Completed` -> `Active`: **Invalid** (error)

### Focusing a Session

```bash
# Switch to a session (set as current)
zjj session focus auth-refactor

# Output (JSON)
{"type":"action","verb":"focus","target":"auth-refactor","status":"in_progress"}
{"type":"session","name":"auth-refactor","status":"active","state":"ready","workspace":"/home/user/dev/zjj-workspaces/auth"}
{"type":"result","kind":"command","status":"success","message":"Focused session 'auth-refactor'"}
```

### Removing a Session

```bash
# Remove a session
zjj session remove auth-refactor

# Remove a session with children (requires --force)
zjj session remove auth-refactor --force

# Output (JSON)
{"type":"action","verb":"remove","target":"auth-refactor","status":"in_progress"}
{"type":"result","kind":"command","status":"success","message":"Removed session 'auth-refactor'"}
```

**Validation Rules:**
- Cannot remove session with children unless `--force` is used
- Session must not be locked by another agent
- Session is marked as `completed` before deletion

---

## Workspace Operations

Workspaces are isolated directories for parallel development. Each workspace has a validated `WorkspaceName`.

### Creating a Workspace

```bash
# Create a new workspace from current directory
zjj workspace create auth-migration

# Output (JSON)
{"type":"action","verb":"create","target":"workspace","status":"in_progress"}
{"type":"workspace","name":"auth-migration","state":"initialized","branch":"feature/auth-migration"}
{"type":"result","kind":"command","status":"success","message":"Created workspace 'auth-migration'"}
```

**Domain Types Involved:**
- `WorkspaceName`: `"auth-migration"` - Same validation as `SessionName`
- `WorkspaceState`: `initialized` - Initial state
- `BranchState`: `OnBranch { name: "feature/auth-migration" }` - Enum for branch tracking

### Checking Workspace Status

```bash
# Show workspace status
zjj workspace status

# Output (JSON)
{"type":"workspace","name":"auth-migration","state":"ready","branch":"feature/auth-migration","status":"ok"}
```

**Workspace States:**
- `initialized`: Workspace created, not ready
- `ready`: Workspace ready for development
- `active`: Development in progress
- `merged`: Changes merged to main
- `abandoned`: Workspace abandoned

---

## Queue Commands

The merge queue coordinates sequential multi-agent work. Queue entries use validated IDs and status enums.

### Adding to Queue

```bash
# Add a workspace to the queue
zjj queue add auth-workspace --priority 100 --agent claude-ops

# Output (JSON)
{"type":"queue_entry","id":1,"workspace":"auth-workspace","status":"pending","priority":100,"agent":"claude-ops"}
{"type":"result","kind":"command","status":"success","message":"Added workspace 'auth-workspace' to queue at position 1/3"}
```

**Domain Types Involved:**
- `WorkspaceName`: `"auth-workspace"` - Validated workspace identifier
- `Priority`: 100 - Priority value (0-1000, lower is higher priority)
- `AgentId`: `"claude-ops"` - Validated agent identifier (1-128 chars)
- `QueueStatus`: `pending` - Initial queue status

**Agent ID Validation:**
- 1-128 characters
- Alphanumeric plus hyphen, underscore, dot, colon
- Cannot be empty or whitespace-only

### Listing Queue

```bash
# List all queue entries
zjj queue list

# Output (JSON)
{"type":"queue_entry","id":1,"workspace":"auth-workspace","status":"pending","priority":100}
{"type":"queue_entry","id":2,"workspace":"cache-refactor","status":"processing","priority":200,"agent":"cursor-dev"}
{"type":"queue_entry","id":3,"workspace":"bugfix-login","status":"completed","priority":50}
{"type":"queue_summary","counts":{"total":3,"pending":1,"ready":1,"blocked":0,"in_progress":1}}
```

**Domain Types Involved:**
- `QueueEntryId`: 1, 2, 3 - Integer queue entry identifiers
- `QueueStatus`: `pending`, `processing`, `completed`, `failed`, `cancelled`
- `ClaimState`: Implicit (unclaimed, claimed, expired)

### Processing Next Entry

```bash
# Get next queue entry without processing
zjj queue next

# Output (JSON)
{"type":"queue_entry","id":1,"workspace":"auth-workspace","status":"pending","priority":100}
{"type":"result","kind":"command","status":"success","message":"Next entry: auth-workspace (ID: 1)"}

# Process next entry
zjj queue process --agent claude-ops

# Output (JSON)
{"type":"action","verb":"claim","target":"queue:1","status":"in_progress"}
{"type":"queue_entry","id":1,"workspace":"auth-workspace","status":"processing","priority":100,"agent":"claude-ops","claimed_at":"2025-02-23T16:30:00Z"}
{"type":"result","kind":"command","status":"success","message":"Processing entry 1: auth-workspace"}
```

**State Machine Validation:**
- `Unclaimed` -> `Claimed { agent, claimed_at, expires_at }`
- `Claimed` -> `Expired { previous_agent, expired_at }`
- `Expired` -> `Unclaimed` (reclaim)

### Queue Statistics

```bash
# Show queue statistics
zjj queue stats

# Output (text)
MERGE QUEUE STATISTICS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Entries:      15
Pending:            8
Processing:         2
Completed:          4
Failed:             1

Oldest Pending:     2h 15m (cache-refactor)
Avg Wait Time:      45m
```

---

## Bead/Task Management

Beads represent units of work with validated IDs and state tracking.

### Creating a Bead

```bash
# Create a new bead (task)
zjj bead add "Implement JWT authentication" --type feature --priority P0

# Output (JSON)
{"type":"action","verb":"create","target":"bead","status":"in_progress"}
{"type":"bead","id":"bd-abc123","title":"Implement JWT authentication","status":"open","priority":"P0","type":"feature"}
{"type":"result","kind":"command","status":"success","message":"Created bead 'bd-abc123'"}
```

**Domain Types Involved:**
- `BeadId`: `"bd-abc123"` - Must start with "bd-" prefix, followed by hex or alphanumeric
- `TaskPriority`: `P0` - Priority level enum (P0-P4, P0 is highest)
- `TaskStatus`: `open` - Initial status (open, in_progress, blocked, closed)
- `NonEmptyString`: Title - Non-empty, trimmed string

**Bead ID Validation:**
- Must start with "bd-" prefix
- Followed by 8-32 character hex or alphanumeric string
- Example: `bd-a1b2c3d4`, `bd-1234567890abcdef`

### Listing Beads

```bash
# List all beads
zjj bead list

# List beads with JSON output
zjj bead list --format json

# Filter by status
zjj bead list --status open

# Output (JSON)
{"type":"bead","id":"bd-abc123","title":"Implement JWT authentication","status":"open","priority":"P0","type":"feature","created_at":"2025-02-23T10:00:00Z"}
{"type":"bead","id":"bd-def456","title":"Fix login redirect","status":"in_progress","priority":"P1","type":"bugfix","created_at":"2025-02-23T11:30:00Z"}
```

### Updating Bead Status

```bash
# Update bead status
zjj bead update bd-abc123 --status in_progress

# Output (JSON)
{"type":"action","verb":"update","target":"bd-abc123","status":"in_progress"}
{"type":"bead","id":"bd-abc123","title":"Implement JWT authentication","status":"in_progress","priority":"P0","type":"feature"}
{"type":"result","kind":"command","status":"success","message":"Updated bead 'bd-abc123'"}
```

**State Machine Validation:**
- `Open` -> `InProgress`: Valid
- `InProgress` -> `Blocked`: Valid
- `Blocked` -> `InProgress`: Valid
- `InProgress` -> `Closed`: Valid
- `Closed` -> `InProgress`: **Invalid** (error)

### Closing a Bead

```bash
# Close a bead
zjj bead close bd-abc123

# Output (JSON)
{"type":"action","verb":"close","target":"bd-abc123","status":"in_progress"}
{"type":"bead","id":"bd-abc123","title":"Implement JWT authentication","status":"closed","priority":"P0","type":"feature","closed_at":"2025-02-23T16:45:00Z"}
{"type":"result","kind":"command","status":"success","message":"Closed bead 'bd-abc123'"}
```

---

## Status Reporting

Status commands provide comprehensive system state information.

### Overall Status

```bash
# Show overall system status
zjj status

# Output (JSON)
{"type":"status","sessions":{"active":2,"paused":1,"completed":5},"workspaces":{"ready":2,"active":1,"merged":3},"queue":{"pending":3,"processing":1}}
{"type":"result","kind":"command","status":"success","message":"System status: 3 active sessions, 2 ready workspaces, 3 pending queue entries"}
```

**Output (text format):**
```
ZJJ SYSTEM STATUS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Sessions:
  Active:     2  (auth-refactor, cache-update)
  Paused:     1  (feature-flags)
  Completed:  5

Workspaces:
  Ready:      2  (auth-workspace, cache-workspace)
  Active:     1  (login-fix)
  Merged:     3

Queue:
  Pending:    3  (bugfix-login [P0], api-refactor [P2], docs-update [P3])
  Processing: 1  (auth-migration by claude-ops)
  Completed:  8

System: OK (2 active sessions, 2 ready workspaces)
```

### Session Status

```bash
# Show status for a specific session
zjj status --session auth-refactor

# Output (JSON)
{"type":"session","name":"auth-refactor","status":"active","state":"ready","workspace":"/home/user/dev/zjj-workspaces/auth"}
{"type":"result","kind":"command","status":"success","message":"Session 'auth-refactor' status: active"}
```

### Workspace Status

```bash
# Show workspace status
zjj status --workspace auth-workspace

# Output (JSON)
{"type":"workspace","name":"auth-workspace","state":"ready","branch":"feature/auth","status":"ok","session":"auth-refactor"}
{"type":"result","kind":"command","status":"success","message":"Workspace 'auth-workspace' is ready"}
```

---

## Error Handling

All commands use domain types that validate input and provide clear error messages.

### Invalid Session Name

```bash
# Try to create session with invalid name (starts with number)
zjj session add 123invalid --workspace /tmp/test

# Output (JSON)
{"type":"error","kind":"validation","field":"name","message":"identifier must start with a letter","value":"123invalid"}
{"type":"result","kind":"command","status":"error","message":"Failed to create session: identifier must start with a letter"}
```

### Empty Name

```bash
# Try to create session with empty name
zjj session add "" --workspace /tmp/test

# Output (JSON)
{"type":"error","kind":"validation","field":"name","message":"identifier cannot be empty"}
{"type":"result","kind":"command","status":"error","message":"Failed to create session: identifier cannot be empty"}
```

### Invalid Agent ID

```bash
# Try to add to queue with invalid agent ID
zjj queue add test-workspace --agent ""

# Output (JSON)
{"type":"error","kind":"validation","field":"agent_id","message":"identifier cannot be empty"}
{"type":"result","kind":"command","status":"error","message":"Failed to add to queue: agent ID cannot be empty"}
```

### Invalid State Transition

```bash
# Try to pause a non-active session
zjj session pause completed-session

# Output (JSON)
{"type":"error","kind":"state_machine","current_state":"completed","target_state":"paused","message":"Cannot transition from 'completed' to 'paused'"}
{"type":"result","kind":"command","status":"error","message":"Cannot pause session in 'completed' state"}
```

### Duplicate Session

```bash
# Try to create duplicate session
zjj session add auth-refactor --workspace /tmp/test

# Output (JSON)
{"type":"error","kind":"conflict","field":"name","message":"Session 'auth-refactor' already exists"}
{"type":"result","kind":"command","status":"error","message":"Failed to create session: Session 'auth-refactor' already exists"}
```

### Invalid Bead ID

```bash
# Try to update bead with invalid ID (missing prefix)
zjj bead update abc123 --status closed

# Output (JSON)
{"type":"error","kind":"validation","field":"bead_id","message":"identifier must have prefix 'bd-' (got: abc123)"}
{"type":"result","kind":"command","status":"error","message":"Failed to update bead: identifier must have prefix 'bd-'"}
```

### Invalid Priority

```bash
# Try to set invalid priority
zjj bead add "Test task" --priority P10

# Output (JSON)
{"type":"error","kind":"validation","field":"priority","message":"must be one of: P0, P1, P2, P3, P4"}
{"type":"result","kind":"command","status":"error","message":"Failed to create bead: priority must be one of: P0, P1, P2, P3, P4"}
```

---

## JSON Output Format

All commands support `--format json` for structured output. Each output line is a complete JSON object:

```json
{"type":"action","verb":"create","target":"auth-refactor","status":"in_progress"}
{"type":"session","name":"auth-refactor","status":"creating","state":"initialized","workspace":"/home/user/dev/zjj-workspaces/auth"}
{"type":"result","kind":"command","status":"success","message":"Created session 'auth-refactor'"}
```

**Output Types:**
- `action`: Operation in progress/completed
- `session`: Session information
- `workspace`: Workspace information
- `queue_entry`: Queue entry information
- `queue_summary`: Queue statistics
- `bead`: Bead/task information
- `status`: System status information
- `result`: Final operation result
- `error`: Error information

**Parsing JSONL Output:**
```bash
# Parse with jq
zjj session add test-session --workspace /tmp/test --format json | jq .

# Filter specific fields
zjj session list --format json | jq '.type | select(. == "session")'

# Extract session names
zjj session list --format json | jq -r 'select(.type == "session") | .name'
```

---

## Configuration Examples

### Setting Configuration

```bash
# Set session limit
zjj config set session.max_count 10

# Set agent timeout
zjj config set agent.timeout_seconds 3600

# Output (JSON)
{"type":"action","verb":"set","target":"config","status":"in_progress"}
{"type":"config","key":"session.max_count","value":"10","scope":"local"}
{"type":"result","kind":"command","status":"success","message":"Set config 'session.max_count' = '10'"}
```

**Domain Types Involved:**
- `ConfigKey`: `"session.max_count"` - Dotted path (section.key)
- `ConfigValue`: `"10"` - Non-empty string value
- `ConfigScope`: `local` - Scope enum (local, global, system)

### Getting Configuration

```bash
# Get configuration value
zjj config get session.max_count

# Output (JSON)
{"type":"config","key":"session.max_count","value":"10","scope":"local"}
```

### Listing Configuration

```bash
# List all configuration
zjj config list

# Output (text)
CONFIGURATION (local)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
session.max_count:      10
agent.timeout_seconds:  3600
queue.max_entries:      100
```

---

## Agent Coordination

### Claiming Queue Entry

```bash
# Claim next queue entry
zjj queue process --agent claude-ops

# Output shows ClaimState transition:
# Unclaimed -> Claimed { agent: "claude-ops", claimed_at: "...", expires_at: "..." }
```

### Releasing Claim

```bash
# Release a claim (marks entry as unclaimed)
zjj queue cancel 1

# Output (JSON)
{"type":"action","verb":"cancel","target":"queue:1","status":"in_progress"}
{"type":"queue_entry","id":1,"workspace":"auth-workspace","status":"cancelled"}
{"type":"result","kind":"command","status":"success","message":"Cancelled queue entry 1"}
```

---

## Best Practices

### 1. Always Use Validated Identifiers

```bash
# GOOD: Valid session names
zjj session add auth-refactor --workspace /tmp/test
zjj session add feature_cache --workspace /tmp/test

# BAD: Invalid session names (will fail with clear error)
zjj session add 123invalid --workspace /tmp/test  # Error: must start with letter
zjj session add "with spaces" --workspace /tmp/test  # Error: invalid characters
```

### 2. Check Status Before Operations

```bash
# Check session status before pausing
zjj status --session auth-refactor
zjj session pause auth-refactor

# Check queue before processing
zjj queue list
zjj queue process --agent claude-ops
```

### 3. Use JSON Output for Scripting

```bash
# Parse session names from list
zjj session list --format json | jq -r 'select(.type == "session") | .name'

# Get queue entry IDs
zjj queue list --format json | jq -r 'select(.type == "queue_entry") | .id'
```

### 4. Respect State Machine Constraints

```bash
# Only pause active sessions
zjj session status auth-refactor  # Verify it's active
zjj session pause auth-refactor   # Then pause

# Can't resume active session (state machine prevents this)
zjj session resume auth-refactor  # Error if already active
```

### 5. Use Structured Output for Monitoring

```bash
# Monitor queue processing
watch -n 5 'zjj queue stats --format json | jq .'

# Track session lifecycle
zjj session add feature-x --workspace /tmp/test --format json | tee session-create.log
```

---

## Domain Type Reference

### Identifiers

| Type | Format | Example | Validation |
|------|--------|---------|------------|
| `SessionName` | 1-100 chars, starts with letter | `auth-refactor` | Alphanumeric, hyphen, underscore |
| `WorkspaceName` | Same as SessionName | `auth-workspace` | Same rules |
| `AgentId` | 1-128 chars | `claude-ops` | Alphanumeric, hyphen, underscore, dot, colon |
| `BeadId` | `bd-` prefix + hex/alnum | `bd-abc123` | Prefix required, 8-32 chars after prefix |
| `QueueEntryId` | Integer | `1`, `42` | Positive integer |

### Status Enums

| Enum | Values |
|------|--------|
| `SessionStatus` | `creating`, `active`, `paused`, `completed`, `failed` |
| `WorkspaceState` | `initialized`, `ready`, `active`, `merged`, `abandoned` |
| `QueueStatus` | `pending`, `processing`, `completed`, `failed`, `cancelled` |
| `TaskStatus` | `open`, `in_progress`, `blocked`, `closed` |
| `AgentStatus` | `pending`, `running`, `completed`, `failed`, `cancelled`, `timeout` |
| `TaskPriority` | `P0`, `P1`, `P2`, `P3`, `P4` (P0 highest) |

### State Types

| Type | Variants |
|------|----------|
| `BranchState` | `Detached`, `OnBranch { name }` |
| `ParentState` | `Root`, `ChildOf { parent }` |
| `ClaimState` | `Unclaimed`, `Claimed { agent, claimed_at, expires_at }`, `Expired { previous_agent, expired_at }` |

### Value Objects

| Type | Purpose |
|------|---------|
| `NonEmptyString` | Non-empty, trimmed string |
| `ConfigKey` | Dotted path (e.g., `session.max_count`) |
| `ConfigValue` | Non-empty configuration value |
| `Limit` | 1-1000 range |
| `Priority` | 0-1000 range (lower is higher priority) |
| `TimeoutSeconds` | 1-86400 range (1 second to 24 hours) |

---

This examples document demonstrates how domain types provide type safety, validation, and clear error messages throughout the ZJJ CLI.
