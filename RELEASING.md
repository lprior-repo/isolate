# Quick Release Guide

Step-by-step guide for creating a new ZJJ release.

## Prerequisites

- [ ] All changes merged to `main` branch
- [ ] CI pipeline passing on `main`
- [ ] Version bumped in `Cargo.toml`
- [ ] CHANGELOG updated

## Release Steps

### 1. Run Full Quality Check

```bash
# Run complete CI pipeline locally
moon run :ci

# Verify all gates pass
moon run :deploy
```

### 2. Create Version Tag

```bash
# Set version (update this for your release)
VERSION=v0.1.0

# Create annotated tag
git tag -a $VERSION -m "Release $VERSION"

# Push tag to trigger release
git push origin $VERSION
```

### 3. Monitor Release Build

1. Go to: https://github.com/lprior-repo/zjj/actions
2. Watch the "Release" workflow
3. Verify all platform builds complete successfully

### 4. Verify Release

```bash
# Check release page
open https://github.com/lprior-repo/zjj/releases/latest

# Verify artifacts present:
# - zjj-x86_64-unknown-linux-gnu.tar.gz + .sha256
# - zjj-aarch64-unknown-linux-gnu.tar.gz + .sha256
# - zjj-x86_64-apple-darwin.tar.gz + .sha256
# - zjj-aarch64-apple-darwin.tar.gz + .sha256
```

### 5. Test Installation

```bash
# Download for your platform
curl -L https://github.com/lprior-repo/zjj/releases/download/$VERSION/zjj-x86_64-unknown-linux-gnu.tar.gz -o zjj.tar.gz

# Verify checksum
curl -L https://github.com/lprior-repo/zjj/releases/download/$VERSION/zjj-x86_64-unknown-linux-gnu.tar.gz.sha256 -o zjj.tar.gz.sha256
sha256sum -c zjj.tar.gz.sha256

# Extract and test
tar xzf zjj.tar.gz
./zjj --version
./zjj --help
```

### 6. Post-Release

- [ ] Announce release (Discord, Twitter, etc.)
- [ ] Update documentation site (if applicable)
- [ ] Close related beads/issues
- [ ] Update project roadmap

## Version Bumping

Update version in these files:

```bash
# Workspace version
vim Cargo.toml
# Update: version = "0.1.0"

# Lock file (automatic)
cargo check

# Commit version bump
git add Cargo.toml Cargo.lock
git commit -m "chore: Bump version to 0.1.0"
git push origin main
```

## Rollback

If a release has critical issues:

```bash
# Delete the tag locally
git tag -d $VERSION

# Delete the tag remotely
git push origin :refs/tags/$VERSION

# Delete the GitHub release (use web UI or gh CLI)
gh release delete $VERSION --yes

# Create a new patch release with the fix
```

## Troubleshooting

### Build Fails for Specific Platform

```bash
# Test locally with cross-compilation
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu

# Or test in Docker
docker run --rm -v $(pwd):/workspace -w /workspace rust:latest \
  cargo build --release --target x86_64-unknown-linux-gnu
```

### Release Workflow Doesn't Trigger

- Verify tag starts with 'v' (e.g., `v0.1.0`, not `0.1.0`)
- Check GitHub Actions is enabled for the repository
- Verify workflow file is on the default branch
- Check repository permissions allow GitHub Actions

### Missing Artifacts

- Check all matrix jobs completed successfully
- Review logs for upload failures
- Manually re-run failed jobs from Actions UI

## Manual Release (Emergency)

If automated release fails:

```bash
# Build for current platform
cargo build --release --package zjj

# Create archive
tar czf zjj-$(rustc -vV | grep host | cut -d' ' -f2).tar.gz \
  -C target/release zjj

# Generate checksum
sha256sum zjj-*.tar.gz > zjj-*.tar.gz.sha256

# Create GitHub release manually
gh release create $VERSION \
  --title "Release $VERSION" \
  --notes "Manual emergency release" \
  zjj-*.tar.gz \
  zjj-*.tar.gz.sha256
```

## See Also

- [Full Release Documentation](docs/RELEASE.md)
- [CI/CD Pipeline](schemas/cicd.cue)
- [GitHub Actions Workflows](.github/workflows/)
