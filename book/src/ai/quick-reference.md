{"cmd": "moon run :fmt-fix", "use": "Auto-fix formatting", "frequency": "Before commit"}
{"cmd": "moon run :test", "use": "Run tests", "frequency": "After changes"}
{"cmd": "moon run :check", "use": "Type check only", "frequency": "Quick validation"}
```

### Issue Tracking (Beads)

```jsonl
{"cmd": "bv --robot-triage", "use": "Find what to work on (entry point)", "frequency": "Start of session"}
{"cmd": "bv --robot-next", "use": "Top pick + claim command", "frequency": "Quick pick"}
{"cmd": "br ready", "use": "List available work", "frequency": "As needed"}
{"cmd": "br show <id>", "use": "View issue details", "frequency": "Before claiming"}
{"cmd": "br update <id> --status in_progress", "use": "Claim work", "frequency": "When starting"}
{"cmd": "br close <id>", "use": "Complete work", "frequency": "When done"}
```

### Workspace (zjj)

```jsonl
{"cmd": "zjj add <name>", "use": "Create session + Zellij tab", "frequency": "New work"}
{"cmd": "zjj focus <name>", "use": "Switch to session tab", "frequency": "Context switch"}
{"cmd": "zjj remove <name>", "use": "Close tab + workspace", "frequency": "Work complete"}
{"cmd": "zjj list", "use": "Show all sessions", "frequency": "Status check"}
{"cmd": "zjj whereami", "use": "Check current location", "frequency": "Orient yourself"}
{"cmd": "zjj work <name>", "use": "Create workspace (simpler than add)", "frequency": "New work"}
{"cmd": "zjj done", "use": "Complete and merge work", "frequency": "Finish work"}
```

---

## 7-Step Parallel Agent Workflow

Each autonomous agent follows this pipeline:

```jsonl
{"step": "1", "name": "TRIAGE", "cmd": "bv --robot-triage --robot-triage-by-track", "output": "Parallel execution tracks", "tool": "bv"}
{"step": "2", "name": "CLAIM", "cmd": "br update <bead-id> --status in_progress", "output": "Reserve bead", "tool": "br"}
{"step": "3", "name": "ISOLATE", "skill": "zjj", "output": "Spawn isolated JJ workspace + Zellij tab", "tool": "Skill tool"}
{"step": "4", "name": "IMPLEMENT", "skill": "functional-rust-generator (Rust) | tdd15-gleam (Gleam)", "output": "ZERO unwrap/expect/panic, Railway-Oriented Programming", "tool": "Skill tool"}
{"step": "5", "name": "REVIEW", "skill": "red-queen", "output": "Adversarial QA, regression hunting", "tool": "Skill tool"}
{"step": "6", "name": "LAND", "skill": "landing-skill", "output": "Moon quick check, commit, sync, push (MANDATORY)", "tool": "Skill tool"}
{"step": "7", "name": "MERGE", "skill": "zjj", "output": "jj rebase -d main, cleanup, tab switch", "tool": "Skill tool"}
```

### Subagent Template

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
