# Contract Specification: context Command Agent Information

**Bead ID:** zjj-3ca4
**Feature:** Add agent information to context command output
**Issue:** The context command does not include agent information in its output, even when `ZJJ_AGENT_ID` is set. JSON shows `agent: null`. Field query returns 'Field not found: agent.id'.

---

## Context

### Domain Terms
- **Session**: Parallel workspace for isolating work (has metadata field)
- **Session Context**: Subset of session information exposed by the context command
- **Agent ID**: Identifier for the AI agent working on a session (from `ZJJ_AGENT_ID` env var)
- **Session Metadata**: Arbitrary JSON data attached to a session (includes `bead_id`, potentially `agent_id`)

### Problem Statement
The context command's `SessionContext` struct is missing agent information:
1. **Missing Field**: `SessionContext` has no `agent` field
2. **Not Read**: `get_session_info()` doesn't check session metadata for `agent_id`
3. **Not Exposed**: Even if `ZJJ_AGENT_ID` was stored in session metadata, it's not exposed in output
4. **Field Query Failure**: `zjj context --field agent.id` returns "Field not found"
5. **Impact**: Cannot determine which agent owns which session, agent-to-session mapping broken

### Root Cause Analysis
- When sessions are created, `ZJJ_AGENT_ID` environment variable should be stored in session metadata
- The `SessionContext` struct needs an `agent` field (Option<String>)
- The `get_session_info()` function needs to extract agent_id from session metadata
- The human-readable output needs to display agent information

### Assumptions
- `ZJJ_AGENT_ID` environment variable is set when an agent creates a session
- Session metadata is the correct place to store agent information
- Other parts of the system may already store `agent_id` in session metadata
- Agent information is optional (None for sessions without agents)

### Open Questions
1. **Storage Location**: Should agent_id be a top-level Session field or in metadata?
   - **Decision**: Keep in metadata (flexible, extensible), expose in SessionContext

2. **Environment Variable**: Which variable stores the agent ID during session creation?
   - **Assumption**: `ZJJ_AGENT_ID` (standard naming convention)

3. **Multiple Agents**: Can a session have multiple agents over time?
   - **Decision**: Single agent (the one that created/owns it currently)

4. **Agent Update**: Can agent ownership change over a session's lifetime?
   - **Decision**: Yes, but out of scope for this fix (just read current value)

---

## Preconditions

### Before `get_session_info()` executes:
- Must be called from within a workspace (Location::Workspace)
- Session database must exist and be accessible
- Current workspace name must match a session in the database
- Session must have valid timestamps (created_at, updated_at)

### Before context command displays agent:
- `SessionContext.agent` field must be populated (Some or None)
- Agent ID must be extracted from session metadata if present
- Agent ID must be a valid string (not empty, not null)

---

## Postconditions

### After `get_session_info()` executes successfully:
- `SessionContext.agent` contains agent_id from session metadata if present
- `SessionContext.agent` is None if session metadata doesn't contain agent_id
- All other SessionContext fields remain unchanged (name, status, bead_id, timestamps)

### After field query with `agent.id`:
- Returns agent ID string if agent exists
- Returns null if agent doesn't exist
- Returns "Field not found" only if field path is invalid (e.g., "agent.nonexistent")

### After JSON output:
- `session.agent` field is present in JSON output
- Field is either string (agent ID) or null (no agent)
- Field is never missing from JSON structure

---

## Invariants

### Data Integrity:
- Agent ID in SessionContext always matches agent_id in session metadata (if present)
- Agent ID is never an empty string (either Some(valid_id) or None)
- Agent ID format is consistent (e.g., "agent-1", "architect-1", "builder-3")

### Backward Compatibility:
- Old sessions without agent_id in metadata still work (agent field is None)
- JSON output includes agent field even for old sessions (value: null)
- Human-readable output gracefully handles missing agent (doesn't display agent line)

### Field Query Consistency:
- `zjj context --field session.agent` returns same value as `session.agent` in full JSON
- Field path separator works: both `session.agent` and `session.agent.id` supported
- Field query fails gracefully with clear error message for invalid paths

---

## Error Taxonomy

### SessionContextError variants

#### SessionContextError::MetadataParseError
- **When**: Session metadata is invalid JSON or malformed
- **Message**: "Failed to parse session metadata: {details}"
- **Resolution**: "Check session metadata format in database"
- **HTTP Status**: 500 Internal Server Error

#### SessionContextError::InvalidAgentId
- **When**: agent_id in metadata is not a valid string (e.g., number, boolean)
- **Message**: "Invalid agent_id format in session metadata: {found_type}"
- **Resolution**: "agent_id must be a string, found {type}. Update session metadata."
- **HTTP Status**: 400 Bad Request

#### SessionContextError::EmptyAgentId
- **When**: agent_id in metadata is an empty string
- **Message**: "agent_id in session metadata is empty"
- **Resolution**: "Remove empty agent_id from metadata or provide valid agent ID"
- **HTTP Status**: 400 Bad Request

#### SessionContextError::FieldNotFound
- **When**: Field query path doesn't exist in context structure
- **Message**: "Field not found: {field_path}"
- **Resolution**: "Check field path syntax. Valid fields: location, session, repository, beads, health"
- **HTTP Status**: 404 Not Found

#### SessionContextError::SessionNotFound
- **When**: Current workspace doesn't match any session in database
- **Message**: "No session found for workspace: {workspace_name}"
- **Resolution**: "Session may have been deleted. Run 'zjj list' to see active sessions."
- **HTTP Status**: 404 Not Found

---

## Contract Signatures

### Main Function (Modified)

#### get_session_info()
```rust
/// Get session context information for current workspace
///
/// # Preconditions
/// - Must be called from within a workspace
/// - Session database must be accessible
/// - Current workspace must have a corresponding session entry
///
/// # Postconditions
/// - Returns SessionContext with all fields populated
/// - agent field contains agent_id from session.metadata if present
/// - agent field is None if session.metadata doesn't contain agent_id
/// - All other fields (name, status, bead_id, timestamps) are populated correctly
///
/// # Errors
/// - Returns SessionError if session not found in database
/// - Returns MetadataError if session metadata is malformed
/// - Returns ValidationError if agent_id has invalid format
async fn get_session_info() -> Result<SessionContext>;
```

### Type Definition (Modified)

#### SessionContext struct
```rust
/// Session context information exposed by context command
///
/// # Contract
/// - `agent`: Optional agent ID extracted from session metadata
/// - `agent`: Some(String) if session.metadata.agent_id exists and is valid
/// - `agent`: None if session.metadata doesn't contain agent_id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Session name
    pub name: String,

    /// Current status
    pub status: String,

    /// Associated bead ID if any
    pub bead_id: Option<String>,

    /// Agent ID if session is owned by an agent
    pub agent: Option<String>,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// Last sync timestamp if any
    pub last_synced: Option<DateTime<Utc>>,
}
```

### Helper Function (New)

#### extract_agent_from_metadata()
```rust
/// Extract agent ID from session metadata
///
/// # Preconditions
/// - metadata is valid JSON value
///
/// # Postconditions
/// - Returns Some(agent_id) if metadata contains valid string agent_id
/// - Returns None if metadata doesn't contain agent_id
/// - Returns None if agent_id is not a string
/// - Returns None if agent_id is empty string
///
/// # Errors
/// - This function never errors (returns Result for API consistency)
/// - All errors are handled by converting to None
fn extract_agent_from_metadata(metadata: &serde_json::Value) -> Option<String>;
```

### Field Query Function (Modified)

#### extract_and_print_field()
```rust
/// Extract and print a single field from context
///
/// # Preconditions
/// - field_path is a dot-separated path (e.g., "session.agent", "repository.branch")
/// - context is a valid ContextOutput with all fields populated
///
/// # Postconditions
/// - Prints field value to stdout
/// - Returns Ok(()) if field path is valid
/// - Returns Err if field path doesn't exist
///
/// # Errors
/// - Returns FieldNotFound error if field_path is invalid
/// - Returns FieldNotFound error if intermediate path doesn't exist
/// - Error message includes full field path for clarity
fn extract_and_print_field(context: &ContextOutput, field_path: &str) -> Result<()>;
```

### Human-Readable Output (Modified)

#### print_human_readable()
```rust
/// Print context in human-readable format
///
/// # Preconditions
/// - context is valid with all fields populated
///
/// # Postconditions
/// - Prints session info including agent if present
/// - Agent line format: "🤖 Agent: {agent_id}" (only if agent is Some)
/// - No agent line printed if agent is None
/// - All other fields printed as before
///
/// # Errors
/// - This function doesn't return errors (prints directly)
fn print_human_readable(context: &ContextOutput);
```

---

## Non-goals

### Out of Scope for This Fix
- **Agent ID Storage**: Not adding agent_id to session metadata (only reading)
- **Agent Registration**: Not implementing agent registration/tracking system
- **Agent Lifecycle**: Not handling agent ownership changes over session lifetime
- **Multi-Agent Sessions**: Not supporting multiple agents per session
- **Agent Validation**: Not validating agent ID format or existence
- **Agent Permissions**: Not checking if agent has permission to access session
- **Agent Heartbeats**: Not tracking agent activity or liveness
- **Agent Commands**: Not implementing agent-specific commands (assign, reassign, etc.)

### Future Enhancements (Not in Current Scope)
- `zjj context --field agent.*` to query all agent fields
- `zjj assign <session> <agent>` to manually assign agents
- `zjj reassign <session> <new-agent>` to change agent ownership
- `zjj agents --sessions` to list all sessions for each agent
- Agent history tracking (which agent owned session over time)
- Agent collaboration (multiple agents working on same session)

### Assumptions About Other Systems
- Agent creation of sessions already stores `ZJJ_AGENT_ID` in session metadata
- If not, that's a separate bug to fix in session creation logic
- This fix only ensures context command *reads* agent_id if present
