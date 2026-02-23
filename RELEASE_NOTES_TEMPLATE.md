# Release Notes Template

**Version**: X.Y.Z | **Release Date**: YYYY-MM-DD | **Commit**: [abc1234]

---

## Quick Summary

One-paragraph overview of this release. Who should care and why?

**Example**: This release introduces multi-agent queue coordination, enabling 6-12 parallel AI agents to work safely on the same repository without conflicts. It also includes critical bug fixes for workspace isolation and performance improvements for large monorepos.

---

## Upgrade Instructions

### Quick Start (Most Users)

```bash
# Via cargo install
cargo install zjj --version X.Y.Z

# Or build from source
git checkout v0.X.Y
moon run :build
cargo install --path .
```

### Critical Upgrade Notes (If Applicable)

> **WARNING**: This section appears only when breaking changes exist. If empty, this is a drop-in replacement.

**Example**: If upgrading from 0.3.x or earlier, you must run `zjj doctor --fix` to migrate your database schema to the new format. Back up `.zjj/state.db` before upgrading.

---

## Breaking Changes

### Section Purpose
Use this section for API changes, command syntax changes, or behavioral changes that may require user action.

### Format
```
### [Change Title] - Impact: HIGH/MEDIUM/LOW

**Before**: `zjj command --old-flag <value>`

**After**: `zjj command --new-flag <value>`

**Reason**: Brief explanation of why this change was necessary.

**Migration**: Steps to migrate existing workflows.

**Affected Users**: Who is affected? (e.g., "Users with >50 workspaces")
```

### Examples

#### Example 1: Command Rename
```markdown
### `zjj queue claim` renamed to `zjj queue --next` - Impact: LOW

**Before**:
```bash
zjj queue claim
```

**After**:
```bash
zjj queue --next
```

**Reason**: Align queue commands with standard `--flag` syntax used throughout ZJJ.

**Migration**: Update any scripts using `zjj queue claim` to use `zjj queue --next`.

**Affected Users**: Users with automated queue worker scripts.
```

#### Example 2: Default Behavior Change
```markdown
### Recovery policy default changed to `warn` - Impact: MEDIUM

**Before**: Database corruption was silently recovered (policy: `silent`)

**After**: Database corruption shows warnings before recovering (policy: `warn`)

**Reason**: Provide visibility into corruption events while maintaining availability.

**Migration**: To maintain previous behavior, set:
```bash
export ZJJ_RECOVERY_POLICY=silent
# Or add to .zjj/config.toml: recovery.policy = "silent"
```

**Affected Users**: All users (new default is more transparent).
```

#### Example 3: JSON Output Format Change
```markdown
### `zjj status --json` output structure changed - Impact: HIGH

**Before**:
```json
{"status": "active", "workspace": "feature-xyz"}
```

**After**:
```json
{"state": "active", "session": {"name": "feature-xyz", "workspace": "feature-xyz"}}
```

**Reason**: Align JSON output with new domain model (sessions vs workspaces).

**Migration**: Update JSON parsers to use `state` field and nested `session` object.

**Affected Users**: Tools/scripts parsing `zjj status --json` output.
```

---

## New Features

### Section Purpose
Highlight significant new capabilities added in this release.

### Format
```
### [Feature Name]

**Description**: What does this feature do?

**Use Case**: When should users use this feature?

**Example**: Code example showing usage.

**Documentation Link**: Full docs URL (if applicable).
```

### Examples

#### Example 1: Major Feature
```markdown
### Multi-Agent Queue Coordination

**Description**: The queue now supports safe coordination of multiple parallel workers (AI agents or humans) with:
- Lease-based claiming (prevents duplicate work)
- Automatic stale lease reclamation (worker crash recovery)
- Priority-based work assignment
- Per-agent claiming hints

**Use Case**: Run 6-12 AI agents in parallel on the same repository without conflicts or duplicate work.

**Example**:
```bash
# Add work items to queue
zjj queue --add feature-auth --bead BD-101 --priority 3
zjj queue --add feature-db --bead BD-102 --priority 5 --agent agent-002

# Start worker (pulls next available item)
zjj queue worker --once

# Or run continuous worker loop
zjj queue worker --loop
```

**Documentation Link**: https://lprior-repo.github.io/zjj/ai/queue.html
```

#### Example 2: Quality-of-Life Feature
```markdown
### `zjj diff` Command

**Description**: Show diff between a session/workspace and main without leaving the session.

**Use Case**: Quick preview of changes before running `zjj done`.

**Example**:
```bash
zjj diff auth-refactor
# Shows diff between auth-refactor workspace and main
```

**Documentation Link**: https://lprior-repo.github.io/zjj/reference/commands.html#zjj-diff
```

#### Example 3: JSON Output for Tooling
```markdown
### JSON Output Mode (All Commands)

**Description**: All commands now support `--json` flag for machine-readable output.

**Use Case**: Build custom tooling, dashboards, or CI/CD integrations on top of ZJJ.

**Example**:
```bash
zjj status --json | jq '.state'
zjj list --json | jq '.[] | select(.state == "active")'
```

**Documentation Link**: https://lprior-repo.github.io/zjj/reference/json-output.html
```

---

## Bug Fixes

### Section Purpose
Document bugs that were fixed and their impact on users.

### Format
```
### [Bug Title] (Severity: CRITICAL/HIGH/MEDIUM/LOW)

**Issue**: #123

**Description**: What was the bug?

**Impact**: Who was affected and what broke?

**Fix**: What changed to fix it?

**Verification**: How to verify the fix works.
```

### Examples

#### Example 1: Critical Bug
```markdown
### Database corruption on concurrent `zjj done` operations (Severity: CRITICAL)

**Issue**: #456

**Description**: Running `zjj done` in parallel for different sessions could corrupt the database, causing subsequent operations to fail.

**Impact**: Users running multiple `zjj done` commands simultaneously (e.g., automated cleanup scripts). Data loss possible in rare cases.

**Fix**: Added proper transaction isolation with `BEGIN IMMEDIATE` and retry logic for SQLite `SQLITE_BUSY` errors.

**Verification**:
```bash
# Create test sessions
zjj add test-a && zjj add test-b

# Run in parallel (should not corrupt)
zjj done test-a & zjj done test-b & wait

# Verify database integrity
zjj doctor
```

**Credits**: Reported by @user123, fixed by @contributor456
```

#### Example 2: High-Impact Bug
```markdown
### Zellij tab creation failed with non-ASCII session names (Severity: HIGH)

**Issue**: #789

**Description**: Creating sessions with non-ASCII characters (e.g., `feature-émojis`) would fail with "invalid Zellij session name" error.

**Impact**: Users working with internationalized session names.

**Fix**: Session names are now sanitized to valid Zellij identifiers while preserving display names.

**Verification**:
```bash
zjj add "feature-日本語"  # Now works
zjj focus "feature-日本語"  # Jumps to sanitized tab
```
```

#### Example 3: Medium-Impact Bug
```markdown
### `zjj clean` removed active sessions (Severity: MEDIUM)

**Issue**: #234

**Description**: `zjj clean` would incorrectly remove sessions marked as "active" if they had no recent activity.

**Impact**: Users relying on `zjj clean` to remove only stale sessions.

**Fix**: `zjj clean` now checks both last activity time AND session state before removal. Active sessions are preserved regardless of activity time.

**Verification**:
```bash
zjj add active-session
zjj clean  # Should NOT remove active-session
zjj list   # Verify active-session still exists
```
```

---

## Performance Improvements

### Section Purpose
Document measurable performance improvements with benchmarks.

### Format
```
### [Optimization Title]

**Before**: Metric (e.g., "2.3s for 100 sessions")

**After**: Metric (e.g., "180ms for 100 sessions" - **12.7x faster**)

**How**: Brief technical explanation.

**Affected Operations**: Which commands/scenarios benefit?
```

### Examples

#### Example 1: Query Optimization
```markdown
### Session list query optimization

**Before**: `zjj list` took 2.3s for 100 sessions

**After**: `zjj list` takes 180ms for 100 sessions (**12.7x faster**)

**How**: Added index on `sessions.last_activity_at` and replaced N+1 queries with single JOIN query.

**Affected Operations**: `zjj list`, `zjj status`, `zjj clean`

**Benchmark**:
```bash
hyperfine 'zjj-cli-0.4.2 list' 'zjj-cli-0.4.3 list'
# Benchmark 1: zjj-cli-0.4.2 list
#   Time (mean ± σ):     2.341 s ±  0.123 s
# Benchmark 2: zjj-cli-0.4.3 list
#   Time (mean ± σ):     180.2 ms ±   8.4 ms
```
```

#### Example 2: Memory Optimization
```markdown
### Database connection pool tuning

**Before**: Peak memory usage of 145MB with 50 concurrent operations

**After**: Peak memory usage of 68MB with 50 concurrent operations (**2.1x reduction**)

**How**: Reduced pool size from 100 to 10 connections (SQLite doesn't benefit from high concurrency) and enabled WAL mode for better read concurrency.

**Affected Operations**: All database operations, especially queue worker loops
```

#### Example 3: Startup Time
```markdown
### Lazy initialization of Zellij

**Before**: `zjj` startup time of 340ms (even for non-Zellij commands)

**After**: `zjj` startup time of 45ms (**7.5x faster**)

**How**: Zellij session discovery is now lazy-initialized only when needed (e.g., `zjj focus`, `zjj attach`).

**Affected Operations**: All `zjj` commands except `focus`/`attach`
```

---

## Migration Notes

### Section Purpose
Step-by-step guides for specific migration scenarios when upgrading.

### Format
```
### [Migration Scenario]

**From**: Version X.Y.Z

**To**: Version A.B.C

**Estimated Time**: X minutes

**Steps**:
1. [Preparation step]
2. [Migration step]
3. [Verification step]

**Rollback**: How to rollback if needed.
```

### Examples

#### Example 1: Database Schema Migration
```markdown
### Database Schema Migration

**From**: 0.3.x or earlier

**To**: 0.4.0

**Estimated Time**: 5 minutes

**Context**: This release migrates from separate JSON files to a unified SQLite database with proper foreign key constraints.

**Prerequisites**:
- Backup your existing `.zjj/` directory
- Ensure no active `zjj` processes are running

**Steps**:
```bash
# 1. Stop any running workers
pkill -f 'zjj queue worker'

# 2. Backup existing state
cp -r .zjj .zjj.backup.$(date +%s)

# 3. Upgrade zjj
cargo install zjj --version 0.4.0

# 4. Run migration
zjj doctor --migrate-db

# 5. Verify migration
zjj list  # Should show all sessions
zjj queue --list  # Should show all queue entries
```

**Rollback**:
```bash
# If migration fails, restore from backup
pkill -f 'zjj'
rm -rf .zjj
mv .zjj.backup.* .zjj
cargo install zjj --version 0.3.5
```
```

#### Example 2: Queue Configuration Migration
```markdown
### Queue Stale Lease Configuration

**From**: 0.4.0 - 0.4.2

**To**: 0.4.3

**Estimated Time**: 2 minutes

**Context**: Queue stale lease detection changed from time-based to heartbeat-based for faster crash recovery.

**Steps**:
```bash
# 1. Update config (optional, new defaults are reasonable)
cat >> .zjj/config.toml << 'EOF'
[queue]
stale_lease_seconds = 300
heartbeat_interval_seconds = 30
EOF

# 2. No data migration needed - existing entries work with new logic

# 3. Verify queue works
zjj queue --add test-entry --bead TEST-123
zjj queue --list  # Should show test-entry
zjj queue --remove $(zjj queue --list | jq -r '.[] | select(.bead == "TEST-123") | .id')
```
```

#### Example 3: CLI Flag Migration
```markdown
### `--strict` Flag Behavior Change

**From**: 0.4.2 and earlier

**To**: 0.4.3

**Estimated Time**: 1 minute

**Context**: `--strict` flag now sets recovery policy to `fail-fast` instead of just enabling warnings.

**Steps**:
```bash
# If you were using --strict for warnings only:
# Old behavior (shows warnings)
zjj --strict status

# New behavior (fails on corruption)
zjj --strict status

# To maintain old behavior, use:
export ZJJ_RECOVERY_POLICY=warn
zjj status

# Or update scripts to remove --strict flag
```
```

---

## Documentation Updates

### Section Purpose
Track documentation improvements and new resources.

### Format
```
### [Documentation Type]

**Links**:
- [Page Title](URL)

**Summary**: What's new or improved?
```

### Examples

```markdown
### New Documentation

**Links**:
- [Multi-Agent Queue Guide](https://lprior-repo.github.io/zjj/ai/queue.html)
- [Recovery Policy Reference](https://lpior-repo.github.io/zjj/reference/recovery.html)

**Summary**: Comprehensive guide for running multiple AI agents with queue coordination, including example worker scripts and retry strategies.

---

### Updated Documentation

**Links**:
- [Quick Start Guide](https://lprior-repo.github.io/zjj/quickstart.html)
- [Command Reference](https://lprior-repo.github.io/zjj/reference/commands.html)

**Summary**: Quick start guide now includes multi-agent workflow. Command reference updated with all new JSON output examples.

---

### API Documentation

**Links**:
- [zjj-core Crate Docs](https://docs.rs/zjj-core/0.4.3/zjj_core/)

**Summary**: Added documentation for `Queue::claim_next` and `Session::sync` with examples. Added domain error documentation.
```

---

## Contributors

This release was made possible by contributions from:

- **@contributor1** - Multi-agent queue coordination (PR #123)
- **@contributor2** - Database performance optimizations (PR #145)
- **@contributor3** - Bug fixes for session cleanup (PR #167)
- **@contributor4** - Documentation improvements (PR #189)

**Special Thanks**:
- @user123 for detailed bug reports on queue reclaim logic
- @user456 for testing the alpha release and providing feedback

---

## Full Changelog

For complete details, see:
- **[GitHub Release](https://github.com/lprior-repo/zjj/releases/tag/v0.X.Y)**
- **[Git Commits](https://github.com/lprior-repo/zjj/compare/v0.A.B...v0.X.Y)**

---

## Known Issues

Track issues that are known but not yet fixed:

```markdown
### [Issue Title]

**Issue**: #789

**Status**: Known workaround available

**Description**: Brief description of the issue.

**Workaround**: Steps to avoid or work around the issue.

**Fix Target**: Version 0.5.0 (planned YYYY-MM-DD)
```

### Example

```markdown
### Zellij tab naming collision

**Issue**: #234

**Status**: Known workaround available

**Description**: Sessions with names that differ only in case (e.g., `feature-a` and `Feature-A`) may collide in Zellij tab naming on case-insensitive filesystems.

**Workaround**: Use session names that differ in more than case (e.g., `feature-auth` and `feature-db` instead of `feature-a` and `Feature-A`).

**Fix Target**: Version 0.5.0 (planned 2026-03-15)
```

---

## Next Release Preview

(Optional) Sneak peek at what's coming in the next version:

```markdown
### Upcoming in 0.5.0

**Planned Features**:
- [ ] Distributed locking for multi-machine coordination
- [ ] Bead/issue integration with GitHub Issues API
- [ ] TUI dashboard for real-time queue monitoring

**Target Date**: 2026-03-15

**Tracking Issue**: #345
```

---

## Appendix: Release Checklist

For release managers, use this checklist when preparing a release:

### Pre-Release
- [ ] All tests passing (`moon run :test`)
- [ ] Documentation updated (`moon run :docs`)
- [ ] Changelog entries categorized correctly
- [ ] Version bumped in `Cargo.toml`
- [ ] Release notes drafted
- [ ] Breaking changes documented with migration guides
- [ ] Performance improvements benchmarked

### Release
- [ ] Git tag created (`git tag -s v0.X.Y`)
- [ ] Tag pushed to GitHub (`git push origin v0.X.Y`)
- [ ] GitHub release published with notes
- [ ] Crate published to crates.io (`cargo publish`)
- [ ] Documentation site deployed

### Post-Release
- [ ] Announced in communication channels
- [ ] Known issues tracked in GitHub
- [ ] User feedback monitored
- [ ] Next release planning started

---

## Template Usage Guide

### How to Use This Template

1. **Copy** this template to `RELEASE_NOTES_X.Y.Z.md`
2. **Fill in** the relevant sections (remove empty sections)
3. **Review** against examples for consistency
4. **Proofread** for clarity and completeness
5. **Link** from GitHub release body

### Section Selection Guidelines

- **Breaking Changes**: Always include if any breaking changes exist
- **New Features**: Include features users should know about
- **Bug Fixes**: Include fixes for high/medium severity bugs
- **Migration Notes**: Include if upgrade requires manual steps
- **Performance**: Include if measurable improvements (>20%)
- **Documentation**: Optional, helpful for tracking docs progress

### Tone and Style

- **Be concise**: Users scan release notes, not read them cover-to-cover
- **Be specific**: Use actual metrics and examples, not vague statements
- **Be actionable**: Every item should tell users what to do (if anything)
- **Be honest**: Admit known issues and provide workarounds

### Examples Summary

This template includes 3 complete examples for each section type:
- Breaking changes (command rename, default behavior, JSON format)
- New features (major feature, quality-of-life, JSON output)
- Bug fixes (critical, high, medium severity)
- Performance improvements (query, memory, startup)
- Migration notes (database, config, CLI flags)

Use these as style guides when writing your own entries.

---

**Template Version**: 1.0 | **Last Updated**: 2026-02-23
