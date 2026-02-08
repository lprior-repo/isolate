# Parallel Agent Workflow

> **🔙 Back to**: [AGENTS.md](../../AGENTS.md) | **📂 agents docs**: [Critical Rules](critical-rules.md) | [Quick Reference](quick-reference.md) | [Project Context](project-context.md) | [Parallel Workflow](parallel-workflow.md) | [Session Completion](session-completion.md) | [BV Reference](bv-reference.md)

---

## 7-Step Pipeline (Each Autonomous Agent)

```jsonl
{"step": "1", "name": "TRIAGE", "cmd": "bv --robot-triage --robot-triage-by-track", "output": "Parallel execution tracks", "tool": "bv"}
{"step": "2", "name": "CLAIM", "cmd": "br update <bead-id> --status in_progress", "output": "Reserve bead", "tool": "br"}
{"step": "3", "name": "ISOLATE", "skill": "zjj", "output": "Spawn isolated JJ workspace + Zellij tab", "tool": "Skill tool"}
{"step": "4", "name": "IMPLEMENT", "skill": "functional-rust-generator (Rust) | tdd15-gleam (Gleam)", "output": "ZERO unwrap/expect/panic, Railway-Oriented Programming", "tool": "Skill tool", "critical": "ALWAYS use functional-rust-generator for Rust - enforces zero unwrap(), unwrap_or(), unwrap_or_else(), unwrap_or_default(), expect(), expect_err(), panic!(), todo!(), unimplemented!()"}
{"step": "5", "name": "REVIEW", "skill": "red-queen", "output": "Adversarial QA, regression hunting", "tool": "Skill tool"}
{"step": "6", "name": "LAND", "skill": "land", "output": "Moon quick check, commit, sync, push (MANDATORY)", "tool": "Skill tool"}
{"step": "7", "name": "MERGE", "skill": "zjj", "output": "jj rebase -d main, cleanup, tab switch", "tool": "Skill tool"}
```

## Orchestrator Responsibilities

```jsonl
{"duty": "1", "action": "Keep context clean", "method": "Delegate to subagents, don't implement yourself"}
{"duty": "2", "action": "Monitor progress", "method": "Use TaskOutput tool (no full context load)"}
{"duty": "3", "action": "Handle failures", "method": "Spawn replacement agents as needed"}
{"duty": "4", "action": "Track completion", "method": "Verify each agent completes all 7 steps"}
{"duty": "5", "action": "Report summary", "method": "Provide final status of all beads completed"}
```

## Subagent Template

```markdown
**BEAD**: <bead-id> - "<title>"

**WORKFLOW**:
1. CLAIM: `br update <bead-id> --status in_progress`
2. ISOLATE: zjj skill → "<session-name>"
3. IMPLEMENT: functional-rust-generator skill (Rust) or tdd15-gleam skill (Gleam)
   - **ZERO unwrap(), unwrap_or(), unwrap_or_else(), unwrap_or_default()**
   - **ZERO expect(), expect_err()**
   - **ZERO panic!(), todo!(), unimplemented!()**
   - Railway-Oriented Programming
   - map, and_then, ? operator
4. REVIEW: red-queen skill (adversarial QA)
5. LAND: land skill (quality gates, sync, push)
6. MERGE: zjj skill (merge to main)

**CRITICAL CONSTRAINTS**:
- **ZERO unwrap/expect/panic variants** (see rule 4)
- Zero unwraps/panics, Moon only, work NOT done until git push succeeds
- **ALWAYS use functional-rust-generator skill for Rust** (rule 7)

Report final status with bead ID.
```

## Parallel Execution Example

```bash
# Get parallel tracks
bv --robot-triage --robot-triage-by-track

# Spawn 8 agents via Task tool
# Each gets unique bead from different track
# All run simultaneously in isolated workspaces
# Orchestrator monitors from clean context
```

## Benefits

| Benefit | Description |
|---------|-------------|
| **Isolation** | Each agent works in separate JJ workspace |
| **Parallel** | 8x throughput with no conflicts |
| **Deterministic** | bv precomputes dependencies and execution tracks |
| **Quality** | Red-queen ensures adversarial testing on each change |
| **Clean handoff** | land skill guarantees all work pushed before completion |

## Skills Reference

- **functional-rust-generator**: Rust with zero panics, zero unwraps, ROP
- **tdd15-gleam**: 15-phase TDD workflow for Gleam
- **red-queen**: Adversarial evolutionary QA, regression hunting
- **land**: Session completion with quality gates, sync, push
- **zjj**: Workspace isolation and management

---

**🔙 Back to**: [AGENTS.md](../../AGENTS.md)
