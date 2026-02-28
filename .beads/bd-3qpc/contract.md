# Contract: handlers: Fix config routing to subcommands

**Bead ID:** isolate-20260228091055-d18wfbda  
**Type:** bug  
**Priority:** P1  
**Effort Estimate:** 2hr

---

## EARS Requirements

### Ubiquitous Language
- THE SYSTEM SHALL route isolate config subcommand to appropriate handler

### Event-Driven Requirements
| Trigger | Shall |
|---------|-------|
| WHEN user runs isolate config list | THE SYSTEM SHALL call config_list handler |
| WHEN user runs isolate config get key | THE SYSTEM SHALL return config value |

### Unwanted Behaviors
| Condition | Shall Not | Because |
|-----------|-----------|----------|
| IF subcommand is missing | THE SYSTEM SHALL NOT fail silently | User needs feedback |

---

## Contracts

### Preconditions
- **auth_required:** false
- **required_inputs:** []
- **system_state:** handle_config receives Args struct with subcommand() method

### Postconditions
**State Changes:**
- config list returns all config
- config get returns key value
- config set updates key
- config schema returns JSON schema

### Invariants
- Legacy isolate config still works with deprecation warning

---

## Research Requirements

### Files to Read
1. `handlers/utility.rs` - Existing patterns
2. `commands/config.rs` - Existing patterns
3. `object_commands.rs` - Existing patterns

---

## Acceptance Tests

### Happy Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_config_list | Valid inputs | User runs 'isolate config list' | Exit code is 0, Output is correct |
| test_config_get | Valid inputs | User runs 'isolate config get key' | Exit code is 0, Output is correct |
| test_config_set | Valid inputs | User runs 'isolate config set key value' | Exit code is 0, Output is correct |
| test_config_schema | Valid inputs | User runs 'isolate config schema' | Exit code is 0, Output is correct |

### Error Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_config_invalid | Invalid inputs | User runs 'isolate config invalid' | Exit code is non-zero, Error message is clear |
| test_config_get_missing | Missing key | User runs 'isolate config get missing_key' | Exit code is non-zero, Error message is clear |

---

## Implementation Tasks

### Phase 0: Research
- [ ] Read handlers/utility.rs handle_config function
- [ ] Document existing patterns in research_notes.md

### Phase 1: Tests First
- [ ] Add subcommand match to handle_config
- [ ] Write failing tests for all subcommands

### Phase 2: Implementation
- [ ] Implement: isolate config list
- [ ] Implement: isolate config get
- [ ] Implement: isolate config set
- [ ] Implement: isolate config schema

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
- `handlers/utility.rs:10` - Related implementation
- `commands/config.rs` - Related implementation

### Similar Implementations
- handle_task uses subcommand() match

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
