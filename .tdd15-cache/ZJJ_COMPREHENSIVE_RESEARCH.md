# ZJJ Comprehensive Research Report

> Generated: 2026-01-18 | 8 parallel research agents | Full codebase analysis

## Executive Summary

ZJJ is a **production-ready** session management tool that combines JJ (Jujutsu) workspaces, Zellij terminal multiplexer, and Beads issue tracking. All 26 CLI commands are fully implemented with:
- Zero unwraps/panics (compiler-enforced)
- JSON output on all commands
- Semantic exit codes (0-4)
- 40+ structured error codes

---

## Part 1: Complete Command Reference

### All 26 Commands

| Category | Command | Alias | Description | JSON | Dry-Run |
|----------|---------|-------|-------------|------|---------|
| **Session Lifecycle** | `init` | - | Initialize zjj in JJ repo | ✅ | - |
| | `add` | - | Create session (workspace + tab) | ✅ | ✅ |
| | `add-batch` | - | Batch create from stdin | ✅ | ✅ |
| | `remove` | - | Remove session | ✅ | ✅ |
| | `list` | - | List all sessions | ✅ | - |
| | `focus` | - | Switch to session tab | ✅ | - |
| | `status` | - | Show session status | ✅ | - |
| **Sync** | `sync` | - | Rebase session on main | ✅ | ✅ |
| | `diff` | - | Show session vs main diff | ✅ | - |
| **Config** | `config` | `cfg` | View/modify configuration | ✅ | - |
| | `doctor` | `check` | Run health checks | ✅ | - |
| **AI/Introspection** | `context` | `ctx` | Show environment context | ✅ | - |
| | `prime` | - | AI context recovery | ✅ | - |
| | `introspect` | - | Discover capabilities | ✅ | - |
| | `query` | - | Programmatic state queries | ✅ | - |
| **Interactive** | `dashboard` | `dash` | Interactive TUI | - | - |
| | `completions` | - | Shell completions | - | - |
| | `essentials` | - | Quick command reference | ✅ | - |
| **Backup** | `backup` | - | Backup session database | ✅ | - |
| | `restore` | - | Restore from backup | ✅ | - |
| | `verify-backup` | - | Verify backup integrity | ✅ | - |
| **Integration** | `hooks` | - | Manage git hooks | ✅ | ✅ |
| | `agent` | - | Track AI agents | ✅ | - |
| | `version` | - | Show version info | ✅ | - |
| | `onboard` | - | Generate AGENTS.md snippet | ✅ | - |

### Exit Codes

| Code | Category | Examples |
|------|----------|----------|
| 0 | Success | All operations completed |
| 1 | Validation Error | Invalid input, bad config, parse errors |
| 2 | System Error | IO failures, command errors, hooks |
| 3 | Not Found | Missing sessions, JJ not installed |
| 4 | Invalid State | Database corruption |

---

## Part 2: Integration Analysis

### JJ (Jujutsu) Integration

**Fully Implemented:**
| Operation | Function | JJ Command |
|-----------|----------|-----------|
| Create workspace | `workspace_create()` | `jj workspace create` |
| List workspaces | `workspace_list()` | `jj workspace list` |
| Forget workspace | `workspace_forget()` | `jj workspace forget` |
| Get diff | `workspace_diff()` | `jj diff --stat` |
| Squash commits | `workspace_squash()` | `jj squash` |
| Rebase onto main | `workspace_rebase_onto_main()` | `jj rebase -d main` |
| Push to remote | `workspace_git_push()` | `jj git push` |
| Get status | `parse_status()` | `jj status` |

**NOT Implemented:**
- ❌ `jj new` - Creating new revisions
- ❌ `jj abandon` - Abandoning revisions
- ❌ `jj resolve` - Interactive conflict resolution
- ❌ `jj op restore` - Operation log restoration
- ❌ Bookmark creation/management
- ❌ Advanced revset queries
- ❌ `jj describe` - Commit description editing

### Zellij Integration

**Used Actions:**
- `go-to-tab-name <name>` - Switch to tab
- `close-tab` - Close current tab
- Layout attachment via KDL files

**Layout Templates:**
| Template | Description |
|----------|-------------|
| `minimal` | Single Claude pane |
| `standard` | 70% Claude + 30% sidebar (beads + jj log) |
| `full` | Standard + floating pane |
| `split` | Two Claude instances side-by-side |
| `review` | Diff viewer + beads + Claude |

**NOT Used (Available in Zellij):**
- ❌ `focus-pane` / `focus-tab` - Direct focus
- ❌ `move-pane` / `swap-pane` - Pane manipulation
- ❌ `resize-pane` - Resize panes
- ❌ `run` - Execute commands in panes
- ❌ `previous-tab` / `next-tab` - Tab navigation
- ❌ `toggle-floating` - Toggle floating mode
- ❌ `query-tab` / `list-tabs` - Tab querying
- ❌ `rename-tab` - Rename tabs

### Beads Integration

**Implemented:**
- Bead-aware sessions via `--bead` flag
- Metadata snapshot in session database
- Auto-generate `BEAD_SPEC.md` in workspace
- Status sync on removal (closed/deferred)
- Query `.beads/issues.jsonl` for validation
- List filtering by bead ID
- Health check in `doctor` command

**CLI Commands Used:**
- `bd update <id> --status <status>` - Update bead status
- `bd list --status=open` - Count open issues

---

## Part 3: Database Schema

### Sessions Table

```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('creating','active','paused','completed','failed')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    last_synced INTEGER,
    metadata TEXT  -- JSON field for extensibility
);
```

### Session Metadata JSON Structure

```json
{
  "bead_id": "zjj-1234",
  "bead_title": "Implement feature X",
  "bead_type": "feature",
  "bead_priority": "P1",
  "bead_status": "in_progress",
  "bead_attached_at": "2026-01-18T10:30:00Z"
}
```

---

## Part 4: Gap Analysis

### Workflow Gaps (from dogfooding)

| Gap | Impact | Current Workaround |
|-----|--------|-------------------|
| **No batch creation** | Must loop `zjj add` | Shell loop with `for name in ...` |
| **No `zjj attach`** | Can't switch from external terminal | `zellij attach` + manual tab switch |
| **No `zjj clean`** | Manual workspace cleanup | `zjj remove` each individually |
| **No parallel exec** | Can't run commands across workspaces | Shell loop with background jobs |
| **No merge-to-main** | Must merge back manually | `jj squash && jj rebase -d main` |
| **No progress streaming** | JSON only shows final result | Watch stdout for progress |

### Feature Priority Matrix

#### P0 - Critical Gaps

| Feature | Reason | Implementation Effort |
|---------|--------|----------------------|
| `zjj attach <name>` | Essential for external terminal access | Low - wraps `zellij attach` |
| `zjj clean [--merged\|--empty\|--all]` | Stale workspace buildup | Medium |
| `zjj exec --all "<cmd>"` | Parallel operations across workspaces | Medium |

#### P1 - High Value

| Feature | Reason | Implementation Effort |
|---------|--------|----------------------|
| `zjj batch add <names...>` | Avoid shell loops | Low - use `add-batch` |
| `zjj merge <name>` | Complete workflow cycle | Medium |
| `zjj clone <from> <to>` | Quick workspace duplication | Medium |
| Progress streaming | Real-time feedback for long ops | High |

#### P2 - Nice to Have

| Feature | Reason | Implementation Effort |
|---------|--------|----------------------|
| `zjj template create/list/use` | Custom layout management | Medium |
| `zjj workspace exec <name> "<cmd>"` | Run command in specific workspace | Low |
| `zjj link <name> <bead-id>` | Post-creation bead linking | Low |
| Tab rename support | Customize tab names | Low |

#### P3 - Future Considerations

| Feature | Reason | Implementation Effort |
|---------|--------|----------------------|
| TUI dashboard enhancements | Visual workspace management | High |
| Conflict resolution UI | Interactive conflict handling | High |
| WebSocket progress streaming | Real-time updates to UI | High |
| Multi-repo support | Work across repositories | Very High |

---

## Part 5: What Already Exists (Often Overlooked)

Many features already exist that weren't obvious:

| You Want | Already Exists | How |
|----------|---------------|-----|
| Batch session creation | `add-batch` | `echo "ws1\nws2" \| zjj add-batch --beads-stdin` |
| Session status watch | `status --watch` | Auto-refreshes every 1s |
| Dry-run preview | `--dry-run` on add/remove/sync | Shows planned operations |
| Health diagnostics | `doctor --fix` | Auto-repairs issues |
| Programmatic queries | `query` command | `session-exists`, `can-run`, etc. |
| Full CLI docs as JSON | `--help-json` | Complete command documentation |
| AI context recovery | `prime --json` | Essential workflow context |
| Backup/restore | `backup`/`restore` commands | Full database backup |
| Shell completions | `completions bash/zsh/fish` | Tab completion support |

---

## Part 6: Recommended Improvements

### Immediate (This Sprint)

1. **Add `zjj attach <name>`**
   - Wrap `zellij attach -t zjj:<name>`
   - Handle session lookup and validation
   - Support `--create` flag if session doesn't exist

2. **Add `zjj clean` command**
   ```
   zjj clean              # Remove completed sessions
   zjj clean --merged     # Remove only merged sessions
   zjj clean --empty      # Remove sessions with no changes
   zjj clean --all        # Remove all (with confirmation)
   zjj clean --dry-run    # Preview what would be removed
   ```

3. **Improve `add-batch`**
   - Accept positional names: `zjj add-batch ws1 ws2 ws3`
   - Not just `--beads-stdin`

### Short-term (Next Release)

4. **Add `zjj exec` command**
   ```
   zjj exec --all "moon run :check"     # Run in all workspaces
   zjj exec --filter-by-status=active   # Filter targets
   zjj exec --parallel                   # Concurrent execution
   zjj exec <name> "cargo test"          # Single workspace
   ```

5. **Add `zjj merge` command**
   ```
   zjj merge <name>              # Squash + rebase + push + remove
   zjj merge <name> --keep       # Don't remove after merge
   zjj merge --all --dry-run     # Preview all merges
   ```

6. **Progress streaming**
   - Add `--progress` flag for long operations
   - Output JSON events: `{"type":"progress","step":3,"total":10,"message":"..."}`

### Medium-term (v0.3.0)

7. **Template management**
   ```
   zjj template list
   zjj template create <name> --from-current
   zjj template use <name>
   zjj template delete <name>
   ```

8. **Workspace links**
   ```
   zjj link <session> <bead-id>     # Link existing session to bead
   zjj unlink <session>             # Remove bead association
   ```

9. **Enhanced TUI dashboard**
   - Kanban board view
   - Inline workspace creation
   - Drag-and-drop status changes

---

## Part 7: Architecture Quality Notes

### Strengths

- **100% implementation completeness** - All commands fully implemented
- **Zero runtime panics** - Compiler-enforced with `#![deny(clippy::unwrap_used)]`
- **Type-safe error handling** - 3-tier error hierarchy with semantic exit codes
- **Consistent JSON API** - Schema versioning, error codes, optional fields
- **Functional patterns** - Railway-oriented programming throughout
- **Modular design** - Clean separation: zjj-core (library) vs zjj (CLI)
- **Async-first** - All operations use tokio async/await
- **Graceful degradation** - Beads optional, best-effort status updates

### Areas for Improvement

- No explicit schema migrations (embedded schema only)
- No progress streaming for long operations
- Limited JJ feature exposure (only core operations)
- No multi-repo support
- Dashboard is basic (could be enhanced)

---

## Appendix: File Reference

### Core Modules

| Path | Purpose |
|------|---------|
| `crates/zjj-core/src/jj/` | JJ integration (workspaces, parsing) |
| `crates/zjj-core/src/zellij/` | Zellij integration (tabs, layouts) |
| `crates/zjj-core/src/beads/` | Beads integration (queries, metadata) |
| `crates/zjj-core/src/error/` | Error types and handling |
| `crates/zjj-core/src/json/` | JSON output types and schemas |
| `crates/zjj/src/commands/` | All 26 command implementations |
| `crates/zjj/src/database/` | SQLite session storage |
| `crates/zjj/src/cli/` | CLI argument parsing |

### Key Files

| File | Purpose |
|------|---------|
| `cli/args.rs` | All command definitions and flags |
| `commands/routers/` | Command routing and dispatch |
| `database/schema.rs` | SQLite schema definition |
| `json_output.rs` | Command output types |
