# Martin Fowler Test Plan: context Command Agent Information

**Bead ID:** zjj-3ca4
**Command:** `zjj context`
**Goal:** Verify agent information is correctly exposed in context command output

---

## Happy Path Tests

### test_context_includes_agent_when_present_in_metadata
**Given:** Session with agent_id stored in metadata
  - session.metadata = {"bead_id": "zjj-abc12", "agent_id": "architect-1"}
**When:** User runs `zjj context` from within workspace
**Then:**
- `session.agent` field contains "architect-1"
- JSON output: `"agent": "architect-1"`
- Human output: "🤖 Agent: architect-1"
- No other fields affected

### test_context_shows_null_agent_when_not_in_metadata
**Given:** Session without agent_id in metadata
  - session.metadata = {"bead_id": "zjj-abc12"}
**When:** User runs `zjj context` from within workspace
**Then:**
- `session.agent` field is None
- JSON output: `"agent": null`
- Human output: No agent line displayed
- All other fields shown normally

### test_context_field_query_returns_agent_id
**Given:** Session with agent_id = "builder-3" in metadata
**When:** User runs `zjj context --field session.agent`
**Then:**
- Output prints: "builder-3"
- Exit code is 0
- No error message

### test_context_field_query_returns_null_when_no_agent
**Given:** Session without agent_id in metadata
**When:** User runs `zjj context --field session.agent`
**Then:**
- Output prints: "null"
- Exit code is 0
- No "Field not found" error

### test_context_agent_field_matches_metadata
**Given:** Session with agent_id in metadata
  - session.metadata.agent_id = "agent-x99"
**When:** User runs `zjj context --json`
**Then:**
- JSON output session.agent equals "agent-x99"
- Value is a string, not an object or array
- Value matches metadata exactly (no truncation, no transformation)

### test_context_handles_multiple_metadata_fields
**Given:** Session with rich metadata
  - session.metadata = {"bead_id": "zjj-123", "agent_id": "architect-1", "priority": 2, "tags": ["feature", "auth"]}
**When:** User runs `zjj context`
**Then:**
- session.agent = "architect-1"
- session.bead_id = "zjj-123"
- Other metadata fields ignored (not exposed in SessionContext)
- No errors from extra metadata fields

---

## Error Path Tests

### test_context_handles_invalid_metadata_json
**Given:** Session with corrupted metadata
  - session.metadata = invalid JSON (malformed)
**When:** User runs `zjj context`
**Then:**
- Error returned: "Failed to parse session metadata"
- Exit code is non-zero
- Error includes details about JSON parsing failure
- Context command fails gracefully (no partial output)

### test_context_handles_non_string_agent_id
**Given:** Session with agent_id as wrong type
  - session.metadata = {"agent_id": 123} (number, not string)
**When:** User runs `zjj context`
**Then:**
- session.agent is None (invalid type ignored)
- Warning logged: "agent_id has invalid type: number, expected string"
- No crash or panic
- Other fields shown correctly

### test_context_handles_empty_string_agent_id
**Given:** Session with empty agent_id
  - session.metadata = {"agent_id": ""}
**When:** User runs `zjj context`
**Then:**
- session.agent is None (empty string treated as missing)
- No error or warning
- JSON output: `"agent": null`
- Human output: No agent line

### test_context_field_query_fails_for_invalid_path
**Given:** Any session state
**When:** User runs `zjj context --field agent.nonexistent`
**Then:**
- Error returned: "Field not found: agent.nonexistent"
- Exit code is non-zero
- Error message includes the invalid path
- Suggestion shows valid field paths

### test_context_field_query_fails_when_session_missing
**Given:** In workspace but session deleted from database
**When:** User runs `zjj context --field session.agent`
**Then:**
- Error returned: "No session found for workspace: {name}"
- Exit code is non-zero
- Error suggests running `zjj list` to see active sessions

---

## Edge Case Tests

### test_context_handles_very_long_agent_id
**Given:** Session with extremely long agent_id
  - session.metadata.agent_id = "agent-very-long-id-" + "x" * 1000
**When:** User runs `zjj context`
**Then:**
- session.agent contains full agent_id string
- No truncation occurs
- JSON serialization succeeds
- Human output shows full ID (may wrap, but not truncated)

### test_context_handles_unicode_agent_id
**Given:** Session with unicode in agent_id
  - session.metadata.agent_id = "agent-中文-тест"
**When:** User runs `zjj context`
**Then:**
- session.agent contains unicode string
- JSON output valid UTF-8
- Human output displays correctly
- No encoding errors

### test_context_handles_null_agent_id_in_metadata
**Given:** Session with explicit null agent_id
  - session.metadata = {"agent_id": null}
**When:** User runs `zjj context`
**Then:**
- session.agent is None (treated same as missing)
- JSON output: `"agent": null`
- Human output: No agent line

### test_context_handles_missing_metadata_entirely
**Given:** Session with no metadata field
  - session.metadata = null (or field missing)
**When:** User runs `zjj context`
**Then:**
- session.agent is None
- session.bead_id is None
- All other fields populated normally
- No errors about missing metadata

### test_context_handles_workspace_without_session
**Given:** User runs `zjj context` from workspace directory
**But:** Session was deleted from database
**When:** Context command tries to get session info
**Then:**
- Error: "No session found for workspace: {name}"
- Exit code is non-zero
- Suggestion to run `zjj list`
- No partial context output

### test_context_field_query_handles_nested_paths
**Given:** Session with agent_id present
**When:** User runs `zjj context --field session.agent`
**Then:**
- Returns agent ID string
- Path "session.agent" works (dot notation)
- Alternative "session.agent.id" also works (if agent is object in future)

---

## Contract Verification Tests

### test_precondition_session_exists_in_database
**Given:** Workspace directory exists
**When:** get_session_info() is called
**Then:**
- Session database is queried for matching session name
- If session not found, error returned immediately
- No attempt to access missing session fields

### test_postcondition_agent_matches_metadata
**Given:** Session with agent_id in metadata
  - session.metadata.agent_id = "test-agent"
**When:** get_session_info() executes successfully
**Then:**
- SessionContext.agent equals "test-agent"
- Value is Some(String), not None
- Value exactly matches metadata (case-sensitive)

### test_postcondition_agent_is_none_when_missing
**Given:** Session without agent_id in metadata
**When:** get_session_info() executes successfully
**Then:**
- SessionContext.agent is None
- No default agent ID assigned
- No placeholder value used

### test_invariant_backward_compatible_with_old_sessions
**Given:** Session created before agent field was added
  - session.metadata exists but doesn't contain agent_id
**When:** New context command runs
**Then:**
- SessionContext.agent is None (graceful degradation)
- No migration needed
- Old sessions work without modification

### test_invariant_agent_never_empty_string
**Given:** Any session state
**When:** get_session_info() returns SessionContext
**Then:**
- If agent is Some, it's never an empty string
- Empty string in metadata converted to None
- No SessionContext.agent == Some("") possible

### test_invariant_json_schema_consistent
**Given:** Multiple context invocations
**When:** JSON output generated each time
**Then:**
- agent field always present in session object
- Field is always either string or null (never missing)
- Schema consistent across all invocations

---

## Given-When-Then Scenarios

### Scenario 1: Agent Session Context Display
**Given:**
- User is in workspace "feature-auth"
- Session metadata: {"bead_id": "zjj-3ca4", "agent_id": "architect-1"}
- Session created 2 hours ago
- Session status: Active

**When:** User runs `zjj context`

**Then:**
- Output shows: "📍 Location: Workspace 'feature-auth'"
- Output shows: "🎯 Session: feature-auth (Active)"
- Output shows: "🤖 Agent: architect-1"
- Output shows: "📋 Bead: zjj-3ca4"
- JSON output: `{"session": {"name": "feature-auth", "status": "Active", "agent": "architect-1", "bead_id": "zjj-3ca4", ...}}`
- Exit code is 0

### Scenario 2: Non-Agent Session Context
**Given:**
- User is in workspace "manual-work"
- Session metadata: {"bead_id": "zjj-xyz99"}
- No agent_id in metadata
- Session created 1 day ago
- Session status: Active

**When:** User runs `zjj context`

**Then:**
- Output shows: "📍 Location: Workspace 'manual-work'"
- Output shows: "🎯 Session: manual-work (Active)"
- Output does NOT show agent line
- Output shows: "📋 Bead: zjj-xyz99"
- JSON output: `{"session": {"name": "manual-work", "status": "Active", "agent": null, "bead_id": "zjj-xyz99", ...}}`
- Exit code is 0

### Scenario 3: Field Query Agent ID
**Given:**
- User is in workspace with agent_id = "builder-3"
- Session has valid metadata

**When:** User runs `zjj context --field session.agent`

**Then:**
- Output prints: "builder-3"
- Only the agent ID is printed (not full JSON)
- No labels, no formatting
- Exit code is 0
- Can be parsed by scripts: `AGENT=$(zjj context --field session.agent)`

### Scenario 4: Field Query No Agent
**Given:**
- User is in workspace without agent
- Session metadata exists but no agent_id

**When:** User runs `zjj context --field session.agent`

**Then:**
- Output prints: "null"
- Not an error (agent field exists, just null)
- Exit code is 0
- Script can check: `if [ "$(zjj context --field session.agent)" != "null" ]; then`

### Scenario 5: Field Query Invalid Path
**Given:**
- Any workspace context

**When:** User runs `zjj context --field agent.nonexistent.nested`

**Then:**
- Error printed: "Field not found: agent.nonexistent.nested"
- Exit code is 1
- No partial output
- Error suggests valid fields

### Scenario 6: JSON Output Consistency
**Given:**
- Workspace with agent_id = "agent-x99"
- User requests JSON output

**When:** User runs `zjj context --json > context.json`

**Then:**
- JSON file contains valid SchemaEnvelope
- session.agent field present: `"agent": "agent-x99"`
- Field type is string
- Can parse with `jq`: `jq '.session.agent' context.json`
- Returns: `"agent-x99"`

### Scenario 7: Multiple Sessions Different Agents
**Given:**
- Session "session-a" with agent_id = "architect-1"
- Session "session-b" with agent_id = "builder-2"
- Session "session-c" without agent

**When:**
- User in workspace "session-a" runs `zjj context --field session.agent`
- User in workspace "session-b" runs `zjj context --field session.agent`
- User in workspace "session-c" runs `zjj context --field session.agent`

**Then:**
- First output: "architect-1"
- Second output: "builder-2"
- Third output: "null"
- Each session shows its own agent correctly

### Scenario 8: Agent ID Format Validation
**Given:**
- Session metadata has agent_id with various formats:
  - "agent-1" (valid)
  - "architect-agent-2" (valid)
  - "" (empty string)
  - 123 (number, invalid)
  - null (explicit null)

**When:** User runs `zjj context` in each session

**Then:**
- "agent-1" → session.agent = "agent-1"
- "architect-agent-2" → session.agent = "architect-agent-2"
- "" → session.agent = None (empty treated as missing)
- 123 → session.agent = None (wrong type ignored)
- null → session.agent = None (explicit null)

---

## Integration Tests

### test_context_json_schema_validates
**Given:** Context command with --json flag
**When:** JSON output is generated
**Then:**
- JSON matches SchemaEnvelope structure
- $schema field is "zjj://context-response/v1"
- session object has agent field in correct position
- JSON can be parsed by serde_json
- JSON can be queried by jq

### test_context_agent_field_accessible_via_jq
**Given:** JSON output saved to file
**When:** User runs `jq '.session.agent' context.json`
**Then:**
- Returns agent ID string or null
- Exit code is 0
- No parse errors
- Query works consistently

### test_context_field_path_separator_works
**Given:** Session with agent_id
**When:** User runs various field queries:
  - `zjj context --field session.agent`
  - `zjj context --field session.bead_id`
  - `zjj context --field repository.branch`
**Then:**
- All queries work with dot separator
- All return correct values
- No "Field not found" errors for valid paths

---

## Regression Prevention Tests

### test_existing_context_behavior_preserved
**Given:** Context command before agent field addition
**When:** Comparing old vs new output
**Then:**
- All existing fields unchanged (location, session, repository, beads, health, suggestions)
- Only addition is agent field
- No fields removed or renamed
- No order changes (except agent added after bead_id)

### test_context_without_agent_works_as_before
**Given:** Session without agent_id (most sessions)
**When:** Running context command
**Then:**
- Output identical to pre-fix version (except agent: null in JSON)
- No extra lines in human output
- No performance degradation
- No warnings about missing agent

### test_context_backward_compatible_with_old_cli
**Given:** Scripts using old context output
**When:** Scripts run with new zjj version
**Then:**
- Scripts still work (agent field ignored)
- jq queries still work
- Field queries still work
- No breaking changes

---

## Performance Tests

### test_context_performance_with_metadata_extraction
**Given:** Large session database (1000 sessions)
**When:** Running context command
**Then:**
- Completes in < 100ms
- Agent extraction doesn't add noticeable overhead
- No additional database queries beyond existing

### test_context_field_query_performance
**Given:** Session with agent in metadata
**When:** Running `zjj context --field session.agent` 100 times
**Then:**
- Each invocation completes in < 50ms
- No memory leaks
- Consistent timing across runs
