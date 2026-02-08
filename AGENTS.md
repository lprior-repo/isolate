# AGENTS.md - Agent Instructions

> **📍 Navigation**: [Index](#agentsmd---agent-instructions) | [Critical Rules](#-critical-rules-read-first) | [Quick Reference](#-quick-reference-90-of-workflows) | [Project Context](#-project-context) | [Parallel Workflow](#-parallel-workflow-7-step-orchestration) | [Session Completion](#-session-completion-mandatory) | [BV Reference](#-bv-complete-reference)
>
> **📂 Full docs**: docs/13-18 (Agent documentation) | **🔙 Back to**: [README.md](README.md) | [docs/](docs/)

---

**Quick index** - Full details in linked files below.

## 🚨 Critical Rules (READ FIRST)

[docs/13_AGENT_CRITICAL_RULES.md](docs/13_AGENT_CRITICAL_RULES.md) - Full details

**7 ABSOLUTE MANDATORY RULES:**

1. **NO_CLIPPY_EDITS** - NEVER modify `.clippy.toml`, `#![allow]`, `#![deny]`, Cargo.toml lint sections. Fix **code**, not rules.
2. **MOON_ONLY** - NEVER `cargo fmt|clippy|test|build`. ALWAYS `moon run :quick|:test|:build|:ci|:fmt-fix|:check`
3. **CODANNA_ONLY** - NEVER Grep|Glob|Read for exploration. ALWAYS use `mcp__codanna__ semantic_search|find_symbol|search_symbols|search_documents|get_calls|find_callers|analyze_impact`
4. **ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC** - **ZERO unwrap()**, **ZERO unwrap_or()**, **ZERO unwrap_or_else()**, **ZERO unwrap_or_default()**. **ZERO expect()**. **ZERO panic!()**, **ZERO todo!()**, **ZERO unimplemented!()**. Required: `Result<T, Error>`, `map`, `and_then`, `?` operator. **Strictly enforced for `src` via workspace `deny`; Permissive for `test` via crate root `allow`.**
5. **GIT_PUSH_MANDATORY** - Work NOT done until `git push` succeeds. NEVER stop before pushing. **YOU** must push (not "ready when you are").
6. **BR_SYNC** - `br` never runs git. After `br sync --flush-only`, manually `git add .beads/ && git commit -m 'sync beads'`
7. **FUNCTIONAL_RUST_SKILL** - **ALWAYS load and use functional-rust-generator skill for ANY Rust implementation**. This skill enforces zero unwrap/expect/panic patterns with Railway-Oriented Programming.

```jsonl
{"rule": "NO_CLIPPY_EDITS", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
{"rule": "MOON_ONLY", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
{"rule": "CODANNA_ONLY", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
{"rule": "ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
{"rule": "GIT_PUSH_MANDATORY", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
{"rule": "BR_SYNC", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
{"rule": "FUNCTIONAL_RUST_SKILL", "severity": "MANDATORY", "link": "docs/13_AGENT_CRITICAL_RULES.md"}
```

## 📚 Quick Reference (90% of workflows)

[docs/14_AGENT_QUICK_REFERENCE.md](docs/14_AGENT_QUICK_REFERENCE.md) - CODE_SEARCH, BUILD, ISSUES, WORKSPACE, INDEX

```jsonl
{"category": "CODE_SEARCH", "tools": ["semantic_search_with_context", "find_symbol", "search_symbols", "search_documents", "analyze_impact"], "link": "docs/14_AGENT_QUICK_REFERENCE.md#code-search-codanna"}
{"category": "BUILD", "cmds": ["moon run :quick", "moon run :ci", "moon run :fmt-fix"], "link": "docs/14_AGENT_QUICK_REFERENCE.md#build-moon"}
{"category": "ISSUES", "cmds": ["bv --robot-triage", "br ready|show|update|close"], "link": "docs/14_AGENT_QUICK_REFERENCE.md#issue-tracking-beads"}
{"category": "WORKSPACE", "cmds": ["zjj add|focus|remove|list"], "link": "docs/14_AGENT_QUICK_REFERENCE.md#workspace-zjj"}
{"category": "INDEX", "cmd": "codanna index && codanna documents index --collection docs", "link": "docs/14_AGENT_QUICK_REFERENCE.md#index-codanna"}
```

## 🏗️ Project Context

[docs/15_AGENT_PROJECT_CONTEXT.md](docs/15_AGENT_PROJECT_CONTEXT.md) - Structure, dependencies, performance

```jsonl
{"structure": "crates/zjj-core (lib) | crates/zjj (CLI binary)", "link": "docs/15_AGENT_PROJECT_CONTEXT.md#structure"}
{"sync": "jj rebase -d main", "link": "docs/15_AGENT_PROJECT_CONTEXT.md#key-decisions"}
{"cache": "bazel-remote 100GB, 6-7ms cached vs 450ms uncached", "link": "docs/15_AGENT_PROJECT_CONTEXT.md#performance"}
```

## 🔄 Parallel Workflow (7-step orchestration)

[docs/16_AGENT_PARALLEL_WORKFLOW.md](docs/16_AGENT_PARALLEL_WORKFLOW.md) - Multi-agent parallel execution pattern

```jsonl
{"pipeline": ["TRIAGE", "CLAIM", "ISOLATE", "IMPLEMENT", "REVIEW", "LAND", "MERGE"], "link": "docs/16_AGENT_PARALLEL_WORKFLOW.md#7-step-pipeline-each-autonomous-agent"}
{"benefits": ["Isolation", "Parallel 8x", "Deterministic", "Quality", "Clean handoff"], "link": "docs/16_AGENT_PARALLEL_WORKFLOW.md#benefits"}
```

## ✅ Session Completion (MANDATORY)

[docs/17_AGENT_SESSION_COMPLETION.md](docs/17_AGENT_SESSION_COMPLETION.md) - Landing the plane, git push is mandatory

```jsonl
{"workflow": ["File issues", "Quality gates", "Update issues", "COMMIT AND PUSH", "Verify cache", "Clean up", "Hand off"], "link": "docs/17_AGENT_SESSION_COMPLETION.md#mandatory-workflow-all-7-steps-required"}
{"critical": "Work NOT done until git push succeeds", "link": "docs/17_AGENT_SESSION_COMPLETION.md#critical-rules"}
```

## 📊 BV Complete Reference

[docs/18_AGENT_BV_REFERENCE.md](docs/18_AGENT_BV_REFERENCE.md) - All 17 `--robot-*` commands, scoping, output structure

```jsonl
{"entry_point": "bv --robot-triage", "returns": "quick_ref, recommendations, quick_wins, blockers, project_health, commands", "link": "docs/18_AGENT_BV_REFERENCE.md#entry-point-triage"}
{"commands": 17, "categories": ["triage", "plan", "insights", "health", "history", "diff", "burndown", "forecast", "alerts", "suggest", "graph"], "link": "docs/18_AGENT_BV_REFERENCE.md#all-robot-commands"}
{"scope_boundary": "bv = what to work on (triage, priority, planning)", "link": "docs/18_AGENT_BV_REFERENCE.md#scope-boundary"}
```

---

## How to Use This

1. **New agent?** Start with [critical-rules.md](docs/13_AGENT_CRITICAL_RULES.md)
2. **Daily work?** Use [quick-reference.md](docs/14_AGENT_QUICK_REFERENCE.md)
3. **Running parallel work?** See [parallel-workflow.md](docs/16_AGENT_PARALLEL_WORKFLOW.md)
4. **Ending session?** Follow [session-completion.md](docs/17_AGENT_SESSION_COMPLETION.md) step-by-step
5. **Need bv details?** Check [bv-reference.md](docs/18_AGENT_BV_REFERENCE.md)

**File convention:** `docs/13_AGENT_*.md` through `docs/18_AGENT_*.md` contains detailed agent documentation. This file (AGENTS.md) is the navigation index.

---

**🔗 Related docs**:
- [CLAUDE.md](CLAUDE.md) - Claude Code project instructions
- [docs/](docs/) - Full documentation index
- [README.md](README.md) - Project overview
