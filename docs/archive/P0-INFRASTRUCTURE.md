# P0 Infrastructure Architecture

This document describes the medical-grade infrastructure components that power ZJJ's parallel-workspace system. It focuses on:

- `workspace_state.rs`: state machine for concurrent agents
- `workspace_integrity.rs`: corruption detection and repair
- `coordination/queue.rs`: sequential merge queue
- `commands/done/conflict.rs`: pre-merge conflict detection

The goal is zero panics, deterministic behavior, and auditable state transitions.

## System Overview

```
               +----------------------+
               |   zjj CLI commands   |
               +----------+-----------+
                          |
                          v
              +-----------------------+
              |    Session Database   |
              |  (SQLite, zjj state)  |
              +----------+------------+
                         |
                         v
  +----------------------+-----------------------+
  |              Core Infrastructure            |
  |  workspace_state   workspace_integrity      |
  |  coordination/queue  done/conflict          |
  +----------------------+-----------------------+
                         |
                         v
              +-----------------------+
              |   JJ + Zellij + FS    |
              +-----------------------+
```

## 1) workspace_state.rs

### Purpose

Provides a type-safe lifecycle state machine for workspaces. It enforces allowed transitions and enables audit trails across 40+ concurrent agents.

### State Machine

```
Created -> Working        (start work)
Working -> Ready          (work complete)
Working -> Conflict       (merge conflict detected)
Working -> Abandoned      (manual abandon)
Ready   -> Working        (needs more work)
Ready   -> Merged         (successful merge)
Ready   -> Conflict       (merge conflict on attempt)
Ready   -> Abandoned      (decided not to merge)
Conflict-> Working        (conflict resolved)
Conflict-> Abandoned      (give up)

Terminal: Merged, Abandoned
```

### API Surface

- `WorkspaceState`: enum (`Created`, `Working`, `Ready`, `Merged`, `Abandoned`, `Conflict`)
- `WorkspaceStateTransition`: event with timestamp, reason, optional agent_id
- `WorkspaceStateFilter`: query helper for filtering lists

Key methods:

- `WorkspaceState::valid_next_states()`
- `WorkspaceState::can_transition_to(next)`
- `WorkspaceStateTransition::validate()`
- `WorkspaceStateFilter::matches(state)`

### Design Decisions

- Exhaustive matching makes illegal transitions impossible to ignore.
- Separate filter type avoids ad-hoc string filters.
- Transition struct carries audit metadata and is serializable.

### Testing Strategy

- Transition matrix tests for every state
- Parsing/serialization roundtrip tests
- Edge cases (invalid state names, terminal transitions)

## 2) workspace_integrity.rs

### Purpose

Detects and repairs workspace corruption with zero data loss. It classifies corruptions, suggests strategies, and enforces backup-first repair.

### Corruption Model

```
CorruptionType
  - MissingDirectory
  - MissingJjDir
  - InvalidJjState
  - StaleWorkingCopy
  - OrphanedWorkspace
  - DatabaseMismatch
  - PermissionDenied
  - StaleLock
  - Unknown
```

### Repair Strategies

```
RepairStrategy
  - RecreateWorkspace
  - UpdateWorkingCopy
  - ForgetAndRecreate
  - SyncDatabase
  - ClearStaleLock
  - NoRepairPossible
```

### Architecture Diagram

```
  detect() -> CorruptionReport
      |           |
      |           +--> recommended_strategy()
      |
      v
  BackupManager (snapshot)
      |
      v
  RepairExecutor (applies strategy)
      |
      v
  Verification (re-run detect)
```

### Design Decisions

- Backups are mandatory before destructive repairs.
- Repairs are idempotent to support retries.
- Severity and auto-repairable flags drive UI/CLI behavior.

### Testing Strategy

- Unit tests for classification and strategy mapping
- Repair dry-run tests to ensure no unintended mutations
- Verification step ensures post-repair correctness

## 3) coordination/queue.rs

### Purpose

Serializes merges across many agents. Prevents parallel merges from colliding by using a database-backed queue with a processing lock.

### Data Model

```
merge_queue
  id, workspace, bead_id, priority, status,
  added_at, started_at, completed_at, error_message, agent_id

queue_processing_lock
  agent_id, acquired_at, expires_at
```

### Flow

```
zjj queue add   -> enqueue workspace
zjj queue next  -> fetch next pending by priority
zjj queue process
   -> acquire lock
   -> mark processing
   -> attempt merge
   -> mark completed/failed
   -> release lock
```

### Design Decisions

- SQLite ensures deterministic ordering and durability.
- A TTL lock prevents concurrent processors from racing.
- Priority + FIFO ordering maximizes throughput while remaining predictable.

### Testing Strategy

- In-memory queue tests for ordering and status transitions
- Lock acquisition/expiration tests
- Failure path ensures error_message persists

## 4) done/conflict.rs

### Purpose

Detects merge conflicts before invoking `zjj done` merge logic. Reduces failed merges and guides agents with actionable output.

### Detection Steps

```
1. Check existing JJ conflicts
2. Determine merge base
3. Collect files changed in workspace
4. Collect files changed in main
5. Compute overlap -> potential conflicts
```

### Result Model

`ConflictDetectionResult` provides:

- existing conflicts list
- overlapping files list
- merge base hash
- summary and timing metadata
- `merge_likely_safe` boolean

### Design Decisions

- Zero false negatives: overlap is treated as risk.
- Text output is human-focused; JSON output is machine-focused.
- Exit codes are explicit for automation.

### Testing Strategy

- Result formatting tests
- Overlap detection tests
- Error propagation tests with mock executor

## Integration Guide

### Session Lifecycle

```
zjj add -> Session created (WorkspaceState::Created)
zjj work -> Transition to Working
zjj done --detect-conflicts -> Conflict detection
zjj done -> Ready -> Merged (or Conflict)
```

### Validation and Repair

```
zjj doctor --validate -> workspace_integrity detect()
zjj doctor --repair  -> backup + repair + verify
zjj queue process     -> serialized merge pipeline
```

### Observability

- `workspace_state` transitions are auditable
- merge queue status is persisted
- conflict detection provides human and JSON output

## API Summary (Key Types)

```
WorkspaceState
WorkspaceStateTransition
WorkspaceStateFilter

CorruptionType
RepairStrategy
CorruptionReport
BackupManager
RepairExecutor

MergeQueue
QueueEntry
QueueStats

ConflictDetectionResult
ConflictError
```

## Design Principles

- Railway-Oriented Programming (`Result` everywhere)
- Zero panics and unwraps
- Deterministic, database-backed coordination
- Explicit preconditions and postconditions

## Test Coverage Notes

- Core types have extensive unit tests
- Integration paths are validated via CLI tests
- Failure scenarios are explicitly asserted
