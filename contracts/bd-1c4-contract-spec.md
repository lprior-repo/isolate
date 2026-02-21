# Contract Specification: Conflict Analysis and Resolution (bd-1c4)

**Bead ID:** bd-1c4
**Title:** Create end-to-end tests for conflict analysis and resolution
**Version:** 1.0.0
**Status:** Draft

---

## 1. Overview

This document specifies the Design by Contract requirements for the conflict analysis and resolution system in zjj. The system provides pre-merge conflict detection for the `done` command workflow, with security implications for multi-agent coordination.

### 1.1 Scope

The contract covers:
- Conflict detection before workspace merge to main
- Existing JJ conflict identification
- Potential conflict detection via file overlap analysis
- Merge base calculation
- Resolution workflow integration
- Security-sensitive operations involving workspace state

### 1.2 Security Context

This system has security implications because:
1. **Data Integrity:** Incorrect conflict detection can lead to lost work or corrupted merge history
2. **Concurrency Safety:** Multiple agents may attempt simultaneous merges
3. **State Consistency:** Conflict state must accurately reflect repository reality
4. **Authorization:** Only workspace owners should be able to resolve conflicts in their workspace
5. **Audit Trail:** Conflict resolution operations must be traceable

---

## 2. Type Definitions

### 2.1 ConflictError (Exhaustive Error Taxonomy)

```rust
/// Semantic error variants for conflict detection and resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictError {
    // === Execution Errors ===
    /// JJ command failed to execute
    JjExecutionFailed {
        command: String,
        source: String,
    },

    /// JJ command returned non-zero exit code
    JjCommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    /// JJ binary not found in PATH
    JjNotFound {
        searched_paths: Vec<String>,
    },

    // === Workspace State Errors ===
    /// Workspace is not in a valid state for conflict detection
    InvalidWorkspaceState {
        workspace: String,
        current_state: WorkspaceState,
        expected_states: Vec<WorkspaceState>,
    },

    /// Workspace does not exist
    WorkspaceNotFound {
        workspace: String,
    },

    /// Not currently in a workspace
    NotInWorkspace {
        current_location: String,
    },

    // === Merge Base Errors ===
    /// Failed to find common ancestor between workspace and trunk
    MergeBaseNotFound {
        workspace: String,
        trunk: String,
    },

    /// Merge base calculation timed out
    MergeBaseTimeout {
        workspace: String,
        timeout_ms: u64,
    },

    // === Diff Analysis Errors ===
    /// Failed to parse diff output
    DiffParseError {
        output: String,
        parse_error: String,
    },

    /// Diff operation failed
    DiffFailed {
        from_ref: String,
        to_ref: String,
        reason: String,
    },

    // === Status Check Errors ===
    /// Failed to check workspace status
    StatusCheckFailed {
        workspace: String,
        reason: String,
    },

    /// Failed to list conflicts via jj resolve
    ConflictListFailed {
        reason: String,
    },

    // === Resolution Errors ===
    /// Conflict resolution failed
    ResolutionFailed {
        file: String,
        reason: String,
    },

    /// Cannot auto-resolve this conflict type
    ConflictNotAutoResolvable {
        file: String,
        conflict_type: ConflictType,
    },

    // === Concurrency Errors ===
    /// Workspace is locked by another agent
    WorkspaceLocked {
        workspace: String,
        holder: String,
        expires_at: DateTime<Utc>,
    },

    /// Conflict detected during merge operation
    MergeConflictDetected {
        workspace: String,
        conflicting_files: Vec<String>,
    },

    // === Validation Errors ===
    /// Invalid file path in conflict report
    InvalidFilePath {
        path: String,
        reason: String,
    },

    /// Output validation failed
    OutputValidationFailed {
        field: String,
        expected: String,
        actual: String,
    },

    // === IO Errors ===
    /// File system error during analysis
    IoError {
        operation: String,
        path: String,
        source: String,
    },
}
```

### 2.2 ConflictDetectionResult

```rust
/// Comprehensive result of conflict detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetectionResult {
    /// Whether there are existing JJ conflicts in the workspace
    pub has_existing_conflicts: bool,

    /// List of files with existing JJ conflicts
    pub existing_conflicts: Vec<ConflictInfo>,

    /// Files modified in both workspace and trunk (potential conflicts)
    pub overlapping_files: Vec<FileOverlapInfo>,

    /// Files modified only in workspace
    pub workspace_only: Vec<String>,

    /// Files modified only in trunk/main
    pub main_only: Vec<String>,

    /// Whether the merge is likely to succeed without conflicts
    pub merge_likely_safe: bool,

    /// Human-readable summary of the detection result
    pub summary: String,

    /// The merge base commit (common ancestor)
    pub merge_base: Option<String>,

    /// Total number of files analyzed
    pub files_analyzed: usize,

    /// Time taken for detection in milliseconds
    pub detection_time_ms: u64,

    /// Workspace name being analyzed
    pub workspace_name: String,

    /// Trunk reference used for comparison
    pub trunk_ref: String,

    /// Timestamp of analysis
    pub analyzed_at: DateTime<Utc>,
}

/// Detailed conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// File path with conflict
    pub path: String,

    /// Type of conflict
    pub conflict_type: ConflictType,

    /// Number of sides in conflict
    pub sides: usize,

    /// Whether auto-resolution is possible
    pub auto_resolvable: bool,
}

/// Type of conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Content conflict in file
    Content,
    /// File deleted in one side, modified in other
    DeleteModify,
    /// File renamed differently in both sides
    RenameRename,
    /// Directory/file type conflict
    TypeChange,
    /// Unknown conflict type
    Unknown,
}

/// File overlap information for potential conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOverlapInfo {
    /// File path
    pub path: String,

    /// Type of change in workspace
    pub workspace_change: FileChange,

    /// Type of change in trunk
    pub trunk_change: FileChange,
}

/// Type of file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChange {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
}
```

### 2.3 Function Signatures

```rust
/// Main conflict detection entry point
///
/// # Preconditions
/// - Must be in a valid JJ repository
/// - Workspace must exist and be in a valid state
/// - JJ must be available in PATH
///
/// # Postconditions
/// - Returns comprehensive conflict analysis
/// - detection_time_ms accurately reflects analysis duration
/// - All file paths are normalized and valid
pub async fn detect_conflicts(
    workspace: &str,
    options: ConflictDetectionOptions,
) -> Result<ConflictDetectionResult, ConflictError>;

/// Quick check for existing conflicts only (optimized for speed)
///
/// # Preconditions
/// - Must be in a valid JJ repository
///
/// # Postconditions
/// - Returns true if any conflicts exist
/// - Completes within 100ms for typical repositories
pub async fn has_existing_conflicts(
    workspace: &str,
) -> Result<bool, ConflictError>;

/// Find merge base between workspace and trunk
///
/// # Preconditions
/// - Both workspace and trunk refs must exist
/// - Must have at least one common ancestor
///
/// # Postconditions
/// - Returns the most recent common ancestor
/// - Returns None if no common ancestor exists
pub async fn find_merge_base(
    workspace: &str,
    trunk: &str,
) -> Result<Option<String>, ConflictError>;

/// Get files modified since merge base
///
/// # Preconditions
/// - Merge base must be valid commit
/// - Target ref must exist
///
/// # Postconditions
/// - Returns all modified files
/// - Paths are relative to repository root
pub async fn get_modified_files(
    from_ref: &str,
    to_ref: &str,
) -> Result<HashSet<String>, ConflictError>;

/// Check if merge is safe to proceed
///
/// # Preconditions
/// - ConflictDetectionResult must be valid
///
/// # Postconditions
/// - Returns true only if no conflicts detected
/// - Returns false if any existing or potential conflicts
pub fn is_merge_safe(result: &ConflictDetectionResult) -> bool;

/// Queue workspace for conflict resolution
///
/// # Preconditions
/// - Workspace must have unresolved conflicts
/// - Caller must hold workspace lock
///
/// # Postconditions
/// - Workspace is added to merge queue
/// - Priority is set appropriately
pub async fn queue_for_resolution(
    workspace: &str,
    bead_id: Option<&str>,
    agent_id: &str,
) -> Result<(), ConflictError>;
```

---

## 3. Preconditions

### 3.1 Repository State Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-REPO-001 | Must be in a JJ repository (`.jj` directory exists) | Check via `jj root` |
| PRE-REPO-002 | Repository must be in a healthy state | Check via `jj status` |
| PRE-REPO-003 | No concurrent operations modifying workspace | Lock check |

### 3.2 Workspace State Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-WS-001 | Workspace must exist in workspace list | Check via `jj workspace list` |
| PRE-WS-002 | Workspace must not be abandoned | Check workspace state |
| PRE-WS-003 | Workspace must not be stale | Check via `jj workspace update-stale` |
| PRE-WS-004 | Caller must hold workspace lock (if multi-agent) | Lock verification |

### 3.3 JJ Binary Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-JJ-001 | JJ binary must exist in PATH | Check file existence |
| PRE-JJ-002 | JJ version must be compatible | Check `jj version` |
| PRE-JJ-003 | JJ must be able to execute commands | Test run |

### 3.4 Reference Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-REF-001 | Trunk reference must exist | Check via `jj log` |
| PRE-REF-002 | Workspace ref must exist | Check via `jj log` |
| PRE-REF-003 | Common ancestor must exist (or explicitly handle none) | Merge base check |

---

## 4. Postconditions

### 4.1 Detection Result Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-DET-001 | `has_existing_conflicts` accurately reflects JJ state | Cross-check with `jj resolve --list` |
| POST-DET-002 | `overlapping_files` contains all files modified in both branches | Cross-check with `jj diff` |
| POST-DET-003 | `merge_likely_safe` is true only when no conflicts exist | Logical verification |
| POST-DET-004 | `detection_time_ms` is within acceptable bounds (<5000ms) | Timing check |
| POST-DET-005 | All file paths are normalized (no `./` or `../`) | Path validation |
| POST-DET-006 | `files_analyzed` equals sum of all file lists | Count verification |

### 4.2 Merge Safety Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-SAFE-001 | If `merge_likely_safe` is true, merge will succeed | Probabilistic guarantee |
| POST-SAFE-002 | False negatives are impossible (all conflicts detected) | Exhaustive check |
| POST-SAFE-003 | False positives may occur (conservative approach) | Acceptable |

### 4.3 Exit Code Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-EXIT-001 | Exit code 0: No conflicts, merge is safe | Result check |
| POST-EXIT-002 | Exit code 1: Conflicts detected | Result check |
| POST-EXIT-003 | Exit code 3: Error during detection | Error check |

---

## 5. Invariants

### 5.1 Data Integrity Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-DATA-001 | `ConflictDetectionResult` is consistent: `has_conflicts() == has_existing_conflicts \|\| !overlapping_files.is_empty()` |
| INV-DATA-002 | File lists are mutually exclusive: no file appears in both `workspace_only` and `main_only` |
| INV-DATA-003 | `files_analyzed >= existing_conflicts.len() + overlapping_files.len() + workspace_only.len() + main_only.len()` |
| INV-DATA-004 | `merge_base` is a valid commit ID if present |
| INV-DATA-005 | All timestamps use UTC timezone |

### 5.2 Security Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-SEC-001 | Conflict detection never modifies repository state |
| INV-SEC-002 | Resolution operations require explicit user/agent confirmation |
| INV-SEC-003 | All conflict operations are logged to audit trail |
| INV-SEC-004 | Workspace lock must be held for any resolution operation |
| INV-SEC-005 | Conflict information is not leaked across workspace boundaries |

### 5.3 Concurrency Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-CONC-001 | Detection is atomic: either completes fully or fails cleanly |
| INV-CONC-002 | No partial state is left after failure |
| INV-CONC-003 | Concurrent detections on same workspace return consistent results |
| INV-CONC-004 | Lock timeout prevents indefinite blocking |

### 5.4 Performance Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-PERF-001 | Detection completes in <5 seconds for repositories with <10,000 files |
| INV-PERF-002 | `has_existing_conflicts()` completes in <100ms |
| INV-PERF-003 | Memory usage is bounded to O(n) where n is number of modified files |

---

## 6. Error Recovery

### 6.1 Recoverable Errors

| Error | Recovery Strategy |
|-------|-------------------|
| `WorkspaceLocked` | Wait and retry, or notify user |
| `MergeBaseTimeout` | Retry with extended timeout |
| `JjExecutionFailed` | Retry with exponential backoff |

### 6.2 Non-Recoverable Errors

| Error | Action |
|-------|--------|
| `JjNotFound` | Abort with installation instructions |
| `InvalidWorkspaceState` | Abort with state correction instructions |
| `WorkspaceNotFound` | Abort with workspace list |

---

## 7. Security Considerations

### 7.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| **Data Loss:** Incorrect conflict detection leading to lost work | Zero false negatives guarantee; conservative approach |
| **Unauthorized Resolution:** Agent resolving conflicts in another's workspace | Lock verification; agent ID validation |
| **Race Conditions:** Concurrent merges causing conflicts | Sequential merge queue with locking |
| **State Corruption:** Partial detection leaving inconsistent state | Atomic operations; rollback on failure |
| **Information Disclosure:** Conflict info visible to unauthorized agents | Workspace isolation; audit logging |

### 7.2 Security Requirements

1. **SR-001:** All conflict resolution operations must be authenticated with agent ID
2. **SR-002:** Audit log must capture: timestamp, agent, workspace, operation, outcome
3. **SR-003:** Workspace locks must have configurable TTL (default 5 minutes)
4. **SR-004:** Conflict reports must not include content of conflicting files
5. **SR-005:** Failed resolution attempts must be logged with full context

---

## 8. Test Coverage Requirements

### 8.1 Contract Verification Tests

Every precondition, postcondition, and invariant must have at least one test:

- All PRE-* conditions must have positive tests (condition met) and negative tests (condition violated)
- All POST-* conditions must have verification tests
- All INV-* invariants must have property-based tests

### 8.2 Error Path Coverage

All `ConflictError` variants must be:

1. Constructible in tests
2. Displayable with meaningful message
3. Serializable to JSON
4. Deserializable from JSON
5. Recoverable where applicable

---

## 9. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-02-18 | Initial contract specification |

---

## 10. References

- `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs` - Implementation
- `/home/lewis/src/zjj/crates/zjj/src/commands/done/types.rs` - Type definitions
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/locks.rs` - Lock management
- `/home/lewis/src/zjj/crates/zjj-core/src/coordination/queue.rs` - Merge queue
