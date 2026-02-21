# Graphite-Style Stacked PR Implementation

## Mission

Implement 32 small beads to add Graphite-style stacked PR support to the zjj merge queue. Each bead must pass ATDD testing before merging to main.

## Critical Workflow Per Bead

**YOU MUST FOLLOW THIS EXACT WORKFLOW FOR EACH BEAD:**

### Phase 1: SCOUT (Research)
- Use `br show <bead-id>` to understand the bead requirements
- Read ALL related files in the codebase
- Identify patterns to follow from existing code
- Document findings in `.ralph/scout-notes.md`

### Phase 2: ATDD TEST DEFINITION (Subagent - HIDDEN)
**CRITICAL: Use a SUBAGENT to define the test. The implementing agent MUST NOT see the test code until Phase 5.**

```
Use Task tool with subagent_type="functional-rust" to:
1. Read the bead requirements
2. Define acceptance tests based on the EARS requirements and contracts
3. Write tests to .ralph/atdd-tests/<bead-id>_test.rs
4. The test file should compile but FAIL initially
5. OUTPUT ONLY the file path - DO NOT show test contents
```

### Phase 3: RED (Write Failing Implementation Stub)
- Create minimal implementation that makes the code compile
- Run `cargo test` - tests MUST fail (this proves the test works)
- If tests pass already, the test is wrong - go back to Phase 2

### Phase 4: IMPLEMENT (Make Tests Pass)
- Write the actual implementation
- Follow existing patterns in the codebase
- NO unwrap(), NO expect(), NO panic!()
- Use Result/Option properly
- Run `cargo test` frequently

### Phase 5: REVIEW & ATDD VERIFICATION
- Run the ATDD test from `.ralph/atdd-tests/<bead-id>_test.rs`
- Run `cargo clippy -- -D warnings`
- Run `cargo test --all`
- If ANY failure: fix and repeat Phase 4-5
- Only proceed when 100% clean

### Phase 6: COMMIT
- Commit with message referencing bead ID
- Move to next bead

## Bead Implementation Order

Implement beads in this exact order (dependencies matter):

### Phase A: Data Model (6 beads)
```
bd-1i1a  queue: Add parent_workspace column
bd-3axf  queue: Add stack_depth column
bd-umr4  queue: Add dependents JSON column
bd-26kg  queue: Add stack_root column
bd-1ued  queue-status: Add StackMergeState enum
bd-36h8  queue: Add stack_merge_state column
```

### Phase B: Pure Helper Functions (5 beads)
```
bd-1ixz  stack: Add StackError enum
bd-1idz  stack: Add calculate_stack_depth function
bd-mmtr  stack: Add find_stack_root function
bd-35xk  stack: Add validate_no_cycle function
bd-i6lp  stack: Add build_dependent_list function
```

### Phase C: Repository Methods (5 beads)
```
bd-2ljz  queue: Add get_children method
bd-38iz  queue: Add get_stack_root method
bd-32u3  queue: Add find_blocked method
bd-3vty  queue: Add update_dependents method
bd-s6pj  queue: Add transition_stack_state method
```

### Phase D: Train Processor (4 beads)
```
bd-24yl  train: Filter blocked in next()
bd-dqr8  train: Add stack_depth to sort
bd-qbcr  train: Add cascade unblock logic
bd-3cb2  train: Queue rebase for unblocked children
```

### Phase E: CLI Commands (8 beads)
```
bd-3mlc  cli: Add --parent flag parsing
bd-170b  cli: Add parent validation
bd-vak5  cli: Pass parent to repository
bd-vz4z  cli: Add stack fields to submit JSONL
bd-3dar  cli: Add stack error exit codes
bd-29gt  cli: Add stack JSONL schema
bd-1e5n  cli: Add stack status command
bd-3p55  cli: Add stack sync command
```

### Phase F: Integration Tests (4 beads)
```
bd-2vew  test: Integration test stack submit
bd-wx26  test: Integration test cascade merge
bd-1has  test: Integration test stack priority
bd-2wx8  test: Integration test deep stack
```

## Quality Gates (ALL must pass before next bead)

```bash
# 1. Tests pass
cargo test --all

# 2. No clippy warnings
cargo clippy -- -D warnings

# 3. Build clean
cargo build --release

# 4. ATDD test passes
cargo test --test <bead-id>_test
```

## Code Standards

- **NO unwrap()** - use `?` or pattern matching
- **NO expect()** - use proper error types
- **NO panic!()** - return Result
- **All functions return Result<T, E>** for fallible operations
- **Use thiserror for error types**
- **Follow existing patterns in codebase**

## File Locations

- ATDD tests: `.ralph/atdd-tests/<bead-id>_test.rs`
- Scout notes: `.ralph/scout-notes.md`
- Progress tracking: `.ralph/progress.md`

## Progress Tracking

After each bead, update `.ralph/progress.md`:
```markdown
## Completed Beads
- [x] bd-1i1a - queue: Add parent_workspace column (2026-02-21)
- [ ] bd-3axf - queue: Add stack_depth column

## Current Bead
Working on: bd-3axf
Phase: 4 (IMPLEMENT)
```

## Completion Promise

When ALL 32 beads are implemented and all quality gates pass, output:

```
ALL BEADS COMPLETE - GRAPHITE STACKING IMPLEMENTED
```

## Important Notes

1. **Read existing code first** - the codebase has patterns to follow
2. **Small commits** - one commit per bead
3. **Test first, always** - ATDD test defined before implementation
4. **No shortcuts** - follow the 6-phase workflow exactly
5. **100% clean** - no warnings, no failing tests, ever

## Start Here

Begin with bead `bd-1i1a` - "queue: Add parent_workspace column"

Run: `br show bd-1i1a` to see the full specification.
