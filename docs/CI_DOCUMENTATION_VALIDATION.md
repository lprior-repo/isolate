# CI Documentation Validation

**Status**: Implementation Complete  
**Version**: 1.0.0  
**Last Updated**: 2026-02-12

---

## Overview

This directory contains validation tools to ensure documentation truthfulness against actual implementation.

---

## Scripts

### `ci-docs-check.sh`

Validates zjj command documentation against actual binary implementation.

**Purpose**:  
- Verify documented zjj commands exist  
- Check for expected flags (e.g., `--json`)  
- Generate JSON validation report

**Usage**:
```bash
# Check all commands against documented commands
./tools/ci-docs-check.sh docs/

# Specify custom binary location
ZJJ_BIN=/path/to/zjj ./tools/ci-docs-check.sh docs/

# Specify custom docs directory
./tools/ci-docs-check.sh /path/to/docs/
```

**Exit Codes**:
- `0` - All documented commands validated successfully
- `1` - Missing commands or flag mismatches detected

**Output**:
- Console: Human-readable validation status
- File: `/tmp/ci-docs-check-report.json` (detailed JSON report)

**Validation Checks**:
1. Command exists in `zjj --help` output
2. Expected flags present (e.g., `--json` for `context`, `status`, `list`, `done`)
3. Command is executable

---

### `ci-tasks-check.sh`

Validates moon task documentation against `.moon/tasks.yml`.

**Purpose**:  
- Verify documented moon tasks exist  
- Check task configuration completeness  
- Detect CI integration gaps

**Usage**:
```bash
# Check all tasks against .moon/tasks.yml
./tools/ci-tasks-check.sh docs/

# Specify custom tasks file
TASKS_FILE=/path/to/tasks.yml ./tools/ci-tasks-check.sh docs/

# Specify custom docs directory
./tools/ci-tasks-check.sh /path/to/docs/
```

**Exit Codes**:
- `0` - All documented tasks validated successfully
- `1` - Missing tasks or configuration issues detected

**Output**:
- Console: Human-readable validation status
- File: `/tmp/ci-tasks-check-report.json` (detailed JSON report)

**Validation Checks**:
1. Task name exists in `.moon/tasks.yml`
2. Task has `command` definition
3. Task has `description`
4. Task is configured for CI (`runInCI: true` or missing)

---

## Integration

### Add to `.moon/tasks.yml`:

```yaml
docs-check:
  command: "sh tools/ci-docs-check.sh"
  description: "Validate zjj command documentation"
  inputs:
    - "docs/**/*.md"
    - "crates/zjj/src/commands/**/*.rs"
  options:
    cache: false
    runInCI: true

tasks-check:
  command: "sh tools/ci-tasks-check.sh"
  description: "Validate moon task documentation"
  inputs:
    - ".moon/tasks.yml"
    - "docs/**/*.md"
  options:
    cache: false
    runInCI: true
```

### Add to CI Pipeline:

```yaml
ci-docs:  # New CI task
  command: "true"
  description: "Documentation validation pipeline"
  deps:
    - "~:fmt"
    - "~:clippy"
    - "~:docs-check"
    - "~:tasks-check"
    - "~:test"
    - "build"
  options:
    cache: false
```

---

## Validation Reports

### `ci-docs-check-report.json`

```json
{
  "timestamp": "2026-02-12T14:30:45+00:00",
  "binary": "/home/lewis/src/zjj/target/release/zjj",
  "summary": {
    "documented": 8,
    "found": 8,
    "missing": 0,
    "flag_issues": 0,
    "total_errors": 0
  },
  "missing_commands": [],
  "flag_issues": []
}
```

### `ci-tasks-check-report.json`

```json
{
  "timestamp": "2026-02-12T14:30:45+00:00",
  "tasks_file": "/home/lewis/src/zjj/.moon/tasks.yml",
  "summary": {
    "documented": 17,
    "found": 8,
    "missing": 9,
    "config_issues": 2,
    "total_errors": 11
  },
  "missing_tasks": ["lint", "mutants", "llm-judge", ...],
  "config_issues": ["audit (no CI)", ...]
}
```

---

## Documentation Standards

### zjj Commands

**Required for all commands**:
- Must be documented in `docs/*.md`
- Command must exist in `zjj --help` output
- Should support `--json` flag where appropriate

**Current Validated Commands**:
- `zjj context --json`
- `zjj status --json`
- `zjj list --json`
- `zjj done --json`
- `zjj work --contract`
- `zjj schema --json`
- `zjj checkpoint`
- `zjj revert`

### Moon Tasks

**Required for all documented tasks**:
- Must be defined in `.moon/tasks.yml`
- Should have `command` and `description`
- Should be configured for CI (`runInCI: true`)

**Current Tasks in `.moon/tasks.yml`**:
- `fmt`, `fmt-fix`, `clippy` - Code quality
- `test`, `test-doc`, `test-research` - Testing
- `coverage`, `coverage-check` - Coverage
- `audit` - Security (placeholder)
- `build`, `build-docs` - Build
- `quick`, `ci`, `dev` - Composite pipelines
- `install`, `clean`, `check` - Utilities

---

## Troubleshooting

### Script fails with "binary not found"
```bash
# Build zjj first
moon run :build

# Or specify binary location
ZJJ_BIN=$(moon run :build && echo target/release/zjj) ./tools/ci-docs-check.sh
```

### Script reports missing commands
1. Check command exists in `docs/*.md`
2. Verify command implemented in `crates/zjj/src/commands/`
3. Build and test: `moon run :build && ./zjj --help`

### Script reports missing tasks
1. Check task exists in `.moon/tasks.yml`
2. Verify task documented in `docs/*.md`
3. Add missing task or remove from docs

---

## Maintenance

### Add New Command

1. Implement command in `crates/zjj/src/commands/`
2. Document in `docs/*.md` with usage examples
3. Add to CI check: `./tools/ci-docs-check.sh docs/`
4. Verify: `moon run :docs-check`

### Add New Task

1. Define in `.moon/tasks.yml`
2. Document in `docs/*.md` with usage examples
3. Add to CI check: `./tools/ci-tasks-check.sh docs/`
4. Verify: `moon run :tasks-check`

---

## Related Documentation

- **[02_MOON_BUILD.md](../docs/02_MOON_BUILD.md)** - Moon build system
- **[10_MOON_CICD_INDEXED.md](../docs/10_MOON_CICD_INDEXED.md)** - Moon tasks reference
- **[CI_GOVERNANCE_REPORT_BD4VP.md](../docs/CI_GOVERNANCE_REPORT_BD4VP.md)** - Bead analysis report

---

**For issues**: Create bead with `br add --title "CI docs check: [issue]" --label bug`
