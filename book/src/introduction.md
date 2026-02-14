# ZJJ — Parallel Workspace Isolation + Queue Coordination

<div class="hero">
  <h1>ZJJ</h1>
  <p>Run multiple parallel workstreams (humans or AI agents) against a single repo without stepping on each other.</p>
</div>

## What is ZJJ?

ZJJ combines three powerful tools to enable **safe parallel development**:

- **JJ (Jujutsu) workspaces** for hard isolation
- **Zellij** tabs/sessions for fast context switching  
- A **SQLite-backed state + merge/processing queue** for coordination, retries, and recovery

If you want "run 6–12 coding agents safely" *or* "work on 5 features at once without trashing main," ZJJ is for that.

---

## Why ZJJ Exists

Parallel work is easy to start and hard to finish cleanly:

- 👥 two workers edit the same area → conflicts
- 📝 multiple tasks get half-done → you lose track
- 🤷 "who is working on what?" becomes tribal knowledge
- 🤖 agents can duplicate work or race each other

ZJJ fixes this by making parallelism **explicit, isolated, and coordinated**.

<div class="features">
  <div class="feature">
    <div class="feature-icon">🔒</div>
    <h3>Workspace Isolation</h3>
    <p>Each task gets its own JJ workspace and Zellij tab. No more context switching pain or accidental conflicts.</p>
  </div>
  
  <div class="feature">
    <div class="feature-icon">📋</div>
    <h3>Queue Coordination</h3>
    <p>SQLite-backed merge queue ensures only one worker claims a given entry at a time. Built-in retries and recovery.</p>
  </div>
  
  <div class="feature">
    <div class="feature-icon">⚡</div>
    <h3>Fast Switching</h3>
    <p>Jump between workspaces instantly with Zellij integration. Keep your flow state.</p>
  </div>
  
  <div class="feature">
    <div class="feature-icon">🤖</div>
    <h3>AI-First Design</h3>
    <p>Run 6-12 AI agents in parallel, each in isolated workspaces with queue-based coordination.</p>
  </div>
</div>

---

## Before & After

**Before**: 6 agents race on the same working copy → duplicated effort + conflicts  

**After**: each agent gets an isolated workspace + the queue enforces safe claiming/landing

---

## Mental Model

ZJJ has three core concepts:

| Concept | What It Is | Example |
|---------|-----------|---------|
| **Session** | Named isolated workspace (+ optional bead/issue) + optional Zellij tab | `zjj add auth-refactor --bead BD-123` |
| **Queue entry** | Unit of work tied to a workspace that a worker/agent can claim and process | `zjj queue --add feature-a --bead BD-101 --priority 3` |
| **Done** | Finish the work and land it back to main | `zjj done` |

---

## Quick Example

```bash
# 1) inside a JJ repo
zjj init

# 2) create an isolated session
zjj add auth-refactor --bead BD-123

# 3) jump into it (Zellij tab)
zjj focus auth-refactor

# 4) keep it synced with main
zjj sync auth-refactor

# 5) finish and land the work
zjj done

# 6) optionally clean it up
zjj remove auth-refactor
```

---

## Multi-Agent Example

```bash
# Add multiple work items
zjj queue --add feature-a --bead BD-101 --priority 3
zjj queue --add feature-b --bead BD-102 --priority 5 --agent agent-002

# Start workers (these can be human-driven or agent-driven wrappers)
zjj queue worker --loop
```

ZJJ ensures only one worker claims a given entry at a time, and provides:
- ✅ retries for failures
- ✅ cancel/remove operations
- ✅ reclaiming stale leases when workers crash

---

## Next Steps

<div class="quickstart-cards">
  <div class="card">
    <h4>🚀 Quick Start</h4>
    <p>Get up and running in 5 minutes</p>
    <a href="./quickstart.html">Start Here →</a>
  </div>
  
  <div class="card">
    <h4>📖 User Guide</h4>
    <p>Learn workspace management and queue coordination</p>
    <a href="./guide/workspaces.html">Read the Guide →</a>
  </div>
  
  <div class="card">
    <h4>🤖 AI Agents</h4>
    <p>Set up AI agents for parallel development</p>
    <a href="./ai/overview.html">AI Guide →</a>
  </div>
  
  <div class="card">
    <h4>📚 Reference</h4>
    <p>Complete command and API reference</p>
    <a href="./reference/commands.html">Browse Docs →</a>
  </div>
</div>

---

## Key Features

- ✨ **Isolated Workspaces**: Each task in its own JJ workspace
- 🔄 **Smart Syncing**: Keep workspaces in sync with main
- 📊 **Queue Management**: Claim, process, complete work items
- 🎯 **Bead Integration**: Track issues through the workflow
- 🔍 **Status Visibility**: Know exactly what's happening where
- 🎨 **Zellij Integration**: Fast tab switching between workspaces
- 🛡️ **Recovery**: Built-in corruption detection and recovery
- 🤖 **AI-Ready**: Designed for multi-agent parallel workflows
- 📦 **SQLite Backend**: Reliable, simple, local state management
- 🔐 **Safe Parallelism**: No conflicts, no trampling

---

## Who Should Use ZJJ?

- **Solo developers** managing multiple features simultaneously
- **Teams** coordinating parallel work streams  
- **AI agents** running automated coding workflows
- **Open source maintainers** juggling multiple PRs
- **Anyone** tired of git branch confusion and merge conflicts

---

## License

MIT License - see [LICENSE](https://github.com/lprior-repo/zjj/blob/main/LICENSE) for details.
