# 12-Step Development Workflow for Bead Issues

## The Complete Loop: Research → Plan → Red → Green → Refactor → Verify → Review → Fowler → Manual → QA → Interrogate → Validate

### Step 1: RESEARCH
**Goal**: Deep understanding of the problem domain and requirements

- [ ] Read the bead issue completely, including all comments and history
- [ ] Identify all related beads/dependencies
- [ ] Research existing patterns in codebase using Codanna
- [ ] Document 5+ key findings about the problem
- [ ] Create RESEARCH.md with findings, patterns, trade-offs

**Output**: RESEARCH.md file with:
- Problem statement (1-2 sentences)
- Key constraints and requirements
- Existing patterns in codebase
- Identified risks and edge cases
- 5+ critical questions answered

---

### Step 2: PLAN
**Goal**: Create detailed, executable implementation plan

- [ ] Use gsd:plan-phase or create PLAN.md manually
- [ ] Break work into atomic tasks (5-15 items)
- [ ] Identify exact files to create/modify
- [ ] Document data flow and dependencies
- [ ] Design error handling strategy
- [ ] Plan test coverage (unit, integration, edge cases)

**Output**: PLAN.md file with:
- Architecture diagram (in text)
- File manifest (create/modify/delete)
- Step-by-step implementation sequence
- Data flow diagrams
- Error handling strategy
- Test plan (specific test cases)

---

### Step 3: VERIFY PLAN
**Goal**: Ensure plan is sound before implementation

- [ ] Review plan against CLAUDE.md requirements
- [ ] Check for zero-unwrap/zero-panic constraints
- [ ] Verify all dependencies are resolvable
- [ ] Validate against existing patterns
- [ ] Ask 5+ clarifying questions about plan
- [ ] Document verification checklist

**Output**: VERIFICATION.md with:
- Checklist of verified constraints
- Question/Answer pairs (5+)
- Risk assessment
- Go/No-go decision

---

### Step 4: RED (Test-Driven Development)
**Goal**: Write comprehensive failing tests first

- [ ] Identify test types needed (unit, integration, edge cases)
- [ ] Write 5-10 failing tests covering:
  - Happy path
  - Error conditions (3+ types)
  - Edge cases (3+ types)
  - Boundary conditions
- [ ] Verify tests fail with clear error messages
- [ ] Document what each test validates

**Output**: Test files with failing tests:
- `tests/test_name_red.rs` or unit test module
- Each test should fail with clear message
- Comments explaining test intent

---

### Step 5: GREEN (Make Tests Pass)
**Goal**: Implement minimum code to pass tests

- [ ] Implement code to make tests pass
- [ ] Focus on correctness, not elegance
- [ ] Pass all RED tests
- [ ] Add implementation comments explaining logic
- [ ] Document any deviations from plan

**Output**: Implementation code that:
- Passes all RED tests
- Has clear comments
- Follows Rust idioms
- Compiles with no warnings

---

### Step 6: REFACTOR
**Goal**: Clean up, optimize, improve code quality

- [ ] Remove duplication (DRY principle)
- [ ] Extract helper functions
- [ ] Improve variable/function names
- [ ] Add documentation comments
- [ ] Apply functional patterns (map, and_then, ?)
- [ ] Verify all tests still pass

**Output**: Refactored code that:
- Is more maintainable
- Has better naming
- Follows codebase patterns
- Passes all tests
- Has comprehensive documentation

---

### Step 7: REVIEW (Self Review)
**Goal**: Catch issues before expert review

- [ ] Review own code for:
  - Correctness
  - Edge cases
  - Performance
  - Security (XSS, injection, etc.)
  - Style consistency
- [ ] Check against CLAUDE.md
- [ ] Verify test coverage
- [ ] Document review findings

**Output**: SELF-REVIEW.md with:
- Code review checklist (✓/✗)
- Issues found and fixed
- Confidence level (1-10)
- Recommendations

---

### Step 8: FOWLER REVIEW
**Goal**: Evaluate against Martin Fowler design patterns

Apply these Fowler principles:
- [ ] **Refactoring Indicators**: Identifying code smells
  - Long methods? Extract method
  - Duplicate code? Extract into utility
  - Complex conditionals? Replace with strategy pattern
- [ ] **Data Clumps**: Are parameters/fields grouped logically?
- [ ] **Temporal Coupling**: Is execution order explicit?
- [ ] **Immutability**: Can values be made immutable?
- [ ] **Composition over Inheritance**: Are patterns correct?
- [ ] **Tell, Don't Ask**: Methods ask instead of tell?
- [ ] **Declarative vs Imperative**: Too imperative?

**Output**: FOWLER-REVIEW.md with:
- Design patterns applied
- Code smells found/fixed
- Architectural improvements
- Trade-offs documented

---

### Step 9: MANUAL VERIFICATION
**Goal**: Actually test it works end-to-end

- [ ] **Manual Run**: Execute the feature in real environment
  - Build with `moon run :build`
  - Test success path manually
  - Test error paths manually (3+ scenarios)
  - Test edge cases manually
- [ ] **Documentation Check**: Does help text work?
- [ ] **Integration Check**: Does it integrate with other commands?
- [ ] **Performance Check**: Does it respond quickly?
- [ ] **Screenshot/Log Output**: Document real behavior

**Output**: MANUAL-VERIFICATION.md with:
- Step-by-step manual test results
- Screenshots/logs of actual execution
- User experience observations
- Issues found during manual testing

---

### Step 10: QA (Quality Assurance)
**Goal**: Comprehensive quality checks

- [ ] **Build**: `moon run :check` (no type errors)
- [ ] **Format**: `moon run :fmt-fix` (code style)
- [ ] **Linting**: `moon run :build` (no clippy warnings)
- [ ] **Tests**: `moon run :test` (all tests pass)
- [ ] **Coverage**: Check test coverage (>80%)
- [ ] **Performance**: Run performance tests
- [ ] **Documentation**: Check docs are complete

**Output**: QA-REPORT.md with:
- Build status (✓/✗)
- Test results (pass/fail count)
- Coverage percentage
- Performance metrics
- Any warnings or issues

---

### Step 11: INTERROGATE LOOP
**Goal**: Deep questioning to validate understanding

Ask yourself (and answer) these questions:

- [ ] **Why this design?** What alternatives were considered?
- [ ] **What breaks it?** What scenarios cause failure?
- [ ] **Edge cases?** What boundary conditions exist?
- [ ] **Performance?** What's the complexity? Is it optimal?
- [ ] **Maintainability?** Will someone understand this in 6 months?
- [ ] **Testing?** What test case did we almost miss?
- [ ] **Security?** Are there any injection/overflow/race conditions?
- [ ] **Integration?** How does this interact with other systems?
- [ ] **Error handling?** Are all error paths covered?
- [ ] **Dependencies?** What could break our dependencies?
- [ ] **Future changes?** What if requirements change?
- [ ] **Backwards compatibility?** Does this break anything?

**Output**: INTERROGATION.md with:
- 12+ Q&A pairs
- Deep insights about implementation
- Discovered improvements
- Risk assessment

---

### Step 12: ISSUES VALIDATION & RICHNESS CHECK
**Goal**: Ensure all identified issues are addressed

- [ ] **Bead Requirements**: Does implementation satisfy bead requirements?
- [ ] **Related Issues**: Are related beads affected?
- [ ] **Issue Richness**: Did we hit diverse scenarios?
  - Happy path ✓
  - Error conditions (3+) ✓
  - Edge cases (3+) ✓
  - Performance scenarios ✓
  - Security scenarios ✓
- [ ] **Stakeholder Check**: Would user be satisfied?
- [ ] **Production Ready**: Can this ship today?

**Output**: VALIDATION.md with:
- Requirement checklist (✓/✗)
- Issues richness assessment
- Production readiness score (1-10)
- Sign-off ready? (YES/NO)

---

## Workflow Output Files

For each bead, you'll produce:

```
workspace/agent-N/
├── RESEARCH.md              (Step 1)
├── PLAN.md                  (Step 2)
├── VERIFICATION.md          (Step 3)
├── tests/test_red.rs        (Step 4)
├── src/implementation.rs    (Step 5-6)
├── SELF-REVIEW.md           (Step 7)
├── FOWLER-REVIEW.md         (Step 8)
├── MANUAL-VERIFICATION.md   (Step 9)
├── QA-REPORT.md             (Step 10)
├── INTERROGATION.md         (Step 11)
└── VALIDATION.md            (Step 12)
```

## Success Criteria

- [ ] All 12 steps completed with outputs
- [ ] All tests passing (`moon run :test`)
- [ ] Code compiles with no warnings
- [ ] Coverage > 80%
- [ ] Manual verification successful
- [ ] Production readiness score ≥ 8/10
- [ ] Can articulate WHY each design choice was made
- [ ] No lingering doubts about implementation

## Key Principles

1. **Test-Driven**: RED before GREEN
2. **Thorough**: Don't skip steps
3. **Documented**: Every step produces written output
4. **Validated**: Manual testing required
5. **Interrogated**: Question everything
6. **Rich**: Cover diverse scenarios
7. **Production-Ready**: Ship-ready code

---

**This workflow ensures bulletproof, well-understood, production-quality implementations.**
