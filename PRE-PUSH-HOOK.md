# Pre-Push Validation Hook

## Overview

A git pre-push hook is installed at `.git/hooks/pre-push` that enforces code quality before every push to remote.

## What It Does

The hook runs **4 mandatory validation gates** before allowing any push:

1. **📝 Format Check** - `moon run :quick`
   - Validates code formatting (rustfmt)
   - Runs clippy lints
   
2. **🔨 Compilation Check** - `moon run :check`  
   - Fast type-checking
   - Catches syntax/type errors

3. **🧪 Test Suite** - `moon run :test`
   - Runs all tests
   - Ensures functionality works

4. **🚀 Full CI Pipeline** - `moon run :ci`
   - Complete validation
   - Format + clippy + tests + build

## Workflow

### Normal Flow (Recommended)
```bash
# 1. Pre-validate locally (catch issues early)
moon run :ci

# 2. Commit changes
git add .
git commit -m "feat: description"

# 3. Push - hook validates automatically
git push
```

### If Hook Blocks Push
```bash
# 1. Read error output (tells you what failed)

# 2. Fix the issue
moon run :fmt-fix      # For formatting
# or fix compilation/test errors

# 3. Verify fix
moon run :ci

# 4. Try push again
git push
```

### Emergency Override (⚠️ Use with caution)
```bash
git push --no-verify   # Bypasses hook
# ONLY for critical emergencies
# MUST fix issues immediately after
```

## Why This Matters

✅ **Protects Main Branch**
- No broken code can reach repository
- All commits are formatted, compilable, tested

⚡ **Fast Feedback**  
- Catch issues locally before CI/CD
- Fix immediately with clear instructions

🔒 **Enforces Standards**
- Zero unwrap/panic/expect (via clippy)
- Functional programming patterns
- Consistent formatting

## Features

- **Colorized Output**: Blue/green/red/yellow for clarity
- **Clear Error Messages**: Tells you exactly what failed
- **Fix Instructions**: Provides commands to resolve issues
- **Blocks Bad Pushes**: Returns exit code 1 to stop push
- **Emergency Bypass**: `--no-verify` option documented

## Integration with TDD15

Phase 15 (LANDING) now includes automatic hook validation:

```
1. Run moon run :ci (catch issues early)
2. Commit with bead ID  
3. git push (hook validates automatically)
4. Close bead (if push succeeds)
```

## Troubleshooting

**Hook says "Formatting issues detected"**
```bash
moon run :fmt-fix
git add .
git commit --amend --no-edit
git push
```

**Hook says "Compilation errors"**
- Fix the code errors shown in output
- Run `moon run :check` to verify
- Push again

**Hook says "Tests failing"**
- Fix failing tests
- Run `moon run :test` to verify  
- Push again

**Hook says "CI pipeline failed"**
- Review full error output
- Fix all issues found
- Run `moon run :ci` to verify
- Push again

## Golden Rule

**If the hook blocks your push, the code isn't ready. Fix it, don't bypass it.**

The hook exists to prevent broken code from reaching the remote repository and breaking the build for everyone. Bypassing it should only happen in true emergencies, and issues must be fixed immediately after.

---

**Created**: 2026-01-25  
**Location**: `.git/hooks/pre-push`  
**Status**: ✅ Active and enforcing quality gates
