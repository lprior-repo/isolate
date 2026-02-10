# Martin Fowler Test Plan: zjj-6bp0 - Agent-to-Session Mapping

## Executive Summary

This test plan covers the **bidirectional** agent-to-session ownership functionality. The core bug is that while `agent → session` works (agents.current_session), the inverse `session → agent` query is missing, and the context command reads from the wrong source.

## Test Categories

1. **Unit Tests**: Individual function contracts
2. **Integration Tests**: Multi-function workflows
3. **Regression Tests**: Existing behavior preserved
4. **Edge Cases**: Boundary conditions and special cases
5. **Contract Tests**: Pre/postconditions and invariants

---

## Part 1: Unit Tests - New Functionality

### Test Suite: `get_session_owner` (NEW FUNCTION)

#### `test_get_session_owner_with_active_agent`
**Purpose**: Verify inverse lookup returns correct agent

**Given**:
- Database with agent registered:
  - agent_id = "agent-abc123"
  - current_session = "workspace-feature-x"
  - last_seen = now (30 seconds ago)

**When**:
```rust
let owner = get_session_owner(&pool, "workspace-feature-x").await?;
```

**Then**:
```rust
assert_eq!(owner, Some("agent-abc123".to_string()));
```

**Verification**:
- Returns Some(agent_id)
- Agent ID matches registered agent
- Query uses index (<10ms)

---

#### `test_get_session_owner_with_stale_agent`
**Purpose**: Verify stale agents are filtered out

**Given**:
- Database with agent registered:
  - agent_id = "agent-stale"
  - current_session = "workspace-old"
  - last_seen = 61 seconds ago (>60s threshold)

**When**:
```rust
let owner = get_session_owner(&pool, "workspace-old").await?;
```

**Then**:
```rust
assert_eq!(owner, None); // Stale agent excluded
```

**Verification**:
- Returns None (agent is stale)
- SQL query includes staleness filter
- Threshold is 60 seconds

---

#### `test_get_session_owner_no_agent`
**Purpose**: Verify unowned sessions return None

**Given**:
- Database with no agents
- Session "workspace-unowned" exists in sessions table

**When**:
```rust
let owner = get_session_owner(&pool, "workspace-unowned").await?;
```

**Then**:
```rust
assert_eq!(owner, None);
```

**Verification**:
- Returns None (no owner)
- No error thrown
- Handles empty result set gracefully

---

#### `test_get_session_owner_multiple_agents_same_session`
**Purpose**: Verify first result returned (should not happen in practice)

**Given**:
- Database with TWO agents registered to same session (data inconsistency):
  - agent_id = "agent-1", current_session = "workspace-conflict"
  - agent_id = "agent-2", current_session = "workspace-conflict"
  - Both last_seen = now

**When**:
```rust
let owner = get_session_owner(&pool, "workspace-conflict").await?;
```

**Then**:
```rust
assert!(owner.is_some()); // Returns one of them
// Either "agent-1" or "agent-2" is acceptable
assert!(owner.unwrap().starts_with("agent-"));
```

**Verification**:
- Returns Some(agent_id) (doesn't crash)
- SQL LIMIT 1 ensures single result
- Logs warning about data inconsistency

---

#### `test_get_session_owner_database_error`
**Purpose**: Verify database errors are propagated

**Given**:
- Database connection closed/corrupted

**When**:
```rust
let owner = get_session_owner(&pool, "any-session").await;
```

**Then**:
```rust
assert!(owner.is_err());
assert!(matches!(owner.unwrap_err(), Error::DatabaseAccess(_)));
```

**Verification**:
- Returns Err
- Error type is DatabaseAccess
- No panic/unwrap

---

### Test Suite: Context Command Fix (AUTHORITATIVE SOURCE)

#### `test_context_uses_agents_table_not_metadata`
**Purpose**: Verify context reads from authoritative source

**Given**:
- Agent registered in agents table:
  - agent_id = "agent-real"
  - current_session = "workspace-test"
- Session metadata has WRONG agent_id:
  - session.metadata.agent_id = "agent-wrong" (stale data)

**When**:
```rust
let context = get_session_info().await?;
```

**Then**:
```rust
assert_eq!(context.agent, Some("agent-real".to_string()));
// NOT "agent-wrong" from metadata
```

**Verification**:
- Context.agent comes from agents table
- Ignores session.metadata.agent_id
- Sources truth from authoritative location

---

#### `test_context_returns_none_when_no_owner`
**Purpose**: Verify context handles missing owners gracefully

**Given**:
- Session "workspace-orphan" exists
- No agents registered to this session
- session.metadata.agent_id = None

**When**:
```rust
let context = get_session_info().await?;
```

**Then**:
```rust
assert_eq!(context.agent, None);
```

**Verification**:
- Returns None (no owner)
- No error thrown
- Handles absence gracefully

---

#### `test_context_filters_stale_owners`
**Purpose**: Verify context excludes stale agents

**Given**:
- Agent registered but stale:
  - agent_id = "agent-expired"
  - current_session = "workspace-abandoned"
  - last_seen = 5 minutes ago

**When**:
```rust
let context = get_session_info().await?;
```

**Then**:
```rust
assert_eq!(context.agent, None); // Stale agent excluded
```

**Verification**:
- Returns None (agent is stale)
- Same staleness rules as agents command
- Consistent behavior

---

### Test Suite: Existing Functions (REGRESSION CHECKS)

#### `test_run_register_stores_session`
**Purpose**: Verify registration still stores session

**Given**:
- Valid RegisterArgs:
  - agent_id = Some("agent-new")
  - session = Some("workspace-assign")

**When**:
```rust
run_register(&args, OutputFormat::Json).await?;
```

**Then**:
```rust
// Query database directly
let row = sqlx::query("SELECT current_session FROM agents WHERE agent_id = ?")
    .bind("agent-new")
    .fetch_one(&pool)
    .await?;
assert_eq!(row.get::<String, _>("current_session"), "workspace-assign");
```

**Verification**:
- current_session column set correctly
- Registration succeeds
- Existing behavior preserved

---

#### `test_run_heartbeat_does_not_clear_session`
**Purpose**: Verify heartbeat preserves session (invariant)

**Given**:
- Agent registered with session:
  - agent_id = "agent-active"
  - current_session = "workspace-busy"

**When**:
```rust
run_heartbeat(&HeartbeatArgs { command: None }, OutputFormat::Json).await?;
```

**Then**:
```rust
// Query database
let row = sqlx::query("SELECT current_session FROM agents WHERE agent_id = ?")
    .bind("agent-active")
    .fetch_one(&pool)
    .await?;
assert_eq!(row.get::<Option<String>, _>("current_session"), Some("workspace-busy".to_string()));
```

**Verification**:
- current_session UNCHANGED
- last_seen updated
- Invariant: heartbeat doesn't modify session

---

#### `test_run_status_includes_session`
**Purpose**: Verify status command returns session info

**Given**:
- Agent registered with session:
  - agent_id = "agent-status"
  - current_session = "workspace-listed"

**When**:
```rust
run_status(OutputFormat::Json).await?;
```

**Then**:
- JSON output includes:
  - agent_id = "agent-status"
  - current_session = "workspace-listed"

**Verification**:
- Output includes session field
- Serialization works correctly
- Existing behavior preserved

---

## Part 2: Integration Tests

### `test_full_ownership_lifecycle`
**Purpose**: End-to-end flow from registration to cleanup

**Steps**:

1. **Register agent with session**
```rust
run_register(&RegisterArgs {
    agent_id: Some("agent-lifecycle".to_string()),
    session: Some("workspace-full".to_string()),
}, OutputFormat::Json).await?;
```
Verify: Agent has session in database

2. **Query ownership by session (new function)**
```rust
let owner = get_session_owner(&pool, "workspace-full").await?;
assert_eq!(owner, Some("agent-lifecycle".to_string()));
```
Verify: Inverse lookup works

3. **Context command includes agent**
```rust
let context = get_session_info().await?;
assert_eq!(context.agent, Some("agent-lifecycle".to_string()));
```
Verify: Context uses authoritative source

4. **Send heartbeat**
```rust
run_heartbeat(&HeartbeatArgs { command: None }, OutputFormat::Json).await?;
```
Verify: Session still persists after heartbeat

5. **Verify ownership still active**
```rust
let owner = get_session_owner(&pool, "workspace-full").await?;
assert_eq!(owner, Some("agent-lifecycle".to_string()));
```
Verify: Ownership survived heartbeat

6. **Unregister agent**
```rust
run_unregister(&UnregisterArgs {
    agent_id: Some("agent-lifecycle".to_string()),
}, OutputFormat::Json).await?;
```
Verify: Agent removed from database

7. **Verify ownership cleared**
```rust
let owner = get_session_owner(&pool, "workspace-full").await?;
assert_eq!(owner, None);
```
Verify: Session now has no owner

**Success Criteria**:
- All steps complete without errors
- Ownership persists across heartbeats
- Cleanup clears all ownership data
- No unwrap/expect in code path

---

### `test_concurrent_agent_registration`
**Purpose**: Verify thread safety under concurrent registration

**Given**:
- 10 concurrent tasks
- All registering different agent_ids
- Some with same session (data inconsistency scenario)

**When**:
```rust
let handles: Vec<_> = (0..10).map(|i| {
    tokio::spawn(async move {
        run_register(&RegisterArgs {
            agent_id: Some(format!("agent-{}", i)),
            session: Some("shared-session".to_string()),
        }, OutputFormat::Json).await
    })
}).collect();

for handle in handles {
    handle.await??;
}
```

**Then**:
- All 10 agents registered successfully
- All have current_session = "shared-session"
- No deadlocks or race conditions
- Database has 10 rows

**Verification**:
- Concurrent inserts don't deadlock
- Each agent gets unique agent_id
- Session ownership is tracked (even if inconsistent)

---

### `test_context_command_integration`
**Purpose**: Verify context command integration with real data

**Given**:
- Workspace "workspace-integration" exists
- Agent "agent-integration" registered to this session
- Session metadata has stale agent_id = "agent-old"

**When**:
```rust
let context = run_context(true, None, false, false).await?;
```

**Then**:
```rust
// Parse JSON output
let json: ContextOutput = serde_json::from_str(&output)?;

assert_eq!(json.session.agent, Some("agent-integration".to_string()));
// NOT "agent-old" from metadata
```

**Verification**:
- Context reads from agents table
- Ignores stale metadata
- JSON schema valid
- Human-readable output also correct

---

## Part 3: Edge Cases

### `test_session_with_special_characters`
**Purpose**: Verify SQL injection protection and special character handling

**Given**:
- Session name with special chars: `workspace-test'; DROP TABLE agents; --`

**When**:
```rust
run_register(&RegisterArgs {
    agent_id: Some("agent-special".to_string()),
    session: Some("workspace-test'; DROP TABLE agents; --".to_string()),
}, OutputFormat::Json).await?;
```

**Then**:
```rust
let owner = get_session_owner(&pool, "workspace-test'; DROP TABLE agents; --").await?;
assert_eq!(owner, Some("agent-special".to_string()));

// Verify agents table still exists
let count = sqlx::query("SELECT COUNT(*) FROM agents")
    .fetch_one(&pool)
    .await?;
assert!(count.get::<i64, _>("COUNT(*)") > 0);
```

**Verification**:
- Special characters escaped correctly
- SQL injection attempt fails
- Query returns correct result
- Table not dropped

---

### `test_very_long_session_name`
**Purpose**: Verify handling of large session names

**Given**:
- Session name = 10KB string

**When**:
```rust
let long_name = "x".repeat(10_000);
run_register(&RegisterArgs {
    agent_id: Some("agent-long".to_string()),
    session: Some(long_name.clone()),
}, OutputFormat::Json).await?;
```

**Then**:
- Either succeeds (SQLite TEXT has no practical limit)
- Or returns Err with validation error

**Verification**:
- No silent truncation
- Error message clear if rejected
- If succeeds, round-trips correctly

---

### `test_agent_reregistration_with_same_session`
**Purpose**: Verify idempotent re-registration

**Given**:
- Agent already registered:
  - agent_id = "agent-repeat"
  - current_session = "workspace-same"

**When**:
```rust
// Re-register with same session
run_register(&RegisterArgs {
    agent_id: Some("agent-repeat".to_string()),
    session: Some("workspace-same".to_string()),
}, OutputFormat::Json).await?;

// Re-register again
run_register(&RegisterArgs {
    agent_id: Some("agent-repeat".to_string()),
    session: Some("workspace-same".to_string()),
}, OutputFormat::Json).await?;
```

**Then**:
```rust
// Only one row in database
let count = sqlx::query("SELECT COUNT(*) FROM agents WHERE agent_id = 'agent-repeat'")
    .fetch_one(&pool)
    .await?;
assert_eq!(count.get::<i64, _>("COUNT(*)"), 1);

// Session still set correctly
let owner = get_session_owner(&pool, "workspace-same").await?;
assert_eq!(owner, Some("agent-repeat".to_string()));
```

**Verification**:
- Idempotent (same result on repeat)
- ON CONFLICT DO UPDATE works
- No duplicate rows

---

### `test_session_switching`
**Purpose**: Verify agent can switch sessions

**Given**:
- Agent registered with session_a:
  - agent_id = "agent-switch"
  - current_session = "session-a"

**When**:
```rust
// Switch to session-b
run_register(&RegisterArgs {
    agent_id: Some("agent-switch".to_string()),
    session: Some("session-b".to_string()),
}, OutputFormat::Json).await?;
```

**Then**:
```rust
// No longer owner of session-a
let owner_a = get_session_owner(&pool, "session-a").await?;
assert_eq!(owner_a, None);

// Now owner of session-b
let owner_b = get_session_owner(&pool, "session-b").await?;
assert_eq!(owner_b, Some("agent-switch".to_string()));
```

**Verification**:
- Old session ownership cleared
- New session ownership established
- No orphaned references

---

## Part 4: Contract Tests

### `test_precondition_agent_id_nonempty`
**Purpose**: Verify precondition enforcement

**Given**:
- Valid database

**When**:
```rust
let result = run_register(&RegisterArgs {
    agent_id: Some("".to_string()), // Empty!
    session: Some("test".to_string()),
}, OutputFormat::Json).await;
```

**Then**:
```rust
assert!(result.is_err());
assert!(matches!(result.unwrap_err(), Error::InvalidInput));
```

**Verification**:
- Precondition checked BEFORE database access
- Returns InvalidInput error
- No database row created

---

### `test_precondition_session_nonempty_if_provided`
**Purpose**: Verify session validation

**Given**:
- Valid database

**When**:
```rust
let result = run_register(&RegisterArgs {
    agent_id: Some("agent-test".to_string()),
    session: Some("".to_string()), // Empty!
}, OutputFormat::Json).await;
```

**Then**:
```rust
assert!(result.is_err());
```

**Verification**:
- Empty session rejected
- Validation happens before INSERT
- Clear error message

---

### `test_postcondition_session_persisted`
**Purpose**: Verify postcondition: session in database

**Given**:
- Valid input arguments

**When**:
```rust
run_register(&RegisterArgs {
    agent_id: Some("agent-post".to_string()),
    session: Some("session-post".to_string()),
}, OutputFormat::Json).await?;
```

**Then**:
```rust
// Postcondition: session persisted
let row = sqlx::query("SELECT current_session FROM agents WHERE agent_id = ?")
    .bind("agent-post")
    .fetch_one(&pool)
    .await?;

assert_eq!(row.get::<String, _>("current_session"), "session-post");
```

**Verification**:
- Postcondition holds after function returns
- Database state matches expectations
- Transactional (all-or-nothing)

---

### `test_invariant_heartbeat_preserves_session`
**Purpose**: Verify invariant: heartbeat doesn't modify session

**Given**:
- Agent with session:
  - agent_id = "agent-inv"
  - current_session = "session-inv"

**When**:
```rust
run_heartbeat(&HeartbeatArgs { command: None }, OutputFormat::Json).await?;
```

**Then**:
```rust
// Invariant: session unchanged
let row = sqlx::query("SELECT current_session, last_seen FROM agents WHERE agent_id = ?")
    .bind("agent-inv")
    .fetch_one(&pool)
    .await?;

assert_eq!(row.get::<String, _>("current_session"), "session-inv"); // Unchanged
let last_seen: String = row.get("last_seen");
assert!(last_seen != original_last_seen); // Timestamp updated
```

**Verification**:
- Session field unchanged (invariant)
- last_seen updated (function purpose)
- Only intended fields modified

---

### `test_invariant_persistence_across_reconnect`
**Purpose**: Verify durability invariant

**Given**:
- File-based database (not :memory:)
- Agent registered with session

**When**:
```rust
// Register
run_register(&RegisterArgs {
    agent_id: Some("agent-durability".to_string()),
    session: Some("session-durability".to_string()),
}, OutputFormat::Json).await?;

// Close connection
drop(pool);

// Reopen database
let new_pool = SqlitePool::connect(&db_path).await?;

// Query from new connection
let owner = get_session_owner(&new_pool, "session-durability").await?;
```

**Then**:
```rust
assert_eq!(owner, Some("agent-durability".to_string()));
```

**Verification**:
- Data survives connection close
- SQLite durability works
- Invariant: persistence

---

## Part 5: Performance Tests

### `test_get_session_owner_performance_with_index`
**Purpose**: Verify query performance with index

**Given**:
- Database with 1000 agents
- Index idx_agents_current_session created

**When**:
```rust
let start = std::time::Instant::now();
let owner = get_session_owner(&pool, "target-session").await?;
let elapsed = start.elapsed();
```

**Then**:
```rust
assert!(elapsed < Duration::from_millis(10)); // <10ms
```

**Verification**:
- Query uses index (EXPLAIN QUERY PLAN)
- Performance acceptable
- Scales with agent count

---

### `test_get_session_owner_performance_without_index`
**Purpose**: Verify performance without index (regression check)

**Given**:
- Database with 1000 agents
- Index idx_agents_current_session dropped

**When**:
```rust
let start = std::time::Instant::now();
let owner = get_session_owner(&pool, "target-session").await?;
let elapsed = start.elapsed();
```

**Then**:
```rust
// Will be slower, but should still complete
assert!(elapsed < Duration::from_secs(1)); // <1s (generous)
```

**Verification**:
- Full table scan works
- Performance degrades gracefully
- Index provides clear benefit

---

## Part 6: Regression Tests

### `test_existing_agents_command_still_works`
**Purpose**: Verify existing functionality unchanged

**Given**:
- Agents registered in database

**When**:
```rust
run_agents(&AgentsArgs {
    all: true,
    session: None,
}, OutputFormat::Json).await?;
```

**Then**:
- Output includes all agents with sessions
- JSON schema unchanged
- Backward compatibility maintained

**Verification**:
- Existing tests still pass
- No breaking changes
- Output format stable

---

### `test_existing_context_command_without_session`
**Purpose**: Verify context works when agent has no session

**Given**:
- Agent registered without session:
  - agent_id = "agent-no-session"
  - current_session = NULL

**When**:
```rust
let context = run_context(true, None, false, false).await?;
```

**Then**:
```rust
assert_eq!(context.session.agent, None);
// No errors thrown
```

**Verification**:
- Handles NULL session gracefully
- No regression from previous behavior
- Optional field works correctly

---

## Test Implementation Checklist

- [ ] All tests use `Result<T, Error>` (no unwrap/expect)
- [ ] Tests organized by suite (unit, integration, edge case, contract)
- [ ] Each test has clear Given-When-Then structure
- [ ] Database setup/teardown in fixtures
- [ ] Mock database for unit tests (if applicable)
- [ ] Real database for integration tests
- [ ] Performance tests use realistic data sizes
- [ ] Concurrent tests use tokio::spawn
- [ ] SQL injection tests use malicious input
- [ ] All edge cases covered (empty, stale, missing, special chars)
- [ ] Regression tests verify existing behavior
- [ ] Contract tests verify pre/postconditions
- [ ] Tests run in <5 seconds total
- [ ] All tests pass before committing

---

## Test Execution Strategy

### Phase 1: Unit Tests
```bash
# Run only unit tests
cargo test --lib ownership_tests
cargo test --lib context_tests
```

### Phase 2: Integration Tests
```bash
# Run integration tests (requires database)
cargo test --test ownership_integration
```

### Phase 3: Full Test Suite
```bash
# Run all tests (Moon preferred)
moon run :test
```

### Phase 4: Regression Check
```bash
# Run existing agent tests (ensure no breakage)
moon run :test agents
moon run :test context
```

---

## Coverage Goals

- **Line Coverage**: >90% for new code
- **Branch Coverage**: >85% for new code
- **Function Coverage**: 100% for new functions
- **Integration Coverage**: All code paths through workflow
- **Edge Case Coverage**: All error conditions tested

---

## References

**Contract Document**: `/tmp/rust-contract-zjj-6bp0.md`
**Code Files**:
- `/home/lewis/src/zjj/crates/zjj/src/commands/agents/mod.rs`
- `/home/lewis/src/zjj/crates/zjj/src/commands/context/mod.rs`

**Bead**: zjj-6bp0
**Architect**: architect-1
**Generated**: 2026-02-08
