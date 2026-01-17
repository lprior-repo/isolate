# Release Process

This document describes the release process for ZJJ (jjz).

## Overview

ZJJ uses automated GitHub Actions workflows to build and publish releases across multiple platforms. Releases are triggered by pushing Git tags following semantic versioning.

## Supported Platforms

| Platform | Architecture | Target Triple |
|----------|-------------|---------------|
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` |
| macOS | Intel | `x86_64-apple-darwin` |
| macOS | Apple Silicon | `aarch64-apple-darwin` |

## Release Workflow

### 1. Pre-Release Checklist

Before creating a release, ensure:

- [ ] All tests pass: `moon run :test`
- [ ] CI pipeline passes: `moon run :ci`
- [ ] Version numbers are updated in `Cargo.toml`
- [ ] CHANGELOG is updated with release notes
- [ ] Documentation is up to date
- [ ] All beads for the milestone are closed

### 2. Create a Release

#### Option A: Automated (Recommended)

1. Create and push a version tag:
   ```bash
   # Example: releasing v0.1.0
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

2. The GitHub Actions workflow will automatically:
   - Build binaries for all platforms
   - Generate checksums (SHA256)
   - Create a GitHub release with changelog
   - Upload all artifacts

#### Option B: Manual Trigger

1. Go to GitHub Actions → Release workflow
2. Click "Run workflow"
3. Enter the version tag (e.g., `v0.1.0`)
4. Click "Run workflow"

### 3. Verify Release

After the workflow completes:

1. Check the [Releases page](https://github.com/lprior-repo/zjj/releases)
2. Verify all platform binaries are present
3. Download and test binaries on target platforms
4. Verify checksums match

## Version Numbering

ZJJ follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** version: Incompatible API changes
- **MINOR** version: Backwards-compatible functionality additions
- **PATCH** version: Backwards-compatible bug fixes

### Pre-release Versions

- Alpha: `v0.1.0-alpha.1`
- Beta: `v0.1.0-beta.1`
- Release Candidate: `v0.1.0-rc.1`

## Release Artifacts

Each release includes:

### Binaries
- `jjz-x86_64-unknown-linux-gnu.tar.gz` - Linux x86_64
- `jjz-aarch64-unknown-linux-gnu.tar.gz` - Linux ARM64
- `jjz-x86_64-apple-darwin.tar.gz` - macOS Intel
- `jjz-aarch64-apple-darwin.tar.gz` - macOS Apple Silicon

### Checksums
- `*.tar.gz.sha256` - SHA256 checksums for verification

### Release Notes
- Auto-generated changelog
- Installation instructions
- Dependency requirements

## Installation Instructions

Installation instructions are automatically included in each release. Users can install using:

### Linux (x86_64)
```bash
curl -L https://github.com/lprior-repo/zjj/releases/latest/download/jjz-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv jjz /usr/local/bin/
```

### Linux (ARM64)
```bash
curl -L https://github.com/lprior-repo/zjj/releases/latest/download/jjz-aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv jjz /usr/local/bin/
```

### macOS (Intel)
```bash
curl -L https://github.com/lprior-repo/zjj/releases/latest/download/jjz-x86_64-apple-darwin.tar.gz | tar xz
sudo mv jjz /usr/local/bin/
```

### macOS (Apple Silicon)
```bash
curl -L https://github.com/lprior-repo/zjj/releases/latest/download/jjz-aarch64-apple-darwin.tar.gz | tar xz
sudo mv jjz /usr/local/bin/
```

## Verifying Downloads

Users should verify checksums after downloading:

```bash
# Download checksum file
curl -L -o jjz.tar.gz.sha256 \
  https://github.com/lprior-repo/zjj/releases/latest/download/jjz-x86_64-unknown-linux-gnu.tar.gz.sha256

# Verify checksum
sha256sum -c jjz.tar.gz.sha256
```

## Build Configuration

### Release Profile

The release builds use the following Cargo profile (from `Cargo.toml`):

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip debug symbols
```

This produces highly optimized, small binaries suitable for distribution.

### Cross-Compilation

- **Linux ARM64**: Uses `gcc-aarch64-linux-gnu` for cross-compilation
- **macOS**: Native compilation on macOS runners
- **Linux x86_64**: Native compilation on Ubuntu runners

## CI/CD Integration

The release workflow integrates with the existing Moon-based CI/CD pipeline:

### Quality Gates (Pre-Release)
Before creating a release tag, run:

```bash
moon run :ci
```

This executes all quality gates:
1. Code formatting (rustfmt)
2. Linting (clippy strict mode)
3. Unit tests
4. Property-based tests
5. Documentation tests
6. Security audit
7. Build verification

### Continuous Deployment
For automated deployments, extend the workflow with:

```yaml
- name: Deploy to production
  if: startsWith(github.ref, 'refs/tags/v')
  run: moon run :deploy
```

## Troubleshooting

### Build Failures

**Symptom**: Build fails for specific platform

**Solution**:
1. Check the Actions log for compilation errors
2. Verify cross-compilation toolchain is installed
3. Test locally using: `cargo build --release --target <TARGET>`

### Missing Artifacts

**Symptom**: Some platform binaries are missing from release

**Solution**:
1. Check if all jobs completed successfully
2. Verify the `matrix` configuration in `.github/workflows/release.yml`
3. Re-run failed jobs from the Actions UI

### Checksum Mismatch

**Symptom**: Downloaded binary checksum doesn't match

**Solution**:
1. Re-download the binary
2. Check for network corruption
3. Verify you're downloading from the official GitHub releases page

## Future Enhancements

Planned improvements to the release process:

- [ ] Windows binaries (MinGW and MSVC)
- [ ] Homebrew tap for macOS installation
- [ ] APT/RPM repositories for Linux
- [ ] Docker images
- [ ] Automated version bumping
- [ ] Release notes generation from beads
- [ ] Automated security scanning (e.g., cargo-deny)
- [ ] Binary signing and notarization (macOS)

## Security Considerations

### Binary Verification

All release binaries include SHA256 checksums. Users should always verify checksums before using binaries.

### Dependency Auditing

Each release runs `cargo audit` to check for known vulnerabilities in dependencies. Releases with security issues will be blocked.

### Supply Chain Security

- All builds run on GitHub-hosted runners (trusted environment)
- Rust toolchain installed via `dtolnay/rust-toolchain` (verified action)
- No third-party build scripts or pre-built binaries

## Release Monitoring

### Metrics to Track

- Download counts per platform
- Installation success rate
- User-reported issues per release
- Time from tag to release completion

### Monitoring Tools

- GitHub Insights → Traffic → Popular content
- GitHub Issues filtered by release milestone
- CI/CD dashboard for build times

## Contact

For release-related questions or issues:

- GitHub Issues: https://github.com/lprior-repo/zjj/issues
- Maintainers: ZJJ Contributors
