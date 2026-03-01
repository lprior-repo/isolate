---
bead_id: bd-3bo
bead_title: dioxus-jsonl: Foundation and Hooks API
phase: p1
updated_at: 2026-03-01T06:00:00Z
---

# Contract: dioxus-jsonl Foundation and Hooks API

## Purpose
Define the skill contract for dioxus-modern with all 20 Dioxus 0.7 hooks signatures.

## Preconditions
- Location /home/lewis/.claude/skills/dioxus-modern/ exists

## Postconditions
- Foundation and Hooks JSONL records written to file
- Valid JSONL format per line

## Invariants
- All hooks must use Dioxus 0.7 API (signals, not use_state)
- Zero unwrap/expect in any generated code

## Acceptance Criteria
1. All 20 Dioxus 0.7 hooks documented with full signatures
2. JSONL output format valid
3. No legacy use_state recommendations

## Functional Requirements
- Use Result<T, Error> throughout
- Zero panic paths
- Railway-oriented programming
