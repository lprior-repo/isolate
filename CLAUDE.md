# CLAUDE.md - Project Instructions for Claude Code

## Critical Rules

**7 ABSOLUTE MANDATORY RULES (READ FIRST):**

1. **NO_CLIPPY_EDITS** - NEVER modify `.clippy.toml`, `#![allow]`, `#![deny]`, Cargo.toml lint sections, `moon.yml` lint rules. Fix **code**, not rules.
2. **MOON_ONLY** - NEVER `cargo fmt|clippy|test|build`. ALWAYS `moon run :quick|:test|:build|:ci|:fmt-fix|:check`
3. **CODANNA_ONLY** - NEVER Grep|Glob|Read for exploration. ALWAYS use `mcp__codanna__` tools for code search

### Codanna-Only Enforcement
- BAN these operations outside Codanna: symbol lookup, fuzzy search, call graph (calls/callers), impact analysis, and docs/code discovery via Grep/Glob/Read.
- REQUIRED flow: `codanna index` (or `codanna documents index --collection docs` when docs changed), then `codanna retrieve search|symbol|describe|calls|callers`.
- Default sequence: `codanna retrieve search "<intent>"` -> `codanna retrieve symbol <name>` -> `codanna retrieve calls|callers symbol_id:<id>`.
4. **ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC** - **ZERO unwrap()**, **ZERO unwrap_or()**, **ZERO unwrap_or_else()**, **ZERO unwrap_or_default()**. **ZERO expect()**, **ZERO expect_err()**. **ZERO panic!()**, **ZERO todo!()**, **ZERO unimplemented!()**. **Strictly enforced for `src` code; Permissive for `test` code.**
5. **GIT_PUSH_MANDATORY** - Work NOT done until `git push` succeeds. NEVER stop before pushing. **YOU** must push (not "ready when you are")
6. **BR_SYNC** - `br` never runs git. After `br sync`, manually commit `.beads/` directory
7. **FUNCTIONAL_RUST_SKILL** - **ALWAYS load and use functional-rust-generator skill for ANY Rust implementation**. This skill enforces zero unwrap/expect/panic patterns with Railway-Oriented Programming.

---

### NEVER Touch Clippy/Lint Configuration
**ABSOLUTE RULE: DO NOT MODIFY clippy or linting configuration files. EVER.**

Files: `.clippy.toml`, `clippy.toml`, `#![allow(...)]`, `#![deny(...)]`, `Cargo.toml` lint sections, `moon.yml` lint rules.
**Fix code, not rules.**

### Build System: Moon Only
**NEVER use raw cargo commands.**

```jsonl
{"cmd": "moon run :quick", "use": "Format + lint check (6-7ms cached)"}
{"cmd": "moon run :test", "use": "Run tests (parallel nextest)"}
{"cmd": "moon run :build", "use": "Release build (cached)"}
{"cmd": "moon run :ci", "use": "Full pipeline (parallel)"}
{"cmd": "moon run :fmt-fix", "use": "Auto-fix formatting"}
{"cmd": "moon run :check", "use": "Quick type check"}
{"cmd": "cargo fmt|clippy|test|build", "use": "NEVER - no caching, slow"}
```

### Code Search: Codanna Only (MANDATORY)
**ALWAYS use Codanna. NEVER Grep/Glob.**
**Why:** 90% fewer tokens, semantic understanding, relationship awareness, pre-indexed.

```jsonl
{"tool": "semantic_search_with_context", "use": "NL → full context (callers/callees/impact)", "rust": "mcp__codanna__semantic_search_with_context(query=\"workspace isolation\", limit: 5)"}
{"tool": "semantic_search_docs", "use": "Semantic code search by intent", "rust": "mcp__codanna__semantic_search_docs(query=\"functional JSON parsing\", limit: 10)"}
{"tool": "search_documents", "use": "Full-text markdown docs search", "rust": "mcp__codanna__search_documents(query=\"moon build config\", limit: 5)"}
{"tool": "find_symbol", "use": "Exact symbol lookup by name", "rust": "mcp__codanna__find_symbol(name=\"Workspace\")"}
{"tool": "search_symbols", "use": "Fuzzy pattern search + filters", "rust": "mcp__codanna__search_symbols(query=\"create_workspace\", kind:\"Struct\", lang:\"rust\", limit: 10)"}
{"tool": "get_calls", "use": "What function calls (dependencies)", "rust": "mcp__codanna__get_calls(symbol_id: 123)"}
{"tool": "find_callers", "use": "What calls function (usage)", "rust": "mcp__codanna__find_callers(symbol_id: 456)"}
{"tool": "analyze_impact", "use": "Complete dependency graph", "rust": "mcp__codanna__analyze_impact(symbol_id: 789)"}
{"tool": "get_index_info", "use": "Index statistics", "rust": "mcp__codanna__get_index_info()"}
```

**Workflow:** semantic_search → find_symbol → search_symbols → search_documents → analyze_impact
**NEVER:** Grep ❌ Glob ❌ Read-for-exploration ❌
**Index:** 5,592 symbols, 155 files, 817 doc chunks. Reindex: `codanna index && codanna documents index --collection docs`

### Code Quality (Strict Source / Permissive Tests)
- **Production Code (`src`):** Zero tolerance for `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dead_code`, or `unused_must_use`. Enforced via workspace `deny`.
- **Test Code (`test`):** Pragmatically relaxed via `#![cfg_attr(test, allow(...))]` in crate roots to allow for brutal test scenarios and placeholders.
- **Patterns:** Always prefer `Result<T, Error>`, `map`, `and_then`, and the `?` operator in production code.

### Project Structure
```
crates/zjj-core/    # Core library (error handling, types, functional utils)
crates/zjj/         # CLI binary (MVP: init, add, list, remove, focus)
```

### Key Decisions
- Sync: `jj rebase -d main`
- Zellij tab: `zjj:<session-name>`
- Beads: `.beads/beads.db` hard requirement
- zjj runs inside Zellij: `zellij action go-to-tab-name`

### Dependencies
- JJ (workspace), Zellij (multiplexing), Beads (tracking), SQLite (persistence)

---

## Quick Reference

### Issue Tracking (Beads)
```jsonl
{"cmd": "br ready", "use": "Find available work"}
{"cmd": "br show <id>", "use": "View issue details"}
{"cmd": "br update <id> --status in_progress", "use": "Claim work"}
{"cmd": "br close <id>", "use": "Complete work"}
{"cmd": "br sync", "use": "Sync with git"}
```

### Development (Moon CI/CD)
```jsonl
{"cmd": "moon run :quick", "use": "Fast checks (6-7ms cached)"}
{"cmd": "moon run :ci", "use": "Full pipeline (parallel)"}
{"cmd": "moon run :fmt-fix", "use": "Auto-fix formatting"}
{"cmd": "moon run :build", "use": "Release build"}
{"cmd": "moon run :install", "use": "Install to ~/.local/bin"}
```

### Zellij
```jsonl
{"cmd": "zjj add <name>", "use": "Create session + Zellij tab"}
{"cmd": "zjj focus <name>", "use": "Switch to session tab"}
{"cmd": "zjj remove <name>", "use": "Close tab + workspace"}
{"cmd": "zjj list", "use": "Show all sessions"}
```

**See [docs/11_ZELLIJ.md](docs/11_ZELLIJ.md) for KDL layouts, templates, troubleshooting.**

## CI/CD Pipeline (Moon + bazel-remote: 98.5% faster)

### Performance
- Cached: 6-7ms | Uncached: ~450ms
- Parallel across all crates
- 100GB local cache (persists)
- Zero sudo (systemd user service)

### Workflow
```jsonl
{"step": "1. Quick iteration", "cmd": "moon run :quick", "ms": "6-7"}
{"step": "2. Before commit", "cmd": "moon run :fmt-fix && moon run :ci", "use": "Auto-fix + full pipeline"}
{"step": "3. Cache stats", "cmd": "curl http://localhost:9090/status | jq", "use": "View cache"}
{"step": "4. Restart cache", "cmd": "systemctl --user restart bazel-remote", "use": "If needed"}
```

### Build Rules
**ALWAYS Moon, NEVER cargo:**
- ✅ `moon run :build|:test|:check` (cached, parallel)
- ❌ `cargo build|test` (no cache, slow)

**Why:** Persistent cache, parallel execution, dependency-aware, 98.5% faster.

See [docs/CI-CD-PERFORMANCE.md](docs/CI-CD-PERFORMANCE.md) for benchmarks.

---

## bv AI Sidecar (Beads Graph Analysis)

**Scope:** `bv` handles *what to work on* (triage, priority, planning).

**⚠️ CRITICAL:** Use ONLY `--robot-*` flags. Bare `bv` launches interactive TUI that blocks.

### Entry Point: Triage
**`bv --robot-triage`** returns everything in one call:
- `quick_ref`: counts + top 3 picks
- `recommendations`: ranked items with scores/reasons/unblocks
- `quick_wins`: low-effort high-impact
- `blockers_to_clear`: max downstream unblocks
- `project_health`: status/type/priority distributions, metrics
- `commands`: copy-paste shell commands

```bash
bv --robot-triage                           # MEGA-COMMAND: start here
bv --robot-next                             # Minimal: top pick + claim cmd
bv --robot-triage --format toon             # Token-optimized output
export BV_OUTPUT_FORMAT=toon                # Default to TOON
```

### Commands Reference
```jsonl
{"cmd": "--robot-triage", "returns": "quick_ref, recommendations, quick_wins, blockers_to_clear, project_health, commands"}
{"cmd": "--robot-next", "returns": "single top pick + claim command"}
{"cmd": "--robot-plan", "returns": "parallel execution tracks with unblocks lists"}
{"cmd": "--robot-priority", "returns": "priority misalignment detection with confidence"}
{"cmd": "--robot-insights", "returns": "PageRank, betweenness, HITS, eigenvector, critical path, cycles, k-core, articulation points, slack"}
{"cmd": "--robot-label-health", "returns": "health_level, velocity_score, staleness, blocked_count per label"}
{"cmd": "--robot-label-flow", "returns": "flow_matrix, dependencies, bottleneck_labels"}
{"cmd": "--robot-label-attention [--attention-limit=N]", "returns": "labels ranked by (pagerank × staleness × block_impact) / velocity"}
{"cmd": "--robot-history", "returns": "bead-to-commit correlations: stats, histories, commit_index"}
{"cmd": "--robot-diff --diff-since <ref>", "returns": "new/closed/modified issues, cycles changes"}
{"cmd": "--robot-burndown <sprint>", "returns": "burndown, scope changes, at-risk items"}
{"cmd": "--robot-forecast <id|all>", "returns": "ETA predictions with dependency-aware scheduling"}
{"cmd": "--robot-alerts", "returns": "stale issues, blocking cascades, priority mismatches"}
{"cmd": "--robot-suggest", "returns": "duplicates, missing deps, label suggestions, cycle breaks"}
{"cmd": "--robot-graph [--graph-format=json|dot|mermaid]", "returns": "dependency graph export"}
{"cmd": "--export-graph <file.html>", "returns": "self-contained interactive HTML visualization"}
```

### Scoping & Filtering
```bash
bv --robot-plan --label backend              # Label subgraph
bv --robot-insights --as-of HEAD~30          # Historical point-in-time
bv --recipe actionable --robot-plan          # Ready to work (no blockers)
bv --recipe high-impact --robot-triage       # Top PageRank scores
bv --robot-triage --robot-triage-by-track    # Parallel work streams
bv --robot-triage --robot-triage-by-label    # Group by domain
```

### Output Structure
**All robot JSON includes:**
- `data_hash` — beads.jsonl fingerprint (verify consistency)
- `status` — per-metric: `computed|approx|timeout|skipped` + elapsed ms
- `as_of` / `as_of_commit` — when using `--as-of`: ref + resolved SHA

**Two-phase:**
- Phase 1 (instant): degree, topo sort, density
- Phase 2 (async, 500ms timeout): PageRank, betweenness, HITS, eigenvector, cycles

**Large graphs (>500 nodes):** Some metrics approximated/skipped. Check `status`.

### jq Reference
```bash
bv --robot-triage | jq '.quick_ref'                                  # At-a-glance
bv --robot-triage | jq '.recommendations[0]'                         # Top pick
bv --robot-plan | jq '.plan.summary.highest_impact'                  # Best unblock
bv --robot-insights | jq '.status'                                   # Metric readiness
bv --robot-insights | jq '.Cycles'                                   # Circular deps
bv --robot-label-health | jq '.results.labels[] | select(.health_level == "critical")'
```

**Performance:** Phase 1 instant, Phase 2 async (500ms timeout). Prefer `--robot-plan` over `--robot-insights` for speed. Results cached by data hash.

---

## Parallel Agent Workflow (Orchestration)

**7-Step Pipeline (each agent):**

```jsonl
{"step": "1. TRIAGE", "cmd": "bv --robot-triage --robot-triage-by-track | bv --robot-next", "use": "Find what to work on"}
{"step": "2. CLAIM", "cmd": "br update <bead-id> --status in_progress", "use": "Reserve bead"}
{"step": "3. ISOLATE", "skill": "zjj", "use": "Spawn isolated JJ workspace + Zellij tab"}
{"step": "4. IMPLEMENT", "skill": "functional-rust-generator | tdd15-gleam", "use": "Zero panics/unwraps, ROP"}
{"step": "5. REVIEW", "skill": "red-queen", "use": "Adversarial QA, regression hunting"}
{"step": "6. LAND", "skill": "land", "use": "Moon quick check, commit, sync, push (MANDATORY)"}
{"step": "7. MERGE", "skill": "zjj", "use": "jj rebase -d main, cleanup, tab switch"}
```

### Orchestrator Responsibilities
```jsonl
{"duty": "1. Keep context clean", "action": "Delegate to subagents, don't implement"}
{"duty": "2. Monitor progress", "tool": "TaskOutput", "use": "Check status without full context load"}
{"duty": "3. Handle failures", "action": "Spawn replacement agents"}
{"duty": "4. Track completion", "action": "Verify all 7 steps done"}
{"duty": "5. Report summary", "action": "Final status of all beads"}
```

### Subagent Template
```markdown
**BEAD**: <bead-id> - "<title>"

**WORKFLOW**:
1. CLAIM: `br update <bead-id> --status in_progress`
2. ISOLATE: zjj skill → "<session-name>"
3. IMPLEMENT: functional-rust-generator skill (or tdd15-gleam)
   - Zero unwraps, zero panics
   - Railway-Oriented Programming
   - map, and_then, ? operator
4. REVIEW: red-queen skill (adversarial QA)
5. LAND: land skill (quality gates, sync, push)
6. MERGE: zjj skill (merge to main)

**CONSTRAINTS**: Zero unwraps/panics, Moon only, work NOT done until git push succeeds
```

### Parallel Execution Example
```bash
bv --robot-triage --robot-triage-by-track    # Get parallel tracks
# Spawn 8 agents via Task tool, each gets unique bead from different track
# All run simultaneously in isolated workspaces
# Orchestrator monitors from clean context
```

### Benefits
```jsonl
{"benefit": "Isolation", "desc": "Separate JJ workspace per agent"}
{"benefit": "Parallel", "desc": "8x throughput, no conflicts"}
{"benefit": "Deterministic", "desc": "bv precomputes dependencies/tracks"}
{"benefit": "Quality", "desc": "Red-queen adversarial testing per change"}
{"benefit": "Clean handoff", "desc": "land skill guarantees push before completion"}
```

---

## Landing the Plane (Session Completion)

**When ending work session, MUST complete all steps. Work NOT done until `git push` succeeds.**

### Mandatory Workflow
```jsonl
{"step": "1. File issues", "for": "Remaining work needing follow-up"}
{"step": "2. Quality gates", "if_code_changed": "moon run :quick (6-7ms) OR moon run :ci (full)"}
{"step": "3. Update issues", "action": "Close finished, update in-progress"}
{"step": "4. COMMIT AND PUSH (MANDATORY)", "cmds": ["git add <files>", "git commit -m 'desc'", "br sync", "git pull --rebase", "git push", "git status (must show 'up to date')"]}
{"step": "5. Verify cache", "cmd": "systemctl --user is-active bazel-remote", "expect": "active"}
{"step": "6. Clean up", "actions": "Clear stashes, prune remote branches"}
{"step": "7. Hand off", "provide": "Context for next session"}
```

### Critical Rules
```jsonl
{"rule": "Completion", "text": "Work NOT done until git push succeeds"}
{"rule": "No early stop", "text": "NEVER stop before pushing - leaves work stranded locally"}
{"rule": "No delegation", "text": "NEVER say 'ready to push when you are' - YOU must push"}
{"rule": "Push failures", "text": "Resolve and retry until push succeeds"}
{"rule": "Build system", "text": "Always use Moon, never raw cargo"}
{"rule": "Clippy", "text": "NEVER touch clippy settings EVER"}
```
