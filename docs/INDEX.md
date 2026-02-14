# ZJJ Complete Documentation Index

All ZJJ documentation in one place, token-efficient and searchable.

---

## 🚀 Start Here

| Document | Purpose | Read Time |
|----------|---------|-----------|
| **[00_START_HERE.md](00_START_HERE.md)** | 5-minute crash course + navigation | 5 min |
| **[AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md)** | Complete AI agent reference (rules, workflow, completion) | 20 min |

---

## 📚 Core Documentation (00-12)

| # | Document | Purpose | Read Time |
|---|----------|---------|-----------|
| **00** | [START HERE](00_START_HERE.md) | 5-minute crash course + navigation | 5 min |
| **01** | [ERROR HANDLING](01_ERROR_HANDLING.md) | Fallible operations, Result patterns | 20 min |
| **02** | [MOON BUILD](02_MOON_BUILD.md) | Building, testing, caching | 15 min |
| **03** | [WORKFLOW](03_WORKFLOW.md) | Daily dev workflow (Beads + jj + Moon) | 20 min |
| **04** | [FUNCTIONAL PATTERNS](04_FUNCTIONAL_PATTERNS.md) | Iterator combinators, FP techniques | 25 min |
| **05** | [RUST STANDARDS](05_RUST_STANDARDS.md) | Zero unwrap/panic law + enforcement | 20 min |
| **06** | [COMBINATORS](06_COMBINATORS.md) | Complete combinator reference | Reference |
| **07** | [TESTING](07_TESTING.md) | Testing without panics | 15 min |
| **08** | [BEADS](08_BEADS.md) | Issue tracking, triage, graph metrics | 25 min |
| **09** | [JUJUTSU](09_JUJUTSU.md) | Version control, stacking commits | 20 min |
| **10** | [MOON CICD INDEXED](10_MOON_CICD_INDEXED.md) | Complete moon task catalog (indexed) | Reference |
| **11** | [ZELLIJ](11_ZELLIJ.md) | Terminal multiplexing, layouts, tab management | 25 min |

---

## 🤖 AI Agent Documentation

| Document | Purpose | Read Time |
|----------|---------|-----------|
| **[AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md)** | Complete AI agent guide (replaces 13-18) | 20 min |

**What's in it:**
- 7 Absolute Mandatory Rules
- Quick Reference (90% of workflows)
- 7-Step Parallel Agent Workflow
- Session Completion (Landing the Plane)
- Environment Variables, JSON Output, Error Handling
- Common Patterns, Skills Reference

---

## 🔧 Operational Guides

| Document | Purpose | Read Time |
|----------|---------|-----------|
| [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md) | Debug and resolve specific errors | 15 min |
| [QUEUE_OPERATIONS_RUNBOOK.md](QUEUE_OPERATIONS_RUNBOOK.md) | Queue management SOP + remediation | 20 min |
| [ROLLOUT_ROLLBACK.md](ROLLOUT_ROLLBACK.md) | Phased deployment and rollback playbook | 15 min |
| [CI-CD-PERFORMANCE.md](CI-CD-PERFORMANCE.md) | Build system performance metrics | 15 min |
| [ASYNC-PATTERNS.md](ASYNC-PATTERNS.md) | Async/await patterns for contributors | 20 min |

---

## 🔬 Advanced Topics

| Document | Purpose | Read Time |
|----------|---------|-----------|
| [19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md) | Code search metrics, benchmarks, optimization | 20 min |

---

## 🗂️ Archived Documentation

Historical and obsolete documents are in `docs/archive/`:
- CI governance reports (bd-4vp deliverables)
- Old agent docs (13-18, replaced by AI_AGENT_GUIDE.md)
- Testing debt matrices (historical)
- Performance docs (superseded by CI-CD-PERFORMANCE.md)
- Navigation docs (merged into main docs)
- JSON standardization (implementation detail)

---

## 🎯 Quick Navigation by Task

### I'm New Here
1. Read [00_START_HERE.md](00_START_HERE.md) (5 min)
2. Read [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) (20 min)
3. Read [02_MOON_BUILD.md](02_MOON_BUILD.md) (15 min)
4. Bookmark [06_COMBINATORS.md](06_COMBINATORS.md) for reference

### I'm an AI Agent
1. Read [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) (20 min) - **Everything you need**
2. Bookmark [08_BEADS.md](08_BEADS.md) for bead commands
3. Reference [02_MOON_BUILD.md](02_MOON_BUILD.md) for build commands

### I'm Working on a Feature
1. See [08_BEADS.md](08_BEADS.md) - `bv --robot-triage` (pick task)
2. See [03_WORKFLOW.md](03_WORKFLOW.md) - Daily workflow
3. See [02_MOON_BUILD.md](02_MOON_BUILD.md) - Testing
4. See [09_JUJUTSU.md](09_JUJUTSU.md) - Committing & pushing

### How Do I Handle Errors?
→ [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 10 patterns with examples  
→ [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md) - Debug and resolve specific errors

### How Do I Roll Out Changes Safely?
→ [ROLLOUT_ROLLBACK.md](ROLLOUT_ROLLBACK.md) - phased rollout and rollback instructions

### What Combinators Can I Use?
→ [06_COMBINATORS.md](06_COMBINATORS.md) - Complete reference

### How Do I Build/Test?
→ [02_MOON_BUILD.md](02_MOON_BUILD.md) - Commands and workflow  
→ [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md) - Complete task catalog

### How Do I Operate the Queue?
→ [QUEUE_OPERATIONS_RUNBOOK.md](QUEUE_OPERATIONS_RUNBOOK.md) - Queue SOP + remediation matrix

### What Are the Rules?
→ [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - The law of zero panics  
→ [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) - 7 Absolute Mandatory Rules

### How Do I Use Functional Programming?
→ [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) - FP techniques

### How Do I Pick Work?
→ [08_BEADS.md](08_BEADS.md) - Using `bv` for triage

### How Do I Commit & Push?
→ [09_JUJUTSU.md](09_JUJUTSU.md) - Version control

### How Do I Test Code?
→ [07_TESTING.md](07_TESTING.md) - Testing patterns

### How Do I Search Code?
→ [19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md) - Code search metrics and optimization  
→ [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) - Codanna quick reference

---

## 📚 By Topic

### The Core Law
- [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - No unwrap, no panic, no unsafe
- [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) - Rule 4: ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC

### Error Handling (The Most Important)
- [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 10 patterns + examples
- [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md) - Debug and resolve specific errors
- [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - Requirements + enforcement
- [07_TESTING.md](07_TESTING.md) - Testing error paths

### Building & Testing
- [02_MOON_BUILD.md](02_MOON_BUILD.md) - Moon build system + caching (user guide)
- [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md) - Complete moon task catalog (reference)
- [07_TESTING.md](07_TESTING.md) - Testing strategy
- [CI-CD-PERFORMANCE.md](CI-CD-PERFORMANCE.md) - Performance metrics

### Functional Programming
- [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) - FP patterns + libraries
- [06_COMBINATORS.md](06_COMBINATORS.md) - Combinator reference

### Development Tools
- [03_WORKFLOW.md](03_WORKFLOW.md) - Daily workflow integration
- [08_BEADS.md](08_BEADS.md) - Issue tracking + triage
- [09_JUJUTSU.md](09_JUJUTSU.md) - Version control
- [11_ZELLIJ.md](11_ZELLIJ.md) - Terminal multiplexing + layouts
- [19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md) - Code search metrics and optimization

### Operations & Deployment
- [QUEUE_OPERATIONS_RUNBOOK.md](QUEUE_OPERATIONS_RUNBOOK.md) - Queue operational procedures
- [ROLLOUT_ROLLBACK.md](ROLLOUT_ROLLBACK.md) - Deployment phases and rollback triggers
- [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md) - Troubleshooting guide

---

## 🔑 Key Commands Quick Reference

### Beads (Issues)
```bash
bv --robot-triage        # Get recommendations
br claim BD-123          # Start working (deprecated, use br update)
br update BD-123 --status in_progress  # Claim work
br close BD-123          # Mark done
br sync --flush-only     # Sync bead state (remember to commit!)
```

### Moon (Build)
```bash
moon run :ci    # Full pipeline
moon run :test  # Tests only
moon run :quick # Lint only
moon run :build # Release build
moon run :check # Type check
moon run :fmt-fix  # Auto-fix formatting
```

### Jujutsu (VCS)
```bash
jj describe -m "feat: description"  # Commit
jj git push                         # Push
jj new                              # Start new change
jj log                              # View history
```

### ZJJ (Workspace Management)
```bash
zjj add <name>           # Create session + workspace + Zellij tab
zjj work <name>          # Simpler workspace creation
zjj whereami             # Check current location
zjj focus <name>         # Switch to session's tab
zjj done                 # Complete and merge work
zjj remove <name>        # Clean up session
zjj list                 # List all sessions
```

### Codanna (Code Search)
```bash
codanna mcp find_symbol --name Workspace          # Exact symbol
codanna mcp search_symbols --pattern work --kind Struct  # Fuzzy
codanna mcp semantic_search_docs --query "error handling"  # Semantic
codanna index            # Reindex code
```

### More Commands
See individual docs for full command lists: [02_MOON_BUILD.md](02_MOON_BUILD.md), [08_BEADS.md](08_BEADS.md), [09_JUJUTSU.md](09_JUJUTSU.md), [19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md)

---

## 🎓 Learning Paths

### Path 1: Quick Start (1 hour)
1. [00_START_HERE.md](00_START_HERE.md) - 5 min
2. [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 20 min
3. [02_MOON_BUILD.md](02_MOON_BUILD.md) - 15 min
4. [03_WORKFLOW.md](03_WORKFLOW.md) - 20 min

### Path 2: AI Agent Onboarding (30 minutes)
1. [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) - 20 min (everything you need!)
2. [08_BEADS.md](08_BEADS.md) - 10 min (optional, for bead details)

### Path 3: Deep Dive (2 hours)
1. [00_START_HERE.md](00_START_HERE.md) - 5 min
2. [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - 20 min
3. [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) - 25 min
4. [06_COMBINATORS.md](06_COMBINATORS.md) - 20 min
5. [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 20 min
6. [07_TESTING.md](07_TESTING.md) - 15 min

### Path 4: Practitioner (1.5 hours)
1. [03_WORKFLOW.md](03_WORKFLOW.md) - 20 min
2. [08_BEADS.md](08_BEADS.md) - 25 min
3. [09_JUJUTSU.md](09_JUJUTSU.md) - 20 min
4. [06_COMBINATORS.md](06_COMBINATORS.md) - 25 min

---

## 📊 Documentation Stats

- **Total Active Pages**: 22 (down from 43!)
- **Core Docs**: 12 (00-11)
- **AI Agent Guide**: 1 comprehensive doc (replaces 7 scattered docs)
- **Operational Guides**: 5
- **Advanced Topics**: 1
- **Archived Pages**: 21 (historical/obsolete)
- **Token Efficiency**: Highly optimized for AI + human reading

---

## 🔍 Search Guide

### By Error Type
→ [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - All error patterns  
→ [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md) - Debug specific errors with solutions

### By Iterator Operation
→ [06_COMBINATORS.md](06_COMBINATORS.md) - Complete iterator reference

### By Testing Scenario
→ [07_TESTING.md](07_TESTING.md) - Test patterns

### Moon CICD Tasks & Pipelines
→ [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md) - All 17 tasks + 4 pipelines indexed

### By Tool Command
- Beads: [08_BEADS.md](08_BEADS.md)
- Moon User Guide: [02_MOON_BUILD.md](02_MOON_BUILD.md)
- Moon CICD Reference: [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md)
- Jujutsu: [09_JUJUTSU.md](09_JUJUTSU.md)
- ZJJ: [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) + README.md
- Queue Operations: [QUEUE_OPERATIONS_RUNBOOK.md](QUEUE_OPERATIONS_RUNBOOK.md)
- Rollout/Rollback: [ROLLOUT_ROLLBACK.md](ROLLOUT_ROLLBACK.md)

---

## 💡 Core Concepts Summary

| Concept | Location | Summary |
|---------|----------|---------|
| Zero Unwrap Law | [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) | No panics enforced by compiler |
| Result Pattern | [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) | All fallible ops return Result |
| Combinators | [06_COMBINATORS.md](06_COMBINATORS.md) | map, filter, fold, and_then, etc. |
| Functional Style | [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) | Immutability, composition, lazy eval |
| Moon Caching | [02_MOON_BUILD.md](02_MOON_BUILD.md) | Smart task skipping for speed |
| Beads Triage | [08_BEADS.md](08_BEADS.md) | Graph-aware issue prioritization |
| Jujutsu Stacking | [09_JUJUTSU.md](09_JUJUTSU.md) | Instant branches, reorderable commits |
| Codanna Search | [19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md) | 40-50x faster than grep, semantic code search |
| 7 Mandatory Rules | [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) | Agent critical constraints |

---

## 🚫 What NOT to Do

**These cause compiler errors (good!)**:
- `unwrap()` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- `expect()` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- `panic!()` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- `unsafe { }` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- Direct `cargo` commands - Use `moon run` instead ([02_MOON_BUILD.md](02_MOON_BUILD.md))
- Grep/Glob for code search - Use Codanna instead ([19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md))

---

## ✅ What TO Do

- Return `Result<T, Error>` for fallible ops ([01_ERROR_HANDLING.md](01_ERROR_HANDLING.md))
- Use `?` operator, match, or combinators ([01_ERROR_HANDLING.md](01_ERROR_HANDLING.md))
- Build with `moon run` ([02_MOON_BUILD.md](02_MOON_BUILD.md))
- Use functional patterns ([04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md))
- Test all paths ([07_TESTING.md](07_TESTING.md))
- Search code with Codanna ([19_CODANNA_QUERY_PERFORMANCE.md](19_CODANNA_QUERY_PERFORMANCE.md))
- **AI Agents**: Follow the 7 Mandatory Rules ([AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md))

---

## 📞 Getting Help

1. **Quick question?** → Find in [00_START_HERE.md](00_START_HERE.md)
2. **AI Agent?** → [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) has everything
3. **Error handling?** → [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md)
4. **Specific error?** → [ERROR_TROUBLESHOOTING.md](ERROR_TROUBLESHOOTING.md)
5. **Which combinator?** → [06_COMBINATORS.md](06_COMBINATORS.md)
6. **Build issue?** → [02_MOON_BUILD.md](02_MOON_BUILD.md)
7. **Workflow question?** → [03_WORKFLOW.md](03_WORKFLOW.md)

---

## 🎯 The Philosophy

> "All fallible operations return `Result<T, Error>`. The compiler enforces this. We write safe, correct, idiomatic Rust without panics."

Everything in these docs supports this principle.

---

**Start Here**: [00_START_HERE.md](00_START_HERE.md)  
**AI Agents Start Here**: [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md)

**The Law**: No unwraps, no panics, no unsafe. Period.
