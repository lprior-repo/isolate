# Moon Task Documentation Cleanup Plan

**Date**: 2026-02-12  
**Source**: `docs/10_MOON_CICD_INDEXED.md` and `docs/02_MOON_BUILD.md`  
**Target**: `.moon/tasks.yml`

---

## Issues Found

### Documented Tasks NOT in `.moon/tasks.yml`

| Task Name | Documented In | Line | Status |
|-----------|--------------|------|--------|
| `lint` | 10_MOON_CICD_INDEXED.md | 63 | ❌ Missing |
| `mutants` | 10_MOON_CICD_INDEXED.md | 121 | ❌ Missing |
| `llm-judge` | 10_MOON_CICD_INDEXED.md | 141 | ❌ Missing |
| `llm-judge-fix-suggestions` | 10_MOON_CICD_INDEXED.md | 161 | ❌ Missing |
| `cd-gates` | 10_MOON_CICD_INDEXED.md | 238 | ❌ Missing |
| `deps-check` | 10_MOON_CICD_INDEXED.md | 191 | ❌ Missing |
| `test-properties` | 10_MOON_CICD_INDEXED.md | 102 | ❌ Missing |
| `quality` | 10_MOON_CICD_INDEXED.md | 272 | ❌ Missing |
| `deploy` | 02_MOON_BUILD.md | 32 | ❌ Missing |

### Actual Tasks in `.moon/tasks.yml`

| Task Name | Description | Has `runInCI` |
|-----------|-------------|---------------|
| `fmt` | Check formatting | ✅ |
| `fmt-fix` | Auto-fix formatting | ❌ |
| `clippy` | Lint with strict mode | ✅ |
| `test` | Run tests (nextest) | ✅ |
| `test-doc` | Run doc tests | ✅ |
| `test-research` | DRQ research lane | ❌ |
| `coverage` | Generate coverage report | ✅ |
| `coverage-check` | Check 80% threshold | ✅ |
| `audit` | Security audit (placeholder) | ❌ |
| `build` | Release build | ✅ |
| `build-docs` | Generate docs | ✅ |
| `quick` | Fast lint check | ❌ |
| `ci` | Full CI pipeline | ❌ |
| `dev` | Build + install | ❌ |
| `install` | Install binary | ❌ |
| `clean` | Clean artifacts | ❌ |
| `check` | Fast type check | ❌ |

---

## Cleanup Actions

### 1. Remove Non-Existent Task References

**In `docs/10_MOON_CICD_INDEXED.md`**:

```diff
# Remove section:
- `lint`
- `mutants`
- `llm-judge`
- `llm-judge-fix-suggestions`
- `cd-gates`
- `deps-check`
- `test-properties`
- `quality`
```

**In `docs/02_MOON_BUILD.md`**:

```diff
# Remove:
- `moon run :deploy`
```

### 2. Update Task Documentation

**`docs/10_MOON_CICD_INDEXED.md` line 77-90**:

```diff
#### `test`
- **Command**: `cargo test --workspace --all-features`
+ **Command**: `cargo nextest run --workspace --all-features --no-fail-fast -E 'not (package(zjj) & binary(drq_adversarial))'`
- **Description**: Run all unit tests
+ **Description**: Run unit tests with nextest (DRQ excluded)
```

**`docs/10_MOON_CICD_INDEXED.md` line 179-190**:

```diff
#### `audit`
- **Command**: `cargo audit --deny warnings`
+ **Command**: `echo 'Skipping audit - cargo-audit not installed'`
- **Description**: Security audit of dependencies
+ **Description**: Security audit placeholder (cargo-audit not installed)
```

**`docs/10_MOON_CICD_INDEXED.md` line 262-271**:

```diff
### `quick` - Fast Local Check
- **Command**: No-op (orchestrator only)
+ **Command**: Actually runs `:fmt` and `:clippy` in parallel
- **Dependencies**:
  - `fmt`
  - `clippy`
+ **What It Runs**:
  1. Code formatting check
  2. Lint checks (strict mode)
```

### 3. Update Tasks Reference

**`docs/10_MOON_CICD_INDEXED.md` line 602**:

```diff
**Tasks**: 
- fmt, fmt-fix, clippy,
- test, test-doc, test-research,
- coverage, coverage-check,
- audit, 
- build, build-docs,
- quick, ci, dev, install, clean, check
```

### 4. Add Missing Tasks

**For Future Implementation**:

```yaml
lint:
  command: "cargo doc --no-deps --document-private-items 2>&1 | grep -E '(warning|error)' || true"
  description: "Check documentation completeness"
  inputs:
    - "crates/**/*.rs"
    - "Cargo.toml"
  options:
    cache: true
    runInCI: false

mutants:
  command: "sh .moon/scripts/mutation-test.sh"
  description: "Run mutation testing"
  inputs:
    - "crates/**/*.rs"
    - "tests/**/*.rs"
    - "Cargo.toml"
  options:
    cache: false
    runInCI: false

llm-judge:
  command: "python3 .moon/scripts/llm-judge.py"
  description: "LLM code review (Claude as judge)"
  inputs:
    - "crates/**/*.rs"
    - "Cargo.toml"
  options:
    cache: false
    runInCI: false

llm-judge-fix-suggestions:
  command: "python3 .moon/scripts/llm-judge.py --suggest-fixes"
  description: "Generate LLM improvement suggestions"
  inputs:
    - "crates/**/*.rs"
    - "Cargo.toml"
  options:
    cache: false
    runInCI: false

cd-gates:
  command: "sh .moon/scripts/cd-gates.sh"
  description: "Verify CD readiness"
  inputs:
    - "crates/**/*.rs"
    - "Cargo.toml"
  options:
    cache: false
    runInCI: false

deps-check:
  command: "cargo tree --duplicates"
  description: "Check for duplicate dependencies"
  inputs:
    - "Cargo.lock"
    - "Cargo.toml"
  options:
    cache: true
    runInCI: false

test-properties:
  command: "cargo test --test '*' --features proptest --workspace --all-features -- --test-threads 1"
  description: "Run property-based tests"
  inputs:
    - "crates/**/*.rs"
    - "tests/**/*.rs"
    - "Cargo.toml"
  options:
    cache: true
    runInCI: false

quality:
  command: "true"
  description: "All quality gates without build"
  deps:
    - "~:fmt"
    - "~:clippy"
    - "~:lint"
    - "~:test"
    - "~:test-doc"
    - "~:audit"
    - "~:deps-check"
    - "~:llm-judge"
  options:
    cache: false

deploy:
  command: "true"
  description: "Full CI + CD gates"
  deps:
    - "~:ci"
    - "cd-gates"
  options:
    cache: false
```

---

## Verification

After cleanup:

```bash
# Check current state
./tools/ci-tasks-check.sh docs/

# Should report: 17 documented, 17 found, 0 missing
```

---

## Timeline

1. **Immediate** (This Session)
   - [ ] Remove non-existent task references from docs
   - [ ] Fix audit task description
   - [ ] Test validation scripts

2. **Short Term** (Next PR)
   - [ ] Add missing tasks to `.moon/tasks.yml`
   - [ ] Update documentation with new tasks
   - [ ] Test CI pipeline integration

3. **Long Term** (Future)
   - [ ] Implement actual mutation testing
   - [ ] Implement LLM code review scripts
   - [ ] Install cargo-audit and enable security checks
