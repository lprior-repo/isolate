# Agent Instructions

This project uses **bd** (beads) for issue tracking and **Moon** for hyper-fast builds.

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

