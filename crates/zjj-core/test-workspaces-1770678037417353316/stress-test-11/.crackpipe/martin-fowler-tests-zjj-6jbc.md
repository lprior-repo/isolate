# Martin Fowler Test Plan: Broadcast Command Implementation

## Test Philosophy
These tests are **executable specifications** that document the contract between the broadcast command and its users. Each test name describes a behavior, and Given-When-Then structure clarifies intent.

---

## Happy Path Tests

### test_broadcast_stores_message_when_active_agents_exist
**Given**:
- Database is initialized with state.db
- 3 active agents in registry: "architect-1", "builder-2", "builder-3"
- Agent "architect-1" sends broadcast message "Hello, world!"

**When**:
- `run(&args, OutputFormat::Human)` is called with agent_id="architect-1"

**Then**:
- Function returns `Ok(())`
- broadcasts table has 1 row with:
  - message = "Hello, world!"
  - sender_id = "architect-1"
  - sent_to = ["builder-2", "builder-3"] (JSON array, sorted)
- Response prints:
  - "Broadcast sent successfully"
  - "Sent to: 2 agents"
  - Recipients list with "builder-2" and "builder-3"

---

### test_broadcast_excludes_sender_from_recipients
**Given**:
- 2 active agents: "agent-1", "agent-2"
- Agent "agent-1" sends broadcast

**When**:
- Broadcast is executed with agent_id="agent-1"

**Then**:
- sent_to list contains exactly ["agent-2"]
- sent_to does NOT contain "agent-1"
- Response success = true

---

### test_broadcast_to_empty_recipient_list_succeeds
**Given**:
- Only 1 active agent in registry: "lone-wolf"
- Agent "lone-wolf" sends broadcast

**When**:
- Broadcast is executed with agent_id="lone-wolf"

**Then**:
- sent_to list is empty []
- Database row created with sent_to = "[]" (empty JSON array)
- Response prints "No other active agents"
- Function returns `Ok(())` (NOT an error)

---

### test_broadcast_handles_no_agents_registered
**Given**:
- Agent registry is empty (no agents at all)
- Broadcast attempted with agent_id="ghost"

**When**:
- Broadcast is executed

**Then**:
- sent_to list is empty []
- Message stored successfully
- Response indicates no recipients

---

### test_broadcast_timestamp_is_rfc3339
**Given**:
- Active agents exist
- Broadcast sent at specific time

**When**:
- Broadcast is executed

**Then**:
- timestamp field in response is valid RFC3339 format
- timestamp in database row matches response timestamp
- timestamp is within 1 second of current time (allow for clock drift)

---

### test_sent_to_list_is_sorted_alphabetically
**Given**:
- 5 active agents: "zebra", "alpha", "charlie", "bravo", "delta"
- Sender is "alpha"

**When**:
- Broadcast is executed

**Then**:
- sent_to list is ["bravo", "charlie", "delta", "zebra"] (alphabetical)
- Order is deterministic across multiple runs

---

### test_sent_to_list_contains_no_duplicates
**Given**:
- Agent registry somehow has duplicate entries (edge case protection)
- Agent "agent-1" sends broadcast

**When**:
- Broadcast is executed

**Then**:
- sent_to list contains unique agent IDs only
- No agent ID appears twice

---

### test_json_output_wrapped_in_schema_envelope
**Given**:
- Active agents exist
- Output format is JSON

**When**:
- `run(&args, OutputFormat::Json)` is called

**Then**:
- Output is valid JSON
- Top-level structure is SchemaEnvelope:
  - `type: "broadcast-response"`
  - `flavor: "single"`
  - `data: { success: true, message: "...", sent_to: [...], timestamp: "..." }`
- JSON is pretty-printed (indented)

---

### test_human_readable_shows_recipient_count
**Given**:
- 4 active agents, sender excluded = 3 recipients
- Output format is Human

**When**:
- Broadcast is executed

**Then**:
- Output includes "Sent to: 3 agents"
- Output includes bulleted list of 3 recipients

---

## Error Path Tests

### test_returns_error_when_database_not_initialized
**Given**:
- state.db does NOT exist (zjj not initialized)
- zjj_data_dir returns valid path but no database file

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::DatabaseNotInitialized)`
- Error message contains "ZJJ not initialized. Run 'zjj init' first."
- No database files created
- No output printed

---

### test_returns_error_when_message_is_empty
**Given**:
- Database exists
- args.message = "" (empty string)
- args.agent_id = "valid-agent"

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::EmptyMessage)`
- Error message indicates message cannot be empty
- No database insertion attempted
- No output printed

---

### test_returns_error_when_message_is_whitespace_only
**Given**:
- Database exists
- args.message = "   " (spaces only)
- args.agent_id = "valid-agent"

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::EmptyMessage)`
- Error message indicates message cannot be empty

---

### test_returns_error_when_agent_id_is_empty
**Given**:
- Database exists
- args.message = "Valid message"
- args.agent_id = "" (empty string)

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::InvalidAgentId)`
- Error message indicates agent_id cannot be empty

---

### test_returns_error_when_database_connection_fails
**Given**:
- Database file exists but is locked by another process
- Or database has permission issues

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::DatabaseConnectionFailed)`
- Error message includes underlying SQLite error
- No partial data written

---

### test_returns_error_when_table_creation_fails
**Given**:
- Database exists
- Filesystem is read-only (CREATE TABLE fails)
- Or disk is full

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::TableCreationFailed)`
- Error message includes table creation failure details
- No broadcast stored

---

### test_returns_error_when_message_insert_fails
**Given**:
- broadcasts table exists
- Disk is full during INSERT
- Or database is locked

**When**:
- `run()` is called

**Then**:
- Function returns `Err(Error::MessageStorageFailed)`
- Error message includes INSERT failure details
- No partial row created

---

### test_returns_error_when_recipient_list_serialization_fails
**Given**:
- This is a defensive test (should never happen with Vec<String>)
- Simulate serde_json failure

**When**:
- store_broadcast attempts to serialize sent_to

**Then**:
- Function returns `Err(Error::RecipientListSerializationFailed)`
- Error message includes serialization error

---

### test_returns_error_when_json_serialization_fails
**Given**:
- This is a defensive test (should never happen)
- Simulate serde_json failure on response

**When**:
- Output format is JSON and serialization fails

**Then**:
- Function returns `Err(Error::ResponseSerializationFailed)`
- Error message includes serialization error
- Database state is unchanged (transaction rolled back)

---

## Edge Case Tests

### test_handles_unicode_message_correctly
**Given**:
- Message contains emojis: "Hello 🌍🚀"
- Message contains non-ASCII: "你好世界"

**When**:
- Broadcast is executed

**Then**:
- Message stored exactly as provided (UTF-8 preserved)
- Response contains original unicode
- JSON serialization succeeds (valid UTF-8)

---

### test_handles_very_long_message
**Given**:
- Message is 10,000 characters
- Or message is 1MB (within SQLite TEXT limits)

**When**:
- Broadcast is executed

**Then**:
- Message stored completely
- No truncation
- Response succeeds

---

### test_handles_message_with_newlines
**Given**:
- Message contains multiple lines: "Line 1\nLine 2\nLine 3"

**When**:
- Broadcast is executed

**Then**:
- Newlines preserved in database
- JSON output escapes newlines correctly (\n)
- Human output displays as multiple lines

---

### test_handles_message_with_special_characters
**Given**:
- Message contains quotes: 'He said "hello"'
- Message contains backslashes: "Path: C:\\Users\\test"
- Message contains null bytes (if SQLite supports)

**When**:
- Broadcast is executed

**Then**:
- All characters preserved
- JSON escapes correctly
- Database stores raw string

---

### test_handles_agent_id_with_special_characters
**Given**:
- Agent IDs contain dashes: "architect-1"
- Agent IDs contain underscores: "builder_2"
- Agent IDs contain dots: "agent.dev.local"

**When**:
- Broadcast is executed

**Then**:
- Agent IDs stored correctly
- sent_to list contains exact IDs
- Filtering works (excludes sender correctly)

---

### test_handles_rapid_successive_broadcasts
**Given**:
- Same agent sends 100 broadcasts rapidly

**When**:
- All broadcasts executed in loop

**Then**:
- All 100 messages stored
- Timestamps are strictly increasing
- No database locks or conflicts
- All responses succeed

---

### test_handles_concurrent_broadcasts_from_different_agents
**Given**:
- 5 agents all broadcast simultaneously

**When**:
- All broadcasts executed concurrently (tokio::spawn)

**Then**:
- All 5 messages stored
- No database corruption
- Each sent_to list is correct for each sender
- No deadlocks

---

### test_broadcasts_table_created_on_first_run
**Given**:
- Database exists but broadcasts table does NOT exist

**When**:
- First broadcast is executed

**Then**:
- broadcasts table created with correct schema
- Message stored successfully
- Table persists for subsequent runs

---

### test_broadcasts_table_not_recreated_on_subsequent_runs
**Given**:
- broadcasts table already exists with data

**When**:
- Another broadcast is executed

**Then**:
- CREATE TABLE IF NOT EXISTS succeeds (no error)
- Existing data preserved
- New message appended

---

## Contract Verification Tests

### test_precondition_database_must_exist
**Given**:
- state.db does not exist

**When**:
- run() called

**Then**:
- Returns Error::DatabaseNotInitialized
- Does NOT create database
- Contract P1 enforced

---

### test_postcondition_sent_to_excludes_sender
**Given**:
- Any set of active agents including sender

**When**:
- Broadcast sent

**Then**:
- sender_id NOT in sent_to list
- Contract PO7 enforced for all possible sender IDs

---

### test_postcondition_sent_to_includes_all_active_agents
**Given**:
- N active agents in registry

**When**:
- Broadcast sent from any agent

**Then**:
- sent_to.len() == N - 1 (all except sender)
- Every active agent (except sender) appears exactly once
- Contract PO8 enforced

---

### test_postcondition_sent_to_excludes_stale_agents
**Given**:
- 3 active agents, 2 stale agents (outside heartbeat timeout)

**When**:
- Broadcast sent

**Then**:
- sent_to contains only the 3 active agents
- sent_to does NOT contain the 2 stale agents
- Contract PO9 enforced

---

### test_invariant_sent_to_never_null
**Given**:
- Any broadcast scenario (including no recipients)

**When**:
- Broadcast executed

**Then**:
- sent_to is always Vec<String> (never nil/null)
- sent_to may be empty but never null
- Invariant I1 enforced

---

### test_invariant_timestamp_always_rfc3339
**Given**:
- Any broadcast

**When**:
- Broadcast executed

**Then**:
- Response timestamp parses as valid RFC3339
- Database timestamp parses as valid RFC3339
- DateTime::parse_from_rfc3339() succeeds
- Invariant I2 enforced

---

### test_invariant_sent_to_json_is_valid_array
**Given**:
- Any broadcast

**When**:
- Database queried for sent_to field

**Then**:
- sent_to is valid JSON array
- serde_json::from_str::<Vec<String>>() succeeds
- Invariant I3 enforced

---

### test_invariant_sent_to_no_duplicates
**Given**:
- Agent registry has no duplicates (normal case)

**When**:
- Broadcast executed

**Then**:
- sent_to.len() == sent_to.iter().collect::<HashSet<_>>().len()
- No duplicates in sent_to list
- Invariant I4 enforced

---

### test_invariant_sent_to_sorted_alphabetically
**Given**:
- 10+ agents with random IDs

**When**:
- Broadcast executed

**Then**:
- sent_to is sorted: sent_to.windows(2).all(|w| w[0] <= w[1])
- Order is deterministic
- Invariant I5 enforced

---

### test_invariant_success_always_true_for_successful_broadcast
**Given**:
- Broadcast that succeeds (no errors)

**When**:
- Response constructed

**Then**:
- response.success == true
- Invariant I6 enforced

---

## Given-When-Then Scenarios

### Scenario: Agent coordinates workflow start
**Given**:
- Orchestrator agent "conductor" is registered
- 5 worker agents are active: "worker-1" through "worker-5"
- Workflow requires all workers to start simultaneously

**When**:
- Conductor sends broadcast: "START_PHASE_1"

**Then**:
- Message stored with timestamp T0
- sent_to = ["worker-1", "worker-2", "worker-3", "worker-4", "worker-5"]
- Response confirms 5 recipients
- Workers can poll for new broadcasts and see START_PHASE_1

---

### Scenario: Agent discovers it's alone
**Given**:
- Agent "lone-explorer" just registered
- No other agents in registry

**When**:
- Agent sends broadcast: "Anyone out there?"

**Then**:
- Message stored successfully
- sent_to = []
- Response: "No other active agents"
- Agent learns it's the only one (sent_to empty)

---

### Scenario: Stale agent excluded from broadcast
**Given**:
- Agent "builder-1" active (heartbeat 5 seconds ago)
- Agent "builder-2" stale (heartbeat 2 minutes ago, timeout 60s)
- Agent "architect" sends broadcast

**When**:
- Architect sends broadcast

**Then**:
- sent_to = ["builder-1"] (only active)
- "builder-2" NOT in sent_to
- Stale agent doesn't receive notification (expected)

---

### Scenario: Multiple orchestrators coordinating
**Given**:
- 3 orchestrator agents: "orch-1", "orch-2", "orch-3"
- 10 worker agents
- Orch-1 needs to delegate work to other orchestrators

**When**:
- Orch-1 sends broadcast: "REBALANCE_WORKERS"

**Then**:
- sent_to includes ["orch-2", "orch-3"] + all 10 workers
- orch-1 NOT in sent_to
- All non-sender agents receive rebroadcast instruction

---

### Scenario: Broadcast audit trail
**Given**:
- System runs for 1 hour
- 150 broadcasts from various agents

**When**:
- Query broadcasts table: SELECT * FROM broadcasts ORDER BY timestamp

**Then**:
- 150 rows exist
- Each row has id, message, sender_id, sent_to, timestamp
- Timestamps monotonically increasing
- sent_to arrays always valid JSON
- Full audit trail available for debugging

---

## End-to-End Test

### test_full_broadcast_workflow
**Given**:
- Fresh zjj init (database initialized)
- Agent registry started with 60s timeout
- 4 agents registered and heartbeated:
  - "architect-1" (last seen: now)
  - "builder-2" (last seen: now - 10s)
  - "tester-3" (last seen: now - 30s)
  - "stale-agent" (last seen: now - 90s)

**When**:
- architect-1 runs: `zjj broadcast "Deploy to production" --json`

**Then**:
1. Response is valid JSON
2. SchemaEnvelope.type = "broadcast-response"
3. Response.data.success = true
4. Response.data.message = "Deploy to production"
5. Response.data.sent_to = ["builder-2", "tester-3"] (sorted, excludes sender and stale)
6. Response.data.timestamp is valid RFC3339 within 1 second
7. Database broadcasts table has 1 row matching response
8. No errors in logs
9. Exit code 0

**And** (verification):
- Query database: `SELECT * FROM broadcasts WHERE sender_id = 'architect-1'`
  - Returns exactly 1 row
  - sent_to JSON array: ["builder-2", "tester-3"]
  - Timestamp matches response

---

## Test Organization

### Unit Tests
- `tests::test_store_broadcast_creates_table`
- `tests::test_store_broadcast_inserts_row`
- `tests::test_sent_to_serialization`
- `tests::test_print_human_readable_format`
- `tests::test_get_db_pool_path`

### Integration Tests
- `tests::integration::test_broadcast_with_real_registry`
- `tests::integration::test_broadcast_concurrent`
- `tests::integration::test_broadcast_full_workflow`

### Property Tests (proptest)
- `prop_tests::test_sent_to_no_duplicates`
- `prop_tests::test_sent_to_sorted`
- `prop_tests::test_timestamp_rfc3339`
- `prop_tests::test_json_roundtrip`

### Golden Tests (insta)
- `golden_tests::test_json_output_format`
- `golden_tests::test_human_output_format`
- `golden_tests::test_empty_recipients_output`

---

## Test Coverage Requirements

- **Line coverage**: 100% for broadcast/mod.rs
- **Branch coverage**: 100% (all if/else/match paths)
- **Error path coverage**: Every error variant triggered
- **Happy path coverage**: All success scenarios
- **Edge case coverage**: All boundary conditions

---

## Performance Tests

### test_broadcast_latency_under_10ms
**Given**:
- 100 active agents in registry
- Database on local SSD

**When**:
- Broadcast executed 100 times

**Then**:
- Median latency < 10ms
- 95th percentile < 20ms
- No outliers > 100ms

---

### test_broadcast_throughput_100_per_second
**Given**:
- Concurrent broadcasts from multiple agents

**When**:
- 1000 broadcasts executed concurrently

**Then**:
- All complete within 10 seconds
- No database deadlocks
- No lost messages

---

## Test Mocking Strategy

### AgentRegistry Mock
```rust
#[cfg(test)]
mockall::mock! {
    pub AgentRegistry {}

    #[async_trait]
    impl AgentRegistryTrait for AgentRegistry {
        async fn get_active(&self) -> Result<Vec<ActiveAgent>, Error>;
    }
}
```

### Database Mock (in-memory SQLite)
```rust
#[cfg(test)]
async fn test_pool() -> Result<SqlitePool, Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}
```

### Time Mock (freeze time for deterministic tests)
```rust
#[cfg(test)]
use mock_instant::MockClock;

// Set time to specific value for test
MockClock::set(SystemTime::UNIX_EPOCH + Duration::from_secs(1234567890));
```

---

## Summary

**Total Test Count**: 45+ test cases

**Distribution**:
- Happy Path: 10 tests
- Error Path: 9 tests
- Edge Cases: 14 tests
- Contract Verification: 11 tests
- End-to-End: 1 test

All tests follow Given-When-Then structure and verify specific behaviors from the contract specification.
