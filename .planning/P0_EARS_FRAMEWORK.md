# P0 CLI Standardization - EARS Framework Continuation

## Executive Summary
**Objective:** Complete remaining 3 P0 integration tests (23/26 → 26/26)
**Current State:** JsonResponse<T> wrapper implemented, config command refactored
**Remaining Work:** Init/List/Status JSON output integration
**Target Timeline:** 1-2 hours total execution
**Quality Gate:** Zero unwraps, 100% test passing, clean clippy

---

## EARS CYCLE FRAMEWORK

### Phase 1: EXAMINE - Current State Assessment

**Examine What Exists:**
```
✓ Working components:
├─ JsonResponse<T> module (crates/zjj-core/src/json_response.rs)
├─ Config command positional arguments working
├─ All config tests passing (9/9)
├─ P0 tests 23/26 passing (88%)
└─ Functional Rust patterns established

✗ Missing components:
├─ Init command JSON output integration
├─ List command JSON response wrapping
├─ Status command JSON response wrapping
└─ Semantic error codes for init/list/status

Error Surface:
├─ test_complete_workflow_json (init JSON)
├─ test_all_commands_support_json_flag (init JSON)
└─ test_error_handling_consistency (semantic codes)
```

**Examine Test Requirements:**
- Init: Expects `{ success: true, ...init_data }`
- List: Expects `{ success: true, sessions: [...] }`
- Status: Expects `{ success: true, ...status_data }`
- All: Must have `success` field at top level

**Examine Code Patterns:**
- Railway-Oriented Programming established
- Type-safe error handling working
- Immutability enforced
- Zero unwraps policy active

---

### Phase 2: ANALYZE - Problem Decomposition

**Analyze Dependencies:**
```
Init Command Flow:
  app.rs (dispatch)
    ↓
  commands/init/mod.rs (run_with_flags)
    ↓
  commands/init/state_management.rs (run_with_cwd_and_flags)
    ↓ [JSON FLAG NEEDED HERE]
  handle_existing_directory() [WRAP OUTPUT HERE]
    ↓
  print! statements [CONVERT TO JsonResponse]

List Command Flow:
  app.rs (dispatch)
    ↓
  commands/list/mod.rs (run)
    ↓ [PARSE JSON FLAG]
  session_data collection
    ↓
  println! output [WRAP IN JsonResponse<ListOutput>]

Status Command Flow:
  app.rs (dispatch)
    ↓
  commands/status/execution.rs (run)
    ↓ [PARSE JSON FLAG]
  status_data collection
    ↓
  println! output [WRAP IN JsonResponse<StatusOutput>]
```

**Analyze Data Flow for Each Command:**

**Init:**
- Input: repair: bool, force: bool, json: bool
- Processing: Initialization logic returns success/error
- Output: JsonResponse<InitOutput>
- Error cases: Already initialized, DB corruption, permission denied

**List:**
- Input: all: bool, json: bool, filters...
- Processing: Query sessions from DB
- Output: JsonResponse<ListOutput { sessions: Vec<Session> }>
- Error cases: DB error, permission denied

**Status:**
- Input: name: Option<&str>, json: bool, watch: bool
- Processing: Query specific session or all sessions
- Output: JsonResponse<StatusOutput { sessions: Vec<SessionStatus> }>
- Error cases: Session not found, DB error

**Analyze Model Requirements:**

By command complexity:
```
SIMPLE (Haiku 4.5)
├─ List JSON wrapping (straightforward iteration)
└─ Status JSON wrapping (similar pattern to list)

MEDIUM (Sonnet 4)
├─ Init JSON integration (dependency chain deeper)
└─ Error semantic codes (requires pattern matching)

COMPLEX (Opus 4.5)
├─ Integration verification (cross-command testing)
└─ Retrospective & optimization (refactoring review)
```

---

### Phase 3: RESEARCH - Investigation & Learning

**Research Question 1: JsonResponse Integration Pattern**
- How to best wire json flag through async call chains?
- Answer: Pass through function signatures, use type inference
- Examples: See config/mod.rs implementation
- Cost: Low (straightforward propagation)

**Research Question 2: Data Type Definitions**
- What fields belong in InitOutput, ListOutput, StatusOutput?
- Answer: Only top-level fields needed (not nested in JsonResponse)
- Pattern: Flatten in serde using #[serde(flatten)]
- Example: See json_response.rs tests

**Research Question 3: Error Handling Strategy**
- How to handle initialization errors in JSON mode?
- Answer: Convert all Result<T> errors to ErrorDetail in JsonResponse
- Pattern: Implement custom error conversion traits
- Scope: Limited to command exit paths

**Research Question 4: Backward Compatibility**
- Will JSON wrapping break text output?
- Answer: Only affects --json flag path, text output unchanged
- Validation: Run full test suite after changes
- Risk: Low (flag-gated changes)

**Research Artifacts:**

```
Model Sizing Research:
═════════════════════

Task: List JSON wrapping
├─ Complexity: Low
├─ Best Model: Haiku 4.5
├─ Reason: Pattern matching, straightforward types
├─ Tokens: ~2K input, ~1K output
└─ Cost: Lowest ($0.001)

Task: Status JSON wrapping
├─ Complexity: Low-Medium
├─ Best Model: Haiku 4.5
├─ Reason: Similar to List, known patterns
├─ Tokens: ~2.5K input, ~1.5K output
└─ Cost: $0.001

Task: Init JSON integration
├─ Complexity: Medium
├─ Best Model: Sonnet 4
├─ Reason: Deeper call chains, multiple code paths
├─ Tokens: ~4K input, ~2K output
└─ Cost: $0.003

Task: Integration testing & verification
├─ Complexity: Medium-High
├─ Best Model: Sonnet 4
├─ Reason: Cross-command interactions, edge cases
├─ Tokens: ~5K input, ~2K output
└─ Cost: $0.004

Total Estimated Cost: ~$0.009 (vs $0.10+ with Opus)
```

---

### Phase 4: SYNTHESIZE - Plan Creation

**Synthesis 1: Integration Strategy**

```
Option A: Parallel Integration (Recommended)
├─ List JSON wrapping (Haiku, 15 min)
├─ Status JSON wrapping (Haiku, 15 min)
└─ Init JSON integration (Sonnet, 30 min)
└─ Verification (Sonnet, 20 min)
Total: 80 minutes

Option B: Sequential Integration
├─ Init JSON (blockers other tests)
├─ List JSON
├─ Status JSON
├─ Full integration test
Total: 90 minutes (less parallelization)

SELECTED: Option A (parallel reduces total time)
```

**Synthesis 2: Type Definitions**

```rust
// Init Output Type
#[derive(Serialize)]
pub struct InitOutput {
    pub initialized: bool,
    pub created_directories: Vec<String>,
    pub message: String,
}

// List Output Type
#[derive(Serialize)]
pub struct ListOutput {
    pub sessions: Vec<SessionSummary>,
    pub total_count: usize,
    pub active_count: usize,
}

// Status Output Type
#[derive(Serialize)]
pub struct StatusOutput {
    pub sessions: Vec<SessionDetail>,
    pub current_session: Option<String>,
}

// All wrapped as:
JsonResponse<InitOutput>
JsonResponse<ListOutput>
JsonResponse<StatusOutput>
```

**Synthesis 3: Error Handling Pattern**

```rust
// Current (wrong):
println!("Error: {}", msg);

// Target (correct):
let error = ErrorDetail::new("ERROR_CODE", msg)
    .with_suggestion("recovery hint");
let response: JsonResponse<InitOutput> = JsonResponse::failure(error);
println!("{}", serde_json::to_string(&response)?);
```

---

## WORKING AGREEMENTS

**Quality Commitments:**
- [ ] Zero unwraps in new code (compiler enforced)
- [ ] Zero panics in new code (compiler enforced)
- [ ] 100% test coverage for new types
- [ ] Type-safe error handling (Result-based)
- [ ] Immutability by default (let, no mut)
- [ ] Railway-Oriented Programming patterns
- [ ] All code is liability reduced (type system guardrails)

**Developer Commitments:**
- [ ] Run `cargo check` before handoff
- [ ] Run `cargo test` before handoff
- [ ] Review clippy output (address new warnings)
- [ ] Verify all P0 tests pass (26/26)
- [ ] Document reasoning in commit messages
- [ ] Mark completed issues in beads

**Retrospective Checkpoints:**
- [ ] What patterns worked well?
- [ ] What could be simplified?
- [ ] Did model selection match task complexity?
- [ ] Were there unexpected complications?
- [ ] Should this become a template for other commands?

---

## DETAILED TASK BREAKDOWN

### Task 1: List Command JSON Wrapping (Haiku 4.5)

**Preconditions:**
- JsonResponse<T> module available ✓
- List command tests passing ✓
- Current output format known ✓

**Steps:**
1. Define ListOutput type (5 min)
2. Modify run() to create ListOutput (5 min)
3. Wrap in JsonResponse::success() (5 min)
4. Test with: `zjj list --json` (5 min)

**Success Criteria:**
- `{ success: true, sessions: [...] }` structure
- `test_all_commands_support_json_flag` passes
- Text output unchanged

**Files to Modify:**
- `crates/zjj/src/commands/list/mod.rs`

**Model Selection Justification:**
- Pattern: Simple data wrapping (known)
- Tokens: ~2K (within Haiku efficiency)
- Cost: Minimal
- Risk: Low (isolated change)

---

### Task 2: Status Command JSON Wrapping (Haiku 4.5)

**Preconditions:**
- ListOutput pattern established ✓
- Status command tests passing ✓

**Steps:**
1. Define StatusOutput type (5 min)
2. Modify run() to create StatusOutput (8 min)
3. Wrap in JsonResponse (5 min)
4. Test: `zjj status session-name --json` (5 min)

**Success Criteria:**
- `{ success: true, ...status_fields }` structure
- `test_all_commands_support_json_flag` passes
- Single session and all-sessions modes work

**Files to Modify:**
- `crates/zjj/src/commands/status/execution.rs`

**Model Selection Justification:**
- Pattern: Identical to ListOutput (copy-paste pattern)
- Tokens: ~2.5K
- Cost: $0.001
- Risk: Very low

---

### Task 3: Init Command JSON Integration (Sonnet 4)

**Preconditions:**
- JsonResponse<T> pattern established ✓
- List/Status wrapping completed ✓
- Init call chain understood ✓

**Steps:**
1. Trace init call chain (5 min)
2. Define InitOutput type (5 min)
3. Wire json flag through state_management.rs (15 min)
4. Wrap output in JsonResponse (10 min)
5. Handle error cases (10 min)
6. Test: `zjj init --json` (10 min)

**Success Criteria:**
- `{ success: true, ...init_fields }` on success
- `{ success: false, error: ErrorDetail }` on error
- `test_all_commands_support_json_flag` passes
- `test_complete_workflow_json` passes (step 1)

**Files to Modify:**
- `crates/zjj/src/app.rs` (pass json flag)
- `crates/zjj/src/commands/init/mod.rs` (signature)
- `crates/zjj/src/commands/init/state_management.rs` (impl)

**Model Selection Justification:**
- Complexity: Medium (deeper call chains)
- Context needed: Understand state_management flow
- Tokens: ~4K (exceeds Haiku sweet spot)
- Better analysis with Sonnet's longer context
- Cost: $0.003 (acceptable for complexity)

---

### Task 4: Verification & Integration Testing (Sonnet 4)

**Preconditions:**
- All 3 commands modified ✓
- Code compiles ✓

**Steps:**
1. Run full test suite: `cargo test --test p0_standardization_suite` (5 min)
2. Verify 26/26 passing (2 min)
3. Check `cargo clippy` output (3 min)
4. Validate error handling coverage (5 min)
5. Generate final report (5 min)

**Success Criteria:**
- 26/26 P0 tests passing
- No clippy warnings (new code)
- No unwraps or panics
- Error cases properly handled

**Model Selection Justification:**
- Cross-command verification
- Edge case analysis
- Pattern validation
- Tokens: ~5K
- Cost: $0.004

---

## CODE LIABILITY ASSESSMENT

**Risk Categories & Mitigation:**

### 1. Type Safety (MITIGATED ✓)
```
Risk: JSON field name mismatches
Severity: High (silent failure at runtime)
Mitigation:
├─ Serde derive ensures correct serialization
├─ JsonResponse<T> generic enforces structure
├─ Compile-time guarantees
└─ Zero runtime validation needed
Status: TYPE SYSTEM ENFORCED
```

### 2. Null/Missing Fields (MITIGATED ✓)
```
Risk: Partial JSON responses
Severity: High (downstream parsing fails)
Mitigation:
├─ Required fields in structs (no Option)
├─ #[serde(skip_serializing_if)] for optionals
├─ Compile-time field checking
└─ Tests validate schema
Status: COMPILE-TIME ENFORCED
```

### 3. Error Handling (MITIGATED ✓)
```
Risk: Uncaught exceptions becoming panics
Severity: Critical
Mitigation:
├─ Result<T, E> everywhere
├─ #![deny(clippy::unwrap_used)]
├─ Explicit error paths
└─ All errors converted to ErrorDetail
Status: COMPILER ENFORCED
```

### 4. State Mutations (MITIGATED ✓)
```
Risk: Hidden side effects, race conditions
Severity: Medium
Mitigation:
├─ Immutability by default (let, no mut)
├─ Pure functions in core
├─ Functional composition
└─ No mutable statics
Status: PATTERN ENFORCED
```

### 5. Documentation Gaps (MITIGATED ✓)
```
Risk: Undocumented contracts, API surprises
Severity: Medium
Mitigation:
├─ Type signatures are documentation
├─ Beads issues document intent
├─ EARS framework traces decisions
├─ Commit messages explain why
Status: TRACKED & DOCUMENTED
```

### 6. Testing Coverage (MONITORED ⚠️)
```
Risk: Edge cases not caught
Severity: Medium
Mitigation:
├─ P0 integration tests as acceptance criteria
├─ Unit tests for types
├─ Boundary value testing in tests
└─ Manual testing before merge
Status: TEST-DRIVEN DEVELOPMENT
```

---

## QUALITY GATES

**Build Gate:**
```bash
✓ cargo check          # Type checking
✓ cargo clippy         # Linting
✓ cargo fmt --check    # Formatting
✓ cargo test           # All tests
```

**Code Quality Gate:**
```
✓ Unwrap count: 0
✓ Panic count: 0
✓ Type safety: 100%
✓ Error handling: Result-based
✓ Test coverage: >80%
```

**Functional Gate:**
```
✓ P0 tests: 26/26 passing
✓ Config tests: 9/9 passing
✓ Unit tests: All passing
✓ Integration: Verified
```

---

## MODEL SIZING REFERENCE

**For Future Work - Model Selection Guide:**

| Complexity | Task Type | Best Model | Token Range | Cost |
|-----------|-----------|-----------|-------------|------|
| Trivial | Bug fixes, typos | Haiku | <1K | $0.0002 |
| Simple | Type definitions, wrapping | Haiku | 1-3K | $0.001 |
| Low | Pattern application, small modules | Haiku | 2-4K | $0.002 |
| Medium | Refactoring, feature implementation | Sonnet | 3-6K | $0.004 |
| High | Complex logic, cross-system changes | Sonnet | 4-8K | $0.006 |
| Very High | Architecture, design, verification | Opus | 6-12K | $0.015 |
| Expert | Deep optimization, novel problems | Opus | 8-16K | $0.025 |

**Cost Optimization:**
- Start with Haiku for well-defined patterns
- Escalate to Sonnet when context complexity exceeds 4K tokens
- Reserve Opus for architectural decisions and complex integrations
- Expected savings: 80% cost reduction vs. Opus-only approach

---

## RETROSPECTIVE TEMPLATE

**After Completion, Answer:**

1. **What worked well?**
   - [ ] Model sizing strategy?
   - [ ] Task breakdown granularity?
   - [ ] Quality gates?
   - [ ] Type safety approach?

2. **What could improve?**
   - [ ] Task estimates accuracy?
   - [ ] Communication clarity?
   - [ ] Error handling patterns?
   - [ ] Test coverage?

3. **Learnings for Next Cycle:**
   - [ ] Patterns to reuse?
   - [ ] Patterns to avoid?
   - [ ] Skills to develop?
   - [ ] Tools to adopt?

4. **Metrics:**
   - Total time: ___ (target: 2 hours)
   - Tests passing: 26/26 ✓
   - Code quality: ___
   - Bugs discovered: ___
   - Issues created: ___

---

## EXECUTION CHECKLIST

**Before Starting:**
- [ ] All beads issues created
- [ ] Haiku/Sonnet quota available
- [ ] No conflicting work in progress
- [ ] Test environment ready

**During Execution:**
- [ ] Document decisions as you go
- [ ] Run quality gates frequently
- [ ] Update beads issue status
- [ ] Capture blockers/learnings

**Before Handoff:**
- [ ] All tests passing (26/26)
- [ ] `cargo check` passes
- [ ] `cargo clippy` clean
- [ ] Retrospective completed
- [ ] Issues marked complete

---

## CONTINUATION COMMAND

To continue from here, execute:

```bash
# Verify current state
cargo test --test p0_standardization_suite 2>&1 | grep "test result"

# Start with List command (Haiku)
# cd crates/zjj/src/commands/list/
# Modify mod.rs to wrap output in JsonResponse<ListOutput>

# Follow same pattern for Status command

# Then wire Init command JSON flag through

# Final verification
cargo test --test p0_standardization_suite
bd close zjj-ircn zjj-xi4m zjj-md35  # Mark complete
```
