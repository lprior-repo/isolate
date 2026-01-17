# JJ Version Compatibility Matrix

**Last Updated:** 2026-01-16
**ZJJ Version:** 0.1.0

## Minimum Requirements

**Minimum Supported JJ Version:** **0.20.0**

This version requirement is set conservatively to ensure workspace command stability. ZJJ has been tested with JJ 0.36.0 and is expected to work with all versions >= 0.20.0.

## Version Detection

ZJJ automatically detects your JJ version using:
```bash
jj --version
```

Expected output format:
```
jj 0.36.0-70fd8f7697fbc20a9329a6e2f790ef86a8e284d1
```

## Tested Versions

| JJ Version | ZJJ Version | Status | Notes |
|------------|-------------|--------|-------|
| 0.36.0     | 0.1.0       | ✅ Tested | Fully compatible |
| 0.20.0+    | 0.1.0       | ⚠️ Expected | Minimum supported version |
| < 0.20.0   | 0.1.0       | ❌ Not supported | May lack workspace stability |

## Commands Used by ZJJ

ZJJ uses the following JJ commands, which have been stable since JJ 0.20.0:

### Core Workspace Commands
- `jj workspace add --name <name> <path>` - Create new workspace
- `jj workspace forget <name>` - Remove workspace
- `jj workspace list` - List all workspaces

### Status and Information
- `jj status` - Get workspace status
- `jj diff --stat` - Get diff statistics
- `jj root` - Get repository root path

### Synchronization
- `jj squash` - Squash changes
- `jj rebase -d <branch>` - Rebase to branch
- `jj git push` - Push to remote

### Initialization
- `jj git init` - Initialize JJ repository
- `jj --version` - Get JJ version

## Breaking Changes in JJ

ZJJ is designed to avoid JJ features that have undergone breaking changes:

### ❌ Not Used (Breaking Changes)
- **Branch → Bookmark terminology** - ZJJ uses workspaces, not branches/bookmarks
- **jj obslog** → **jj evolution-log** - ZJJ doesn't use obslog
- **jj unsquash** - ZJJ uses jj squash
- **git.push-branch-prefix config** - ZJJ doesn't configure git prefixes

### ✅ Used (Stable Since 0.20.0)
- **Workspace commands** - Core stable API
- **Status and diff commands** - Output format stable
- **Git integration** - Stable interop layer

## Compatibility Concerns

### Low Risk Areas
- Workspace management (core ZJJ functionality)
- Git interoperability
- Status and diff parsing

### Medium Risk Areas
- JJ CLI output format changes
- Error message format changes
- Exit code changes

### Mitigation Strategies

1. **Graceful Parsing** - ZJJ parses JJ output defensively, handling variations
2. **Version Detection** - ZJJ checks minimum version at initialization
3. **Comprehensive Tests** - 202+ tests verify JJ integration behavior
4. **Conservative Minimum** - 0.20.0 requirement provides stability margin

## Version Checking

### Automatic Checking

ZJJ can check version compatibility programmatically:

```rust
use zjj_core::jj::{get_jj_version, check_jj_version_compatible};

// Get current JJ version
let version = get_jj_version()?;
println!("JJ version: {}.{}.{}", version.major, version.minor, version.patch);

// Check compatibility
check_jj_version_compatible()?; // Returns error if incompatible
```

### Manual Checking

Users can verify their JJ version:
```bash
jj --version
```

If version is < 0.20.0, upgrade JJ:
```bash
cargo install --git https://github.com/jj-vcs/jj jj-cli
```

## Error Messages

### Version Too Old

If JJ version is below minimum:
```
Error: JJ version 0.19.0 is not supported. Minimum required version: 0.20.0

Please upgrade JJ to continue:
  cargo install --git https://github.com/jj-vcs/jj jj-cli
```

### JJ Not Found

If JJ is not installed:
```
Error: JJ not found in PATH

Please install JJ:
  cargo install --git https://github.com/jj-vcs/jj jj-cli
```

## Future Compatibility

### Monitoring Strategy

ZJJ development team monitors:
1. [JJ Changelog](https://github.com/jj-vcs/jj/blob/main/CHANGELOG.md) for breaking changes
2. [JJ Releases](https://github.com/jj-vcs/jj/releases) for new versions
3. Community feedback on compatibility issues

### Updating Minimum Version

Minimum version may be raised if:
- Critical bugs are found in older JJ versions
- New essential features require newer JJ
- Breaking changes make older versions incompatible

Version increases will be:
- Announced in ZJJ release notes
- Documented in this compatibility matrix
- Tested across supported versions

## Testing Across Versions

### Continuous Integration

ZJJ CI tests against:
- Latest stable JJ release
- Minimum supported version (0.20.0)
- Development builds (when available)

### Manual Testing

Contributors should test against:
- Their local JJ version (report in issues)
- Minimum supported version (via Docker/VM)
- Latest JJ main branch (optional)

## Reporting Compatibility Issues

If you encounter compatibility problems:

1. **Check your JJ version**: `jj --version`
2. **Verify minimum requirement**: >= 0.20.0
3. **Report issue** with:
   - JJ version
   - ZJJ version
   - Command that failed
   - Error message
   - JJ output/behavior

## References

- [JJ Documentation](https://docs.jj-vcs.dev/latest/)
- [JJ Changelog](https://github.com/jj-vcs/jj/blob/main/CHANGELOG.md)
- [JJ Releases](https://github.com/jj-vcs/jj/releases)
- [JJ GitHub Repository](https://github.com/jj-vcs/jj)

### Recent Breaking Changes Research

Per [JJ Changelog](https://github.com/jj-vcs/jj/blob/main/CHANGELOG.md) and [documentation](https://docs.jj-vcs.dev/latest/changelog/):

- **Branch → Bookmark terminology** (2024-2025) - Renamed to better describe behavior
- **jj obslog** → **jj evolution-log/evolog** (with alias maintained)
- **jj unsquash deprecated** - Use jj squash instead
- **Minimum Rust 1.84.0** - For building JJ from source

ZJJ avoids all deprecated features and uses only stable workspace APIs.

---

*This compatibility matrix is maintained with each ZJJ release*
