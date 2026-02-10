# Contract Specification: `--idempotent` Flag Verification

## Context
- **Feature**: `--idempotent` flag for `add`, `work`, and `remove` commands
- **Bead**: zjj-ftds - "add: Implement or remove --idempotent flag"
- **Domain Terms**:
  - *Idempotent operation*: An operation that can be applied multiple times with the same result
  - *Safe retry*: Operation that succeeds whether or not the resource already exists
- **Assumptions**:
  - The flag IS currently implemented in code (lines 133-149 of add.rs, lines 83-86/108-115 of work.rs)
  - The flag IS registered in CLI (commands.rs lines 143-146, 1603-1607, 532-536)
  - The issue is that **implementation exists but is not verified/tested**
  - Tests exist (drq_adversarial.rs) but are incomplete - don't actually test the flag
- **Open Questions**:
  - Should the flag return different exit codes or output when idempotent path is taken?
  - Should JSON output include an "idempotent" field to indicate the path taken?
  - Is the flag needed for `remove` command if `-f` (force) already exists?

## Decision Analysis

### Current State
- ✅ Flag defined in CLI for `add`, `work`, `remove`
- ✅ Handler reads flag value
- ✅ Logic exists in `add::run_with_options()` and `work::run()`
- ❌ No tests verify the flag actually works
- ❌ Help text mentions flag but no examples demonstrate it
- ❌ DRQ tests marked as "designed to fail" but don't test flag

### Root Cause
The bead description says "doesn't work" but actual investigation reveals:
1. **Implementation EXISTS** - code is there and looks correct
2. **Tests EXIST** - but are incomplete (test `test_add_is_not_idempotent` doesn't actually use `--idempotent` flag)
3. **Documentation EXISTS** - help text and examples mention it

### Recommended Resolution
**Option A: Verify and Document** (RECOMMENDED)
- Add comprehensive tests proving the flag works
- Update help text with idempotent-specific examples
- Add JSON output field indicating idempotent path
- Effort: 30min

**Option B: Remove Flag**
- Remove from CLI, handlers, and options structs
- Remove from help text and documentation
- Justification: `-f` on remove is already idempotent, users can check existence before add
- Effort: 15min

**Option C: Full Implementation**
- Add JSON schema field "already_exists": true
- Add human-readable message "Session already exists (idempotent mode)"
- Ensure consistent behavior across all three commands
- Effort: 45min

**Decision**: This contract recommends **Option A** - the flag is implemented and appears correct, but needs verification through tests.

## Preconditions

### For `add --idempotent <name>`:
- [x] JJ repository must be initialized (`.jj` directory exists)
- [x] ZJJ must be initialized (`.zjj/state.db` exists)
- [x] Session name must pass validation (alphanumeric, starts with letter)
- [x] If session exists, it must be in `active` or `creating` state
- [x] Workspace directory must exist if session record exists

### For `work --idempotent <name>`:
- [x] All `add` preconditions
- [x] If already in a workspace, must be the target workspace name
- [x] Agent registration must not conflict (if agent-id provided)

### For `remove --idempotent <name>`:
- [x] JJ repository must be initialized
- [x] ZJJ must be initialized
- [x] If session doesn't exist, succeed anyway (definition of idempotent remove)

## Postconditions

### For `add --idempotent <name>` when session EXISTS:
- [x] Exit code: 0 (success)
- [x] No changes to session database
- [x] No changes to workspace directory
- [x] No changes to Zellij tabs
- [x] Output indicates "already exists" or similar
- [x] JSON output includes `status: "already exists (idempotent)"` or similar

### For `add --idempotent <name>` when session DOESN'T EXIST:
- [x] Exit code: 0 (success)
- [x] Session created in database
- [x] Workspace directory created
- [x] Zellij tab created (unless `--no-open` or `--no-zellij`)
- [x] Output indicates "created" or similar

### For `work --idempotent <name>` when already in target workspace:
- [x] Exit code: 0 (success)
- [x] No new workspace created
- [x] Output includes workspace path and environment variables
- [x] JSON output includes `created: false`

### For `remove --idempotent <name>` when session doesn't exist:
- [x] Exit code: 0 (success)
- [x] No error raised
- [x] Output indicates "already removed" or similar

## Invariants

- [x] Idempotent operations NEVER modify existing state
- [x] Idempotent operations ALWAYS return exit code 0
- [x] Idempotent operations are safe to retry indefinitely
- [x] Non-idempotent operations fail fast on conflict (exit code 1)
- [x] Session database is source of truth for existence
- [x] Workspace directory existence matches session database state

## Error Taxonomy

### Command Errors (apply to all idempotent operations)

**Error::InvalidSessionName**
- When: Session name fails validation (empty, non-ASCII, starts with number, contains spaces)
- Exit code: 1
- Idempotent behavior: Same as non-idempotent (validation is pre-condition)

**Error::NotInitialized**
- When: ZJJ not initialized (no `.zjj/state.db`)
- Exit code: 1
- Idempotent behavior: Same as non-idempotent (initialization is pre-condition)

**Error::NotInJJRepo**
- When: Not in a JJ repository
- Exit code: 1
- Idempotent behavior: Same as non-idempotent (JJ repo is pre-condition)

### Add-Specific Errors

**Error::SessionCreationFailed**
- When: Atomic session creation fails (race condition, DB error)
- Exit code: 1
- Idempotent behavior: NOT applicable - creation is internal detail

**Error::HookExecutionFailed**
- When: Post-create hook fails
- Exit code: 1
- Idempotent behavior: If session exists and `--idempotent`, skip hooks entirely

**Error::ZellijAttachFailed**
- When: Cannot attach to or create Zellij session
- Exit code: 1
- Idempotent behavior: If session exists and `--idempotent`, may skip Zellij operations

### Work-Specific Errors

**Error::AlreadyInDifferentWorkspace**
- When: Already in a workspace, but not the target workspace
- Exit code: 1
- Idempotent behavior: Fail unless target workspace matches current

**Error::AgentRegistrationFailed**
- When: Cannot register agent
- Exit code: 1
- Idempotent behavior: If session exists and `--idempotent`, agent re-registration should succeed

### Remove-Specific Errors

**Error::SessionRemovalFailed**
- When: Cannot remove session (DB error, workspace deletion failed)
- Exit code: 1
- Idempotent behavior: If session doesn't exist and `--idempotent`, succeed

**Error::WorkspaceCleanupFailed**
- When: Cannot delete workspace directory
- Exit code: 1 (but session removed from DB)
- Idempotent behavior: If workspace doesn't exist and `--idempotent`, succeed

## Contract Signatures

```rust
// add command
pub async fn run_with_options(options: &AddOptions) -> Result<()>
where
    AddOptions {
        pub idempotent: bool,  // Controls behavior on existing session
        // ... other fields
    }

// work command
pub async fn run(options: &WorkOptions) -> Result<()>
where
    WorkOptions {
        pub idempotent: bool,  // Controls behavior on existing session
        // ... other fields
    }

// remove command (hypothetical - need to verify)
pub async fn run_with_options(options: &RemoveOptions) -> Result<()>
where
    RemoveOptions {
        pub idempotent: bool,  // Controls behavior on non-existent session
        // ... other fields
    }
```

### Expected JSON Output Structure

When `--idempotent --json` is used:

```json
{
  "schema": "add-response",
  "type": "single",
  "data": {
    "name": "session-name",
    "workspace_path": "/path/to/workspace",
    "zellij_tab": "zjj:session-name",
    "status": "already exists (idempotent)",  // OR "active"
    "idempotent": true,  // NEW FIELD - indicates idempotent path taken
    "created": false  // NEW FIELD - false when idempotent path
  }
}
```

For `work` command (already has `created` field):
```json
{
  "schema": "work-response",
  "type": "single",
  "data": {
    "name": "session-name",
    "workspace_path": "/path/to/workspace",
    "zellij_tab": "zjj:session-name",
    "created": false,  // false when idempotent path
    "agent_id": "agent-123",
    "env_vars": [...],
    "enter_command": "cd /path/to/workspace"
  }
}
```

## Non-goals

- [ ] Don't add `--idempotent` to commands that don't create resources (e.g., `list`, `status`)
- [ ] Don't change behavior of `-f` (force) flag on `remove` command
- [ ] Don't add idempotent mode to `init` (already idempotent by design)
- [ ] Don't implement transactional operations (beyond current atomic session creation)
- [ ] Don't add rollback mechanisms for failed idempotent operations
- [ ] Don't change exit codes for non-idempotent errors

## Acceptance Criteria

1. **Test Coverage**:
   - [ ] Test `add --idempotent` succeeds when session exists
   - [ ] Test `add --idempotent` creates session when doesn't exist
   - [ ] Test `add --idempotent` returns appropriate JSON output
   - [ ] Test `work --idempotent` succeeds when already in target workspace
   - [ ] Test `work --idempotent` creates workspace when doesn't exist
   - [ ] Test `remove --idempotent` succeeds when session doesn't exist
   - [ ] Test DRQ adversarial tests pass with flag

2. **Documentation**:
   - [ ] Help text includes idempotent examples
   - [ ] Examples demonstrate retry scenarios
   - [ ] JSON schema documented for idempotent responses

3. **Backward Compatibility**:
   - [ ] Non-idempotent behavior unchanged
   - [ ] Default behavior (flag omitted) unchanged
   - [ ] Exit codes consistent with existing errors

4. **Edge Cases**:
   - [ ] Session exists but in `failed` state
   - [ ] Session exists but workspace directory missing
   - [ ] Concurrent `add --idempotent` calls (race condition)
   - [ ] `add --idempotent --dry-run` combination
