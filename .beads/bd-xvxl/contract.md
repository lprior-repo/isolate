# Contract: test: Verify CLI routing tests pass

**Bead ID:** isolate-20260228085443-8xecddas  
**Type:** task  
**Priority:** P1  
**Effort Estimate:** 1hr

---

## EARS Requirements

### Ubiquitous Language
- THE SYSTEM SHALL pass all CLI parsing tests

### Event-Driven Requirements
| Trigger | Shall |
|---------|-------|
| WHEN tests run | THE SYSTEM SHALL report pass/fail for each test |

### Unwanted Behaviors
| Condition | Shall Not | Because |
|-----------|-----------|----------|
| IF any test fails | THE SYSTEM SHALL NOT proceed to commit | Regression prevention |

---

## Contracts

### Preconditions
- **auth_required:** false
- **required_inputs:** []
- **system_state:** All handler fixes are implemented

### Postconditions
**State Changes:**
- All CLI tests pass
- Object commands work
- Legacy commands work with warnings

### Invariants
- No existing functionality is broken

---

## Research Requirements

### Files to Read
1. `tests/` - Existing patterns
2. `Cargo.toml` - Existing patterns

---

## Acceptance Tests

### Happy Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_cli_tests_pass | All handlers implemented | Tests run | Exit code is 0, All tests pass |

### Error Paths
| Test Name | Given | When | Then |
|-----------|-------|------|------|
| test_test_failure | Test failure | Run tests | Exit code is non-zero, Failure reported |

---

## Implementation Tasks

### Phase 0: Research
- [ ] Check existing test files
- [ ] Document existing patterns in research_notes.md

### Phase 1: Tests First
- [ ] Run moon run :quick
- [ ] Document test status

### Phase 2: Implementation
- [ ] Run moon run :test
- [ ] Fix any failures

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
- `tests/` - Related implementation

### Similar Implementations
- Standard Rust test verification

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
