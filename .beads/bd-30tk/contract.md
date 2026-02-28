# Contract: handlers: Fix doctor routing to subcommands

**Bead ID:** isolate-20260228085443-2szfswp0  
**Type:** bug  
**Priority:** P1  
**Effort Estimate:** 1hr

---

## EARS Requirements

### Ubiquitous Language
- THE SYSTEM SHALL route 'isolate doctor <subcommand>' to appropriate handler

### Event-Driven Requirements
| Trigger | Shall |
|---------|-------|
| WHEN user runs 'isolate doctor check' | THE SYSTEM SHALL run health check |
| WHEN user runs 'isolate doctor fix' | THE SYSTEM SHALL attempt fixes |

### Unwanted Behaviors
| Condition | Shall Not | Because |
|-----------|-----------|----------|
| IF subcommand is missing | THE SYSTEM SHALL NOT fail silently | User needs feedback |

---

## Contracts

### Preconditions
- **auth_required:** false
- **required_inputs:** []
- **system_state:** handle_doctor receives Args struct with subcommand() method

### Postconditions
**State Changes:**
- doctor check returns health status
- doctor fix attempts repairs
- doctor integrity checks DB
- doctor clean removes temp files

### Invariants
- Legacy 'isolate doctor' still works with deprecation warning

---

## Research Requirements

### Files to Read
1. `handlers/integrity.rs` - Existing patterns
2. `commands/doctor.rs` - Existing patterns
3. `object_commands.rs` - Existing patterns

---

## Acceptance Tests

### Happy Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_doctor_check | Valid inputs | User runs 'isolate doctor check' | Exit code is 0, Output is correct |
| test_doctor_fix | Valid inputs | User runs 'isolate doctor fix' | Exit code is 0, Output is correct |
| test_doctor_integrity | Valid inputs | User runs 'isolate doctor integrity' | Exit code is 0, Output is correct |
| test_doctor_clean | Valid inputs | User runs 'isolate doctor clean' | Exit code is 0, Output is correct |

### Error Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_doctor_invalid | Invalid inputs | User runs 'isolate doctor invalid' | Exit code is non-zero, Error message is clear |
| test_doctor_missing | Missing subcommand | User runs 'isolate doctor' | Exit code is non-zero, Help message shown |

---

## Implementation Tasks

### Phase 0: Research
- [ ] Read handlers/integrity.rs handle_doctor function
- [ ] Document existing patterns in research_notes.md

### Phase 1: Tests First
- [ ] Add subcommand match to handle_doctor
- [ ] Write failing tests for all subcommands

### Phase 2: Implementation
- [ ] Implement: isolate doctor check
- [ ] Implement: isolate doctor fix
- [ ] Implement: isolate doctor integrity
- [ ] Implement: isolate doctor clean

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
- `handlers/integrity.rs:73` - Related implementation
- `commands/doctor.rs` - Related implementation

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
