# Release Process

This document describes how to release new versions of `bevy_perf_hud`.

## Automated Release Process

The project uses GitHub Actions to automate releases when version tags are pushed.

### How to Release

1. **Update Version**: Update the version in `Cargo.toml`
   ```toml
   [package]
   version = "0.1.1"  # Update this
   ```

2. **Update Documentation**: Ensure README.md and docs are up to date

3. **Create and Push Tag**:
   ```bash
   # Create annotated tag
   git tag -a v0.1.1 -m "Release v0.1.1"

   # Push tag to trigger release
   git push origin v0.1.1
   ```

4. **Automated Process**: GitHub Actions will:
   - ✅ Wait for CI to pass
   - ✅ Build example programs for multiple platforms
   - ✅ Publish to crates.io (stable releases only)
   - ✅ Create GitHub Release with artifacts
   - ✅ Generate release notes automatically

## Version Types

- **Stable Release**: `v1.2.3` (published to crates.io)
- **Pre-release**: `v1.2.3-alpha.1`, `v1.2.3-beta.2` (GitHub only)

## Prerequisites

### First-time Setup

1. **Configure crates.io Trusted Publishing** (recommended):
   - Visit: https://crates.io/settings/publishing
   - Add GitHub repository: `ZoOLForge/bevy_perf_hud`
   - Configure workflow: `.github/workflows/release.yaml`

2. **Alternative: API Token** (if OIDC not available):
   - Generate token at: https://crates.io/settings/tokens
   - Add as repository secret: `CRATES_IO_TOKEN`

### Repository Settings

1. **Create Release Environment** (optional, for enhanced security):
   - Go to Settings → Environments → New environment
   - Name: `release`
   - Add protection rules as needed

2. **Permissions**: Ensure GitHub Actions has:
   - `id-token: write` (for OIDC)
   - `contents: write` (for releases)

## Release Checklist

Before creating a release tag:

- [ ] Version updated in `Cargo.toml`
- [ ] CHANGELOG.md updated (if present)
- [ ] Documentation updated
- [ ] All CI checks passing on main branch
- [ ] Examples build and run correctly

## Troubleshooting

### Common Issues

1. **CI Timeout**: If CI takes too long, the workflow will wait up to default timeout
2. **Version Mismatch**: Tag version must match `Cargo.toml` version exactly
3. **Build Failures**: Check platform-specific dependencies in the workflow

### Manual Recovery

If automatic release fails, you can:

1. **Manually publish to crates.io**:
   ```bash
   cargo publish --dry-run  # Test first
   cargo publish
   ```

2. **Create GitHub Release manually** using the web interface

## Security

- Uses OIDC trusted publishing for crates.io (no long-term tokens)
- Minimal permissions following principle of least privilege
- Builds in isolated GitHub-hosted runners
- All builds are reproducible and auditable

## Monitoring

- Check GitHub Actions tab for release progress
- Monitor crates.io for successful publication
- Verify GitHub Releases are created with correct artifacts