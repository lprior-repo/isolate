# CLI Contracts Refactoring Checklist

## Phase 1: Foundation (DONE)

- [x] Create `domain_types.rs` with semantic newtypes
- [x] Implement identifier types (SessionName, TaskId, AgentId, ConfigKey)
- [x] Implement state enums (SessionStatus, QueueStatus, AgentStatus, etc.)
- [x] Implement value objects (Limit, Priority, TimeoutSeconds)
- [x] Add TryFrom implementations for all types
- [x] Add Display implementations for all types
- [x] Export domain types from mod.rs
- [x] Create `session_v2.rs` refactored example
- [x] Create `queue_v2.rs` refactored example
- [x] Create comprehensive test suite
- [x] Document refactoring approach

## Phase 2: Module Refactoring (TODO)

For each contract module, complete these steps:

### 2.1 Config Module
- [ ] Update `GetConfigInput.key` to use `ConfigKey`
- [ ] Update `SetConfigInput.key` to use `ConfigKey`
- [ ] Update `SetConfigInput.value` to use `ConfigValue`
- [ ] Update `SetConfigInput.scope` to use `Option<ConfigScope>`
- [ ] Update `ListConfigInput.scope` to use `Option<ConfigScope>`
- [ ] Update `ConfigValue` to use domain types where appropriate
- [ ] Remove `ConfigContracts::validate_key()`
- [ ] Remove `ConfigContracts::validate_scope()`
- [ ] Update contract implementations
- [ ] Update all tests

### 2.2 Task Module
- [ ] Update `CreateTaskInput.title` to use `NonEmptyString`
- [ ] Update `CreateTaskInput.priority` to use `Option<TaskPriority>`
- [ ] Update `UpdateTaskInput.status` to use `Option<TaskStatus>`
- [ ] Update `ListTasksInput.status` to use `Option<TaskStatus>`
- [ ] Update `ListTasksInput.priority` to use `Option<TaskPriority>`
- [ ] Update `ListTasksInput.limit` to use `Option<Limit>`
- [ ] Update `TaskResult.status` to use `TaskStatus`
- [ ] Update `TaskListResult` to use domain types
- [ ] Remove `TaskContracts::validate_title()`
- [ ] Remove `TaskContracts::validate_priority()`
- [ ] Remove `TaskContracts::validate_status()`
- [ ] Remove `TaskContracts::validate_limit()`
- [ ] Update contract implementations
- [ ] Update all tests

### 2.3 Agent Module
- [ ] Update `SpawnAgentInput.session` to use `SessionName`
- [ ] Update `SpawnAgentInput.agent_type` to use `AgentType`
- [ ] Update `SpawnAgentInput.timeout` to use `Option<TimeoutSeconds>`
- [ ] Update `ListAgentsInput.status` to use `Option<AgentStatus>`
- [ ] Update `ListAgentsInput.session` to use `Option<SessionName>`
- [ ] Update `StopAgentInput.agent_id` to use `AgentId`
- [ ] Update `WaitAgentInput.agent_id` to use `AgentId`
- [ ] Update `WaitAgentInput.timeout` to use `Option<TimeoutSeconds>`
- [ ] Update `AgentResult.agent_type` to use `AgentType`
- [ ] Update `AgentResult.status` to use `AgentStatus`
- [ ] Remove `AgentContracts::validate_agent_type()`
- [ ] Remove `AgentContracts::validate_status()`
- [ ] Remove `AgentContracts::validate_timeout()`
- [ ] Remove `AgentContracts::validate_pid()`
- [ ] Update contract implementations
- [ ] Update all tests

### 2.4 Stack Module
- [ ] Update `PushInput.name` to use `SessionName`
- [ ] Update `PushInput.parent` to use `Option<SessionName>`
- [ ] Update `PopInput.session` to use `Option<SessionName>`
- [ ] Update `PopInput.force` to use `ForceMode` enum
- [ ] Update `ListStackInput.root` to use `Option<SessionName>`
- [ ] Update `SyncStackInput.session` to use `Option<SessionName>`
- [ ] Update `StackResult.parent` to use `Option<SessionName>`
- [ ] Consider adding `StackDepth` value object
- [ ] Update `StackListResult.current` to use `Option<SessionName>`
- [ ] Simplify `validate_depth()` or move to `StackDepth` type
- [ ] Update contract implementations
- [ ] Update all tests

### 2.5 Status Module
- [ ] Update `GetStatusInput.session` to use `Option<SessionName>`
- [ ] Update `GetStatusInput.format` to use `Option<OutputFormat>`
- [ ] Update `DiffInput.session` to use `Option<SessionName>`
- [ ] Update `LogInput.session` to use `Option<SessionName>`
- [ ] Update `LogInput.limit` to use `Option<Limit>`
- [ ] Update `StatusResult.status` to use `SessionStatus`
- [ ] Update `FileDiff.status` to use `FileStatus`
- [ ] Remove `StatusContracts::validate_format()`
- [ ] Remove `StatusContracts::validate_limit()`
- [ ] Remove `StatusContracts::validate_file_status()`
- [ ] Update contract implementations
- [ ] Update all tests

### 2.6 Doctor Module
- [ ] Update `CheckComponentInput.component` to use appropriate type
- [ ] Consider adding `DoctorCheck` enum for known checks
- [ ] Already has `DoctorStatus` and `CheckStatus` enums (good!)
- [ ] Simplify validation using domain types where applicable
- [ ] Update contract implementations
- [ ] Update all tests

## Phase 3: Handler Integration (TODO)

For each handler in `/home/lewis/src/zjj/crates/zjj/src/cli/handlers/`:

### 3.1 Session Handlers
- [ ] Update `session_handler.rs` to parse `SessionName` at boundary
- [ ] Handle `ContractError` conversions to user-friendly messages
- [ ] Use `SessionStatus` enum for status checks
- [ ] Update force flags to use `ForceMode`

### 3.2 Task Handlers
- [ ] Update task handlers to parse domain types
- [ ] Use `TaskPriority` and `TaskStatus` enums
- [ ] Handle validation errors gracefully

### 3.3 Queue Handlers
- [ ] Update queue handlers to parse `SessionName`
- [ ] Use `QueueStatus` enum
- [ ] Use `Priority` value object

### 3.4 Agent Handlers
- [ ] Update agent handlers to parse domain types
- [ ] Use `AgentType`, `AgentStatus` enums
- [ ] Use `TimeoutSeconds` for timeouts

### 3.5 Config Handlers
- [ ] Update config handlers to parse `ConfigKey`
- [ ] Use `ConfigScope` enum
- [ ] Use `ConfigValue` for values

### 3.6 Status Handlers
- [ ] Update status handlers to parse `SessionName`
- [ ] Use `OutputFormat` enum
- [ ] Use `FileStatus` enum

## Phase 4: Testing (TODO)

- [ ] Run `cargo test` on all refactored modules
- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt` on all files
- [ ] Add property-based tests using proptest
- [ ] Test error messages are user-friendly
- [ ] Test all state machine transitions
- [ ] Test all domain type validations
- [ ] Integration tests for handlers

## Phase 5: Documentation (TODO)

- [ ] Update module documentation to use domain types
- [ ] Add examples of using domain types in handlers
- [ ] Document migration guide for other contributors
- [ ] Update CLI help text if needed
- [ ] Add "Contributing" section about domain types

## Phase 6: Cleanup (TODO)

- [ ] Remove old `session.rs` (renamed to `session_v2.rs`)
- [ ] Remove old `queue.rs` (renamed to `queue_v2.rs`)
- [ ] Remove any unused `String` usages
- [ ] Remove all `validate_*()` methods (in types now)
- [ ] Search for remaining `.unwrap()` calls
- [ ] Search for remaining `.expect()` calls
- [ ] Ensure no `panic!()` calls in production code
- [ ] Remove any `todo!()` or `unimplemented!()` calls

## Quality Checks

### Before Merging Each Module

- [ ] Zero unwrap/expect/panic in new code
- [ ] All public functions have `#[must_use]` where appropriate
- [ ] All types implement `Debug`, `Clone` where appropriate
- [ ] Error types implement `Display` for user messages
- [ ] All tests pass
- [ ] Clippy passes with `-D warnings`
- [ ] Code is formatted with `cargo fmt`
- [ ] Documentation is accurate and complete

### Final Review

- [ ] All modules refactored
- [ ] All handlers updated
- [ ] All tests passing
- [ ] Full test suite green
- [ ] Documentation complete
- [ ] No legacy code remaining
- [ ] Benchmarks run (performance impact acceptable)
- [ ] Code review completed
- [ ] Merge to main branch

## Progress Tracking

| Module | Domain Types | Tests | Handlers | Done |
|--------|--------------|-------|----------|------|
| domain_types | ✓ | ✓ | N/A | ✓ |
| session | ✓ | ✓ | - | ⏸ |
| queue | ✓ | ✓ | - | ⏸ |
| config | - | - | - | ⬜ |
| task | - | - | - | ⬜ |
| agent | - | - | - | ⬜ |
| stack | - | - | - | ⬜ |
| status | - | - | - | ⬜ |
| doctor | - | - | - | ⬜ |

Legend:
- ✓ Complete
- ⏸ Partially done (v2 examples created)
- ⬜ Not started
- - Not applicable

## Notes

- Always work on one module at a time
- Commit frequently with clear messages
- Run tests after each change
- Keep code review in mind
- Document any deviations from the plan
- Update this checklist as you go
