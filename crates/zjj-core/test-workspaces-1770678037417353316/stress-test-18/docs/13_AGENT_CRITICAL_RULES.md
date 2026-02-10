# Critical Rules

> **🔙 Back to**: [AGENTS.md](../AGENTS.md) | **📂 agents docs**: [Quick Reference](14_AGENT_QUICK_REFERENCE.md) | [Project Context](15_AGENT_PROJECT_CONTEXT.md) | [Parallel Workflow](16_AGENT_PARALLEL_WORKFLOW.md) | [Session Completion](17_AGENT_SESSION_COMPLETION.md) | [BV Reference](18_AGENT_BV_REFERENCE.md)

---

## 7 ABSOLUTE MANDATORY RULES

### 1. NO_CLIPPY_EDITS
**NEVER modify clippy or linting configuration files. EVER.**

Files: `.clippy.toml`, `clippy.toml`, `#![allow(...)]`, `#![deny(...)]`, `Cargo.toml` lint sections, `moon.yml` lint rules.
**Fix the code, not the rules.**

### 2. MOON_ONLY
**NEVER use raw cargo commands. ALWAYS use Moon.**

- ❌ NEVER: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build`
- ✅ ALWAYS: `moon run :quick`, `moon run :test`, `moon run :build`, `moon run :ci`, `moon run :fmt-fix`, `moon run :check`

**Why:** Moon provides 98.5% faster builds via persistent caching and parallel execution.

### 3. CODANNA_ONLY
**NEVER use Grep/Glob/Read for exploration. ALWAYS use Codanna.**

- ❌ NEVER: `Grep`, `Glob`, `Read` for code exploration
- ✅ ALWAYS: `mcp__codanna__semantic_search_with_context`, `find_symbol`, `search_symbols`, `search_documents`, `get_calls`, `find_callers`, `analyze_impact`

**Why:** Codanna is pre-indexed with semantic understanding. 90% fewer tokens, 10x faster.

### 4. ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC
**ZERO unwrap. ZERO expect. ZERO panic. EVER.**

**COMPLETELY FORBIDDEN:**
- ❌ `unwrap()` - under NO circumstances
- ❌ `unwrap_or()` - NO variants allowed
- ❌ `unwrap_or_else()` - NO variants allowed
- ❌ `unwrap_or_default()` - NO variants allowed
- ❌ `expect()` - under NO circumstances
- ❌ `expect_err()` - NO variants allowed
- ❌ `panic!()` - under NO circumstances
- ❌ `todo!()` - under NO circumstances
- ❌ `unimplemented!()` - under NO circumstances

**REQUIRED:**
- ✅ `Result<T, Error>` for all fallible operations
- ✅ `map`, `and_then`, `?` operator for error propagation
- ✅ Railway-Oriented Programming patterns
- ✅ **USE functional-rust-generator SKILL for ALL Rust implementation**

**Why:** Panic-based code is unmaintainable and crashes in production. Unwrap variants are panics in disguise. Expect is just panic with a message.

### 5. GIT_PUSH_MANDATORY
**Work is NOT done until `git push` succeeds.**

- ❌ NEVER: Stop before pushing, say "ready to push when you are", leave work stranded locally
- ✅ ALWAYS: Push yourself, verify `git status` shows "up to date", resolve and retry on failure

**Why:** Unpushed work = lost work. "I'll push later" = stranded commits.

### 6. BR_SYNC
**`br` never runs git commands. You must commit bead changes manually.**

After `br sync --flush-only`, you MUST run:
```bash
git add .beads/
git commit -m "sync beads"
```

**Why:** Beads is non-invasive by design. It modifies JSONL only. You must commit those changes.

### 7. FUNCTIONAL_RUST_SKILL
**ALWAYS load and use functional-rust-generator skill for ANY Rust implementation.**

- ✅ ALWAYS: Load `functional-rust-generator` skill before writing Rust code
- ✅ This skill enforces zero unwrap/expect/panic patterns
- ✅ Uses Railway-Oriented Programming
- ✅ Provides functional patterns: `map`, `and_then`, `?`

**Why:** The skill exists for a reason - it enforces these patterns automatically. Don't reinvent the wheel.

```jsonl
{"rule": "NO_CLIPPY_EDITS", "severity": "MANDATORY", "text": "NEVER modify .clippy.toml, #![allow], #![deny], Cargo.toml lint sections, moon.yml lint rules. Fix code, not rules."}
{"rule": "MOON_ONLY", "severity": "MANDATORY", "text": "NEVER cargo fmt|clippy|test|build. ALWAYS moon run :quick|:test|:build|:ci|:fmt-fix|:check"}
{"rule": "CODANNA_ONLY", "severity": "MANDATORY", "text": "NEVER Grep|Glob|Read for exploration. ALWAYS use mcp__codanna__ semantic_search|find_symbol|search_symbols|search_documents|get_calls|find_callers|analyze_impact"}
{"rule": "ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC", "severity": "MANDATORY", "text": "ZERO unwrap(), ZERO unwrap_or(), ZERO unwrap_or_else(), ZERO unwrap_or_default(). ZERO expect(), ZERO expect_err(). ZERO panic!(), ZERO todo!(), ZERO unimplemented!(). Required: Result<T, Error>, map, and_then, ?, Railway-Oriented Programming. USE functional-rust-generator SKILL."}
{"rule": "GIT_PUSH_MANDATORY", "severity": "MANDATORY", "text": "Work NOT done until git push succeeds. NEVER stop before pushing. YOU must push (not 'ready when you are')."}
{"rule": "BR_SYNC", "severity": "MANDATORY", "text": "br never runs git. After br sync --flush-only, manually git add .beads/ && git commit -m 'sync beads'"}
{"rule": "FUNCTIONAL_RUST_SKILL", "severity": "MANDATORY", "text": "ALWAYS load and use functional-rust-generator skill for ANY Rust implementation. Enforces zero unwrap/expect/panic patterns with Railway-Oriented Programming."}
```

## Rationale

| Rule | Problem | Solution |
|------|---------|----------|
| NO_CLIPPY_EDITS | Lint rules intentionally configured | Code must conform to rules |
| MOON_ONLY | Cargo bypasses 98.5% speedup | Moon provides persistent cache |
| CODANNA_ONLY | Grep/Glob are 10x slower | Pre-indexed semantic search |
| ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC | Unwrap/expect/panic are production crashes | Functional error handling with Result types |
| GIT_PUSH_MANDATORY | Unpushed work gets lost | Push = work complete |
| BR_SYNC | Beads is non-invasive | Manual git commits required |
| FUNCTIONAL_RUST_SKILL | Reimplementing patterns is wasteful | Skill enforces patterns automatically |

## Violation Consequences

Breaking these rules risks:
- Data loss (unpushed commits)
- Broken builds (bypassing cache)
- Wasted tokens (inefficient searching)
- Production crashes (unwrap/expect/panic)
- Technical debt (panic-based code)
- Merge conflicts (improper sync)
- Reimplementation waste (not using functional-rust-generator skill)

---

## AI-Native CLI Usage

### Query Command Contracts

All zjj commands support `--contract` flag for machine-readable contracts:

```bash
zjj add --contract      # Show JSON schema of inputs/outputs/side-effects
zjj work --contract     # Show workflow contract for automated tasks
zjj spawn --contract    # Show agent spawning contract
zjj done --contract     # Show completion/merge contract
```

**Contract includes:**
- Intent description (what command does)
- Prerequisites (what must exist first)
- Side effects (what gets created/modified)
- Input schema with types and validation
- Output schema with success/error states
- Example invocations
- Next commands in workflow

### Get AI Hints

Use `--ai-hints` flag for workflow guidance:

```bash
zjj add --ai-hints     # Shows typical workflow patterns
zjj work --ai-hints    # Shows agent integration patterns
```

**Hints include:**
- Typical workflows (manual, automated, parallel)
- Command prerequisites and ordering
- Error recovery strategies
- Common failure modes

### Structured JSON Output

All commands support `--json` for machine-readable output:

```bash
zjj list --json        # Returns SchemaEnvelopeArray
zjj status --json      # Returns session state as JSON
zjj context --json     # Returns full environment context (default)
```

### Example: AI Workflow

```bash
# 1. Check contract before starting work
zjj work --contract

# 2. Get hints on typical workflow
zjj work --ai-hints

# 3. Start work with bead ID
zjj work zjj-abc123 --agent claude

# 4. Check status as JSON
zjj status --json

# 5. Complete and merge
zjj done --json
```

### Query System State

Use `zjj context` for complete AI-queryable state:

```bash
zjj context --json | jq .
{
  "repository": {...},
  "sessions": [...],
  "beads": {...},
  "health": {...},
  "environment": {...}
}
```

---

**🔙 Back to**: [AGENTS.md](../AGENTS.md)
