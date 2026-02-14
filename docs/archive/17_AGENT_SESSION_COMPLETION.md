# Session Completion (Landing the Plane)

> **🔙 Back to**: [AGENTS.md](../AGENTS.md) | **📂 agents docs**: [Critical Rules](13_AGENT_CRITICAL_RULES.md) | [Quick Reference](14_AGENT_QUICK_REFERENCE.md) | [Project Context](15_AGENT_PROJECT_CONTEXT.md) | [Parallel Workflow](16_AGENT_PARALLEL_WORKFLOW.md) | [BV Reference](18_AGENT_BV_REFERENCE.md)

---

## CRITICAL: Work is NOT Done Until `git push` Succeeds

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

## Critical Rules

```jsonl
{"rule": "Completion", "text": "Work NOT done until git push succeeds"}
{"rule": "No early stop", "text": "NEVER stop before pushing - that leaves work stranded locally"}
{"rule": "No delegation", "text": "NEVER say 'ready to push when you are' - YOU must push"}
{"rule": "Push failures", "text": "If push fails, resolve and retry until it succeeds"}
{"rule": "Build system", "text": "Always use Moon, never raw cargo"}
{"rule": "Clippy", "text": "NEVER touch clippy settings EVER"}
```

## Example Session End

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
br sync
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

## What "Ready to Push When You Are" Means

**DO NOT SAY THIS.** It means:
- You're offloading responsibility
- Work might never get pushed
- Next session starts with stranded commits
- Potential merge conflicts accumulate

**Instead:** Push yourself. Verify `git status` shows "up to date". Only then is work complete.

## Failure Recovery

If `git push` fails:
1. Check network: `ping github.com`
2. Check auth: `git remote -v` && `ssh -T git@github.com`
3. Pull rebase: `git pull --rebase`
4. Resolve conflicts if any
5. Push again: `git push`
6. Repeat until success
7. Only then report completion

## Cache Health

Cache must be active for fast builds:
```bash
systemctl --user is-active bazel-remote # Should output: active
curl http://localhost:9090/status | jq  # Should show cache stats
```

If inactive: `systemctl --user start bazel-remote`

---

**🔙 Back to**: [AGENTS.md](../AGENTS.md)
