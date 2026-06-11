# Build and Release Process

**Satisfies: PRD §1.1, §1.2, §1.3, §1.4** — Cross-platform builds, reproducibility, signing, and installation

## Overview

This document describes the build, packaging, signing, and distribution process for the Alpen Multisig desktop application. The release pipeline produces trustworthy, installable binaries for all supported platforms with cryptographic verification capabilities.

## Supported Platforms

| Platform | Artifact | Installation |
|----------|----------|--------------|
| **Linux (Debian/Ubuntu)** | `.deb` | `dpkg -i` or double-click |
| **Linux (RPM-based)** | `.rpm` | `rpm -i` or package manager |
| **Linux (Universal)** | `.AppImage` | Double-click or `chmod +x` then run |
| **macOS** | `.dmg` | Drag to Applications |
| **Windows** | `.exe` | Double-click to launch (requires WebView2 runtime) |

## System Requirements

- **OS:** Latest LTS release of Debian Linux, macOS, or Windows
- **RAM:** 8 GB minimum
- **CPU:** 2 cores, 4 threads
- **Storage:** 1 TB SSD
- **Network:** 20 Mbps internet connection

## Build Pipeline

The release pipeline is automated via GitHub Actions and triggered on version tags (`v*`). The pipeline:

1. **Builds** the Tauri application from source on each target platform
2. **Packages** the application into platform-specific installers
3. **Generates** checksums (`SHA256SUMS`) for all artifacts
4. **Signs** the checksums with authorized PGP keys
5. **Publishes** reproducible build digests (`REPRODUCIBLE-DIGESTS.txt`)
6. **Creates** a GitHub Release with all artifacts and verification files

## Release Artifacts

Each release includes:

- Platform-specific installers (`.deb`, `.rpm`, `.AppImage`, `.dmg`, `.exe`)
- `SHA256SUMS` — SHA-256 checksums of all artifacts
- `SHA256SUMS.<signer>.asc` — Detached PGP signature(s) from authorized signers
- `REPRODUCIBLE-DIGESTS.txt` — SHA-256 digests of the binary and frontend bundle for reproducibility verification

## Reproducible Builds

The application supports reproducible builds at the binary and frontend level. An independent party can rebuild from source and verify bit-for-bit identity with the published release.

**What is reproducible:**
- The Rust binary (`target/release/desktop-app`)
- The frontend bundle (`dist/`)

**What is not reproducible:**
- Installer wrappers (`.deb`, `.rpm`, `.AppImage`) — wrapper metadata may not be deterministic
- Signed/notarized macOS `.dmg` — signing and notarization are non-deterministic by design

See [Reproducible Builds](./reproducible-builds.md) for verification instructions.

## Release Signing

All releases are signed using OpenPGP. The signing keys belong to individual Alpen Labs employees, and multiple signatures can be attached to each release.

See [Release Signing](./release-signing.md) for details on the signing approach and [Verifying Releases](./verifying-releases.md) for verification instructions.

## Installation

### Linux

**Debian/Ubuntu:**
```bash
sudo dpkg -i alpen-multisig_*.deb
```

**RPM-based:**
```bash
sudo rpm -i alpen-multisig-*.rpm
```

**AppImage:**
```bash
chmod +x alpen-multisig-*.AppImage
./alpen-multisig-*.AppImage
```

### macOS

Open the `.dmg` file and drag the application to the Applications folder.

### Windows

Double-click the `.exe` file to launch the application. The WebView2 runtime must be installed on the system.

## Verification

After downloading a release, verify both integrity and authenticity:

```bash
# Import authorized signing keys
gpg --import release-keys/*.asc

# Verify the signature
gpg --verify SHA256SUMS.<signer>.asc SHA256SUMS

# Verify the checksum
sha256sum --ignore-missing -c SHA256SUMS
```

See [Verifying Releases](./verifying-releases.md) for complete verification instructions.

## Related Documents

- [Verifying Releases](./verifying-releases.md) — Step-by-step verification guide
- [Reproducible Builds](./reproducible-builds.md) — Independent build verification
- [Release Signing](./release-signing.md) — Signing approach and multi-signer support
