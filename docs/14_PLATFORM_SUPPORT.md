# Platform Support Matrix

ZJJ platform compatibility, limitations, and feature availability across Linux, macOS, and Windows.

**Last Updated**: 2026-01-11
**Tested Platforms**: Linux (Arch 6.18.3), macOS (untested), Windows (untested)

## Quick Reference

| Feature | Linux | macOS | Windows | Notes |
|---------|-------|-------|---------|-------|
| **Core Commands** | ✅ Full | ✅ Expected | ⚠️ Partial | See limitations below |
| **JJ Integration** | ✅ Full | ✅ Expected | ✅ Expected | JJ is cross-platform |
| **Zellij Integration** | ✅ Full | ✅ Expected | ❌ No | Zellij Linux/macOS only |
| **Auto-spawn Zellij** | ✅ Yes | ✅ Expected | ❌ No | Unix `exec()` required |
| **Permission Checks** | ✅ Full | ✅ Expected | ⚠️ Basic | Unix permissions only |
| **SQLite Database** | ✅ Full | ✅ Full | ✅ Full | Bundled, cross-platform |
| **Process Execution** | ✅ Full | ✅ Full | ⚠️ Partial | `which` command differs |
| **Configuration** | ✅ Full | ✅ Full | ✅ Full | Cross-platform |

**Legend**: ✅ = Full support, ⚠️ = Partial/degraded, ❌ = Not supported

## Platform Details

### Linux (Primary Platform)

**Status**: ✅ **Fully Supported & Tested**

**Tested Distribution**: Arch Linux 6.18.3-arch1-1
**Rust Version**: 1.80+
**JJ Version**: 0.36.0
**Zellij Version**: 0.43.1

**Features**:
- ✅ All core commands (`init`, `add`, `list`, `remove`, `focus`, `status`, `sync`, etc.)
- ✅ JJ workspace management
- ✅ Zellij tab integration with KDL layouts
- ✅ Auto-spawn Zellij session (via Unix `exec()`)
- ✅ Full Unix permission checking (read/write/execute)
- ✅ Symlink validation for security
- ✅ SQLite session database
- ✅ Configuration system (TOML)
- ✅ Beads integration (optional)

**Known Issues**: None

**Installation**:
```bash
# Install dependencies
sudo pacman -S jj zellij rust sqlite

# Build ZJJ
git clone https://github.com/lprior-repo/zjj.git
cd zjj
moon run :build
cargo install --path crates/zjj

# Verify
zjj --version
```

---

### macOS

**Status**: ✅ **Expected to Work** (Untested)

**Requirements**:
- macOS 10.15+ (Catalina or later recommended)
- Rust 1.80+
- JJ 0.8.0+
- Zellij 0.35.1+
- Homebrew (for dependencies)

**Expected Features**:
- ✅ All core commands
- ✅ JJ workspace management
- ✅ Zellij tab integration
- ✅ Auto-spawn Zellij (Unix `exec()` available)
- ✅ Full Unix permission checking
- ✅ Symlink validation
- ✅ SQLite database
- ✅ Configuration system

**Potential Issues**:
- ⚠️ Filesystem case sensitivity (APFS defaults to case-insensitive)
  - Impact: Session names `foo` and `Foo` may conflict
  - Mitigation: Use lowercase session names

- ⚠️ `which` command location
  - Modern macOS may use different PATH for `which`
  - Handled by `which` crate (v7.0), should work

**Installation** (Expected):
```bash
# Install dependencies via Homebrew
brew install jujutsu zellij rust sqlite

# Build ZJJ (same as Linux)
git clone https://github.com/lprior-repo/zjj.git
cd zjj
moon run :build
cargo install --path crates/zjj

# Verify
zjj --version
```

**Testing Needed**:
- [ ] Verify all commands work
- [ ] Test Zellij integration
- [ ] Verify permission checks
- [ ] Test with case-insensitive filesystem
- [ ] Document any macOS-specific issues

---

### Windows

**Status**: ⚠️ **Partial Support** (Untested)

**Limitations**: Zellij does not support Windows. ZJJ is designed to run inside Zellij, making it **not fully functional on Windows**.

**What Works**:
- ✅ JJ workspace management (JJ is cross-platform)
- ✅ SQLite database
- ✅ Configuration system
- ✅ Commands that don't require Zellij:
  - `zjj config` - View/edit configuration
  - `zjj doctor` - Check dependencies (will report Zellij missing)
  - `zjj introspect` - View internal state

**What Doesn't Work**:
- ❌ Auto-spawn Zellij - Unix `exec()` not available
  - Code location: `crates/zjj/src/cli.rs:127-130`
  - Error: "Auto-spawning Zellij is only supported on Unix systems"

- ❌ Zellij tab management - Zellij not available
  - `zjj add` - Creates workspace but fails on tab creation
  - `zjj focus` - Cannot switch tabs
  - `zjj remove` - Cannot close tabs

- ⚠️ Permission checking - Degraded
  - Code location: `crates/zjj/src/commands/add.rs:343-348`
  - Windows implementation: No-op, lets OS handle errors
  - Impact: Less helpful error messages for permission issues

**Workarounds**:

None currently. ZJJ fundamentally requires Zellij, which is Unix-only.

**Future Windows Support**:

To support Windows, ZJJ would need:
1. **Alternative to Zellij** - Windows Terminal, Windows Terminal tabs, or similar
2. **Platform abstraction layer** - Abstract terminal multiplexer operations
3. **Windows permission API** - Use Windows ACL instead of Unix permissions
4. **Path handling** - Handle Windows paths (`C:\`, backslashes)
5. **Process spawning** - Replace Unix `exec()` with Windows equivalent

This is a **significant undertaking** and not currently planned.

**Recommendation**: Use WSL2 (Windows Subsystem for Linux) on Windows:

```powershell
# In PowerShell (as Administrator)
wsl --install -d Ubuntu

# Inside WSL Ubuntu
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev sqlite3
cargo install jj-cli
cargo install --locked zellij

# Clone and build ZJJ (same as Linux)
git clone https://github.com/lprior-repo/zjj.git
cd zjj
moon run :build
cargo install --path crates/zjj
```

---

## Platform-Specific Code

### Unix-Only Code

**Permission Checking** (`crates/zjj/src/commands/add.rs:260-292`):
```rust
#[cfg(unix)]
fn check_workspace_writable(workspace_path: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // ... Unix permission checking with mode bits (0o200, etc.)
}
```

**Windows Stub** (`crates/zjj/src/commands/add.rs:343-348`):
```rust
#[cfg(not(unix))]
fn check_workspace_writable(_workspace_path: &str) -> Result<()> {
    // No-op: let OS handle permission errors
    Ok(())
}
```

**Auto-spawn Zellij** (`crates/zjj/src/cli.rs:72-125`):
```rust
#[cfg(unix)]
pub fn attach_to_zellij_session(layout_content: Option<&str>) -> Result<()> {
    // Uses Unix exec() to replace process
    cmd.exec(); // Only available on Unix
}
```

**Windows Stub** (`crates/zjj/src/cli.rs:127-130`):
```rust
#[cfg(not(unix))]
pub fn attach_to_zellij_session(_layout_content: Option<&str>) -> Result<()> {
    anyhow::bail!("Auto-spawning Zellij is only supported on Unix systems");
}
```

### Cross-Platform Dependencies

All dependencies are cross-platform compatible:

- `rusqlite` (v0.34) - SQLite with bundled library
- `clap` (v4.5) - CLI parsing
- `serde` (v1.0) - Serialization
- `toml` (v0.8) - Configuration
- `directories` (v6) - Cross-platform paths
- `which` (v7.0) - Command lookup (handles Windows/Unix differences)
- `anyhow` (v1.0) - Error handling
- `tokio` (v1) - Async runtime

## Testing Matrix

| Test Suite | Linux | macOS | Windows |
|-------------|-------|-------|---------|
| Unit tests | ✅ Pass | ❓ | ❓ |
| Integration tests | ✅ Pass | ❓ | ⚠️ Expected failures |
| E2E MVP commands | ✅ Pass | ❓ | ❌ Requires Zellij |
| Error recovery | ✅ Pass | ❓ | ⚠️ Partial |
| TTY detection | ✅ Pass | ❓ | ❓ |

**Note**: ❓ = Not tested yet

## Recommendations

### For Users

- **Linux**: ✅ Use ZJJ directly, fully supported
- **macOS**: ✅ Should work, please report issues
- **Windows**: ❌ Use WSL2, see WSL setup above

### For Contributors

**To improve macOS support**:
1. Test on macOS 10.15+
2. Document any macOS-specific issues
3. Verify with `moon run :ci` locally
4. Update this document with findings

**To add Windows support** (future):
1. Abstract Zellij operations behind trait
2. Implement Windows Terminal backend
3. Add Windows permission checking
4. Extend Moon CI/CD support
5. Update platform matrix

## CI/CD Platform Coverage

**Current CI** (Moon-based):
- ✅ Linux - Primary
- ✅ macOS - Expected (test locally with `moon run :ci`)
- ❌ Windows - Zellij dependency blocks support

**Run CI Locally**:
```bash
# Run full Moon CI pipeline
moon run :ci

# Run specific platform tests (if on that platform)
moon run :test
moon run :check
moon run :clippy
```

## External Dependencies Platform Support

### JJ (Jujutsu)

**Platforms**: Linux, macOS, Windows
**Status**: ✅ Full cross-platform support
**Source**: https://github.com/martinvonz/jj

### Zellij

**Platforms**: Linux, macOS
**Status**: ❌ No Windows support
**Source**: https://github.com/zellij-org/zellij
**Issue**: https://github.com/zellij-org/zellij/issues/641

**This is the blocking issue for Windows support.**

### Beads (Optional)

**Platforms**: Linux, macOS, Windows (likely)
**Status**: ✅ Cross-platform (Go-based)
**Source**: https://github.com/beadorg/beads

## Related Documentation

- [00_START_HERE.md](00_START_HERE.md) - Installation guide
- [11_ARCHITECTURE.md](11_ARCHITECTURE.md) - System architecture
- [../README.md](../README.md) - Prerequisites

## Version History

- **2026-01-11**: Initial platform matrix documentation
  - Documented Linux (tested), macOS (expected), Windows (limited)
  - Identified Zellij as Windows blocker
  - Recommended WSL2 for Windows users

## Future Work

- [ ] Test on macOS 10.15+
- [ ] Test on macOS with case-sensitive APFS
- [ ] Document Windows Terminal API research
- [ ] Evaluate Windows support effort (likely 2-3 weeks)
- [ ] Consider platform abstraction layer design

---

**Summary**: ZJJ fully supports **Linux**, likely supports **macOS**, and has **limited Windows support** due to Zellij. Windows users should use **WSL2**.
