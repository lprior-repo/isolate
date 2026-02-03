# AGENTS.md - Agent Instructions for AI Agents

## Critical Rules

### NEVER Touch Clippy/Lint Configuration
**ABSOLUTE RULE: DO NOT MODIFY clippy or linting configuration files. EVER.**

This includes but is not limited to:
- `.clippy.toml`
- `clippy.toml`
- Any `#![allow(...)]` or `#![deny(...)]` attributes in `lib.rs` or `main.rs`
- Clippy-related sections in `Cargo.toml`
- Any lint configuration in `moon.yml` or build scripts

If clippy reports warnings or errors, fix the **code**, not the lint rules.
The user has explicitly configured these rules. Do not second-guess them.

### Build System: Moon Only
**NEVER use raw cargo commands.** Always use Moon for all build operations:

```bash
# Correct
moon run :quick      # Format + lint check
moon run :test       # Run tests
moon run :build      # Release build
moon run :ci         # Full pipeline
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check

# WRONG - Never do this
cargo fmt            # NO
cargo clippy         # NO
cargo test           # NO
cargo build          # NO
```

### Code Quality
- Zero unwraps: `unwrap()` and `expect()` are forbidden
- Zero panics: `panic!`, `todo!`, `unimplemented!` are forbidden
- All errors must use `Result<T, Error>` with proper propagation
- Use functional patterns: `map`, `and_then`, `?` operator

### Project Structure
```
crates/
  zjj-core/     # Core library (error handling, types, functional utils)
  zjj/          # CLI binary (MVP: init, add, list, remove, focus)
```

### Key Decisions
- **Sync strategy**: Rebase (`jj rebase -d main`)
- **Zellij tab naming**: `zjj:<session-name>`
- **Beads**: Hard requirement, always integrate with `.beads/beads.db`
- **zjj runs inside Zellij**: Tab switching via `zellij action go-to-tab-name`

### Dependencies
- JJ (Jujutsu) for workspace management
- Zellij for terminal multiplexing
- Beads for issue tracking integration
- SQLite for session state persistence

---

## Quick Reference

### Issue Tracking (Beads)
```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

### Development (Moon CI/CD)
```bash
moon run :quick       # Fast checks (6-7ms with cache!)
moon run :ci          # Full pipeline (parallel)
moon run :fmt-fix     # Auto-fix formatting
moon run :build       # Release build
moon run :install     # Install to ~/.local/bin
```

### Zellij (Terminal Multiplexing)
```bash
zjj add <name>        # Create session + Zellij tab
zjj focus <name>      # Switch to session tab
zjj remove <name>     # Close tab + workspace
zjj list              # Show all sessions
```

**See [docs/11_ZELLIJ.md](docs/11_ZELLIJ.md) for complete layout configuration, KDL syntax, templates, and troubleshooting.**

## Hyper-Fast CI/CD Pipeline

This project uses **Moon + bazel-remote** for 98.5% faster builds:

### Performance Characteristics
- **6-7ms** for cached tasks (vs ~450ms uncached)
- **Parallel execution** across all crates
- **100GB local cache** persists across sessions
- **Zero sudo** required (systemd user service)

### Development Workflow

**1. Quick Iteration Loop** (6-7ms with cache):
```bash
# Edit code...
moon run :quick  # Parallel fmt + clippy check
```

**2. Before Committing**:
```bash
moon run :fmt-fix  # Auto-fix formatting
moon run :ci       # Full pipeline (if tests pass)
```

**3. Cache Management**:
```bash
# View cache stats
curl http://localhost:9090/status | jq

# Restart cache if needed
systemctl --user restart bazel-remote
```

### Build System Rules

**ALWAYS use Moon, NEVER raw cargo:**
- ✅ `moon run :build` (cached, fast)
- ✅ `moon run :test` (parallel with nextest)
- ✅ `moon run :check` (quick type check)
- ❌ `cargo build` (no caching, slow)
- ❌ `cargo test` (no parallelism)

**Why**: Moon provides:
- Persistent remote caching (survives `moon clean`)
- Parallel task execution
- Dependency-aware rebuilds
- 98.5% faster with cache hits

See [docs/CI-CD-PERFORMANCE.md](docs/CI-CD-PERFORMANCE.md) for benchmarks and optimization guide.

---

## Using bv as an AI Sidecar

bv is a graph-aware triage engine for Beads projects (.beads/beads.jsonl). Instead of parsing JSONL or hallucinating graph traversal, use robot flags for deterministic, dependency-aware outputs with precomputed metrics (PageRank, betweenness, critical path, cycles, HITS, eigenvector, k-core).

**Scope boundary:** bv handles *what to work on* (triage, priority, planning). For agent-to-agent coordination (messaging, work claiming, file reservations), use [MCP Agent Mail](https://github.com/Dicklesworthstone/mcp_agent_mail).

**⚠️ CRITICAL: Use ONLY `--robot-*` flags. Bare `bv` launches an interactive TUI that blocks your session.**

### The Workflow: Start With Triage

**`bv --robot-triage` is your single entry point.** It returns everything you need in one call:
- `quick_ref`: at-a-glance counts + top 3 picks
- `recommendations`: ranked actionable items with scores, reasons, unblock info
- `quick_wins`: low-effort high-impact items
- `blockers_to_clear`: items that unblock the most downstream work
- `project_health`: status/type/priority distributions, graph metrics
- `commands`: copy-paste shell commands for next steps

```bash
bv --robot-triage        # THE MEGA-COMMAND: start here
bv --robot-next          # Minimal: just the single top pick + claim command

# Token-optimized output (TOON) for lower LLM context usage:
bv --robot-triage --format toon
export BV_OUTPUT_FORMAT=toon
bv --robot-next
```

### Other Commands

**Planning:**
| Command | Returns |
|---------|---------|
| `--robot-plan` | Parallel execution tracks with `unblocks` lists |
| `--robot-priority` | Priority misalignment detection with confidence |

**Graph Analysis:**
| Command | Returns |
|---------|---------|
| `--robot-insights` | Full metrics: PageRank, betweenness, HITS (hubs/authorities), eigenvector, critical path, cycles, k-core, articulation points, slack |
| `--robot-label-health` | Per-label health: `health_level` (healthy\|warning\|critical), `velocity_score`, `staleness`, `blocked_count` |
| `--robot-label-flow` | Cross-label dependency: `flow_matrix`, `dependencies`, `bottleneck_labels` |
| `--robot-label-attention [--attention-limit=N]` | Attention-ranked labels by: (pagerank × staleness × block_impact) / velocity |

**History & Change Tracking:**
| Command | Returns |
|---------|---------|
| `--robot-history` | Bead-to-commit correlations: `stats`, `histories` (per-bead events/commits/milestones), `commit_index` |
| `--robot-diff --diff-since <ref>` | Changes since ref: new/closed/modified issues, cycles introduced/resolved |

**Other Commands:**
| Command | Returns |
|---------|---------|
| `--robot-burndown <sprint>` | Sprint burndown, scope changes, at-risk items |
| `--robot-forecast <id\|all>` | ETA predictions with dependency-aware scheduling |
| `--robot-alerts` | Stale issues, blocking cascades, priority mismatches |
| `--robot-suggest` | Hygiene: duplicates, missing deps, label suggestions, cycle breaks |
| `--robot-graph [--graph-format=json\|dot\|mermaid]` | Dependency graph export |
| `--export-graph <file.html>` | Self-contained interactive HTML visualization |

### Scoping & Filtering

```bash
bv --robot-plan --label backend              # Scope to label's subgraph
bv --robot-insights --as-of HEAD~30          # Historical point-in-time
bv --recipe actionable --robot-plan          # Pre-filter: ready to work (no blockers)
bv --recipe high-impact --robot-triage       # Pre-filter: top PageRank scores
bv --robot-triage --robot-triage-by-track    # Group by parallel work streams
bv --robot-triage --robot-triage-by-label    # Group by domain
```

### Understanding Robot Output

**All robot JSON includes:**
- `data_hash` — Fingerprint of source beads.jsonl (verify consistency across calls)
- `status` — Per-metric state: `computed|approx|timeout|skipped` + elapsed ms
- `as_of` / `as_of_commit` — Present when using `--as-of`; contains ref and resolved SHA

**Two-phase analysis:**
- **Phase 1 (instant):** degree, topo sort, density — always available immediately
- **Phase 2 (async, 500ms timeout):** PageRank, betweenness, HITS, eigenvector, cycles — check `status` flags

**For large graphs (>500 nodes):** Some metrics may be approximated or skipped. Always check `status`.

### jq Quick Reference

```bash
bv --robot-triage | jq '.quick_ref'                        # At-a-glance summary
bv --robot-triage | jq '.recommendations[0]'               # Top recommendation
bv --robot-plan | jq '.plan.summary.highest_impact'        # Best unblock target
bv --robot-insights | jq '.status'                         # Check metric readiness
bv --robot-insights | jq '.Cycles'                         # Circular deps (must fix!)
bv --robot-label-health | jq '.results.labels[] | select(.health_level == "critical")'
```

**Performance:** Phase 1 instant, Phase 2 async (500ms timeout). Prefer `--robot-plan` over `--robot-insights` when speed matters. Results cached by data hash.

Use bv instead of parsing beads.jsonl—it computes PageRank, critical paths, cycles, and parallel tracks deterministically.

---

## Parallel Agent Workflow (Orchestration Pattern)

For high-throughput parallel work, use this multi-agent workflow orchestrated through subagents:

### The Complete Pipeline

Each autonomous agent follows this workflow from triage to merge:

```bash
# Step 1: TRIAGE - Find what to work on
bv --robot-triage --robot-triage-by-track  # Get parallel execution tracks
# OR for single issue:
bv --robot-next  # Get top recommendation + claim command

# Step 2: CLAIM - Reserve the bead
bd update <bead-id> --status in_progress

# Step 3: ISOLATE - Create isolated workspace
# Use zjj skill to spawn isolated JJ workspace + Zellij tab
zjj add <session-name>

# Step 4: IMPLEMENT - Build with functional patterns
# For Rust: functional-rust-generator skill
# For Gleam: tdd15-gleam skill (15-phase TDD workflow)
# Implements with: zero panics, zero unwraps, Railway-Oriented Programming

# Step 5: REVIEW - Adversarial QA
# Use red-queen skill for evolutionary testing
# Drives regression hunting and quality gates

# Step 6: LAND - Finalize and push
# Use land skill for mandatory quality gates:
# - Moon quick check (6-7ms cached)
# - git commit with proper message
# - bd sync
# - git push (MANDATORY - work not done until pushed)

# Step 7: MERGE - Reintegrate to main
# Use zjj skill to merge workspace back to main
# This handles: jj rebase -d main, cleanup, tab switching
```

### Orchestrator Responsibilities

As orchestrator, your job is to:
1. **Keep context clean** - Delegate work to subagents, don't implement yourself
2. **Monitor progress** - Use `TaskOutput` to check agent status without loading full context
3. **Handle failures** - Spawn replacement agents if needed
4. **Track completion** - Verify each agent completes all 7 steps
5. **Report summary** - Provide final status of all beads completed

### Subagent Prompt Template

```markdown
You are a parallel autonomous agent. Complete this workflow:

**BEAD TO WORK ON**: <bead-id> - "<title>"

**WORKFLOW**:
1. CLAIM: `bd update <bead-id> --status in_progress`
2. ISOLATE: Use the zjj skill to spawn an isolated workspace named "<session-name>"
3. IMPLEMENT: Use functional-rust-generator skill (or tdd15-gleam for Gleam)
   - Zero unwraps, zero panics
   - Railway-Oriented Programming
   - Functional patterns (map, and_then, ? operator)
4. REVIEW: Use red-queen skill for adversarial QA
5. LAND: Use land skill to finalize (quality gates, sync, push)
6. MERGE: Use zjj skill to merge back to main

**CRITICAL CONSTRAINTS**:
- Zero unwraps, zero panics
- Use Moon for builds (never raw cargo)
- Work is NOT done until git push succeeds

Report your final status with the bead ID.
```

### Parallel Execution Example

```bash
# Run bv triage to get parallel tracks
bv --robot-triage --robot-triage-by-track

# Spawn 8 parallel agents using Task tool
# Each gets unique bead from different track
# All run simultaneously in isolated workspaces
# Orchestrator monitors from clean context
```

### Key Benefits

- **Isolation**: Each agent works in separate JJ workspace
- **Parallel**: 8x throughput with no conflicts
- **Deterministic**: bv precomputes dependencies and execution tracks
- **Quality**: Red-queen ensures adversarial testing on each change
- **Clean handoff**: land skill guarantees all work pushed before completion

---

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed):
   ```bash
   moon run :quick  # Fast check (6-7ms)
   # OR for full validation:
   moon run :ci     # Complete pipeline
   ```
3. **Update issue status** - Close finished work, update in-progress items
4. **COMMIT AND PUSH** - This is MANDATORY:
   ```bash
   git add <files>
   git commit -m "description"
   bd sync  # Sync beads
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Verify cache health**:
   ```bash
   systemctl --user is-active bazel-remote  # Should be "active"
   ```
6. **Clean up** - Clear stashes, prune remote branches
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
- Always use Moon for builds (never raw cargo)
- YOU ARE TO NEVER TOUCH CLIPPY SETTINGS EVER
