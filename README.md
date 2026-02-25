# ZJJ - Parallel Workspace Isolation & Queue Coordination

[![Coverage](https://codecov.io/gh/lprior-repo/zjj/branch/main/graph/badge.svg)](https://codecov.io/gh/lprior-repo/zjj)
[![CI](https://github.com/lprior-repo/zjj/actions/workflows/ci.yml/badge.svg)](https://github.com/lprior-repo/zjj/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-mdBook-blue)](https://lprior-repo.github.io/zjj/)

> **ZJJ** helps you run **multiple parallel workstreams (humans or AI agents)** against a single repo **without stepping on each other**.

It combines **JJ (Jujutsu) workspaces** for hard isolation and a **SQLite-backed state + merge/processing queue** for coordination, retries, and recovery.

---

## Quick Navigation

### Documentation Index

| Topic | Description | Link |
|-------|-------------|------|
| **Getting Started** | Installation, quick start, basic commands | [Quick Start Guide](#quick-start) |
| **Architecture** | System design, layers, data flow | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Domain Types** | All domain primitives, identifiers, state machines | [DOMAIN_TYPES_GUIDE.md](DOMAIN_TYPES_GUIDE.md) |
| **Quick Reference** | Single-page cheat sheet for domain types | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| **Value Objects** | DDD value objects reference | [VALUE_OBJECTS.md](VALUE_OBJECTS.md) |
| **State Machines** | All state machines and transitions | [STATE_MACHINES.md](STATE_MACHINES.md) |
| **Error Handling** | Zero-panic error handling patterns | [ERROR_HANDLING_GUIDE.md](ERROR_HANDLING_GUIDE.md) |
| **Migration Guide** | Migrating to DDD architecture | [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) |
| **Contributing** | Development setup, PR process | [CONTRIBUTING.md](CONTRIBUTING.md) |
| **Refactoring** | Code refactoring checklist | [REFACTORING_CHECKLIST.md](REFACTORING_CHECKLIST.md) |

### Development Reports

| Report | Description | Link |
|--------|-------------|------|
| **DDD Audit** | Domain-Driven Design audit report | [DDD_AUDIT_REPORT.md](DDD_AUDIT_REPORT.md) |
| **Benchmarks** | Performance benchmarks | [BENCHMARKS_REPORT.md](BENCHMARKS_REPORT.md) |
| **Test Coverage** | Domain test coverage summary | [DOMAIN_TEST_COVERAGE_SUMMARY.md](DOMAIN_TEST_COVERAGE_SUMMARY.md) |
| **Validation API** | Validation API reference | [VALIDATION_API_REFERENCE.md](VALIDATION_API_REFERENCE.md) |

### Historical Reports

| Report | Description | Link |
|--------|-------------|------|
| **CLI Contracts** | CLI contracts refactoring | [CLI_CONTRACTS_REFACTORING.md](CLI_CONTRACTS_REFACTORING.md) |
| **Beads DDD** | Beads domain refactoring | [BEADS_DDD_REFACTORING_REPORT.md](BEADS_DDD_REFACTORING_REPORT.md) |
| **Coordination** | Coordination layer refactor | [COORDINATION_REFACTOR_SUMMARY.md](COORDINATION_REFACTOR_SUMMARY.md) |

---

## Table of Contents

1. [Why ZJJ Exists](#why-zjj-exists)
2. [Mental Model](#mental-model)
3. [Quick Start](#quick-start)
4. [Key Commands](#key-commands)
5. [Architecture Overview](#architecture-overview)
6. [Design Philosophy](#design-philosophy)
7. [Installation](#installation)
8. [Development](#development)
9. [Documentation](#documentation)
10. [Agent Development](#agent-development)

---

## Why ZJJ Exists

Parallel work is easy to start and hard to finish cleanly:

- **Two workers edit the same area** → conflicts
- **Multiple tasks get half-done** → you lose track
- **"Who is working on what?"** becomes tribal knowledge
- **Agents can duplicate work** or race each other

**ZJJ fixes this** by making parallelism **explicit, isolated, and coordinated**.

**Before**: 6 agents race on the same working copy → duplicated effort + conflicts
**After**: Each agent gets an isolated workspace + the queue enforces safe claiming/landing

---

## Mental Model

**Session** = a named isolated workspace (+ optional bead/issue)
**Queue entry** = a unit of work tied to a workspace that a worker/agent can claim and process
**Done** = finish the work and land it back to main

```
┌─────────────────────────────────────────────────────────────┐
│                        MAIN BRANCH                           │
│                      (root workspace)                        │
└──────────────────────────┬──────────────────────────────────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
  │  Session A  │   │  Session B  │   │  Session C  │
  │ (workspace) │   │ (workspace) │   │ (workspace) │
  └─────────────┘   └─────────────┘   └─────────────┘
         │                 │                 │
         ▼                 ▼                 ▼
  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
  │   Bead-1    │   │   Bead-2    │   │   Bead-3    │
  │  (issue)    │   │  (issue)    │   │  (issue)    │
  └─────────────┘   └─────────────┘   └─────────────┘
```

---

## Quick Start

### 60-Second Quick Start

```bash
# 1) Inside a JJ repo
zjj init

# 2) Create an isolated session
zjj add auth-refactor --bead BD-123

# 3) Jump into it
zjj focus auth-refactor

# 4) Keep it synced with main
zjj sync auth-refactor

# 5) Finish and land the work
zjj done

# 6) Optionally clean it up
zjj remove auth-refactor
```

### Multi-Agent Workflow (Example)

```bash
# Add multiple work items
zjj queue --add feature-a --bead BD-101 --priority 3
zjj queue --add feature-b --bead BD-102 --priority 5 --agent agent-002

# Start workers (can be human-driven or agent-driven)
zjj queue worker --loop
```

ZJJ ensures only one worker claims a given entry at a time, with retries, cancel/remove, and stale lease reclamation.

---

## Key Commands

### Core Session Commands

| Command | Description |
|---------|-------------|
| `zjj init` | Initialize ZJJ in a JJ repo |
| `zjj add <name>` | Create an isolated session (workspace) |
| `zjj add <name> --bead <BEAD_ID>` | Create a session tied to an issue/bead |
| `zjj list [--verbose]` | List sessions |
| `zjj status [name]` | Detailed status + changes |
| `zjj focus <name>` | Switch to that session's workspace |
| `zjj sync [name]` | Rebase/sync workspace onto main |
| `zjj done` | Complete work and merge to main |
| `zjj remove <name>` | Remove session + workspace |
| `zjj whereami` | Show current location (main or workspace) |
| `zjj switch [name]` | Switch between workspaces (interactive if no name) |
| `zjj diff <name>` | Show diff between session and main |
| `zjj clean` | Remove stale sessions |

> **All commands support `--json` for machine-readable output.**

### Queue (Multi-Worker / Multi-Agent Coordination)

| Command | Description |
|---------|-------------|
| `zjj queue --add <workspace> --bead <BEAD_ID> [--priority N] [--agent AGENT_ID]` | Add work to queue |
| `zjj queue --list` | List queue entries |
| `zjj queue --next` | Get next entry without processing |
| `zjj queue --status <workspace>` | Show workspace status |
| `zjj queue --retry <ID>` | Retry failed entry |
| `zjj queue --cancel <ID>` | Cancel entry |
| `zjj queue --remove <ID>` | Remove entry |
| `zjj queue --reclaim-stale [seconds]` | Reclaim stale entries |
| `zjj queue worker --once | --loop` | Worker mode |

### System & Diagnostics

| Command | Description |
|---------|-------------|
| `zjj config [key] [value]` | View or modify configuration |
| `zjj doctor` | Run system health checks |
| `zjj introspect [cmd]` | Discover ZJJ capabilities and command details |
| `zjj query <type>` | Query system state programmatically |
| `zjj context` | Show complete environment context |
| `zjj dashboard` | Launch interactive TUI dashboard |

---

## Architecture Overview

ZJJ follows the **Functional Core, Imperative Shell** pattern with **Domain-Driven Design (DDD)** principles:

```
┌─────────────────────────────────────────────────────────────┐
│                      SHELL LAYER (zjj)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   CLI Parser │  │  I/O Handlers│  │   Database   │      │
│  │   (clap)     │  │  (async/tokio)│  │  (sqlx)      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└────────────────────────────┬────────────────────────────────┘
                             │
                        ┌────▼─────┐
                        │  KIRK    │  ← Design-by-Contract
                        │Contracts │     (preconditions,
                        └────┬─────┘      postconditions,
                             │           invariants)
┌────────────────────────────┼────────────────────────────────┐
│                            │                                 │
│         ┌──────────────────▼──────────────────┐              │
│         │         CORE LAYER (zjj-core)       │              │
│         │  ┌────────────────────────────────┐  │              │
│         │  │  Domain Primitives (DDD)       │  │              │
│         │  │  - Semantic Newtypes           │  │              │
│         │  │  - Aggregates                  │  │              │
│         │  │  - Value Objects               │  │              │
│         │  └────────────────────────────────┘  │              │
│         │  ┌────────────────────────────────┐  │              │
│         │  │  Business Logic (Pure)         │  │              │
│         │  │  - State transitions           │  │              │
│         │  │  - Validation                  │  │              │
│         │  │  - Coordination               │  │              │
│         │  └────────────────────────────────┘  │              │
│         └─────────────────────────────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

**Shell Layer (`crates/zjj/`)**
- Parse CLI arguments with `clap`
- Handle all I/O operations
- Manage async runtime with `tokio`
- Execute external commands (JJ)
- Write to database with `sqlx`
- Emit JSONL output

**Core Layer (`crates/zjj-core/`)**
- Pure business logic (no I/O)
- Domain primitives and types
- State transition logic
- Validation and invariants
- Coordination algorithms

### Key Design Principles

1. **Zero Unwrap Law** - No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
2. **Parse at Boundaries** - Validate input once, use semantic types throughout
3. **Make Illegal States Unrepresentable** - Use enums instead of optional fields
4. **Pure Core** - Domain logic is deterministic, no I/O or global state

**See [ARCHITECTURE.md](ARCHITECTURE.md) for complete architecture documentation.**

---

## Design Philosophy

### Functional Rust

**Zero Unwrap Law** (compiler-enforced):
- No `unwrap()`, `expect()`, `panic()`, `todo!()`, `unimplemented!()`
- All fallible operations return `Result<T, E>`
- Railway-oriented error propagation with `?` operator

**Immutability by Default**:
- Prefer `let` over `let mut`
- Use persistent data structures (`rpds`)
- Iterator pipelines over loops (`itertools`)

**Pure Functions**:
- Core logic: deterministic, no I/O, no global state
- Shell layer: handles I/O, async, external APIs

### Domain-Driven Design (DDD)

**Bounded Contexts**:
- Each module is a clear boundary
- Explicit interfaces between contexts
- Ubiquitous language in code

**Aggregates**:
- Cluster entities and value objects
- Enforce invariants at aggregate root
- `Session`, `Bead`, `QueueEntry`, `Workspace`

**Value Objects**:
- Immutable types for domain concepts
- Equality by value, not identity
- Semantic newtypes with validation

**Repository Pattern**:
- Abstract persistence behind traits
- Domain doesn't know storage details
- Database operations isolated in shell layer

---

## Installation

### Quick Install (Automated)

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Run automated setup (checks prerequisites, installs deps, builds)
./scripts/dev-setup.sh
```

The setup script will:
- Check for Rust 1.80+, Moon, and JJ
- Install missing dependencies (with your permission)
- Set up the development database
- Run initial build and tests
- Print next steps

### Manual Setup

#### Prerequisites

- **Moon** - Install from https://moonrepo.dev/docs/install
- **JJ** (Jujutsu) - Install from https://github.com/martinvonz/jj#installation
- **Rust** 1.80 or later

#### From Source (with Moon)

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Install Moon (if not already installed)
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Build with Moon
moon run :build

# Run the binary
./target/release/zjj --help
```

**Important**: All commands in this project must be run through Moon. Do not use `cargo` directly.

---

## Development

### Quick Development Loop

```bash
# Format and type-check (6-7ms with cache!)
moon run :quick

# Full pipeline (parallel execution)
moon run :ci

# Individual tasks
moon run :fmt        # Check formatting
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check
moon run :test       # Run tests
moon run :build      # Release build
```

### Hyper-Fast CI/CD Pipeline

This project uses **Moon** + **bazel-remote** for production-grade CI/CD with **98.5% faster** cached builds:

- **6-7ms** cached task execution (vs ~450ms cold)
- **100GB local cache** with zstd compression
- **Parallel task execution** across all crates
- **Persistent cache** survives clean/rebuild cycles

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed benchmarks.

### Code Style Guidelines

**Core Principles (Non-Negotiable)**

1. **Zero Unwrap** - Never use `unwrap()`, `expect()`, or panics
2. **Parse at Boundaries** - Validate once, use semantic types
3. **Pure Core** - No I/O in domain logic
4. **DDD Patterns** - Model domain explicitly

**Required File Headers**:

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

**See [CONTRIBUTING.md](CONTRIBUTING.md) for complete contribution guidelines.**

---

## Documentation

### Core Documentation

| Document | Description | Link |
|----------|-------------|------|
| **Architecture** | System design, layers, data flow, diagrams | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Domain Types Guide** | All domain primitives, identifiers, aggregates | [DOMAIN_TYPES_GUIDE.md](DOMAIN_TYPES_GUIDE.md) |
| **Quick Reference** | Single-page cheat sheet for domain types | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| **Value Objects** | DDD value objects reference and patterns | [VALUE_OBJECTS.md](VALUE_OBJECTS.md) |
| **State Machines** | All state machines and transitions | [STATE_MACHINES.md](STATE_MACHINES.md) |
| **Error Handling** | Zero-panic error handling patterns | [ERROR_HANDLING_GUIDE.md](ERROR_HANDLING_GUIDE.md) |
| **Migration Guide** | Migrating to DDD architecture | [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) |
| **Refactoring Checklist** | Code refactoring checklist | [REFACTORING_CHECKLIST.md](REFACTORING_CHECKLIST.md) |

### Example & Reference Documentation

| Document | Description | Link |
|----------|-------------|------|
| **Domain Examples** | Real-world domain examples | [DOMAIN_EXAMPLES.md](DOMAIN_EXAMPLES.md) |
| **CLI Examples** | CLI usage examples | [CLI_EXAMPLES.md](CLI_EXAMPLES.md) |
| **Unified Error Examples** | Error handling examples | [UNIFIED_ERROR_EXAMPLES.md](UNIFIED_ERROR_EXAMPLES.md) |
| **Builders Documentation** | Builder pattern reference | [BUILDERS_DOCUMENTATION.md](BUILDERS_DOCUMENTATION.md) |

### Audit & Test Reports

| Document | Description | Link |
|----------|-------------|------|
| **DDD Audit Report** | Domain-Driven Design audit | [DDD_AUDIT_REPORT.md](DDD_AUDIT_REPORT.md) |
| **Domain Test Coverage** | Test coverage summary | [DOMAIN_TEST_COVERAGE_SUMMARY.md](DOMAIN_TEST_COVERAGE_SUMMARY.md) |
| **Benchmarks Report** | Performance benchmarks | [BENCHMARKS_REPORT.md](BENCHMARKS_REPORT.md) |
| **Integration Tests** | Integration test summary | [INTEGRATION_TESTS_SUMMARY.md](INTEGRATION_TESTS_SUMMARY.md) |
| **Validation API** | Validation API reference | [VALIDATION_API_REFERENCE.md](VALIDATION_API_REFERENCE.md) |

### Historical Reports

| Document | Description | Link |
|----------|-------------|------|
| **CLI Contracts Refactoring** | CLI contracts refactoring | [CLI_CONTRACTS_REFACTORING.md](CLI_CONTRACTS_REFACTORING.md) |
| **Beads DDD Refactoring** | Beads domain refactoring | [BEADS_DDD_REFACTORING_REPORT.md](BEADS_DDD_REFACTORING_REPORT.md) |
| **Coordination Refactoring** | Coordination layer refactor | [COORDINATION_REFACTOR_SUMMARY.md](COORDINATION_REFACTOR_SUMMARY.md) |
| **Test Unwrap Improvements** | Zero-unwrap test improvements | [TEST_UNWRAP_IMPROVEMENTS.md](TEST_UNWRAP_IMPROVEMENTS.md) |

### External Documentation

**Full Documentation Site**: https://lprior-repo.github.io/zjj/

Source markdown documentation is available in `/docs`:
- **[docs/INDEX.md](docs/INDEX.md)** - Complete documentation index
- **[docs/AI_AGENT_GUIDE.md](docs/AI_AGENT_GUIDE.md)** - AI agent guide

---

## Agent Development

For AI agents working with ZJJ, see:

- **[AGENTS.md](AGENTS.md)** - Agent development guidelines (single source of truth)
- **[docs/AI_AGENT_GUIDE.md](docs/AI_AGENT_GUIDE.md)** - AI agent integration guide

### Agent Guidelines

**Mandatory Rules**:
- NO_CLIPPY_EDITS - Fix code, not lint config
- MOON_ONLY - Use Moon for all commands
- CODANNA_MANDATORY - Use Codanna MCP tools for exploration
- ZERO_UNWRAP_PANIC - Never use unwrap, expect, panic
- FUNCTIONAL_RUST_SKILL - Use functional-rust-generator skill
- DOMAIN_DRIVEN_DESIGN - Model domain logic explicitly

**Workflow**:
1. IMPLEMENT - Load functional-rust-generator, implement with Result<T,E> + DDD
2. MANUAL_TEST - Run actual CLI commands, verify real behavior
3. REVIEW - Run quality gates (`moon run :ci`)
4. LAND - Commit and push changes

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Quick Links by Topic

### For New Users
- [Quick Start Guide](#quick-start)
- [Key Commands](#key-commands)
- [Installation](#installation)

### For Contributors
- [Development](#development)
- [Contributing Guide](CONTRIBUTING.md)
- [Refactoring Checklist](REFACTORING_CHECKLIST.md)

### For Domain Understanding
- [Architecture Overview](ARCHITECTURE.md)
- [Domain Types Guide](DOMAIN_TYPES_GUIDE.md)
- [Quick Reference](QUICK_REFERENCE.md)
- [Value Objects](VALUE_OBJECTS.md)
- [State Machines](STATE_MACHINES.md)

### For Error Handling
- [Error Handling Guide](ERROR_HANDLING_GUIDE.md)
- [Unified Error Examples](UNIFIED_ERROR_EXAMPLES.md)

### For Migration
- [Migration Guide](MIGRATION_GUIDE.md)
- [Migration Quick Reference](MIGRATION_GUIDE_QUICK.md)

### For Agents
- [Agent Guidelines](AGENTS.md)
- [AI Agent Guide](docs/AI_AGENT_GUIDE.md)

### For Audits & Reports
- [DDD Audit Report](DDD_AUDIT_REPORT.md)
- [Benchmarks Report](BENCHMARKS_REPORT.md)
- [Domain Test Coverage](DOMAIN_TEST_COVERAGE_SUMMARY.md)
