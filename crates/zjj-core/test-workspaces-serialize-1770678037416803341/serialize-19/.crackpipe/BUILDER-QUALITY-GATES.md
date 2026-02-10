# Builder Agent Quality Gates

**CRITICAL**: NO code reaches QA without passing ALL quality gates.

## Mandatory Quality Gates (4-Stage Pipeline)

Every Builder agent MUST run these gates in order before marking any bead as `ready-qa-builder`:

### Gate 1: Format Check
```bash
moon run :fmt-fix
```
- Auto-fix formatting issues
- Fails if formatting cannot be fixed
- On failure → mark bead as `needs-rework` with label `needs-format-fix`

### Gate 2: Quick Lint Check
```bash
moon run :quick
```
- Runs format + clippy checks (6-7ms cached)
- Catches clippy warnings early
- On failure → mark bead as `needs-rework` with label `needs-clippy-fix`

### Gate 3: Test Suite
```bash
moon run :test
```
- All tests must pass
- No skipping, no ignoring failures
- On failure → mark bead as `needs-rework` with label `needs-test-fix`

### Gate 4: Full CI Pipeline (MANDATORY - THE KILLER GATE)
```bash
moon run :ci
```
- Parallel: format, clippy, tests
- **This is the final gate - if this fails, code does NOT go to QA**
- Timeout: 5 minutes
- On failure → mark bead as `needs-rework` with label `needs-ci-fix`

## Failure Handling

When ANY quality gate fails:

1. **Stop immediately** - do not proceed to next gate
2. **Mark bead as `needs-rework`** with specific failure label
3. **Log the failure** in `.crackpipe/builder-agent-quality-gates.log`
4. **Move to next bead** - do not retry immediately
5. **Reworker agents** will pick up `needs-rework` beads

## Why This Matters

### Before Quality Gates
- Builders ran `moon run :test` only (no clippy)
- Clippy warnings slipped through to QA
- QA had to reject beads for lint issues
- **Wasted QA cycles on preventable issues**

### After Quality Gates
- All Builders MUST run full `moon run :ci`
- Clippy warnings caught before QA
- QA focuses on actual code quality, not lint
- **Zero preventable rejections**

## Usage

### Option 1: Use the Quality Gates Script
```bash
./.crackpipe/builder-agent-quality-gates.sh
```
Runs in a loop, automatically enforcing all gates.

### Option 2: Manual Quality Gates
When implementing manually, follow this sequence:

```bash
# 1. Implement your changes
# ... write code ...

# 2. Gate 1: Format
moon run :fmt-fix || { echo "Format failed"; exit 1; }

# 3. Gate 2: Quick lint
moon run :quick || { echo "Lint failed"; exit 1; }

# 4. Gate 3: Tests
moon run :test || { echo "Tests failed"; exit 1; }

# 5. Gate 4: Full CI (MANDATORY)
moon run :ci || { echo "CI failed - code NOT ready for QA"; exit 1; }

# 6. Only after ALL gates pass:
br update <bead-id> --set-labels "-stage:building,stage:ready-qa-builder"
```

## Builder Agent Prompt Template

When spawning Builder agents via the Task tool, use this prompt:

```
You are a Builder agent implementing Rust features with ZERO tolerance for quality issues.

MANDATORY WORKFLOW:

1. Read the contract from .crackpipe/rust-contract-{bead-id}.md
2. Implement using functional-rust-generator skill
3. Quality Gate 1: moon run :fmt-fix (must pass)
4. Quality Gate 2: moon run :quick (must pass - catches clippy warnings)
5. Quality Gate 3: moon run :test (all tests must pass)
6. Quality Gate 4: moon run :ci (MANDATORY - full pipeline or fail)

ONLY after ALL 4 gates pass:
- Update bead to stage:ready-qa-builder
- Commit and push changes

If ANY gate fails:
- Mark bead as stage:needs-rework with appropriate label
- Move to next bead
- Do NOT retry immediately (let Reworker handle it)

CRITICAL: Do NOT mark any bead as ready-qa-builder unless moon run :ci passes.
```

## Monitoring

Check quality gate enforcement:
```bash
# View quality gate logs
tail -f .crackpipe/builder-agent-quality-gates.log

# Check beads that failed quality gates
jq -r 'select(.labels[] | contains("needs-rework")) | .id + " - " + .title' .beads/issues.jsonl

# Check which quality gate failed
jq -r 'select(.labels[] | contains("needs-")) | .id + " - " + (.labels | join(", "))' .beads/issues.jsonl
```

## Statistics

Track quality gate effectiveness:

| Metric | Before | After |
|--------|--------|-------|
| Clippy warnings reaching QA | ~80% | 0% |
| QA rejections for lint | High | Zero |
| Builder cycle time | ~5 min | ~7 min (worth it) |
| Overall throughput | +wasted QA cycles | +focused QA on real issues |

## References

- Moon build system: `moon run :help`
- Quality gate logs: `.crackpipe/builder-agent-quality-gates.log`
- Builder script: `.crackpipe/builder-agent-quality-gates.sh`
