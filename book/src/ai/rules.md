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

---

## Quick Reference: 90% of Workflows

### Code Search (Codanna)

```jsonl
{"tool": "semantic_search_with_context", "when": "Natural language query → full context", "call": "mcp__codanna__semantic_search_with_context(query=\"your intent\", limit: 5)"}
{"tool": "find_symbol", "when": "Know exact name", "call": "mcp__codanna__find_symbol(name=\"ExactName\")"}
{"tool": "search_symbols", "when": "Fuzzy pattern search", "call": "mcp__codanna__search_symbols(query=\"pattern\", kind:\"Struct|Function|Trait\", lang:\"rust\", limit: 10)"}
{"tool": "search_documents", "when": "Search markdown docs", "call": "mcp__codanna__search_documents(query=\"topic\", limit: 5)"}
{"tool": "analyze_impact", "when": "Dependency graph", "call": "mcp__codanna__analyze_impact(symbol_id: 123)"}
```

**Workflow:** semantic_search → find_symbol → search_symbols → search_documents → analyze_impact

**When to reindex:** Codanna returns no results, search seems outdated, or after large code changes.
```bash
codanna index && codanna documents index --collection docs
# Current stats: 5,592 symbols, 817 doc chunks
```

### Build (Moon)

```jsonl
{"cmd": "moon run :quick", "use": "Fast check (6-7ms cached)", "frequency": "Every edit"}
