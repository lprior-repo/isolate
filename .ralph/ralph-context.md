# Ralph Context: Graphite Stacking Implementation

## Current State
- Working through 32 beads for Graphite-style stacked PR support
- Following strict ATDD workflow with hidden tests

## Key Files to Read
- `crates/zjj-core/src/coordination/queue_entities.rs` - QueueEntry struct
- `crates/zjj-core/src/coordination/queue_status.rs` - Status enums
- `crates/zjj-core/src/coordination/queue.rs` - MergeQueue implementation
- `crates/zjj-core/src/coordination/queue_repository.rs` - Repository trait
- `crates/zjj-core/src/coordination/train.rs` - Train processor

## Patterns to Follow
- Use `sqlx::FromRow` derive for DB entities
- Use `thiserror::Error` for error types
- NO unwrap/expect/panic
- All fallible functions return `Result<T, E>`
- Async methods use `#[async_trait::async_trait]`

## Reminders
- Scout first, then hidden ATDD test, then implement
- 100% clean before committing each bead
- Update progress.md after each bead

## Context added at 2026-02-21T21:32:26.115Z
bd-1i1a is COMPLETE. All ATDD tests pass (5/5). The 3 failing tests (test_moon_gate_step_*, test_rebase_step_conflict*) are PRE-EXISTING environmental issues NOT related to parent_workspace changes. Mark bd-1i1a as done and move to bd-3axf. Output READY_FOR_NEXT_TASK

## Context added at 2026-02-21T21:39:36.918Z
bd-1i1a is marked [x] COMPLETE in ralph-tasks.md. The next task is bd-3axf. Output READY_FOR_NEXT_TASK to advance to iteration 2 and start working on bd-3axf (stack_depth column).

## Context added at 2026-02-21T21:41:48.263Z
STOP trying to fix pre-existing test failures (test_moon_gate_step_*, test_rebase_step*). These are environmental issues unrelated to our changes. bd-1i1a ATDD tests PASS (5/5). Output READY_FOR_NEXT_TASK immediately to move to bd-3axf.
