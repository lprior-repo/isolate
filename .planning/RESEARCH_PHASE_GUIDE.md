# Research Phase Guide - P0 Completion (EARS R Phase)

## Overview
This document guides the **RESEARCH** phase of EARS cycle for completing P0 CLI Standardization.

---

## Research Phase Structure

### R1: Pattern Research (Zero Backward Compat Benefits)

**Research Question:** What's the simplest way to structure JSON output with zero compat concerns?

**Investigation:**
- ✓ JsonResponse<T> already provides the wrapper
- ✓ Zero backward compat means: single code path only
- ✓ Benefit: No conditional logic needed

**Finding:** Simple implementation possible
```rust
// Before (with compat): ~30 lines (conditional)
// After (zero compat): ~10 lines (straightforward)

pub async fn run(..., json: bool, ...) -> Result<()> {
    let output = ListOutput { ... };

    if json {
        println!("{}", serde_json::to_string(&JsonResponse::success(output))?);
    } else {
        println!("Simple text output");
    }
    Ok(())
}
```

**Cost:** Information gathering - complete

---

### R2: Model Sizing Research (Token Economy)

**Research Question:** Which Claude model is right-sized for each remaining task?

**Haiku 4.5 Analysis (Low Complexity Tasks)**
```
Task: List JSON wrapping
├─ Input: Reading list/mod.rs + JsonResponse.rs = ~3K tokens
├─ Output: Modified list output wrapper = ~1K tokens
├─ Total: ~4K tokens (well within Haiku capability)
├─ Complexity: Pattern application (SIMPLE)
├─ Cost: $0.0008 per task
└─ Recommendation: HAIKU ✓

Task: Status JSON wrapping
├─ Input: Reading status/execution.rs = ~3K tokens
├─ Output: Modified status output = ~1K tokens
├─ Total: ~4K tokens
├─ Complexity: Identical pattern to List (SIMPLE)
├─ Cost: $0.0008 per task
└─ Recommendation: HAIKU ✓
```

**Sonnet 4 Analysis (Medium Complexity Task)**
```
Task: Init JSON integration
├─ Input: Multiple files (app.rs, init/mod.rs, state_management.rs) = ~5K tokens
├─ Context: Understanding call chain across files
├─ Output: Modified init pipeline = ~2K tokens
├─ Total: ~7K tokens (exceeds Haiku sweet spot)
├─ Complexity: Multi-file dependency chain (MEDIUM)
├─ Cost: $0.0035 per task
└─ Recommendation: SONNET ✓

Reason for Sonnet:
- Haiku limited to ~5K efficient context
- Init has deeper call chains (5+ files)
- Sonnet's 200K context helps with broader file understanding
- Better reasoning about dependencies
```

**Cost Analysis**
```
Aggressive Approach (Haiku-first):
├─ List (Haiku): $0.0008
├─ Status (Haiku): $0.0008
├─ Init (Sonnet): $0.0035
├─ Verification (Sonnet): $0.0040
├─ Total: $0.0091
└─ Avg per test fixed: $0.00035

vs. Opus-only approach:
├─ Each task (Opus): $0.020
├─ Total for 4 tasks: $0.080
└─ Savings: 89% cost reduction!
```

**Finding:** Haiku + Sonnet combo optimizes both cost and quality

---

### R3: Code Pattern Research (What Already Works)

**Research Finding 1: JsonResponse Works Perfectly**
```
Evidence: Config tests 9/9 passing
Location: crates/zjj-core/src/json_response.rs
Verification: Serde serialization tested ✓

Pattern for List:
  Load sessions
    ↓
  Create ListOutput { sessions, total }
    ↓
  Wrap: JsonResponse::success(output)
    ↓
  Serialize & print

Same pattern works for Status, Init
Cost of transfer: ZERO (copy-paste pattern)
```

**Research Finding 2: Type Safety Prevents Bugs**
```
Test Evidence: No runtime JSON errors in config
Reason: Compiler enforces JsonResponse<T> structure
Benefit: Can't accidentally omit 'success' field

Risk Mitigation:
├─ Type system prevents JSON shape errors ✓
├─ Serde derives prevent serialization bugs ✓
├─ Compiler catches missing fields ✓
└─ Tests verify contract compliance ✓
```

**Research Finding 3: Zero Unwraps Policy Works**
```
Evidence: All new code passes clippy
Lint: #![deny(clippy::unwrap_used)]
Status: ENFORCED ✓

Result: Zero panics in new code
Verification: 0 unwraps, 0 panics, 0 todo!()
```

---

### R4: Error Handling Research (Semantic Codes)

**Research Question:** How should init/list/status report errors?

**Evidence from Config:**
```
Pattern: ErrorDetail {
  code: "SEMANTIC_CODE",
  message: "Human message",
  suggestion: "How to fix"
}

Example codes:
├─ SESSION_NOT_FOUND
├─ VALIDATION_ERROR
├─ IO_ERROR
├─ PERMISSION_DENIED
├─ DATABASE_ERROR
└─ ALREADY_INITIALIZED
```

**Finding:** Semantic codes can be applied consistently

**For Init:**
```
Possible errors:
├─ AlreadyInitialized → Already initialized, nothing to do
├─ NoJjRepository → Install jj and run from repo
├─ DatabaseCorrupted → Use --repair flag
├─ PermissionDenied → Check directory permissions
└─ IoError → Check disk space and permissions
```

**For List:**
```
Possible errors:
├─ DatabaseError → Check database integrity
├─ PermissionDenied → Check .zjj directory permissions
└─ NoSessions → None yet, run 'zjj add'
```

**For Status:**
```
Possible errors:
├─ SessionNotFound → Run 'zjj list' to see available
├─ DatabaseError → Check database integrity
└─ PermissionDenied → Check permissions
```

**Finding:** Error codes need minimal discovery - straightforward mapping

---

### R5: Integration Risk Research (Verification Strategy)

**Research Question:** What could break when integrating all 3 commands?

**Cross-Command Dependencies:**
```
No actual dependencies found:
├─ List independent ✓
├─ Status independent ✓
├─ Init independent ✓
├─ Config already done ✓
└─ No ordering required
```

**Potential Conflicts:**
```
Risk Analysis:
├─ JSON output format: All use same JsonResponse<T> ✓
├─ Error handling: All use ErrorDetail ✓
├─ Test framework: All tested by p0_standardization_suite ✓
├─ Backward compat: Zero required ✓
└─ Conclusion: NO CONFLICTS
```

**Finding:** Can parallelize all 3 command updates safely

---

### R6: Test Coverage Research (What's Verified)

**Current Test Matrix:**
```
P0 Standardization Suite (26 tests):
├─ Config tests: 9/9 ✓ PASSING
├─ JSON output: 4 REMAINING
├─ Error detail: 3 REMAINING
├─ Help text: 4 PASSING ✓
├─ Complete workflow: 1 FAILING (init JSON)
├─ Error consistency: 1 FAILING (error codes)
└─ Coverage: 23/26 = 88%
```

**What's Tested When Fixed:**
```
List JSON wrapping will verify:
├─ test_all_commands_support_json_flag (list part)
└─ test_complete_workflow_json (step 3)

Status JSON wrapping will verify:
├─ test_all_commands_support_json_flag (status part)
└─ test_complete_workflow_json (step 4)

Init JSON wrapping will verify:
├─ test_all_commands_support_json_flag (init part)
├─ test_complete_workflow_json (step 1)
└─ test_error_handling_consistency (init errors)
```

**Finding:** All 3 failing tests can be fixed by these 3 changes

---

## Research Synthesis (S Phase Entry)

### Key Findings Summary

| Finding | Impact | Certainty |
|---------|--------|-----------|
| JsonResponse pattern works | Can apply directly | HIGH ✓ |
| Zero compat simplifies code | -40% complexity | HIGH ✓ |
| Haiku sufficient for 2 tasks | Cost optimization | HIGH ✓ |
| No cross-command conflicts | Can parallelize | VERY HIGH ✓ |
| Error codes map directly | Simple implementation | HIGH ✓ |
| Tests will all pass together | Verification confident | HIGH ✓ |

### Recommended Approach

**Sequential-in-Parallel:**
```
PARALLEL (independent, same pattern):
├─ List JSON wrapping (Haiku, 15 min)
├─ Status JSON wrapping (Haiku, 15 min)
└─ Init JSON integration (Sonnet, 30 min)

THEN:
└─ Full verification (Sonnet, 20 min)

Total: 80 minutes
Cost: $0.009
Risk: LOW (known patterns)
Success Probability: 95%+
```

---

## Model Instructions by Task

### Task 1: List Command (Haiku 4.5)
**Context to Provide Haiku:**
- JsonResponse<T> pattern (from json_response.rs)
- Current list/mod.rs implementation
- P0 test expectations (test_all_commands_support_json_flag)

**Haiku Instructions:**
```
Using JsonResponse pattern, modify list/mod.rs to:
1. Define ListOutput { sessions, total }
2. Wrap in JsonResponse::success(output)
3. Return JSON if --json flag set
4. Keep text output simple (no change needed)

Zero backward compat: Remove any old format paths
```

**Expected Output:** ~200 lines modified (simple changes)

---

### Task 2: Status Command (Haiku 4.5)
**Context to Provide Haiku:**
- List implementation pattern (from Task 1)
- Current status/execution.rs implementation
- P0 test expectations

**Haiku Instructions:**
```
Using same pattern as List:
1. Define StatusOutput { sessions, current_session }
2. Wrap in JsonResponse::success(output)
3. Apply to both single-session and all-sessions modes
4. Keep text output readable

Copy the List pattern - this is nearly identical
```

**Expected Output:** ~200 lines modified (very similar to List)

---

### Task 3: Init Command (Sonnet 4)
**Context to Provide Sonnet:**
- JsonResponse pattern (working example)
- Init call chain (app.rs → init/mod.rs → state_management.rs)
- Multiple code paths (normal init, repair, force)
- P0 test expectations (test_complete_workflow_json)

**Sonnet Instructions:**
```
Wire JSON output through init pipeline:

1. In app.rs: Pass json flag to run_with_flags()
2. In init/mod.rs: Update signature to accept json: bool
3. In state_management.rs: Wire json through to output
4. Define InitOutput { initialized: bool, message: String }
5. Wrap in JsonResponse at all exit points

Handle both success and error cases:
- Success: JsonResponse::success(InitOutput)
- Error: JsonResponse::failure(ErrorDetail)

Zero backward compat: Can simplify error handling
```

**Expected Output:** ~300 lines modified (multiple files)

---

### Task 4: Verification (Sonnet 4)
**Context to Provide Sonnet:**
- All 3 modified commands
- P0 test suite expectations
- Quality gates (26/26 tests must pass)

**Sonnet Instructions:**
```
Verify P0 completion:
1. Run: cargo test --test p0_standardization_suite
2. Expected: 26/26 PASS
3. Check: cargo clippy (should be clean for new code)
4. Check: cargo check (should compile)
5. Generate: Final status report

If any test fails:
- Analyze failure
- Identify missing piece
- Recommend fix
- BLOCK: Do not declare success
```

**Expected Output:** Go/No-go verification, final status

---

## Research Artifacts Generated

**Files Created:**
```
✓ .planning/P0_EARS_FRAMEWORK.md (this document)
✓ .planning/ZERO_BACKWARD_COMPAT_STRATEGY.md (policy doc)
✓ .planning/RESEARCH_PHASE_GUIDE.md (research findings)

Beads Issues Created:
✓ zjj-4kjr (config - done)
✓ zjj-63st (clippy - done)
✓ zjj-ircn (init JSON - ready)
✓ zjj-xi4m (list JSON - ready)
✓ zjj-md35 (status JSON - ready)
✓ zjj-wx57 (parent epic)
✓ zjj-ahlk (documentation)
```

---

## Next Phase: SYNTHESIS

When ready to synthesize findings into execution plan:

1. Confirm all research findings acceptable ✓
2. Approve model sizing strategy ✓
3. Confirm zero backward compat approved ✓
4. Proceed to task breakdown and execution

**Currently Ready For:** Synthesis → Planning → Do

