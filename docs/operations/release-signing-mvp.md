# Release signing MVP (P-011d)

D3 delivers signed releases with a **single named signer** as the first trust anchor;
D7 completes the same mechanism with **multiple employees** and an M-of-N verification
policy (PRD NF-3). The signing mechanism is identical across D3 and D7 — D7 only adds
more keys and more signatures. Nothing in D3 is throwaway.

## Approach: signed `SHA256SUMS` manifest (Option A)

We sign a single `SHA256SUMS` manifest that covers every platform artifact, rather than
signing each binary individually. This is the manifest-and-keyring model used by Bitcoin
Core, Tor, and Debian. Benefits:

- One signature covers Linux (`.deb`/`.rpm`/AppImage) and macOS (`.dmg`).
- Scales to N signers additively: each employee attaches `SHA256SUMS.<signer>.asc`.
- Standard, well-understood verification flow for users.

This is **not** multi-employee approval yet (NF-3 is a `SHOULD`); a single key proves
origin and integrity, not separation of authority. The keys are **individual employees'
personal keys**, never a shared "project key" — so the jump to D7 is purely additive.

## What ships in a release

- The platform artifacts (`.deb`, `.rpm`, `.AppImage`, `.dmg`).
- `SHA256SUMS` — checksums of all artifacts (integrity; published unconditionally).
- `SHA256SUMS.<signer>.asc` — detached OpenPGP signature(s) over the manifest
  (authenticity; published once a signing key is configured).

Automated in `.github/workflows/release.yml` (`release` job). If the `PGP_PRIVATE_KEY`
secret is not set, the release still publishes `SHA256SUMS` but no signature, with a
workflow warning.

## Setup (human action)

A real Alpen Labs employee must:

1. Generate a personal OpenPGP key and commit its public half to `release-keys/`.
2. Set repo secrets `PGP_PRIVATE_KEY`, `PGP_PASSPHRASE` and variable `PGP_SIGNER_ID`.

Full steps: [`release-keys/README.md`](../../release-keys/README.md).

## Verification

User-facing guide: [`verifying-releases.md`](./verifying-releases.md).

```bash
gpg --import release-keys/*.asc
gpg --verify SHA256SUMS.<signer>.asc SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS
```

## Deferred (D5–D7)

- macOS notarization / Apple Developer ID and Windows Authenticode (D6) — native OS
  trust, distinct from this PGP manifest signing.
- Multi-employee signing ceremony with M-of-N verification per PRD NF-3 (D7).
