# Security Fixes Applied

Date: 2026-02-14

## Summary

Successfully addressed all security vulnerabilities and warnings found by `cargo audit` and configured `cargo-deny` for ongoing license and security compliance.

## Changes Made

### 1. Replaced Unmaintained `atty` Dependency ✅

**Issue:** `atty v0.2.14` is unmaintained and has an unsound API (potential unaligned read)

**Fix:**
- Removed `atty = "0.2"` from `crates/zjj/Cargo.toml`
- Replaced with standard library `std::io::IsTerminal`
- Updated code in `crates/zjj/src/commands/status.rs`:
  ```rust
  // Old: let is_tty = atty::is(atty::Stream::Stdout);
  // New: let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
  ```

### 2. Removed Unmaintained `paste` Dependency ✅

**Issue:** `paste v1.0.15` is no longer maintained

**Fix:**
- Removed `paste = "1.0"` from dev-dependencies (it wasn't being used)

### 3. Eliminated RSA Vulnerability ✅

**Issue:** `rsa v0.9.10` has Marvin Attack vulnerability (RUSTSEC-2023-0071)
- Came from `sqlx-mysql` transitive dependency

**Fix:**
- Updated sqlx configuration to explicitly disable default features:
  ```toml
  sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "sqlite", "macros"] }
  ```
- This prevents pulling in MySQL support and the vulnerable `rsa` crate
- Applied to both `crates/zjj/Cargo.toml` and `crates/zjj-core/Cargo.toml`

### 4. Fixed `lru` Unsound Warning ✅

**Issue:** `lru v0.12.5` has unsound API (RUSTSEC-2026-0002)
- Used by `ratatui v0.26.3`

**Fix:**
- Updated `ratatui` from `0.26` to `0.30`
- This pulls in `lru v0.16.3` which fixes the issue
- Updated deprecated API calls: `.size()` → `.area()`

### 5. Configured `cargo-deny` ✅

**Created:** `deny.toml` configuration file

**Features:**
- License compliance checking with allowed licenses:
  - MIT, Apache-2.0, ISC, 0BSD, Unlicense
  - MPL-2.0 (Mozilla Public License)
  - Unicode-3.0 (for ICU crates)
  - Zlib, CC0-1.0
- Security advisory scanning
- Multiple version detection (warns)
- Unknown source detection (warns)

## Verification

### cargo audit Results
```
✅ 0 vulnerabilities found
✅ 0 warnings
```

Previously:
- 1 vulnerability (RSA Marvin Attack)
- 4 warnings (atty unmaintained/unsound, paste unmaintained, lru unsound)

### cargo deny Results
```
✅ advisories ok
✅ bans ok  
✅ licenses ok
✅ sources ok
```

## Files Modified

1. `crates/zjj/Cargo.toml` - Updated dependencies
2. `crates/zjj-core/Cargo.toml` - Updated sqlx configuration
3. `crates/zjj/src/commands/status.rs` - Replaced atty with std::io
4. `crates/zjj/src/selector.rs` - Updated ratatui API calls
5. `deny.toml` - New file for cargo-deny configuration
6. `Cargo.lock` - Updated dependency tree

## Dependencies Updated

| Crate | Old Version | New Version | Reason |
|-------|-------------|-------------|--------|
| atty | 0.2.14 | removed | Unmaintained, replaced with stdlib |
| paste | 1.0.15 | removed | Unmaintained, unused |
| ratatui | 0.26 | 0.30 | Fix lru vulnerability |
| lru | 0.12.5 | 0.16.3 | (transitive) Fix unsound API |
| sqlx | 0.8 | 0.8 | Disabled default features to exclude MySQL |

## Recommendations

1. **Add to CI/CD:** Run `cargo audit` and `cargo deny check` in your CI pipeline
2. **Regular Updates:** Check for security advisories weekly
3. **Dependency Review:** Review new dependencies before adding them
4. **Monitor:** Set up automated alerts for security advisories

## Commands for Future Use

```bash
# Check for security vulnerabilities
cargo audit

# Check licenses and advisories
cargo deny check

# Update advisory database
cargo audit --update

# Check for outdated dependencies
cargo outdated
```

