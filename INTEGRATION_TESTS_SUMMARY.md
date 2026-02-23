# Integration Tests for Core ZJJ Workflows

## Summary

Created comprehensive integration tests for ZJJ's critical user workflows in `/home/lewis/src/zjj/crates/zjj/tests/integration_tests.rs`.

## Test Coverage

### Workflow 1: Session → Workspace → Close Lifecycle

1. **`integration_session_workspace_close_workflow`**
   - Initialize zjj
   - Create session (creates workspace)
   - Verify session in list
   - Check whoami shows current session
   - Close session (remove workspace)
   - Verify session removed from list

2. **`integration_multiple_sessions_workflow`**
   - Create multiple sessions
   - Verify all sessions listed
   - Verify all workspaces exist
   - Cleanup all sessions

3. **`integration_session_branch_transitions`**
   - Create session on main branch
   - Create new bookmark/branch
   - Create session on new branch
   - Verify both sessions exist

### Workflow 2: Queue Operations

4. **`integration_queue_enqueue_dequeue_workflow`**
   - Create sessions
   - Enqueue sessions to queue
   - List queue entries
   - Get queue status
   - Dequeue sessions
   - Verify queue empty

5. **`integration_queue_status_with_entries`**
   - Create and enqueue session
   - Get queue status with JSON
   - Verify session appears in status

6. **`integration_queue_dequeue_nonexistent`**
   - Test error handling for non-existent session

### Workflow 3: Session State Transitions

7. **`integration_session_full_lifecycle_transitions`**
   - Initial state: no session (unregistered)
   - Create session (Creating → Ready)
   - Session becomes active
   - Switch away (Ready persists)
   - Switch back
   - Close session (Active → Removed)
   - Verify cleanup

8. **`integration_session_switch_workflow`**
   - Create multiple sessions
   - Test switch command
   - Verify whoami updates correctly

### Workflow 4: Workspace Creation and Removal

9. **`integration_workspace_creation_removal_workflow`**
   - Verify workspace doesn't exist initially
   - Create workspace (via add session)
   - Verify JJ files exist
   - Verify valid JJ repo
   - Remove workspace

10. **`integration_multiple_workspaces_isolation`**
    - Create multiple workspaces
    - Verify isolation (files don't leak)
    - Cleanup all workspaces

11. **`integration_workspace_state_persistence`**
    - Create commit in workspace
    - Switch away and back
    - Verify state persisted

### Workflow 5: Complex Multi-Aggregate Scenarios

12. **`integration_status_across_all_aggregates`**
    - Create multiple sessions
    - Add to queue
    - Get comprehensive status

13. **`integration_error_recovery_workflow`**
    - Test error handling (remove non-existent session)
    - Verify system still functional

14. **`integration_session_with_sync_workflow`**
    - Make change in workspace
    - Sync with main
    - Verify sync completes

15. **`integration_context_command_workflow`**
    - Get context before/after session creation
    - Verify context includes session info

16. **`integration_whereami_command_workflow`**
    - Check location before session (main)
    - Check location in session (workspace)
    - Check location after cleanup

17. **`integration_list_with_filters`**
    - Create multiple sessions
    - List all sessions

18. **`integration_diff_command_workflow`**
    - Make change in workspace
    - Get diff
    - Verify diff output

19. **`integration_done_workflow`**
    - Make change in workspace
    - Use done command to merge and cleanup
    - Verify workspace cleaned up

## Test Architecture

### Design Principles

- **Multiple state changes across aggregates**: Tests span sessions, workspaces, and queues
- **Database/file I/O interactions**: Real SQLite databases and file system operations
- **Full workflow scenarios**: Complete user journeys from start to finish
- **Uses test helpers**: Leverages `common::mod` for harness setup
- **Tests real integration behavior**: Actual persistence, not mocks
- **Independent tests**: Each test cleans up after itself

### Test Helpers

- `TestHarness::new()` - Creates isolated test environment with JJ repo
- `harness.zjj(&[args])` - Run zjj commands
- `harness.jj(&[args])` - Run JJ commands
- `harness.assert_success(&[args])` - Assert command succeeds
- `harness.workspace_path(session)` - Get workspace path
- `harness.assert_workspace_exists(session)` - Verify workspace exists
- `result.assert_stdout_contains(string)` - Assert output contains text

### Command Coverage

**Session Management:**
- `zjj init` - Initialize zjj in JJ repo
- `zjj add <name>` - Create session
- `zjj list` - List sessions
- `zjj remove <name> --merge` - Remove session
- `zjj switch <name>` - Switch sessions
- `zjj whoami` - Show current session

**Queue Operations:**
- `zjj queue enqueue <session>` - Add to queue
- `zjj queue dequeue <session>` - Remove from queue
- `zjj queue list` - List queue entries
- `zjj queue status` - Show queue status

**Workspace Operations:**
- `zjj sync` - Sync workspace with main
- `zjj diff` - Show diff
- `zjj done` - Complete work and merge

**Query Commands:**
- `zjj context` - Show environment context
- `zjj whereami` - Show current location
- `zjj status` - Show detailed status

## Running the Tests

```bash
# Run all integration tests
cargo test -p zjj --test integration_tests

# Run specific test
cargo test -p zjj --test integration_tests integration_session_workspace_close_workflow

# Run with output
cargo test -p zjj --test integration_tests -- --nocapture --test-threads=1
```

## Current Status

**Compilation Blocked:** The integration tests cannot currently run due to pre-existing compilation errors in the `zjj-core` library, specifically in the domain aggregates module:

- Duplicate method definitions (`is_active`, `is_blocked`, `is_closed`)
- Private enum imports
- Type trait bound issues with `AgentId: Copy`

These errors are unrelated to the integration test implementation and exist in the codebase's domain layer refactoring.

## File Locations

- **Integration Tests**: `/home/lewis/src/zjj/crates/zjj/tests/integration_tests.rs` (691 lines)
- **Test Helpers**: `/home/lewis/src/zjj/crates/zjj/tests/common/mod.rs`

## Next Steps

Once the core library compilation issues are resolved:

1. Run the integration tests to verify they pass
2. Add any necessary test fixtures or mock data
3. Extend tests to cover edge cases and error scenarios
4. Add performance benchmarks for critical workflows
5. Consider adding tests for task/bead operations once CLI commands are available

## Implementation Notes

The integration tests follow the functional Rust patterns required by the project:

- **No unwraps in production code** (tests use `unwrap()` for assertions)
- **Result types** for error handling
- **Domain types** from `zjj-core` (SessionName, WorkspaceName, etc.)
- **Repository pattern** for data access
- **Pure functions** in core, I/O in shell

Each test is designed to be independent and clean up after itself, ensuring reliable CI/CD execution.
