# Martin Fowler Test Plan: Conflict Analysis and Resolution (bd-1c4)

**Bead ID:** bd-1c4
**Title:** Create end-to-end tests for conflict analysis and resolution
**Test Framework:** BDD with Given-When-Then scenarios
**Version:** 1.0.0

---

## 1. Overview

This test plan follows Martin Fowler's BDD approach with Given-When-Then scenarios for end-to-end testing of the conflict analysis and resolution system. Tests are organized by category: Happy Path, Error Path, Edge Cases, Contract Verification, and End-to-End Scenarios.

### 1.1 Test Categories

| Category | Purpose | Count |
|----------|---------|-------|
| Happy Path | Normal successful operations | 15 |
| Error Path | Failure handling and recovery | 18 |
| Edge Cases | Boundary conditions | 12 |
| Contract Verification | Pre/post/invariant validation | 20 |
| End-to-End | Full workflow scenarios | 10 |
| **Total** | | **75** |

---

## 2. Happy Path Tests

### 2.1 HP-001: Clean Workspace Merge Detection

**Given** a workspace with no conflicts
**And** the workspace has commits not in trunk
**And** trunk has no commits since workspace was created
**When** conflict detection is run
**Then** `has_existing_conflicts` is false
**And** `overlapping_files` is empty
**And** `merge_likely_safe` is true
**And** exit code is 0

```rust
#[tokio::test]
async fn hp_001_clean_workspace_merge_detection() {
    // Arrange: Create clean workspace with unique commits
    // Act: Run conflict detection
    // Assert: merge_likely_safe == true, no conflicts
}
```

---

### 2.2 HP-002: Detect Single Existing Conflict

**Given** a workspace with one file containing a JJ conflict marker
**When** conflict detection is run
**Then** `has_existing_conflicts` is true
**And** `existing_conflicts` contains exactly one entry
**And** `overlapping_files` may also contain entries
**And** `merge_likely_safe` is false
**And** exit code is 1

```rust
#[tokio::test]
async fn hp_002_detect_single_existing_conflict() {
    // Arrange: Create workspace with conflicting file
    // Act: Run conflict detection
    // Assert: existing_conflicts.len() == 1
}
```

---

### 2.3 HP-003: Detect Multiple Existing Conflicts

**Given** a workspace with three files containing JJ conflict markers
**When** conflict detection is run
**Then** `has_existing_conflicts` is true
**And** `existing_conflicts` contains exactly three entries
**And** each conflict has correct file path

```rust
#[tokio::test]
async fn hp_003_detect_multiple_existing_conflicts() {
    // Arrange: Create workspace with 3 conflicting files
    // Act: Run conflict detection
    // Assert: existing_conflicts.len() == 3
}
```

---

### 2.4 HP-004: Detect Overlapping File Modifications

**Given** a workspace that modified `src/lib.rs`
**And** trunk that also modified `src/lib.rs`
**And** no existing conflicts in workspace
**When** conflict detection is run
**Then** `has_existing_conflicts` is false
**And** `overlapping_files` contains `src/lib.rs`
**And** `merge_likely_safe` is false

```rust
#[tokio::test]
async fn hp_004_detect_overlapping_file_modifications() {
    // Arrange: Modify same file in workspace and trunk
    // Act: Run conflict detection
    // Assert: overlapping_files contains the file
}
```

---

### 2.5 HP-005: Identify Workspace-Only Changes

**Given** a workspace that added `src/new_feature.rs`
**And** trunk has no changes to `src/new_feature.rs`
**When** conflict detection is run
**Then** `workspace_only` contains `src/new_feature.rs`
**And** `overlapping_files` does not contain `src/new_feature.rs`

```rust
#[tokio::test]
async fn hp_005_identify_workspace_only_changes() {
    // Arrange: Add file only in workspace
    // Act: Run conflict detection
    // Assert: file appears in workspace_only
}
```

---

### 2.6 HP-006: Identify Main-Only Changes

**Given** trunk that modified `src/other.rs`
**And** workspace has no changes to `src/other.rs`
**When** conflict detection is run
**Then** `main_only` contains `src/other.rs`
**And** `overlapping_files` does not contain `src/other.rs`

```rust
#[tokio::test]
async fn hp_006_identify_main_only_changes() {
    // Arrange: Modify file only in trunk
    // Act: Run conflict detection
    // Assert: file appears in main_only
}
```

---

### 2.7 HP-007: Find Merge Base Successfully

**Given** a workspace branched from trunk at commit X
**And** both workspace and trunk have new commits
**When** merge base detection is run
**Then** `merge_base` is Some(X)
**And** X is a valid commit ID

```rust
#[tokio::test]
async fn hp_007_find_merge_base_successfully() {
    // Arrange: Create branch point
    // Act: Find merge base
    // Assert: merge_base == branch_point_commit
}
```

---

### 2.8 HP-008: JSON Output Format

**Given** a workspace with detected conflicts
**When** conflict detection is run with JSON format
**Then** output is valid JSON
**And** contains all required fields
**And** is parseable by serde_json

```rust
#[tokio::test]
async fn hp_008_json_output_format() {
    // Arrange: Create workspace with conflicts
    // Act: Run detection with --format json
    // Assert: JSON is valid and contains all fields
}
```

---

### 2.9 HP-009: Human-Readable Summary

**Given** a workspace with two overlapping files
**When** conflict detection is run
**Then** summary mentions "2 files"
**And** summary includes file names
**And** summary provides resolution hints

```rust
#[tokio::test]
async fn hp_009_human_readable_summary() {
    // Arrange: Create overlapping changes
    // Act: Run detection
    // Assert: summary is human-readable and actionable
}
```

---

### 2.10 HP-010: Detection Time Measurement

**Given** a workspace with any state
**When** conflict detection is run
**Then** `detection_time_ms` is greater than 0
**And** `detection_time_ms` is less than 5000
**And** measurement is accurate within 100ms

```rust
#[tokio::test]
async fn hp_010_detection_time_measurement() {
    // Arrange: Any workspace
    // Act: Run detection, measure external time
    // Assert: detection_time_ms matches wall clock within tolerance
}
```

---

### 2.11 HP-011: Quick Conflict Check

**Given** a workspace with existing conflicts
**When** quick conflict check is run
**Then** it returns true
**And** completes in less than 100ms

```rust
#[tokio::test]
async fn hp_011_quick_conflict_check() {
    // Arrange: Create conflicting workspace
    // Act: Run has_existing_conflicts()
    // Assert: returns true quickly
}
```

---

### 2.12 HP-012: Empty Workspace Detection

**Given** a workspace with no commits
**And** no uncommitted changes
**When** conflict detection is run
**Then** all file lists are empty
**And** `merge_likely_safe` is true

```rust
#[tokio::test]
async fn hp_012_empty_workspace_detection() {
    // Arrange: Create empty workspace
    // Act: Run detection
    // Assert: no conflicts, safe to merge
}
```

---

### 2.13 HP-013: Renamed File Handling

**Given** a file renamed in workspace: `old.rs` -> `new.rs`
**And** `old.rs` modified in trunk
**When** conflict detection is run
**Then** rename is detected
**And** appropriate overlap is reported

```rust
#[tokio::test]
async fn hp_013_renamed_file_handling() {
    // Arrange: Rename file in workspace, modify in trunk
    // Act: Run detection
    // Assert: rename detected correctly
}
```

---

### 2.14 HP-014: Deleted File Handling

**Given** a file deleted in workspace
**And** same file modified in trunk
**When** conflict detection is run
**Then** delete-modify conflict is detected
**And** reported in overlapping_files

```rust
#[tokio::test]
async fn hp_014_deleted_file_handling() {
    // Arrange: Delete in workspace, modify in trunk
    // Act: Run detection
    // Assert: delete-modify conflict detected
}
```

---

### 2.15 HP-015: Files Analyzed Count

**Given** 5 files in workspace_only
**And** 3 files in main_only
**And** 2 files in overlapping_files
**When** conflict detection completes
**Then** `files_analyzed` is at least 10

```rust
#[tokio::test]
async fn hp_015_files_analyzed_count() {
    // Arrange: Create specific file distribution
    // Act: Run detection
    // Assert: files_analyzed >= sum of all lists
}
```

---

## 3. Error Path Tests

### 3.1 EP-001: Not in JJ Repository

**Given** the current directory is not a JJ repository
**When** conflict detection is run
**Then** `ConflictError::NotInWorkspace` is returned
**And** error message indicates "not a JJ repository"

```rust
#[tokio::test]
async fn ep_001_not_in_jj_repository() {
    // Arrange: cd to non-jj directory
    // Act: Run detection
    // Assert: Error::NotInWorkspace variant
}
```

---

### 3.2 EP-002: Workspace Not Found

**Given** a workspace name that does not exist
**When** conflict detection is run for that workspace
**Then** `ConflictError::WorkspaceNotFound` is returned
**And** error includes the workspace name

```rust
#[tokio::test]
async fn ep_002_workspace_not_found() {
    // Arrange: Reference non-existent workspace
    // Act: Run detection
    // Assert: Error::WorkspaceNotFound with correct name
}
```

---

### 3.3 EP-003: JJ Binary Not Found

**Given** JJ is not installed or not in PATH
**When** conflict detection is run
**Then** `ConflictError::JjNotFound` is returned
**And** error includes searched paths

```rust
#[tokio::test]
async fn ep_003_jj_binary_not_found() {
    // Arrange: Remove jj from PATH
    // Act: Run detection
    // Assert: Error::JjNotFound with path info
}
```

---

### 3.4 EP-004: JJ Command Failed

**Given** JJ is available
**And** JJ command returns non-zero exit code
**When** conflict detection is run
**Then** `ConflictError::JjCommandFailed` is returned
**And** error includes exit code and stderr

```rust
#[tokio::test]
async fn ep_004_jj_command_failed() {
    // Arrange: Create condition causing jj to fail
    // Act: Run detection
    // Assert: Error::JjCommandFailed with details
}
```

---

### 3.5 EP-005: Invalid Workspace State

**Given** workspace is in "abandoned" state
**When** conflict detection is run
**Then** `ConflictError::InvalidWorkspaceState` is returned
**And** error includes current and expected states

```rust
#[tokio::test]
async fn ep_005_invalid_workspace_state() {
    // Arrange: Create abandoned workspace
    // Act: Run detection
    // Assert: Error::InvalidWorkspaceState
}
```

---

### 3.6 EP-006: Workspace Locked by Another Agent

**Given** workspace is locked by agent-A
**And** agent-B attempts conflict detection
**When** agent-B runs detection
**Then** `ConflictError::WorkspaceLocked` is returned
**And** error includes holder agent ID and expiry

```rust
#[tokio::test]
async fn ep_006_workspace_locked_by_another_agent() {
    // Arrange: Lock workspace with agent-A
    // Act: Agent-B attempts detection
    // Assert: Error::WorkspaceLocked with holder info
}
```

---

### 3.7 EP-007: Merge Base Not Found

**Given** workspace has no common ancestor with trunk
**When** conflict detection is run
**Then** `ConflictError::MergeBaseNotFound` is returned
**And** detection continues with fallback comparison

```rust
#[tokio::test]
async fn ep_007_merge_base_not_found() {
    // Arrange: Create unrelated branches
    // Act: Run detection
    // Assert: Error::MergeBaseNotFound or graceful handling
}
```

---

### 3.8 EP-008: Diff Parse Error

**Given** JJ returns malformed diff output
**When** conflict detection parses the output
**Then** `ConflictError::DiffParseError` is returned
**And** error includes the problematic output

```rust
#[tokio::test]
async fn ep_008_diff_parse_error() {
    // Arrange: Mock malformed JJ output
    // Act: Run detection
    // Assert: Error::DiffParseError with output snippet
}
```

---

### 3.9 EP-009: Resolution Failed

**Given** a file with conflicts
**When** auto-resolution is attempted
**And** resolution fails
**Then** `ConflictError::ResolutionFailed` is returned
**And** error includes file path and reason

```rust
#[tokio::test]
async fn ep_009_resolution_failed() {
    // Arrange: Create unresolvable conflict
    // Act: Attempt resolution
    // Assert: Error::ResolutionFailed with details
}
```

---

### 3.10 EP-010: Conflict Not Auto-Resolvable

**Given** a complex 3-way conflict
**When** auto-resolution is attempted
**Then** `ConflictError::ConflictNotAutoResolvable` is returned
**And** conflict type is identified

```rust
#[tokio::test]
async fn ep_010_conflict_not_auto_resolvable() {
    // Arrange: Create complex conflict
    // Act: Attempt auto-resolution
    // Assert: Error::ConflictNotAutoResolvable
}
```

---

### 3.11 EP-011: Status Check Failed

**Given** JJ status command fails
**When** conflict detection runs
**Then** `ConflictError::StatusCheckFailed` is returned
**And** error includes failure reason

```rust
#[tokio::test]
async fn ep_011_status_check_failed() {
    // Arrange: Cause jj status to fail
    // Act: Run detection
    // Assert: Error::StatusCheckFailed
}
```

---

### 3.12 EP-012: Merge Conflict During Merge Operation

**Given** workspace with no detected potential conflicts
**When** merge is attempted
**And** actual merge creates conflicts
**Then** `ConflictError::MergeConflictDetected` is returned
**And** conflicting files are listed

```rust
#[tokio::test]
async fn ep_012_merge_conflict_during_merge_operation() {
    // Arrange: Create hidden conflict scenario
    // Act: Attempt merge
    // Assert: Error::MergeConflictDetected with file list
}
```

---

### 3.13 EP-013: IO Error During Analysis

**Given** a file becomes unreadable during analysis
**When** conflict detection runs
**Then** `ConflictError::IoError` is returned
**And** error includes operation and path

```rust
#[tokio::test]
async fn ep_013_io_error_during_analysis() {
    // Arrange: Make file unreadable mid-analysis
    // Act: Run detection
    // Assert: Error::IoError with operation context
}
```

---

### 3.14 EP-014: Lock Timeout

**Given** workspace is locked
**And** lock holder does not release
**When** waiting for lock exceeds timeout
**Then** `ConflictError::LockTimeout` is returned (if applicable)

```rust
#[tokio::test]
async fn ep_014_lock_timeout() {
    // Arrange: Hold lock indefinitely
    // Act: Attempt detection with short timeout
    // Assert: Timeout error or WorkspaceLocked
}
```

---

### 3.15 EP-015: Invalid File Path in Report

**Given** JJ returns a file path with invalid characters
**When** conflict detection processes the path
**Then** `ConflictError::InvalidFilePath` is returned
**And** path is sanitized or rejected

```rust
#[tokio::test]
async fn ep_015_invalid_file_path_in_report() {
    // Arrange: Create path with special characters
    // Act: Run detection
    // Assert: Error::InvalidFilePath or sanitization
}
```

---

### 3.16 EP-016: Trunk Reference Not Found

**Given** trunk reference (main) does not exist
**When** conflict detection is run
**Then** appropriate error is returned
**And** error suggests checking trunk configuration

```rust
#[tokio::test]
async fn ep_016_trunk_reference_not_found() {
    // Arrange: Remove main branch
    // Act: Run detection
    // Assert: Error about missing trunk
}
```

---

### 3.17 EP-017: Output Validation Failed

**Given** internal state inconsistency detected
**When** building detection result
**Then** `ConflictError::OutputValidationFailed` is returned
**And** inconsistency is logged

```rust
#[tokio::test]
async fn ep_017_output_validation_failed() {
    // Arrange: Create internal inconsistency
    // Act: Build result
    // Assert: Validation error
}
```

---

### 3.18 EP-018: Merge Base Timeout

**Given** very large repository
**When** merge base calculation takes too long
**Then** `ConflictError::MergeBaseTimeout` is returned
**And** fallback comparison is used

```rust
#[tokio::test]
async fn ep_018_merge_base_timeout() {
    // Arrange: Create large repo or mock slow operation
    // Act: Run detection with short timeout
    // Assert: Timeout error or graceful degradation
}
```

---

## 4. Edge Case Tests

### 4.1 EC-001: Empty Repository

**Given** a freshly initialized JJ repository
**And** no commits exist
**When** conflict detection is run
**Then** it handles empty state gracefully
**And** returns safe result

```rust
#[tokio::test]
async fn ec_001_empty_repository() {
    // Arrange: Fresh jj init
    // Act: Run detection
    // Assert: No panic, reasonable result
}
```

---

### 4.2 EC-002: Very Long File Paths

**Given** a file with path > 255 characters
**When** conflict detection processes this file
**Then** path is handled correctly
**And** appears in appropriate list

```rust
#[tokio::test]
async fn ec_002_very_long_file_paths() {
    // Arrange: Create deeply nested file
    // Act: Run detection
    // Assert: Path handled correctly
}
```

---

### 4.3 EC-003: Unicode File Names

**Given** files with Unicode characters in names
**When** conflict detection processes these files
**Then** Unicode is preserved correctly
**And** no encoding errors occur

```rust
#[tokio::test]
async fn ec_003_unicode_file_names() {
    // Arrange: Create files with emoji, CJK, etc.
    // Act: Run detection
    // Assert: Unicode preserved
}
```

---

### 4.4 EC-004: Binary Files

**Given** binary files modified in both branches
**When** conflict detection runs
**Then** binary files are detected
**And** reported appropriately

```rust
#[tokio::test]
async fn ec_004_binary_files() {
    // Arrange: Modify binary file in both branches
    // Act: Run detection
    // Assert: Binary conflict detected
}
```

---

### 4.5 EC-005: Symlink Handling

**Given** symlinks in the repository
**When** conflict detection runs
**Then** symlinks are handled correctly
**And** symlink targets are not confused with files

```rust
#[tokio::test]
async fn ec_005_symlink_handling() {
    // Arrange: Create symlinks
    // Act: Run detection
    // Assert: Symlinks handled correctly
}
```

---

### 4.6 EC-006: Submodule Handling

**Given** git submodules in the repository
**When** conflict detection runs
**Then** submodules are detected
**And** submodule conflicts are reported

```rust
#[tokio::test]
async fn ec_006_submodule_handling() {
    // Arrange: Add submodule
    // Act: Run detection
    // Assert: Submodule conflicts detected
}
```

---

### 4.7 EC-007: Large Number of Files

**Given** 10,000 files modified in workspace
**When** conflict detection runs
**Then** it completes within time limit
**And** memory usage is bounded

```rust
#[tokio::test]
async fn ec_007_large_number_of_files() {
    // Arrange: Create many modified files
    // Act: Run detection
    // Assert: Completes in reasonable time
}
```

---

### 4.8 EC-008: Concurrent Detection Requests

**Given** multiple concurrent detection requests on same workspace
**When** all requests execute simultaneously
**Then** all return consistent results
**And** no race conditions occur

```rust
#[tokio::test]
async fn ec_008_concurrent_detection_requests() {
    // Arrange: Spawn multiple detection tasks
    // Act: Run all concurrently
    // Assert: All return same result
}
```

---

### 4.9 EC-009: Stale Workspace

**Given** a workspace that is stale
**When** conflict detection is run
**Then** appropriate warning is given
**And** detection may fail or proceed with caution

```rust
#[tokio::test]
async fn ec_009_stale_workspace() {
    // Arrange: Create stale workspace
    // Act: Run detection
    // Assert: Warning or error about stale state
}
```

---

### 4.10 EC-010: Empty Conflict Markers

**Given** a file with conflict markers but no actual content
**When** conflict detection runs
**Then** conflict is detected
**And** handled without panic

```rust
#[tokio::test]
async fn ec_010_empty_conflict_markers() {
    // Arrange: Create file with empty conflict
    // Act: Run detection
    // Assert: Detected without panic
}
```

---

### 4.11 EC-011: Nested Conflicts

**Given** a conflict within a conflict (nested markers)
**When** conflict detection runs
**Then** nested conflict is detected
**And** reported accurately

```rust
#[tokio::test]
async fn ec_011_nested_conflicts() {
    // Arrange: Create nested conflict markers
    // Act: Run detection
    // Assert: All conflicts detected
}
```

---

### 4.12 EC-012: Directory vs File Conflict

**Given** a path that is a directory in one branch and file in another
**When** conflict detection runs
**Then** type conflict is detected
**And** conflict type is TypeChange

```rust
#[tokio::test]
async fn ec_012_directory_vs_file_conflict() {
    // Arrange: Create dir/file conflict
    // Act: Run detection
    // Assert: TypeChange conflict detected
}
```

---

## 5. Contract Verification Tests

### 5.1 CV-001: PRE-REPO-001 Verification

**Test:** Must be in JJ repository
**Verify:** Precondition fails outside JJ repo

```rust
#[tokio::test]
async fn cv_001_pre_repo_001_verification() {
    // Test: cd to /tmp (non-jj), run detection
    // Assert: Precondition violated, error returned
}
```

---

### 5.2 CV-002: PRE-WS-001 Verification

**Test:** Workspace must exist
**Verify:** Precondition fails for non-existent workspace

```rust
#[tokio::test]
async fn cv_002_pre_ws_001_verification() {
    // Test: Reference "nonexistent-workspace"
    // Assert: WorkspaceNotFound error
}
```

---

### 5.3 CV-003: PRE-WS-004 Verification

**Test:** Caller must hold workspace lock
**Verify:** Precondition fails without lock

```rust
#[tokio::test]
async fn cv_003_pre_ws_004_verification() {
    // Test: Attempt detection without acquiring lock
    // Assert: WorkspaceLocked error (if lock required)
}
```

---

### 5.4 CV-004: POST-DET-001 Verification

**Test:** has_existing_conflicts accurately reflects JJ state
**Verify:** Postcondition holds after detection

```rust
#[tokio::test]
async fn cv_004_post_det_001_verification() {
    // Test: Run detection, check has_existing_conflicts
    // Verify: Matches actual jj resolve --list output
}
```

---

### 5.5 CV-005: POST-DET-002 Verification

**Test:** overlapping_files contains all files modified in both branches
**Verify:** Postcondition holds

```rust
#[tokio::test]
async fn cv_005_post_det_002_verification() {
    // Test: Modify files in both branches
    // Verify: All overlap files detected
}
```

---

### 5.6 CV-006: POST-DET-003 Verification

**Test:** merge_likely_safe is true only when no conflicts exist
**Verify:** Postcondition logic is correct

```rust
#[tokio::test]
async fn cv_006_post_det_003_verification() {
    // Test: Create various conflict scenarios
    // Verify: merge_likely_safe == !has_conflicts()
}
```

---

### 5.7 CV-007: POST-DET-004 Verification

**Test:** detection_time_ms is within acceptable bounds
**Verify:** Timing postcondition

```rust
#[tokio::test]
async fn cv_007_post_det_004_verification() {
    // Test: Measure detection time
    // Verify: 0 < time < 5000ms
}
```

---

### 5.8 CV-008: POST-DET-005 Verification

**Test:** All file paths are normalized
**Verify:** No `./` or `../` in paths

```rust
#[tokio::test]
async fn cv_008_post_det_005_verification() {
    // Test: Create files with various path formats
    // Verify: All paths normalized
}
```

---

### 5.9 CV-009: POST-EXIT-001 Verification

**Test:** Exit code 0 when no conflicts
**Verify:** Exit code postcondition

```rust
#[tokio::test]
async fn cv_009_post_exit_001_verification() {
    // Test: Clean workspace
    // Verify: Exit code 0
}
```

---

### 5.10 CV-010: POST-EXIT-002 Verification

**Test:** Exit code 1 when conflicts detected
**Verify:** Exit code postcondition

```rust
#[tokio::test]
async fn cv_010_post_exit_002_verification() {
    // Test: Workspace with conflicts
    // Verify: Exit code 1
}
```

---

### 5.11 CV-011: INV-DATA-001 Verification

**Test:** ConflictDetectionResult consistency
**Verify:** has_conflicts() logic invariant

```rust
#[tokio::test]
async fn cv_011_inv_data_001_verification() {
    // Property test: has_conflicts() == has_existing || !overlapping.is_empty()
}
```

---

### 5.12 CV-012: INV-DATA-002 Verification

**Test:** File lists are mutually exclusive
**Verify:** No file in both workspace_only and main_only

```rust
#[tokio::test]
async fn cv_012_inv_data_002_verification() {
    // Property test: workspace_only ∩ main_only = ∅
}
```

---

### 5.13 CV-013: INV-DATA-003 Verification

**Test:** files_analyzed count invariant
**Verify:** Count is at least sum of all lists

```rust
#[tokio::test]
async fn cv_013_inv_data_003_verification() {
    // Property test: files_analyzed >= sum of list lengths
}
```

---

### 5.14 CV-014: INV-SEC-001 Verification

**Test:** Detection never modifies repository state
**Verify:** Read-only operation invariant

```rust
#[tokio::test]
async fn cv_014_inv_sec_001_verification() {
    // Test: Record repo state before/after detection
    // Verify: State unchanged
}
```

---

### 5.15 CV-015: INV-SEC-003 Verification

**Test:** All conflict operations are logged
**Verify:** Audit trail invariant

```rust
#[tokio::test]
async fn cv_015_inv_sec_003_verification() {
    // Test: Run detection, check audit log
    // Verify: Entry exists with correct details
}
```

---

### 5.16 CV-016: INV-CONC-001 Verification

**Test:** Detection is atomic
**Verify:** Either completes fully or fails cleanly

```rust
#[tokio::test]
async fn cv_016_inv_conc_001_verification() {
    // Test: Kill detection mid-process
    // Verify: No partial state left
}
```

---

### 5.17 CV-017: INV-PERF-001 Verification

**Test:** Detection completes in <5s for <10k files
**Verify:** Performance invariant

```rust
#[tokio::test]
async fn cv_017_inv_perf_001_verification() {
    // Test: Create repo with 5000 files, measure time
    // Verify: <5 seconds
}
```

---

### 5.18 CV-018: INV-PERF-002 Verification

**Test:** Quick check completes in <100ms
**Verify:** Performance invariant

```rust
#[tokio::test]
async fn cv_018_inv_perf_002_verification() {
    // Test: Run has_existing_conflicts()
    // Verify: <100ms
}
```

---

### 5.19 CV-019: Error Variant Serialization

**Test:** All ConflictError variants are serializable
**Verify:** JSON round-trip for each variant

```rust
#[tokio::test]
async fn cv_019_error_variant_serialization() {
    // Test: Serialize/deserialize each ConflictError variant
    // Verify: Round-trip succeeds
}
```

---

### 5.20 CV-020: Error Display Implementation

**Test:** All ConflictError variants have meaningful Display
**Verify:** Each error has actionable message

```rust
#[tokio::test]
async fn cv_020_error_display_implementation() {
    // Test: Display each error variant
    // Verify: Message contains context and is actionable
}
```

---

## 6. End-to-End Scenarios

### 6.1 E2E-001: Full Happy Path Workflow

**Scenario:** Developer completes work with no conflicts

**Given** a developer has completed work in workspace "feature-auth"
**And** workspace has 5 commits
**And** trunk has 2 new commits (no overlap)
**When** developer runs `zjj done --detect-conflicts`
**Then** conflict detection shows "No conflicts detected"
**And** merge proceeds successfully
**And** workspace is cleaned up

```rust
#[tokio::test]
async fn e2e_001_full_happy_path_workflow() {
    // Setup: Create workspace, make commits, ensure no conflicts
    // Execute: Run zjj done --detect-conflicts
    // Verify: No conflicts, merge succeeds, cleanup occurs
}
```

---

### 6.2 E2E-002: Conflict Detection Blocks Merge

**Scenario:** Developer attempts merge with conflicts

**Given** a developer has work in workspace "feature-api"
**And** same file modified in trunk
**When** developer runs `zjj done --detect-conflicts`
**Then** conflicts are detected and reported
**And** merge is blocked
**And** developer receives resolution hints

```rust
#[tokio::test]
async fn e2e_002_conflict_detection_blocks_merge() {
    // Setup: Create overlapping changes
    // Execute: Run zjj done --detect-conflicts
    // Verify: Merge blocked, conflict report shown
}
```

---

### 6.3 E2E-003: Multi-Agent Workspace Access

**Scenario:** Two agents access same workspace

**Given** agent-A is working in workspace "shared-feature"
**And** agent-A holds the workspace lock
**When** agent-B attempts conflict detection on same workspace
**Then** agent-B receives WorkspaceLocked error
**And** agent-B can see who holds the lock

```rust
#[tokio::test]
async fn e2e_003_multi_agent_workspace_access() {
    // Setup: Agent-A acquires lock
    // Execute: Agent-B attempts detection
    // Verify: Agent-B gets WorkspaceLocked with holder info
}
```

---

### 6.4 E2E-004: Conflict Queued for Resolution

**Scenario:** Conflict detected, queued for later resolution

**Given** a workspace with detected conflicts
**When** conflict is detected during `zjj done`
**Then** workspace is added to merge queue
**And** queue entry has correct priority
**And** conflict details are preserved

```rust
#[tokio::test]
async fn e2e_004_conflict_queued_for_resolution() {
    // Setup: Create conflicting workspace
    // Execute: Run zjj done
    // Verify: Entry in merge queue with details
}
```

---

### 6.5 E2E-005: Dry Run with Conflict Preview

**Scenario:** Developer previews merge before executing

**Given** a workspace with potential conflicts
**When** developer runs `zjj done --dry-run --detect-conflicts`
**Then** detailed conflict preview is shown
**And** no changes are made
**And** developer can see all affected files

```rust
#[tokio::test]
async fn e2e_005_dry_run_with_conflict_preview() {
    // Setup: Create workspace with various changes
    // Execute: Run with --dry-run --detect-conflicts
    // Verify: Preview shown, no state changes
}
```

---

### 6.6 E2E-006: Lock Expiry Allows Access

**Scenario:** Expired lock allows new agent access

**Given** agent-A held a lock that has now expired
**When** agent-B attempts conflict detection
**Then** agent-B acquires lock successfully
**And** detection proceeds normally

```rust
#[tokio::test]
async fn e2e_006_lock_expiry_allows_access() {
    // Setup: Create expired lock
    // Execute: Agent-B attempts detection
    // Verify: Lock acquired, detection succeeds
}
```

---

### 6.7 E2E-007: Audit Trail Verification

**Scenario:** All operations are logged for audit

**Given** a workspace with conflicts
**When** detection is run
**And** conflict is resolved
**Then** audit log contains all operations
**And** timestamps are accurate
**And** agent IDs are recorded

```rust
#[tokio::test]
async fn e2e_007_audit_trail_verification() {
    // Setup: Enable audit logging
    // Execute: Run detection and resolution
    // Verify: Audit log contains all operations
}
```

---

### 6.8 E2E-008: JSON Output for Automation

**Scenario:** CI/CD pipeline consumes JSON output

**Given** a CI pipeline runs conflict detection
**When** detection runs with JSON format
**Then** output is valid JSON
**And** contains all required fields
**And** can be parsed by standard tools

```rust
#[tokio::test]
async fn e2e_008_json_output_for_automation() {
    // Setup: Configure JSON output
    // Execute: Run detection
    // Verify: JSON is valid and complete
}
```

---

### 6.9 E2E-009: Recovery from Interrupted Detection

**Scenario:** Detection is interrupted and restarted

**Given** detection was interrupted mid-process
**When** detection is run again
**Then** it completes successfully
**And** no partial state affects results

```rust
#[tokio::test]
async fn e2e_009_recovery_from_interrupted_detection() {
    // Setup: Simulate interruption
    // Execute: Run detection again
    // Verify: Clean completion, correct results
}
```

---

### 6.10 E2E-010: Full Resolution Workflow

**Scenario:** Complete conflict resolution workflow

**Given** a workspace with detected conflicts
**When** developer resolves conflicts
**And** runs `zjj done --detect-conflicts` again
**Then** no conflicts are detected
**And** merge proceeds
**And** bead status is updated

```rust
#[tokio::test]
async fn e2e_010_full_resolution_workflow() {
    // Setup: Create conflicts, then resolve
    // Execute: Run done command
    // Verify: Merge succeeds, status updated
}
```

---

## 7. Test Implementation Guidelines

### 7.1 Test Structure

```rust
// Standard test structure for E2E tests
#[tokio::test]
async fn test_name() {
    // === ARRANGE ===
    // Set up repository state
    // Create workspaces, files, conflicts as needed

    // === ACT ===
    // Execute the operation under test

    // === ASSERT ===
    // Verify results match expected outcomes
    // Check all relevant postconditions
}
```

### 7.2 Test Helpers Required

```rust
/// Create a test repository with specified state
async fn create_test_repo(config: RepoConfig) -> TempRepo;

/// Create a workspace with specific file state
async fn create_workspace(repo: &TempRepo, name: &str, files: Vec<FileSpec>);

/// Create a conflict in a specific file
async fn create_conflict(repo: &TempRepo, workspace: &str, file: &str);

/// Assert detection result matches expected
fn assert_detection_result(actual: &ConflictDetectionResult, expected: &ExpectedResult);

/// Capture audit log entries
async fn capture_audit_entries(repo: &TempRepo) -> Vec<AuditEntry>;
```

### 7.3 Test Isolation

- Each test must use a temporary repository
- Tests must not share state
- Tests must clean up resources even on failure
- Tests must be parallelizable

### 7.4 Security Test Requirements

- All security tests must verify audit logging
- Security tests must test both positive and negative cases
- Authentication tests must use different agent IDs
- Lock tests must test expiration scenarios

---

## 8. Coverage Matrix

| Component | Happy Path | Error Path | Edge Case | Contract | E2E |
|-----------|------------|------------|-----------|----------|-----|
| detect_conflicts | 3 | 4 | 5 | 5 | 4 |
| has_existing_conflicts | 2 | 1 | 1 | 2 | 1 |
| find_merge_base | 1 | 2 | 1 | 1 | 1 |
| get_modified_files | 2 | 2 | 2 | 1 | 1 |
| is_merge_safe | 1 | 1 | 1 | 2 | 1 |
| queue_for_resolution | 1 | 2 | 0 | 2 | 2 |
| Error handling | 0 | 6 | 2 | 7 | 0 |
| **Total** | **10** | **18** | **12** | **20** | **10** |

---

## 9. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-02-18 | Initial test plan |

---

## 10. References

- Contract Specification: `/home/lewis/src/zjj/contracts/bd-1c4-contract-spec.md`
- Implementation: `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs`
- Martin Fowler BDD: https://martinfowler.com/bliki/GivenWhenThen.html
