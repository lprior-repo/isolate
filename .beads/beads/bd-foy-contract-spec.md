# Contract Specification: bd-foy

## Replace add, remove, sync, focus Commands with JSONL Output

**Bead ID**: bd-foy
**Created**: 2026-02-21
**Status**: Specification Phase

---

## Context

### Feature
Convert CLI commands (`add`, `remove`, `sync`, `focus`) to emit AI-first JSONL output using the `OutputLine` enum and `emit_stdout()` function from `zjj_core::output`.

### Domain Terms
- **JSONL**: JSON Lines format - one JSON object per line, each line independently parseable
- **OutputLine**: Enum in `zjj_core::output::types` wrapping all output variants
- **emit_stdout()**: Function that serializes and writes an `OutputLine` to stdout
- **Session**: A zjj workspace with associated metadata (name, path, status, state)
- **Idempotent**: Operation that succeeds even if resource already exists/absent

### Reference Implementation
- `list.rs` - Already converted, emits `SessionOutput` per session, then `Summary`

### Assumptions
1. Commands should emit **only** JSONL output (no mixed text/JSON)
2. Each JSONL line is self-describing with a `type` field
3. Error cases emit `Issue` or `ResultOutput::failure` variants
4. Success cases emit appropriate `SessionOutput`, `Action`, or `ResultOutput::success` variants
5. The `SchemaEnvelope` pattern is deprecated in favor of direct JSONL emission

### Open Questions
1. Should `add` emit intermediate `Action` lines during creation, or only final `SessionOutput`?
   - **Resolution**: Emit `Action` lines for each step, then `SessionOutput` on success
2. Should commands emit a final `ResultOutput` even on success?
   - **Resolution**: Yes, for consistency - `ResultOutput::success` as the final line

---

## Preconditions

### add.rs Preconditions

| ID | Condition | Error if Violated |
|----|-----------|-------------------|
| ADD-PRE-01 | Session name is valid (ASCII, starts with letter, no spaces) | `Issue` with `Validation` kind |
| ADD-PRE-02 | zjj is initialized (`.zjj` directory exists) | `Issue` with `Configuration` kind |
| ADD-PRE-03 | Session does not already exist (unless `--idempotent`) | `Issue` with `StateConflict` kind |
| ADD-PRE-04 | Parent directory is writable | `Issue` with `PermissionDenied` kind |
| ADD-PRE-05 | JJ repository exists | `Issue` with `Configuration` kind |

### remove.rs Preconditions

| ID | Condition | Error if Violated |
|----|-----------|-------------------|
| RM-PRE-01 | Session exists (unless `--idempotent`) | `Issue` with `ResourceNotFound` kind |
| RM-PRE-02 | Session is not locked by another process | `Issue` with `StateConflict` kind |
| RM-PRE-03 | Workspace directory is accessible (or already gone) | `Issue` with `PermissionDenied` kind |

### sync.rs Preconditions

| ID | Condition | Error if Violated |
|----|-----------|-------------------|
| SYNC-PRE-01 | Session exists (for named sync) | `Issue` with `ResourceNotFound` kind |
| SYNC-PRE-02 | At least one session exists (for `--all`) | `Summary` with 0 count (not error) |
| SYNC-PRE-03 | In a JJ repository | `Issue` with `Configuration` kind |
| SYNC-PRE-04 | Main branch is determinable | `Issue` with `Configuration` kind |

### focus.rs Preconditions

| ID | Condition | Error if Violated |
|----|-----------|-------------------|
| FOC-PRE-01 | Session name is provided | `Issue` with `Validation` kind |
| FOC-PRE-02 | Session exists | `Issue` with `ResourceNotFound` kind |
| FOC-PRE-03 | Zellij is available (for tab switching) | Warning + continue with info only |

---

## Postconditions

### add.rs Postconditions

**Success Path:**
1. Emits `Action` lines for each creation step (workspace, db record, zellij tab)
2. Emits `SessionOutput` line with full session details
3. Emits `ResultOutput::success` with kind `Command`
4. Session exists in database with status `Active`
5. Workspace directory exists at specified path
6. Zellij tab created (if `--no-zellij` not set)

**Failure Path:**
1. Emits `Action` lines for steps attempted (with `Failed` status)
2. Emits `Issue` line with appropriate kind/severity
3. Emits `ResultOutput::failure` with kind `Command`
4. No orphaned resources (atomic rollback if partial state)

### remove.rs Postconditions

**Success Path:**
1. Emits `Action` lines for cleanup steps (workspace, db record, zellij tab)
2. Emits `ResultOutput::success` with kind `Command`
3. Session removed from database
4. Workspace directory removed
5. Zellij tab closed (if applicable)

**Idempotent Path (session not found):**
1. Emits `ResultOutput::success` with message "already removed"
2. No error emitted

**Failure Path:**
1. Emits `Action` lines for steps attempted (with `Failed` status)
2. Emits `Issue` line with appropriate kind/severity
3. Emits `ResultOutput::failure` with kind `Command`

### sync.rs Postconditions

**Single Session Success:**
1. Emits `Action` line for rebase operation with `Completed` status
2. Emits `ResultOutput::success` with kind `Operation`
3. `last_synced` timestamp updated in database

**All Sessions Success:**
1. Emits `Action` line per session synced
2. Emits `Summary` with counts
3. Emits `ResultOutput::success` with counts in data field

**Partial Failure (--all):**
1. Emits `Action` lines (successes with `Completed`, failures with `Failed`)
2. Emits `Issue` lines for failed sessions
3. Emits `ResultOutput::failure` with partial counts

**Complete Failure:**
1. Emits `Action` lines with `Failed` status
2. Emits `Issue` line
3. Emits `ResultOutput::failure`

### focus.rs Postconditions

**Inside Zellij:**
1. Emits `SessionOutput` line
2. Emits `ResultOutput::success`
3. Zellij tab switched

**Outside Zellij (--no-zellij):**
1. Emits `SessionOutput` line
2. Emits `ResultOutput::success` with message
3. No Zellij interaction

**Outside Zellij (with Zellij):**
1. Emits `SessionOutput` line
2. Emits `ResultOutput::success` with "attaching" message
3. Process execs into Zellij (may not return)

---

## Invariants

### Global Invariants (apply to ALL commands)

| ID | Invariant |
|----|-----------|
| INV-GLOBAL-01 | Every emitted line is valid, parseable JSONL |
| INV-GLOBAL-02 | Every `OutputLine` has a `type` field indicating its variant |
| INV-GLOBAL-03 | Error cases emit `Issue` OR `ResultOutput::failure`, never both for same error |
| INV-GLOBAL-04 | Success cases emit `ResultOutput::success` as final line |
| INV-GLOBAL-05 | No `unwrap()`, `expect()`, or `panic!()` in output path |
| INV-GLOBAL-06 | All fallible operations return `Result<T, E>` |
| INV-GLOBAL-07 | stdout is flushed after each line |

### Command-Specific Invariants

| ID | Command | Invariant |
|----|---------|-----------|
| INV-ADD-01 | add | Session name in output matches input name |
| INV-ADD-02 | add | Workspace path is absolute |
| INV-RM-01 | remove | `--idempotent` never returns error for missing session |
| INV-SYNC-01 | sync | `synced_count + failed_count == total_sessions` |
| INV-SYNC-02 | sync | Timestamps are ISO 8601 or epoch milliseconds |
| INV-FOC-01 | focus | Zellij tab name is always `zjj:{session_name}` |

---

## Error Taxonomy

### Domain Error Types

```rust
/// Errors that can occur during command execution
#[derive(Debug, Error)]
pub enum CommandError {
    // Input Validation (exit code 1)
    #[error("invalid {field}: {reason}")]
    InvalidInput {
        field: String,
        reason: String,
        value: Option<String>,
    },

    #[error("missing required argument: {arg}")]
    MissingArgument { arg: String },

    // Resource Not Found (exit code 2)
    #[error("{resource} not found: {id}")]
    NotFound {
        resource: String,
        id: String,
    },

    // State Conflicts (exit code 1)
    #[error("cannot {action}: {reason}")]
    StateConflict {
        action: String,
        reason: String,
        current_state: String,
    },

    #[error("session '{name}' already exists")]
    SessionExists { name: String },

    #[error("session '{name}' is locked by {owner}")]
    SessionLocked { name: String, owner: String },

    // IO/Permission Errors (exit code 3)
    #[error("permission denied: {resource}")]
    PermissionDenied { resource: String },

    #[error("io error on {operation}: {source}")]
    IoError {
        operation: String,
        #[source]
        source: std::io::Error,
    },

    // External Service Errors (exit code 3)
    #[error("external command failed: {command}")]
    ExternalCommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("zellij error: {reason}")]
    ZellijError { reason: String },

    // Configuration Errors (exit code 1)
    #[error("not initialized: {reason}")]
    NotInitialized { reason: String },

    #[error("invalid configuration: {reason}")]
    InvalidConfig { reason: String },
}
```

### Error to OutputLine Mapping

| Error Variant | OutputLine Variant | Issue Kind | Issue Severity |
|--------------|-------------------|------------|----------------|
| `InvalidInput` | `Issue` | `Validation` | `Error` |
| `MissingArgument` | `Issue` | `Validation` | `Error` |
| `NotFound` | `Issue` | `ResourceNotFound` | `Error` |
| `StateConflict` | `Issue` | `StateConflict` | `Error` |
| `SessionExists` | `Issue` | `StateConflict` | `Warning` (if idempotent) |
| `SessionLocked` | `Issue` | `StateConflict` | `Error` |
| `PermissionDenied` | `Issue` | `PermissionDenied` | `Error` |
| `IoError` | `Issue` | `External` | `Error` |
| `ExternalCommandFailed` | `Issue` | `External` | `Error` |
| `ZellijError` | `Issue` | `External` | `Warning` |
| `NotInitialized` | `Issue` | `Configuration` | `Error` |
| `InvalidConfig` | `Issue` | `Configuration` | `Error` |

---

## Contract Signatures

### add.rs

```rust
/// Create a new session and emit JSONL output
///
/// # Output Contract
///
/// Emits in order:
/// 1. Zero or more `Action` lines for creation steps
/// 2. `SessionOutput` line on success
/// 3. `ResultOutput` line (success or failure)
///
/// # Errors
///
/// Returns `CommandError` if:
/// - Name validation fails
/// - Session already exists (and not idempotent)
/// - Workspace creation fails
/// - Database write fails
pub async fn run_with_options(options: &AddOptions) -> Result<(), CommandError>;

/// Convert internal session to OutputLine::Session
fn session_to_output(session: &Session) -> Result<OutputLine, OutputLineError>;

/// Convert error to OutputLine::Issue
fn error_to_issue(error: &CommandError, session_name: Option<&str>) -> Result<OutputLine, OutputLineError>;
```

### remove.rs

```rust
/// Remove a session and emit JSONL output
///
/// # Output Contract
///
/// Emits in order:
/// 1. Zero or more `Action` lines for cleanup steps
/// 2. `ResultOutput` line (success or failure)
///
/// # Idempotent Behavior
///
/// When `--idempotent` and session not found:
/// - Emits `ResultOutput::success` with message
/// - Does NOT emit `Issue` line
pub async fn run_with_options(name: &str, options: &RemoveOptions) -> Result<(), CommandError>;
```

### sync.rs

```rust
/// Sync session(s) with main and emit JSONL output
///
/// # Output Contract
///
/// Single session:
/// 1. `Action` line for rebase
/// 2. `ResultOutput` line
///
/// All sessions:
/// 1. `Action` line per session
/// 2. `Summary` line with counts
/// 3. `ResultOutput` line
///
/// # Errors
///
/// Returns error only if ALL sessions fail.
/// Partial failures are reported in output, not as error return.
pub async fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<(), CommandError>;
```

### focus.rs

```rust
/// Focus a session and emit JSONL output
///
/// # Output Contract
///
/// Emits in order:
/// 1. `SessionOutput` line
/// 2. `ResultOutput` line
///
/// # Note
///
/// May exec into Zellij and not return.
pub async fn run_with_options(name: Option<&str>, options: &FocusOptions) -> Result<(), CommandError>;
```

---

## OutputLine Variant Selection

| Command | Success Primary | Success Final | Failure Primary | Failure Final |
|---------|-----------------|---------------|-----------------|---------------|
| add | `SessionOutput` | `ResultOutput::success` | `Issue` | `ResultOutput::failure` |
| remove | N/A | `ResultOutput::success` | `Issue` | `ResultOutput::failure` |
| sync (single) | `Action` | `ResultOutput::success` | `Issue` | `ResultOutput::failure` |
| sync (all) | `Action` + `Summary` | `ResultOutput::success` | `Issue` | `ResultOutput::failure` |
| focus | `SessionOutput` | `ResultOutput::success` | `Issue` | `ResultOutput::failure` |

---

## Non-goals

1. **NOT** changing command-line argument parsing
2. **NOT** modifying core business logic (only output path)
3. **NOT** adding new features (only output format change)
4. **NOT** changing exit codes (already defined per error type)
5. **NOT** removing human-readable output entirely (future: `--format human` option)

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/zjj/src/commands/add.rs` | Replace `output_result()` with `emit_stdout()` calls |
| `crates/zjj/src/commands/add/output.rs` | Convert to emit `OutputLine` variants |
| `crates/zjj/src/commands/remove.rs` | Replace JSON envelope with `emit_stdout()` |
| `crates/zjj/src/commands/sync.rs` | Replace JSON envelope with `emit_stdout()` |
| `crates/zjj/src/commands/focus.rs` | Replace JSON envelope with `emit_stdout()` |
| `crates/zjj-core/src/output/types.rs` | Add any missing fields (if needed) |

---

## Implementation Order

1. **Phase 1**: Add helper functions to convert errors to `Issue` lines
2. **Phase 2**: Convert `focus.rs` (simplest, good pattern)
3. **Phase 3**: Convert `remove.rs` (medium complexity)
4. **Phase 4**: Convert `sync.rs` (complex, multiple paths)
5. **Phase 5**: Convert `add.rs` (most complex, many steps)
