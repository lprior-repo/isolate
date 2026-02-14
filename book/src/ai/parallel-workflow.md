6. MERGE: zjj skill (merge to main)

**CRITICAL CONSTRAINTS**:
- **ZERO unwrap/expect/panic variants** (see rule 4)
- Zero unwraps/panics, Moon only, work NOT done until git push succeeds
- **ALWAYS use functional-rust-generator skill for Rust** (rule 7)

Report final status with bead ID.
```

### Parallel Execution Example

```bash
# Get parallel tracks
bv --robot-triage --robot-triage-by-track

# Spawn 8 agents via Task tool
# Each gets unique bead from different track
# All run simultaneously in isolated workspaces
# Orchestrator monitors from clean context
```

### Benefits

| Benefit | Description |
|---------|-------------|
| **Isolation** | Each agent works in separate JJ workspace |
| **Parallel** | 8x throughput with no conflicts |
| **Deterministic** | bv precomputes dependencies and execution tracks |
| **Quality** | Red-queen ensures adversarial testing on each change |
| **Clean handoff** | landing-skill guarantees all work pushed before completion |

---

## Session Completion (Landing the Plane)

### CRITICAL: Work is NOT Done Until `git push` Succeeds

**MANDATORY WORKFLOW (All 7 Steps Required)**

```jsonl
{"step": "1", "action": "File issues", "details": "Create issues for anything that needs follow-up", "tool": "br"}
{"step": "2", "action": "Run quality gates", "details": "If code changed: moon run :quick (6-7ms) OR moon run :ci (full)", "tool": "moon"}
{"step": "3", "action": "Update issues", "details": "Close finished work, update in-progress items", "tool": "br"}
{"step": "4", "action": "COMMIT AND PUSH (MANDATORY)", "details": "git add <files> | git commit -m 'desc' | br sync | git pull --rebase | git push | git status (must show 'up to date')", "tool": "git, br"}
{"step": "5", "action": "Verify cache", "details": "systemctl --user is-active bazel-remote (expect 'active')", "tool": "systemctl"}
{"step": "6", "action": "Clean up", "details": "Clear stashes, prune remote branches", "tool": "git"}
{"step": "7", "action": "Hand off", "details": "Provide context for next session", "output": "Summary message"}
```

### Example Session End

```bash
# 1. File any follow-up work
br create "Need to add error handling for edge case"

# 2. Quality gates
moon run :quick

# 3. Update issues
br close 123
br update 456 --status done

# 4. COMMIT AND PUSH (MANDATORY)
git add crates/zjj-core/src/error.rs
git commit -m "fix: add error handling for edge case"
br sync --flush-only
git add .beads/
git commit -m "sync beads"
git pull --rebase
git push
git status # MUST show: "Your branch is up to date with 'origin/main'"

# 5. Verify cache
systemctl --user is-active bazel-remote # Output: active

# 6. Clean up
git stash clear
git remote prune origin

# 7. Hand off
# "Completed bead #123 (error handling). Bead #456 in progress. Cache healthy. No conflicts."
```

### What "Ready to Push When You Are" Means

**DO NOT SAY THIS.** It means:
- You're offloading responsibility
- Work might never get pushed
