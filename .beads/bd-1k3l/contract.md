# Contract: handlers: Fix status routing to subcommands

**Bead ID:** isolate-20260228085443-9nzwinvl  
**Type:** bug  
**Priority:** P1  
**Effort Estimate:** 1hr

---

## EARS Requirements

### Ubiquitous Language
- THE SYSTEM SHALL route 'isolate status <subcommand>' to appropriate handler

### Event-Driven Requirements
| Trigger | Shall |
|---------|-------|
| WHEN user runs 'isolate status show' | THE SYSTEM SHALL call status_show handler |
| WHEN user runs 'isolate status whereami' | THE SYSTEM SHALL call whereami handler |

### Unwanted Behaviors
| Condition | Shall Not | Because |
|-----------|-----------|----------|
| IF subcommand is missing | THE SYSTEM SHALL NOT fail silently | User needs feedback |

---

## Contracts

### Preconditions
- **auth_required:** false
- **required_inputs:** []
- **system_state:** handle_status receives Args struct with subcommand() method

### Postconditions
**State Changes:**
- status show returns workspace status
- status whereami returns path
- status whoami returns agent
- status context returns context

### Invariants
- Legacy 'isolate status' still works with deprecation warning

---

## Research Requirements

### Files to Read
1. `handlers/workspace.rs` - Existing patterns
2. `commands/status.rs` - Existing patterns
3. `object_commands.rs` - Existing patterns

---

## Acceptance Tests

### Happy Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_status_show | Valid inputs | User runs 'isolate status show' | Exit code is 0, Output is correct |
| test_status_whereami | Valid inputs | User runs 'isolate status whereami' | Exit code is 0, Output is correct |
| test_status_whoami | Valid inputs | User runs 'isolate status whoami' | Exit code is 0, Output is correct |
| test_status_context | Valid inputs | User runs 'isolate status context' | Exit code is 0, Output is correct |

### Error Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_invalid_subcommand | Invalid inputs | User runs 'isolate status invalid' | Exit code is non-zero, Error message is clear |
| test_missing_subcommand | Missing subcommand | User runs 'isolate status' | Exit code is non-zero, Help message shown |

---

## Implementation Tasks

### Phase 0: Research
- [ ] Read handlers/workspace.rs handle_status function
- [ ] Document existing patterns in research_notes.md

### Phase 1: Tests First
- [ ] Add subcommand match to handle_status
- [ ] Write failing tests for all subcommands

### Phase 2: Implementation
- [ ] Implement: isolate status show
- [ ] Implement: isolate status whereami
- [ ] Implement: isolate status whoami
- [ ] Implement: isolate status context

### Phase 4: Verification
- [ ] Run moon run :ci
- [ ] Verify all tests pass

---

## Completion Checklist

- [ ] All acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real data
- [ ] No mocks or fake data in any test
- [ ] Implementation uses Result<T, Error> throughout
- [ ] Zero unwrap or expect calls
- [ ] moon run :ci passes

---

## Context

### Related Files
- `handlers/workspace.rs:132` - Related implementation
- `commands/status.rs` - Related implementation

### Similar Implementations
- handle_task in commands/task.rs:731 uses subcommand() match

---

## AI Hints

### Do
- Use functional patterns: map, and_then, ?
- Return Result<T, Error> from all fallible functions
- READ files before modifying them

### Do Not
- Do NOT use unwrap or expect
- Do NOT use panic!, todo!, or unimplemented!
- Do NOT modify clippy configuration

### Constitution
- Zero unwrap law: NEVER use .unwrap or .expect
- Test first: Tests MUST exist before implementation
