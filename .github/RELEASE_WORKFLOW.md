# Release Workflow Documentation

This document describes the GitHub Actions workflow for creating multi-platform releases of the `aaa` tool.

## Overview

The release workflow (`.github/workflows/release.yml`) automatically builds and publishes packages for multiple platforms when a new GitHub release is created.

## Supported Platforms

The workflow generates the following packages:

1. **Ubuntu/Debian** - `.deb` package
2. **RHEL/Fedora/CentOS** - `.rpm` package
3. **macOS Intel (x86_64)** - Binary tarball (`.tar.gz`)
4. **macOS ARM64 (Apple Silicon)** - Binary tarball (`.tar.gz`)
5. **Windows** - MSI installer (`.msi`) and executable zip (`.zip`)
6. **Source Code** - Source archive in both `.tar.gz` and `.zip` formats

## Triggering a Release

To trigger the workflow:

1. Create a new release on GitHub:
   ```bash
   # Tag your commit
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. Go to GitHub and create a release:
   - Navigate to the repository
   - Click "Releases" → "Create a new release"
   - Select the tag you just pushed
   - Fill in release title and description
   - Click "Publish release"

3. The workflow will automatically:
   - Update the version in `Cargo.toml` to match the tag
   - Build packages for all supported platforms
   - Upload all packages to the release

## Version Format

The workflow supports version tags with or without the `v` prefix:
- `v1.0.0` → Version `1.0.0`
- `1.0.0` → Version `1.0.0`

For Windows MSI packages, the workflow handles both 3-part and 4-part semantic versions:
- `1.0.0` → `1.0.0.0` (for MSI)
- `1.0.0.1` → `1.0.0.1` (unchanged)

## Workflow Jobs

The workflow consists of the following jobs:

### 1. Update Version
- Extracts version from the git tag
- Updates `Cargo.toml` with the correct version
- Shares the updated `Cargo.toml` with other jobs

### 2. Build Linux DEB
- Builds on Ubuntu
- Uses `cargo-deb` to generate `.deb` package
- Uploads package to release

### 3. Build Linux RPM
- Builds on Ubuntu
- Uses `cargo-generate-rpm` to generate `.rpm` package
- Uploads package to release

### 4. Build macOS Intel
- Builds on macOS Intel runner (macos-13)
- Targets x86_64-apple-darwin
- Creates a tarball with the Intel binary
- Uploads tarball to release

### 5. Build macOS ARM64
- Builds on macOS ARM64 runner (macos-latest)
- Targets aarch64-apple-darwin
- Creates a tarball with the ARM64 binary
- Uploads tarball to release

### 6. Build Windows
- Builds on Windows
- Creates `.msi` installer using WiX Toolset
- Creates `.zip` archive with executable
- Uploads both packages to release

### 7. Create Source Archive
- Creates source code archives in `.tar.gz` and `.zip` formats
- Uploads archives to release

## Package Metadata

The workflow uses metadata defined in `Cargo.toml`:

- **General metadata**: name, description, license, repository, etc.
- **DEB metadata**: `[package.metadata.deb]` section
- **RPM metadata**: `[package.metadata.generate-rpm]` section

## Security

The workflow follows security best practices:
- Explicit permissions set to `contents: write` (minimum required)
- Uses official GitHub Actions and trusted third-party actions
- No secrets are exposed in the build process

## Installation Instructions

After the workflow completes, users can download and install the packages:

### Ubuntu/Debian
```bash
wget https://github.com/manoelhc/aaa/releases/download/v1.0.0/aaa_1.0.0_amd64.deb
sudo dpkg -i aaa_1.0.0_amd64.deb
```

### RHEL/Fedora/CentOS
```bash
wget https://github.com/manoelhc/aaa/releases/download/v1.0.0/aaa-1.0.0-1.x86_64.rpm
sudo rpm -i aaa-1.0.0-1.x86_64.rpm
```

### macOS Intel (x86_64)
```bash
wget https://github.com/manoelhc/aaa/releases/download/v1.0.0/aaa-1.0.0-macos-x86_64.tar.gz
tar -xzf aaa-1.0.0-macos-x86_64.tar.gz
sudo mv aaa /usr/local/bin/
```

### macOS ARM64 (Apple Silicon)
```bash
wget https://github.com/manoelhc/aaa/releases/download/v1.0.0/aaa-1.0.0-macos-aarch64.tar.gz
tar -xzf aaa-1.0.0-macos-aarch64.tar.gz
sudo mv aaa /usr/local/bin/
```

### Windows
Download the `.msi` installer from the release page and run it, or download the `.zip` file and extract it.

## Troubleshooting

### Build Failures

If a build fails:
1. Check the GitHub Actions logs for the specific job
2. Verify that `Cargo.toml` has correct metadata
3. Ensure all dependencies are properly specified

### Version Mismatch

If the version in packages doesn't match the tag:
1. Verify the tag format (should be `vX.Y.Z` or `X.Y.Z`)
2. Check the workflow logs for the "Get version from tag" step

### Upload Failures

If packages fail to upload to the release:
1. Ensure the release was created before the workflow ran
2. Check that `GITHUB_TOKEN` has sufficient permissions
3. Verify the workflow has `contents: write` permission

## Future Improvements

Potential enhancements for the workflow:
- Add checksums file for all packages
- Add ARM64 support for Linux and macOS
- Add code signing for Windows and macOS binaries
- Add automated testing before packaging
- Add changelog generation
