#!/usr/bin/env nu
# oc-tdd15.nu — TDD15 Job Definition + Phase Prompts + Gate Evaluators
# Defines per-bead TDD15 workflow as Tork-style job records

# ── Complexity Routes ────────────────────────────────────────────────────────

# All TDD15 phases
const PHASES_COMPLEX = [0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15]
const PHASES_MEDIUM = [0 1 2 4 5 6 7 9 11 15]
const PHASES_SIMPLE = [0 4 5 6 14 15]

# Map complexity label to phase route
export def tdd15-route [complexity: string]: nothing -> list<int> {
  match ($complexity | str downcase) {
    "complex" | "high" => { $PHASES_COMPLEX }
    "medium" | "med" => { $PHASES_MEDIUM }
    "simple" | "low" => { $PHASES_SIMPLE }
    _ => { $PHASES_SIMPLE }
  }
}

# ── Job Definition Builder ───────────────────────────────────────────────────

# Build a complete TDD15 job record for a bead
export def tdd15-job [bead_id: string, --position: int = 0]: nothing -> record {
  {
    name: $"tdd15-($bead_id)"
    position: $position
    inputs: { bead_id: $bead_id }
    defaults: {
      retry: { limit: 3, initial_delay: 1, scaling_factor: 2 }
      timeout: 600
    }
    tasks: [
      {
        name: "triage"
        var: "triage"
        run: "phase-0-triage"
        gate: "complexity_assessed"
      }
      {
        name: "research"
        needs: ["triage"]
        if: "{{ tasks.triage.route contains 1 }}"
        var: "research"
        run: "phase-1-research"
        agent: { type: "explore", model: "haiku" }
        gate: "sufficient_context"
      }
      {
        name: "plan"
        needs: ["research"]
        if: "{{ tasks.triage.route contains 2 }}"
        var: "plan"
        run: "phase-2-plan"
        agent: { type: "plan", model: "sonnet" }
        gate: "plan_verified"
      }
      {
        name: "verify"
        needs: ["plan"]
        if: "{{ tasks.triage.route contains 3 }}"
        run: "phase-3-verify"
        gate: "user_approval"
      }
      {
        name: "red"
        needs: ["triage"]
        var: "red"
        run: "phase-4-red"
        agent: { type: "general-purpose", model: "haiku" }
        gate: "tests_fail"
      }
      {
        name: "green"
        needs: ["red"]
        var: "green"
        run: "phase-5-green"
        agent: { type: "general-purpose", model: "sonnet" }
        gate: "tests_pass"
      }
      {
        name: "refactor"
        needs: ["green"]
        var: "refactor"
        run: "phase-6-refactor"
        agent: { type: "general-purpose", model: "haiku" }
        gate: "tests_green"
      }
      {
        name: "mf1"
        needs: ["refactor"]
        if: "{{ tasks.triage.route contains 7 }}"
        run: "phase-7-mf1"
        agent: { type: "code-reviewer", model: "sonnet" }
        gate: "martin_fowler_1"
        on_fail: { regress_to: "refactor" }
      }
      {
        name: "implement"
        needs: ["mf1"]
        if: "{{ tasks.triage.route contains 8 }}"
        var: "implement"
        run: "phase-8-implement"
        agent: { type: "general-purpose", model: "sonnet" }
        gate: "implementation_complete"
      }
      {
        name: "verify_criteria"
        needs: ["refactor"]
        if: "{{ tasks.triage.route contains 9 }}"
        run: "phase-9-verify-criteria"
        agent: { type: "general-purpose", model: "haiku" }
        gate: "criteria_met"
      }
      {
        name: "fp_gates"
        needs: ["refactor"]
        if: "{{ tasks.triage.route contains 10 }}"
        run: "phase-10-fp-gates"
        agent: { type: "code-reviewer", model: "sonnet" }
        gate: "no_critical_issues"
        on_fail: { regress_to: "green" }
      }
      {
        name: "qa"
        needs: ["refactor"]
        if: "{{ tasks.triage.route contains 11 }}"
        run: "phase-11-qa"
        agent: { type: "general-purpose", model: "haiku" }
        gate: "qa_pass"
        on_fail: { regress_to: "green" }
      }
      {
        name: "mf2"
        needs: ["qa"]
        if: "{{ tasks.triage.route contains 12 }}"
        run: "phase-12-mf2"
        agent: { type: "code-reviewer", model: "opus" }
        gate: "martin_fowler_2"
        on_fail: { regress_to: "refactor" }
      }
      {
        name: "consistency"
        needs: ["mf2"]
        if: "{{ tasks.triage.route contains 13 }}"
        run: "phase-13-consistency"
        agent: { type: "code-reviewer", model: "haiku" }
        gate: "standards_met"
      }
      {
        name: "liability"
        needs: ["refactor"]
        if: "{{ tasks.triage.route contains 14 }}"
        run: "phase-14-liability"
        gate: "minimized"
      }
      {
        name: "landing"
        needs: ["refactor"]
        run: "phase-15-landing"
        gate: "push_succeeded"
      }
    ]
  }
}

# ── Phase Prompt Builders ────────────────────────────────────────────────────
# Each returns a prompt string for the opencode agent

export def prompt-triage [bead_id: string, bead_info: record]: nothing -> string {
  $"# Phase 0: TRIAGE — Bead ($bead_id)

## Bead Details
Title: ($bead_info.title? | default 'unknown')
Type: ($bead_info.type? | default 'unknown')
Priority: ($bead_info.priority? | default 'unknown')
Description: ($bead_info.description? | default 'none')

## Task
Assess complexity of this bead. Classify as SIMPLE, MEDIUM, or COMPLEX.

Output a JSON object:
```json
{\"complexity\": \"SIMPLE|MEDIUM|COMPLEX\", \"route\": [0, 4, 5, 6, 14, 15], \"reasoning\": \"...\"}
```

Criteria:
- SIMPLE: Single function, clear fix, no architectural impact
- MEDIUM: Multiple files, some design needed, limited scope
- COMPLEX: Architectural changes, multiple components, needs research"
}

export def prompt-research [bead_id: string, triage_output: string]: nothing -> string {
  $"# Phase 1: RESEARCH — Bead ($bead_id)

## Prior: Triage
($triage_output)

## Task
Research the codebase to gather context for implementing this bead.
- Find relevant files, functions, and patterns
- Understand existing architecture that this change touches
- Document dependencies and potential impacts

Output: A structured research summary with file paths and key findings."
}

export def prompt-plan [bead_id: string, research_output: string]: nothing -> string {
  $"# Phase 2: PLAN — Bead ($bead_id)

## Prior: Research
($research_output)

## Task
Create an implementation PLAN for this bead.
- List files to create/modify
- Define the approach step by step
- Identify risks and mitigations
- Define acceptance criteria

Output: A detailed implementation plan."
}

export def prompt-verify [bead_id: string, plan_output: string]: nothing -> string {
  $"# Phase 3: VERIFY — Bead ($bead_id)

## Prior: Plan
($plan_output)

## Task
Verify the plan is sound. Check for:
- Missing edge cases
- Architectural conflicts
- Test coverage gaps

Output: APPROVED or list of issues."
}

export def prompt-red [bead_id: string, context: string]: nothing -> string {
  $"# Phase 4: RED — Write Failing Tests — Bead ($bead_id)

## Context
($context)

## Task
Write test(s) that define the expected behavior for this bead.
Tests MUST fail when run (they test functionality not yet implemented).

Rules:
- Use the project's existing test patterns
- Tests must be specific and meaningful
- Run `moon run :test` to confirm tests fail

Output: The test code written and confirmation tests fail."
}

export def prompt-green [bead_id: string, red_output: string]: nothing -> string {
  $"# Phase 5: GREEN — Make Tests Pass — Bead ($bead_id)

## Prior: RED phase tests
($red_output)

## Task
Write the minimum implementation to make ALL tests pass.
- Do NOT over-engineer
- Do NOT add features beyond what tests require
- Run `moon run :test` to confirm tests pass
- Run `moon run :check` to confirm type checking passes

Output: Implementation code and test results showing all pass."
}

export def prompt-refactor [bead_id: string, green_output: string]: nothing -> string {
  $"# Phase 6: REFACTOR — Bead ($bead_id)

## Prior: GREEN phase
($green_output)

## Task
Refactor the implementation for clarity and quality:
- Remove duplication
- Improve naming
- Ensure functional patterns (map, and_then, ? operator)
- Zero unwraps, zero panics
- Run `moon run :quick` to verify formatting + lints

Output: Refactored code and passing quick check."
}

export def prompt-mf1 [bead_id: string, refactor_output: string]: nothing -> string {
  $"# Phase 7: Martin Fowler Review #1 — Bead ($bead_id)

## Prior: REFACTOR phase
($refactor_output)

## Task
Review the code changes as Martin Fowler would:
- Code smells?
- Missing abstractions?
- Unclear intent?
- Duplication?

Verdict: PASS or FAIL with specific issues.
If PASS, output contains 'PASS'.
If FAIL, list concrete issues to fix."
}

export def prompt-implement [bead_id: string, mf1_output: string]: nothing -> string {
  $"# Phase 8: IMPLEMENT — Full Implementation — Bead ($bead_id)

## Prior: MF1 Review
($mf1_output)

## Task
Complete the full implementation incorporating MF1 feedback.
Address any review issues. Ensure all tests still pass.

Output: 'DONE' or 'implemented' when complete."
}

export def prompt-verify-criteria [bead_id: string, context: string]: nothing -> string {
  $"# Phase 9: VERIFY CRITERIA — Bead ($bead_id)

## Context
($context)

## Task
Verify all acceptance criteria are met:
- Original bead requirements satisfied
- Tests cover the requirements
- No regressions introduced

Output: 'criteria met' or list of unmet criteria."
}

export def prompt-fp-gates [bead_id: string]: nothing -> string {
  $"# Phase 10: Functional Programming Gates — Bead ($bead_id)

## Task
Check ALL of these FP gates on the changed code:

1. **Immutability**: No mutable state where immutable suffices
2. **Purity**: Functions are pure where possible
3. **No Panic**: Zero unwrap\(\), expect\(\), panic!, todo!, unimplemented!
4. **Exhaustive Match**: All match arms covered, no wildcards hiding cases
5. **Railway**: Error handling uses Result/? operator, not exceptions

Output: For each gate, PASS or FAIL with details.
If ANY gate has CRITICAL issues, include 'critical' in output."
}

export def prompt-qa [bead_id: string, context: string]: nothing -> string {
  $"# Phase 11: QA — Bead ($bead_id)

## Context
($context)

## Task
Run comprehensive QA:
- Run `moon run :test` — all tests pass?
- Run `moon run :check` — type check clean?
- Run `moon run :quick` — format + lint clean?
- Manual review of edge cases

Output: 'PASS' if all checks pass, 'FAIL' with details otherwise."
}

export def prompt-mf2 [bead_id: string, qa_output: string]: nothing -> string {
  $"# Phase 12: Martin Fowler Review #2 — Bead ($bead_id)

## Prior: QA
($qa_output)

## Task
Final architectural review (Opus-level depth):
- Does the code fit the overall architecture?
- Are abstractions at the right level?
- Is the public API clean?
- Would this pass a senior engineer's review?

Verdict: PASS or FAIL."
}

export def prompt-consistency [bead_id: string, mf2_output: string]: nothing -> string {
  $"# Phase 13: CONSISTENCY — Bead ($bead_id)

## Prior: MF2
($mf2_output)

## Task
Check code consistency with project standards:
- Naming conventions followed
- Error handling patterns consistent
- Module structure matches project patterns
- Documentation where needed (not excessive)

Output: 'consistent' or 'PASS' if standards met."
}

export def prompt-liability [bead_id: string]: nothing -> string {
  $"# Phase 14: LIABILITY — Bead ($bead_id)

## Task
Final liability check:
- Run `moon run :test && moon run :check`
- Verify no security issues introduced
- Verify no breaking changes to public API
- Confirm all changes are intentional

Output: Results of final validation."
}

export def prompt-landing [bead_id: string]: nothing -> string {
  $"# Phase 15: LANDING — Bead ($bead_id)

## Task
Prepare for landing:
- Run `moon run :ci` (full pipeline)
- Stage changed files with git
- Create commit with descriptive message
- Verify clean git status

Output: Commit hash or 'push_succeeded'."
}

# ── Prompt Dispatcher ────────────────────────────────────────────────────────

# Get the prompt for a given phase, with context from prior outputs
export def phase-prompt [phase_name: string, bead_id: string, bead_info: record, prior_outputs: record]: nothing -> string {
  let triage_out = ($prior_outputs.triage? | default "")
  let research_out = ($prior_outputs.research? | default "")
  let plan_out = ($prior_outputs.plan? | default "")
  let red_out = ($prior_outputs.red? | default "")
  let green_out = ($prior_outputs.green? | default "")
  let refactor_out = ($prior_outputs.refactor? | default "")
  let mf1_out = ($prior_outputs.mf1? | default "")
  let implement_out = ($prior_outputs.implement? | default "")
  let qa_out = ($prior_outputs.qa? | default "")
  let mf2_out = ($prior_outputs.mf2? | default "")

  match $phase_name {
    "phase-0-triage" => { prompt-triage $bead_id $bead_info }
    "phase-1-research" => { prompt-research $bead_id $triage_out }
    "phase-2-plan" => { prompt-plan $bead_id $research_out }
    "phase-3-verify" => { prompt-verify $bead_id $plan_out }
    "phase-4-red" => { prompt-red $bead_id $triage_out }
    "phase-5-green" => { prompt-green $bead_id $red_out }
    "phase-6-refactor" => { prompt-refactor $bead_id $green_out }
    "phase-7-mf1" => { prompt-mf1 $bead_id $refactor_out }
    "phase-8-implement" => { prompt-implement $bead_id $mf1_out }
    "phase-9-verify-criteria" => { prompt-verify-criteria $bead_id $implement_out }
    "phase-10-fp-gates" => { prompt-fp-gates $bead_id }
    "phase-11-qa" => { prompt-qa $bead_id $refactor_out }
    "phase-12-mf2" => { prompt-mf2 $bead_id $qa_out }
    "phase-13-consistency" => { prompt-consistency $bead_id $mf2_out }
    "phase-14-liability" => { prompt-liability $bead_id }
    "phase-15-landing" => { prompt-landing $bead_id }
    _ => { $"Unknown phase: ($phase_name)" }
  }
}
