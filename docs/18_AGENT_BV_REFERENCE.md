# BV Complete Command Reference

> **🔙 Back to**: [AGENTS.md](../../AGENTS.md) | **📂 agents docs**: [Critical Rules](critical-rules.md) | [Quick Reference](quick-reference.md) | [Project Context](project-context.md) | [Parallel Workflow](parallel-workflow.md) | [Session Completion](session-completion.md) | [BV Reference](bv-reference.md)

---

## Entry Point: Triage

**`bv --robot-triage`** is your single entry point. Returns everything in one call:

- `quick_ref`: at-a-glance counts + top 3 picks
- `recommendations`: ranked actionable items with scores, reasons, unblock info
- `quick_wins`: low-effort high-impact items
- `blockers_to_clear`: items that unblock the most downstream work
- `project_health`: status/type/priority distributions, graph metrics
- `commands`: copy-paste shell commands for next steps

```bash
bv --robot-triadge                           # THE MEGA-COMMAND: start here
bv --robot-next                             # Minimal: just top pick + claim cmd
bv --robot-triage --format toon             # Token-optimized output (TOON)
export BV_OUTPUT_FORMAT=toon                # Default to TOON
```

## All Robot Commands

```jsonl
{"cmd": "--robot-triage", "returns": "quick_ref, recommendations, quick_wins, blockers_to_clear, project_health, commands", "use": "Entry point - find what to work on"}
{"cmd": "--robot-next", "returns": "single top pick + claim command", "use": "Quick pick without full analysis"}
{"cmd": "--robot-plan", "returns": "parallel execution tracks with unblocks lists", "use": "Planning parallel work"}
{"cmd": "--robot-priority", "returns": "priority misalignment detection with confidence", "use": "Check if priorities match dependencies"}
{"cmd": "--robot-insights", "returns": "PageRank, betweenness, HITS, eigenvector, critical path, cycles, k-core, articulation points, slack", "use": "Full graph metrics (slower)"}
{"cmd": "--robot-label-health", "returns": "health_level, velocity_score, staleness, blocked_count per label", "use": "Check label/project health"}
{"cmd": "--robot-label-flow", "returns": "flow_matrix, dependencies, bottleneck_labels", "use": "Cross-label dependency analysis"}
{"cmd": "--robot-label-attention [--attention-limit=N]", "returns": "labels ranked by (pagerank × staleness × block_impact) / velocity", "use": "Which labels need attention most"}
{"cmd": "--robot-history", "returns": "bead-to-commit correlations: stats, histories, commit_index", "use": "Historical analysis"}
{"cmd": "--robot-diff --diff-since <ref>", "returns": "new/closed/modified issues, cycles introduced/resolved", "use": "Changes since a commit"}
{"cmd": "--robot-burndown <sprint>", "returns": "burndown, scope changes, at-risk items", "use": "Sprint tracking"}
{"cmd": "--robot-forecast <id|all>", "returns": "ETA predictions with dependency-aware scheduling", "use": "Timeline estimates"}
{"cmd": "--robot-alerts", "returns": "stale issues, blocking cascades, priority mismatches", "use": "Health check"}
{"cmd": "--robot-suggest", "returns": "duplicates, missing deps, label suggestions, cycle breaks", "use": "Hygiene/cleanup suggestions"}
{"cmd": "--robot-graph [--graph-format=json|dot|mermaid]", "returns": "dependency graph export", "use": "Visualization"}
{"cmd": "--export-graph <file.html>", "returns": "self-contained interactive HTML visualization", "use": "Interactive graph exploration"}
```

## Scoping & Filtering

```bash
bv --robot-plan --label backend              # Scope to label's subgraph
bv --robot-insights --as-of HEAD~30          # Historical point-in-time analysis
bv --recipe actionable --robot-plan          # Pre-filter: ready to work (no blockers)
bv --recipe high-impact --robot-triage       # Pre-filter: top PageRank scores
bv --robot-triage --robot-triage-by-track    # Group by parallel work streams
bv --robot-triage --robot-triage-by-label    # Group by domain/label
```

## Output Structure

**All robot JSON includes:**
- `data_hash` — Fingerprint of source beads.jsonl (verify consistency across calls)
- `status` — Per-metric state: `computed|approx|timeout|skipped` + elapsed ms
- `as_of` / `as_of_commit` — Present when using `--as-of`; contains ref and resolved SHA

**Two-phase analysis:**
- **Phase 1 (instant):** degree, topo sort, density — always available immediately
- **Phase 2 (async, 500ms timeout):** PageRank, betweenness, HITS, eigenvector, cycles — check `status` flags

**For large graphs (>500 nodes):** Some metrics may be approximated or skipped. Always check `status`.

## jq Quick Reference

```bash
# At-a-glance summary
bv --robot-triage | jq '.quick_ref'

# Top recommendation
bv --robot-triage | jq '.recommendations[0]'

# Best unblock target
bv --robot-plan | jq '.plan.summary.highest_impact'

# Check metric readiness
bv --robot-insights | jq '.status'

# Circular dependencies (must fix!)
bv --robot-insights | jq '.Cycles'

# Critical health issues
bv --robot-label-health | jq '.results.labels[] | select(.health_level == "critical")'
```

## Performance Notes

- **Phase 1 instant** - Basic metrics available immediately
- **Phase 2 async (500ms timeout)** - Advanced metrics may take time
- **Prefer `--robot-plan` over `--robot-insights`** when speed matters
- **Results cached by data hash** - Same data = instant replay

## Scope Boundary

**bv handles:** *what to work on* (triage, priority, planning)

## ⚠️ CRITICAL WARNING

**Use ONLY `--robot-*` flags.** Bare `bv` launches an interactive TUI that **blocks your session**.

```bash
# GOOD - robot flag (non-blocking)
bv --robot-triage

# BAD - interactive TUI (BLOCKS SESSION)
bv
```

---

**🔙 Back to**: [AGENTS.md](../../AGENTS.md)
