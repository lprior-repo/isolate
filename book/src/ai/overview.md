
## Quick Start (3 commands)

```bash
zjj whereami        # Check: "main" or "workspace:<name>"
zjj work my-task    # Start: Create workspace
zjj done            # Finish: Merge and cleanup
```

Or use the AI helpers:
```bash
zjj ai status       # Full status with guided next action
zjj ai workflow     # 7-step parallel agent workflow
zjj ai quick-start  # Minimum commands reference
```

---

## 7 Absolute Mandatory Rules

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

---

## Next Steps

- **[7 Mandatory Rules](./rules.md)** - Critical constraints for all agents
- **[Quick Reference](./quick-reference.md)** - 90% of workflows
- **[Parallel Workflow](./parallel-workflow.md)** - Multi-agent coordination
- **[Session Completion](./session-completion.md)** - Landing the plane
