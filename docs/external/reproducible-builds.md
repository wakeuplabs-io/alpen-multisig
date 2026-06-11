# Reproducible Builds

**Satisfies: PRD §1.2** — Builds must be reproducible

This document describes how an independent party can rebuild the desktop application from source and confirm it matches a published release.

The application builds are [reproducible](https://reproducible-builds.org/docs/definition/): rebuilding from the same source yields a bit-for-bit identical artifact.

## What Is Verified

Reproducibility is asserted at the layer where the trust actually lives — **the code that runs** — not the OS installer wrappers:

| Layer | Artifact | Reproducible? |
|---|---|---|
| **Tier 1** | `target/release/desktop-app` binary + Vite `dist/` frontend | **Yes — verified, bit-for-bit** |
| Tier 2 | `.deb` / `.rpm` / AppImage installer wrappers | Partial — wrapper metadata may not be deterministic |
| Tier 3 | Signed/notarized macOS `.dmg` | **No, by construction** — signing/notarization is non-deterministic; reproduce the unsigned `.app` payload instead |

A verifier rebuilds and compares the **binary** and **frontend** digests. The installer files are covered for integrity by the signed `SHA256SUMS` manifest but are not claimed to be bit-for-bit reproducible.

## How Determinism Is Achieved

Three controls make Tier 1 deterministic:

1. **Pinned toolchain & dependencies** — `rust-toolchain.toml` pins the exact `rustc`; `Cargo.lock` and `package-lock.json` pin the full dependency graph; `.nvmrc` pins the Node major used for the frontend.

2. **`trim-paths`** — `[profile.release] trim-paths = true` (root `Cargo.toml`) strips absolute build paths (`$HOME`, `$CARGO_HOME`, git/registry dep paths) from the binary, so output does not depend on *where* it was built.

3. **`SOURCE_DATE_EPOCH`** — set to the release commit's author date, fixing every embedded build timestamp. The release workflow exports it; the verification script below derives the same value from the checked-out commit.

## Verifying a Published Release

From a **clean checkout of the exact release tag**:

```bash
git checkout vX.Y.Z          # the tag you are verifying

# Download the release reference (and, ideally, verify its PGP signature first —
# REPRODUCIBLE-DIGESTS.txt is covered by the signed SHA256SUMS; see
# Verifying Releases).
gh release download vX.Y.Z --pattern 'REPRODUCIBLE-DIGESTS.txt'

scripts/verify-reproducible-build.sh REPRODUCIBLE-DIGESTS.txt
```

The script sets `SOURCE_DATE_EPOCH`, rebuilds the Tauri bundle, computes the binary and frontend digests, and checks them against the reference. Expected output ends with:

```
OK   binary digest matches the published reference
OK   frontend digest matches the published reference
```

### Tying Reproducibility to the Signature

`REPRODUCIBLE-DIGESTS.txt` is one of the files listed in `SHA256SUMS`, which is PGP-signed during the release. Verifying the signature over `SHA256SUMS` and then `sha256sum -c SHA256SUMS` proves the reproducibility reference itself was not tampered with — so a matching local rebuild ties back to a signed anchor.

## If a Digest Does Not Match

A mismatch means some non-determinism leaked in. Explain it with [`diffoscope`](https://diffoscope.org/) against the published binary:

```bash
diffoscope /path/to/published/desktop-app target/release/desktop-app
```

Common causes: a different toolchain or Node version (re-check `rust-toolchain.toml` / `.nvmrc`), a dirty working tree, or a cross-distro difference in the system `cc`/glibc used for C dependencies — reproducibility is only guaranteed relative to the recorded CI build environment (`ubuntu-latest`).

## Known Limits

- Signed macOS `.dmg` is **out of scope** for bit-for-bit — reproduce the unsigned binary instead.
- Installer-wrapper reproducibility depends on `tauri-bundler` honoring `SOURCE_DATE_EPOCH` and may need `strip-nondeterminism` post-processing.
- Cross-distro rebuilds may differ at the binary level due to the system `cc`/glibc; match the recorded build environment for an exact comparison.

## Related Documents

- [Verifying Releases](./verifying-releases.md) — PGP signature and `SHA256SUMS` verification
