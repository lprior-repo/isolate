# Contract Specification

## Context
- Feature: append-only event log with single-writer reactor, startup replay, and idempotent commands for `state.db` writes
- Domain terms: session state, command id, event envelope, replay rebuild, single-writer reactor
- Assumptions:
- `state.db` remains the materialized read model for fast queries.
- Event log is stored beside DB as `.zjj/state.events.jsonl`.
- Command idempotency is opt-in via `command_id` on write APIs.
- Open questions:
- Should replay run when DB is non-empty but partially inconsistent?
- Should processed command IDs also be rebuilt from event log on full recovery?

## Preconditions
- Write commands must pass session-name validation when creating records.
- Event log directory must be writable for persistence guarantees.
- A command id, when supplied, must be stable for retries of the same intent.

## Postconditions
- Every successful create/update/delete write appends exactly one event envelope.
- Event append and DB write happen in single-writer order.
- Startup replay reconstructs equivalent DB session state when DB is empty and event log exists.
- Duplicate command ids do not duplicate side effects.

## Invariants
- At most one state-writer reactor consumes write requests for a `SessionDb` instance.
- Event envelopes are append-only; no in-place edits.
- Session read APIs (`get`, `list`) observe serialized writer outcomes.
- Write failures do not terminate the reactor loop.

## Error Taxonomy
- `Error::ValidationError` - invalid session name or invalid transition input
- `Error::DatabaseError` - SQL errors, lock contention exhaustion, processed-command lookup failures
- `Error::IoError` - event-log file open/write/flush/read failures
- `Error::ParseError` - malformed event envelopes during replay

## Contract Signatures
- `SessionDb::create_with_command_id(name, workspace_path, command_id) -> Result<Session, Error>`
- `SessionDb::update_with_command_id(name, update, command_id) -> Result<(), Error>`
- `SessionDb::delete_with_command_id(name, command_id) -> Result<bool, Error>`
- `replay_event_log_if_needed(pool, event_log_path) -> Result<(), Error>`

## Non-goals
- Cross-process distributed consensus for multiple independent writer processes
- Full event sourcing migration for all non-session tables
- Automatic repair of non-empty but logically corrupted databases
