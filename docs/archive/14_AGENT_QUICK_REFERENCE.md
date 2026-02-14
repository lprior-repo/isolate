# Quick Reference

> **🔙 Back to**: [AGENTS.md](../AGENTS.md) | **📂 agents docs**: [Critical Rules](13_AGENT_CRITICAL_RULES.md) | [Project Context](15_AGENT_PROJECT_CONTEXT.md) | [Parallel Workflow](16_AGENT_PARALLEL_WORKFLOW.md) | [Session Completion](17_AGENT_SESSION_COMPLETION.md) | [BV Reference](18_AGENT_BV_REFERENCE.md)

---

## 90% of Workflows

### Code Search (Codanna)

```jsonl
{"category": "CODE_SEARCH", "tool": "semantic_search_with_context", "when": "Natural language query → full context", "call": "mcp__codanna__semantic_search_with_context(query=\"your intent\", limit: 5)"}
{"category": "CODE_SEARCH", "tool": "find_symbol", "when": "Know exact name", "call": "mcp__codanna__find_symbol(name=\"ExactName\")"}
{"category": "CODE_SEARCH", "tool": "search_symbols", "when": "Fuzzy pattern search", "call": "mcp__codanna__search_symbols(query=\"pattern\", kind:\"Struct|Function|Trait\", lang:\"rust\", limit: 10)"}
{"category": "CODE_SEARCH", "tool": "search_documents", "when": "Search markdown docs", "call": "mcp__codanna__search_documents(query=\"topic\", limit: 5)"}
{"category": "CODE_SEARCH", "tool": "analyze_impact", "when": "Dependency graph", "call": "mcp__codanna__analyze_impact(symbol_id: 123)"}
```

**Workflow:** semantic_search → find_symbol → search_symbols → search_documents → analyze_impact

### Build (Moon)

```jsonl
{"category": "BUILD", "cmd": "moon run :quick", "use": "Fast check (6-7ms cached)", "frequency": "Every edit"}
{"category": "BUILD", "cmd": "moon run :ci", "use": "Full pipeline (parallel)", "frequency": "Before commit"}
{"category": "BUILD", "cmd": "moon run :fmt-fix", "use": "Auto-fix formatting", "frequency": "Before commit"}
```

### Issue Tracking (Beads)

```jsonl
{"category": "ISSUES", "cmd": "bv --robot-triage", "use": "Find what to work on (entry point)", "frequency": "Start of session"}
{"category": "ISSUES", "cmd": "bv --robot-next", "use": "Top pick + claim command", "frequency": "Quick pick"}
{"category": "ISSUES", "cmd": "br ready", "use": "List available work", "frequency": "As needed"}
{"category": "ISSUES", "cmd": "br show <id>", "use": "View issue details", "frequency": "Before claiming"}
{"category": "ISSUES", "cmd": "br update <id> --status in_progress", "use": "Claim work", "frequency": "When starting"}
{"category": "ISSUES", "cmd": "br close <id>", "use": "Complete work", "frequency": "When done"}
```

### Workspace (zjj)

```jsonl
{"category": "WORKSPACE", "cmd": "zjj add <name>", "use": "Create session + Zellij tab", "frequency": "New work"}
{"category": "WORKSPACE", "cmd": "zjj focus <name>", "use": "Switch to session tab", "frequency": "Context switch"}
{"category": "WORKSPACE", "cmd": "zjj remove <name>", "use": "Close tab + workspace", "frequency": "Work complete"}
{"category": "WORKSPACE", "cmd": "zjj list", "use": "Show all sessions", "frequency": "Status check"}
```

### Index (Codanna)

```jsonl
{"category": "INDEX", "cmd": "codanna index && codanna documents index --collection docs", "use": "Reindex codebase", "frequency": "When stale", "stats": "5,592 symbols, 817 doc chunks"}
```

**When to reindex:** Codanna returns no results, search seems outdated, or after large code changes.

---

**🔙 Back to**: [AGENTS.md](../AGENTS.md)
