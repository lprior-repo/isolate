# Project Context

> **🔙 Back to**: [AGENTS.md](../AGENTS.md) | **📂 agents docs**: [Critical Rules](13_AGENT_CRITICAL_RULES.md) | [Quick Reference](14_AGENT_QUICK_REFERENCE.md) | [Parallel Workflow](16_AGENT_PARALLEL_WORKFLOW.md) | [Session Completion](17_AGENT_SESSION_COMPLETION.md) | [BV Reference](18_AGENT_BV_REFERENCE.md)

---

## Structure

```jsonl
{"crates": ["zjj-core (lib)", "zjj (CLI binary)"], "lang": "Rust", "vcs": "Jujutsu (jj)"}
```

**zjj-core:** Error handling, types, functional utils, workspace management core
**zjj:** CLI binary (MVP commands: init, add, list, remove, focus)

## Key Decisions

```jsonl
{"sync": "jj rebase -d main", "rationale": "Keeps main clean, rebases feature branches"}
{"zellij": "zjj:<session-name> tab naming", "rationale": "Easy identification in Zellij"}
{"beads": ".beads/beads.db hard requirement", "rationale": "All work tracked in issues"}
{"zellij_integration": "zellij action go-to-tab-name", "rationale": "Automated tab switching"}
```

## Dependencies

| Tool | Purpose | Link |
|------|---------|------|
| JJ (Jujutsu) | Workspace management | https://martinvonz.github.io/jj/ |
| Zellij | Terminal multiplexing | https://zellij.dev/ |
| Beads (br) | Issue tracking | .beads/README.md |
| SQLite | Session state persistence | sql_schemas/ |
| Moon | Build orchestration | docs/02_MOON_BUILD.md |
| Codanna | Code intelligence | .codanna/settings.toml |

## Performance

```jsonl
{"cache": "bazel-remote 100GB", "check_cmd": "curl localhost:9090/status | jq", "restart": "systemctl --user restart bazel-remote"}
{"moon_cached": "6-7ms", "moon_uncached": "~450ms", "speedup": "98.5%"}
```

## Documentation

- [docs/11_ZELLIJ.md](11_ZELLIJ.md) - Complete Zellij layout configuration, KDL syntax, templates
- [docs/CI-CD-PERFORMANCE.md](CI-CD-PERFORMANCE.md) - Benchmarks and optimization guide
- [docs/INDEX.md](INDEX.md) - Documentation index

---

**🔙 Back to**: [AGENTS.md](../AGENTS.md)
