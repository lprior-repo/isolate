# Graphite Stacking Implementation Progress

## Completed Beads
- [x] bd-1i1a - queue: Add parent_workspace column (2026-02-21)

## Current Bead
Ready to start: bd-3axf

## Bead Checklist

### Phase A: Data Model
- [x] bd-1i1a - queue: Add parent_workspace column
- [ ] bd-3axf - queue: Add stack_depth column
- [ ] bd-umr4 - queue: Add dependents JSON column
- [ ] bd-26kg - queue: Add stack_root column
- [ ] bd-1ued - queue-status: Add StackMergeState enum
- [ ] bd-36h8 - queue: Add stack_merge_state column

### Phase B: Pure Helpers
- [ ] bd-1ixz - stack: Add StackError enum
- [ ] bd-1idz - stack: Add calculate_stack_depth function
- [ ] bd-mmtr - stack: Add find_stack_root function
- [ ] bd-35xk - stack: Add validate_no_cycle function
- [ ] bd-i6lp - stack: Add build_dependent_list function

### Phase C: Repository Methods
- [ ] bd-2ljz - queue: Add get_children method
- [ ] bd-38iz - queue: Add get_stack_root method
- [ ] bd-32u3 - queue: Add find_blocked method
- [ ] bd-3vty - queue: Add update_dependents method
- [ ] bd-s6pj - queue: Add transition_stack_state method

### Phase D: Train Processor
- [ ] bd-24yl - train: Filter blocked in next()
- [ ] bd-dqr8 - train: Add stack_depth to sort
- [ ] bd-qbcr - train: Add cascade unblock logic
- [ ] bd-3cb2 - train: Queue rebase for unblocked children

### Phase E: CLI Commands
- [ ] bd-3mlc - cli: Add --parent flag parsing
- [ ] bd-170b - cli: Add parent validation
- [ ] bd-vak5 - cli: Pass parent to repository
- [ ] bd-vz4z - cli: Add stack fields to submit JSONL
- [ ] bd-3dar - cli: Add stack error exit codes
- [ ] bd-29gt - cli: Add stack JSONL schema
- [ ] bd-1e5n - cli: Add stack status command
- [ ] bd-3p55 - cli: Add stack sync command

### Phase F: Integration Tests
- [ ] bd-2vew - test: Integration test stack submit
- [ ] bd-wx26 - test: Integration test cascade merge
- [ ] bd-1has - test: Integration test stack priority
- [ ] bd-2wx8 - test: Integration test deep stack

## Statistics
- Total: 32
- Completed: 1
- Remaining: 31
