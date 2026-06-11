# Release Signing

**Satisfies: PRD §1.3** — Cryptographic verification of application binary

## Overview

Every release is signed using a manifest-and-keyring model, the same approach used by Bitcoin Core, Tor, and Debian. This provides both integrity and authenticity guarantees for end users.

## What Ships in a Release

Each release publishes:

- **Platform artifacts** — `.deb`, `.rpm`, `.AppImage` (Linux), `.dmg` (macOS), `.exe` (Windows)
- **`SHA256SUMS`** — SHA-256 checksums of all artifacts (integrity verification)
- **`SHA256SUMS.<signer>.asc`** — Detached OpenPGP signature(s) over the manifest (authenticity verification)

## Signing Approach

The system signs a single `SHA256SUMS` manifest that covers every platform artifact, rather than signing each binary individually. This approach provides:

- **Unified coverage** — One signature covers all platform artifacts
- **Scalability** — Multiple signers can each attach their signature file (`SHA256SUMS.<signer>.asc`)
- **Standard verification** — Well-understood verification flow using standard tools

The signing keys are **individual employees' personal keys**, never a shared "project key". Each authorized signer maintains their own key pair, and the public keys are published in the project repository.

## Multi-Signer Support

The release infrastructure supports multiple signature files without collision. When multiple employees sign a release, each attaches their detached signature, allowing users to verify that the release was approved by multiple authorized signers.

## Verification

Users can verify both the integrity and authenticity of their download:

```bash
# Import authorized signing keys
gpg --import release-keys/*.asc

# Verify the signature on the manifest
gpg --verify SHA256SUMS.<signer>.asc SHA256SUMS

# Verify the downloaded artifact against the manifest
sha256sum --ignore-missing -c SHA256SUMS
```

See [Verifying Releases](./verifying-releases.md) for detailed verification instructions.

## Related Documents

- [Verifying Releases](./verifying-releases.md) — Step-by-step verification guide
- [Reproducible Builds](./reproducible-builds.md) — Independent build verification
- [Build and Release Process](./build-and-release-process.md) — Overview of the release pipeline
