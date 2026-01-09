# P0 Critical Fixes - Quick Implementation Guide

**Estimated Total Time**: 4-6 hours
**Target**: Get test suite passing and fix critical panics

---

## Issue 1: Unicode Names Cause Panic (zjj-oez)
**Time**: 1 hour | **Files**: `crates/zjj/src/session.rs`

### The Problem
```rust
// Current validation allows unicode
"中文名字" -> validation PASS -> create workspace -> launch Zellij -> PANIC!
```

### The Fix
Update `validate_name()` in `src/session.rs`:

```rust
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!(ValidationError::EmptyName);
    }

    // NEW: Check for non-ASCII characters
    if !name.is_ascii() {
        bail!(ValidationError::NonAsciiName);
    }

    if name.len() > MAX_NAME_LENGTH {
        bail!(ValidationError::NameTooLong);
    }

    // Existing alphanumeric check
    let valid_chars = name.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    });

    if !valid_chars {
        bail!(ValidationError::InvalidChars);
    }

    Ok(())
}
```

Add to `ValidationError` enum:
```rust
#[derive(Debug)]
pub enum ValidationError {
    EmptyName,
    NonAsciiName,  // NEW
    NameTooLong,
    InvalidChars,
    StartsWithDash,  // For issue #3
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Session name cannot be empty"),
            Self::NonAsciiName => write!(f, "Session name must contain only ASCII characters (a-z, 0-9, -, _)"),
            // ... rest
        }
    }
}
```

### Test Cases to Add
```rust
#[test]
fn test_validate_name_rejects_unicode() {
    let cases = vec![
        "中文名字",
        "日本語",
        "café",
        "Ñoño",
        "🚀rocket",
    ];

    for name in cases {
        assert!(validate_name(name).is_err());
    }
}
```

---

## Issue 2: Test Suite Thread Safety (zjj-pxv)
**Time**: 2-3 hours | **Files**: `crates/zjj/src/commands/init.rs`

### The Problem
Tests use `std::env::set_current_dir()` which mutates global process state:
```rust
// Thread 1 sets cwd to /tmp/a
// Thread 2 sets cwd to /tmp/b
// Thread 1 checks files in /tmp/a but is now in /tmp/b -> FAIL
```

### The Fix (Option A: Recommended)
Pass working directory to `run()`:

```rust
// Update function signature
pub fn run(working_dir: Option<&Path>) -> Result<()> {
    check_dependencies()?;
    ensure_jj_repo(working_dir)?;

    let root = if let Some(dir) = working_dir {
        dir.to_str().ok_or_else(|| anyhow!("Invalid path"))?
    } else {
        &jj_root()?
    };

    let zjj_dir = format!("{root}/.jjz");
    // ... rest of implementation
}
```

Update tests:
```rust
#[test]
fn test_init_creates_jjz_directory() -> Result<()> {
    let temp_dir = setup_test_jj_repo()?;

    // NO MORE set_current_dir!
    let result = run(Some(temp_dir.path()));

    result?;

    let jjz_path = temp_dir.path().join(".jjz");
    assert!(jjz_path.exists());

    Ok(())
}
```

### The Fix (Option B: Simpler)
Use `#[serial]` attribute from `serial_test` crate:

```toml
[dev-dependencies]
serial_test = "3.0"
```

```rust
use serial_test::serial;

#[test]
#[serial]  // Forces sequential execution
fn test_init_creates_jjz_directory() -> Result<()> {
    // Original code works now
}
```

**Recommendation**: Use Option A (pass working_dir) for proper test isolation.

---

## Issue 3: Dash-Prefixed Names (zjj-hv7)
**Time**: 30 minutes | **Files**: `crates/zjj/src/session.rs`

### The Problem
```bash
$ jjz add "-myname"
# Clap tries to parse -m, -y, -n... as flags!
```

### The Fix
Update `validate_name()`:

```rust
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!(ValidationError::EmptyName);
    }

    // NEW: Check first character
    if let Some(first) = name.chars().next() {
        if first == '-' {
            bail!(ValidationError::StartsWithDash);
        }
        if first == '_' {
            bail!(ValidationError::StartsWithUnderscore);
        }
        if !first.is_ascii_alphabetic() {
            bail!(ValidationError::MustStartWithLetter);
        }
    }

    // ... rest of validation
}
```

Update error messages:
```rust
impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::StartsWithDash => {
                write!(f, "Session name cannot start with a dash (-)\n")?;
                write!(f, "Tip: Use a letter instead, e.g., 'my-feature'")
            }
            Self::MustStartWithLetter => {
                write!(f, "Session name must start with a letter (a-z, A-Z)\n")?;
                write!(f, "Example: 'feature-123' not '123-feature'")
            }
            // ...
        }
    }
}
```

### Test Cases
```rust
#[test]
fn test_validate_name_rejects_dash_prefix() {
    assert!(validate_name("-name").is_err());
    assert!(validate_name("--name").is_err());
    assert!(validate_name("_name").is_err());
    assert!(validate_name("123-name").is_err());
}

#[test]
fn test_validate_name_accepts_valid_names() {
    assert!(validate_name("name").is_ok());
    assert!(validate_name("my-name").is_ok());
    assert!(validate_name("name123").is_ok());
    assert!(validate_name("n-a-m-e").is_ok());
}
```

---

## Implementation Order

1. **Fix Issue 3 first** (30 min) - Easiest, unblocks testing
2. **Fix Issue 1** (1 hour) - Critical panic, straightforward fix
3. **Fix Issue 2** (2-3 hours) - Most complex, but unblocks CI/CD

## Verification Checklist

After implementing fixes:

```bash
# 1. Build succeeds
moon run :build

# 2. Tests pass
moon run :test
# Should see: test result: ok. 131 passed; 0 failed

# 3. Manual verification
jjz add "-badname"     # Should error clearly
jjz add "中文"          # Should error clearly
jjz add "goodname"     # Should work

# 4. Full CI pipeline
moon run :ci
# All steps should pass
```

---

## Expected Results

### Before Fixes
```
zjj:test | test result: FAILED. 125 passed; 6 failed; 0 ignored
```

### After Fixes
```
zjj:test | test result: ok. 131 passed; 0 failed; 0 ignored
```

---

## Notes

- All three fixes are **backward compatible** (stricter validation)
- No database migrations needed
- No config changes needed
- Existing valid sessions continue to work
- Only new sessions with invalid names are rejected

---

## Questions?

Check the full audit report: `JJZ_AUDIT_REPORT.md`
Or view the beads issues:
```bash
bd show zjj-oez  # Unicode panic
bd show zjj-pxv  # Test failures
bd show zjj-hv7  # Dash prefix
```
