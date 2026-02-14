# Martin Fowler Test Plan

## Happy Path Tests
- `test_create_session_success`
- `test_idempotent_create_with_command_id_returns_existing_session`
- `test_replay_rebuilds_state_from_event_log_on_empty_db`

## Error Path Tests
- `test_unique_constraint_enforced`
- `test_reactor_continues_after_failed_write`

## Edge Case Tests
- `test_sqlite_database_too_small_rejected`
- `test_wal_magic_bytes_invalid_fails`

## Contract Verification Tests
- `test_precondition_validates_session_name_before_write`
- `test_postcondition_appends_event_for_successful_write`
- `test_invariant_single_writer_reactor_survives_failed_request`

## Given-When-Then Scenarios
### Scenario 1: Idempotent create returns previous result
Given: a session was created with `command_id=cmd-123`
When: create is invoked again with the same session name and `command_id=cmd-123`
Then:
- operation succeeds
- returned session identity matches existing row
- no duplicate session rows are present

### Scenario 2: Recovery replay reconstructs read model
Given: event log contains `upsert(replay-a)`, `upsert(replay-b)`, `delete(replay-a)`
When: `state.db` is removed and DB is reopened
Then:
- replay executes from event log
- only `replay-b` exists in sessions table

### Scenario 3: Reactor survives failed command and processes next command
Given: reactor processed `create(reactor-a)` successfully
When: duplicate `create(reactor-a)` fails and another `create(reactor-b)` is sent
Then:
- duplicate returns error
- follow-up write succeeds
- reactor remains active after failure
