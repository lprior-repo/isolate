# Ralph Loop Iteration 14 - Planning

**Date:** 2026-01-16
**Focus:** Phase 8/9 (AI-Native Features) - Help Text Optimization
**Status:** PLANNING
**Priority:** P3 (unblocked enhancement work)

---

## Context

**Previous Iterations:**
- Iterations 1-11: Technical debt cleanup COMPLETE
- Iterations 12-13: Machine-readable exit codes COMPLETE (zjj-8en6)
- Current: Continue Phase 8/9 AI-native enhancements

**Available Work:**
- P2: zjj-2a4, zjj-so2 (BLOCKED - require profiling)
- P3: zjj-g80p, zjj-bjoj, zjj-t157 (UNBLOCKED)
- P3: zjj-eca, P4: zjj-im1 (documentation)

---

## Selected Work: zjj-g80p (Help Text for AI Parsing)

**Bead:** zjj-g80p
**Priority:** P3
**Type:** Feature (AI-native documentation)
**Status:** Open, ready to work

**Description:**
"Help text must be AI-parseable: structured format, examples included, clear parameter descriptions. Add --help-json for machine-readable help. Success: AI can understand command usage from help."

**Why This Work:**
1. Natural follow-up to exit codes (just added help text)
2. Builds on Phase 8 AI-native foundation
3. No blockers or dependencies
4. Improves AI agent experience
5. Complements existing JSON output modes

---

## Current Help Text Analysis

**Existing Help Implementation:**
- Uses clap builder API (not derives)
- Basic help text for each command
- Recently added exit code documentation (Iteration 13)
- Existing JSON modes for command output

**Gaps for AI Parsing:**
1. Help text not available in machine-readable format
2. Examples embedded in string, not structured
3. Parameter descriptions not extractable programmatically
4. No schema for command structure

---

## Implementation Plan

### Option 1: --help-json Flag (Recommended)
Add machine-readable help output that AI agents can parse.

**Approach:**
1. Add `--help-json` flag to main CLI and all subcommands
2. Create structured help representation
3. Include examples, parameters, exit codes in JSON schema
4. Keep existing text help for humans

**Benefits:**
- Clean separation of human vs machine help
- Structured data easy for AI to parse
- Can include more detail than text help
- Backwards compatible

**Effort:** 2-3 hours

### Option 2: Improve Text Help Structure
Enhance existing help text to be more parseable.

**Approach:**
1. Add more structured sections (USAGE, EXAMPLES, etc.)
2. Consistent formatting across all commands
3. More detailed parameter descriptions
4. Clear exit code tables

**Benefits:**
- Works with existing --help
- Human-readable remains primary
- No new flags needed

**Drawbacks:**
- Still requires text parsing by AI
- Limited structure
- Harder to maintain consistency

**Effort:** 1-2 hours

### Option 3: Combined Approach
Improve text help AND add --help-json.

**Approach:**
- Enhance text help structure (Option 2)
- Add --help-json for AI agents (Option 1)
- Best of both worlds

**Effort:** 3-4 hours

---

## Recommended Approach

**Start with Option 1 (--help-json):**

Rationale:
- Most valuable for AI agents
- Clean, structured data
- Complements existing --json for outputs
- Can iterate on text help separately

**Implementation Steps:**

1. **Design JSON Schema**
   - Command metadata (name, version, description)
   - Subcommands list
   - Parameters (flags, args, types, defaults)
   - Examples with descriptions
   - Exit codes mapping
   - Related commands

2. **Implement --help-json in main.rs**
   - Detect --help-json flag early
   - Extract command metadata from clap builder
   - Serialize to structured JSON
   - Output and exit

3. **Add Subcommand Help JSON**
   - Each subcommand can output its own help-json
   - Include command-specific examples
   - Parameter details with constraints

4. **Documentation**
   - Update help text to mention --help-json
   - Document JSON schema
   - Add examples for AI agents

5. **Testing**
   - Verify JSON output is valid
   - Ensure completeness of metadata
   - Test for all commands

---

## Success Criteria

- [ ] --help-json flag outputs valid JSON
- [ ] JSON includes all command metadata
- [ ] Examples are structured and complete
- [ ] Exit codes documented in JSON
- [ ] All subcommands support --help-json
- [ ] Documentation updated
- [ ] Tests verify JSON structure
- [ ] zjj-g80p bead can be closed

---

## Related Work

**zjj-bjoj:** Also about help text for AI parsing
- Check if this is duplicate of zjj-g80p
- May be same work or complementary
- Will assess after zjj-g80p

**zjj-t157:** Output composability
- Related but different focus (command output vs help)
- Good follow-up work after help text

---

## Iteration Goal

**Complete zjj-g80p:** Implement --help-json for machine-readable help

Estimated effort: 2-3 hours
Expected outcome: AI agents can parse command structure programmatically

---

**Status:** READY TO BEGIN
**Next:** Implement --help-json flag and structured help output
