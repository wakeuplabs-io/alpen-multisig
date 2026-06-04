# Reproducible builds — research & D4 plan (P-011 / NF-2)

Research feeding **D4** of the [executable delivery plan](./executable-delivery-plan.md).
PRD §1.2 / NF-2 requires builds to be [reproducible](https://reproducible-builds.org/docs/definition/):
an independent party rebuilding from the same source MUST obtain a bit-for-bit identical
artifact.

This document is the honest, evidence-based assessment of **what is achievable today** for
this specific stack (Tauri 2 = Rust binary + Vite frontend + OS installer packaging), and the
recommended D4 scope. It is research, not yet implementation.

## What "reproducible" means here

Per reproducible-builds.org, reproducibility requires that the **build inputs** are fully
recorded and the **build process** is deterministic. Two independent builds from the same
recorded inputs must produce identical outputs. The hard part is never the source — it is the
ambient inputs that leak into outputs: timestamps, build paths, hostnames, locale, file
ordering, toolchain versions, and packaging metadata.

## Current foundation (what already helps)

Inspection of the repo shows several preconditions already met:

- **Toolchain pinned** — `rust-toolchain.toml` pins `nightly-2026-01-01`; CI uses the same. A
  fixed compiler version is mandatory (rustc output differs across versions).
- **Dependency versions pinned** — Alpen/Strata crates are git deps pinned to exact revs;
  `Cargo.lock` is committed (root + `desktop-app/src-tauri/Cargo.lock`), and
  `package-lock.json` is committed. This pins the full dependency graph.
- **Containerized CI** — release builds run on a known `ubuntu-latest` / `macos-latest` image,
  a controlled environment.

Gaps that block bit-for-bit today are below.

> Note: there is both a root `Cargo.lock` and a `desktop-app/src-tauri/Cargo.lock`. Since
> `src-tauri` is a workspace member, the workspace lock at the root is authoritative; the nested
> lock should be confirmed redundant and removed to avoid drift (track separately).

## Sources of non-determinism, by artifact layer

Reproducibility must be assessed per layer, because each has different difficulty.

### Layer 1 — The Rust binary (`desktop-app` executable)

| Source | Effect | Mitigation |
|---|---|---|
| Absolute build paths (`/home/<user>/...`, `$CARGO_HOME`) embedded in panic messages / debuginfo | Differs per machine | `[profile.release] trim-paths = true` (Cargo, available on this toolchain) + `--remap-path-prefix` for registry/git deps |
| Build timestamps | Differs per build | `SOURCE_DATE_EPOCH` set to a fixed commit-derived value |
| Incremental compilation | Non-deterministic object layout | Off by default in release; keep it off |
| C-built deps (`secp256k1`, etc. via `cc`) | Can embed paths/timestamps | Honor `SOURCE_DATE_EPOCH`; remap paths; pin `cc` |
| Codegen units / parallelism | rustc is deterministic given fixed version + inputs | No action beyond pinning |

**Verdict: achievable.** A bit-for-bit reproducible Rust binary is realistic with `trim-paths`,
`SOURCE_DATE_EPOCH`, the pinned toolchain, and committed lockfiles.

### Layer 2 — The frontend bundle (Vite `dist/`)

| Source | Effect | Mitigation |
|---|---|---|
| Node/npm version | Output can differ across majors | Pin Node (CI uses 20; pin locally too, e.g. `.nvmrc`/`engines`) |
| Dependency graph | — | `npm ci` against committed `package-lock.json` (already done) |
| Asset hashes | Content-based, so deterministic | None needed |
| Plugin-injected timestamps / build IDs | Rare, plugin-specific | Audit Vite plugins; honor `SOURCE_DATE_EPOCH` |

**Verdict: achievable**, contingent on pinning the Node version as tightly as the Rust toolchain.

### Layer 3 — OS installer packaging (`.deb`, `.rpm`, AppImage, `.dmg`)

This is the hard layer. `tauri-bundler` wraps platform packaging tools, and the wrappers embed
metadata that is non-deterministic and not all exposed as knobs:

| Artifact | Non-determinism | Notes |
|---|---|---|
| `.deb` | `ar`/`tar` member mtimes; gzip header timestamp + OS byte; file ordering | `strip-nondeterminism` (Debian tool) can post-process; depends on bundler honoring `SOURCE_DATE_EPOCH` |
| `.rpm` | Build time, host, file mtimes in header | rpm has `SOURCE_DATE_EPOCH` support upstream; bundler must pass it through |
| AppImage | squashfs mtimes; embedded runtime | squashfs supports reproducible mode via `SOURCE_DATE_EPOCH`; depends on the AppImage tool version bundled |
| `.dmg` | HFS/APFS timestamps; **code signing + notarization are inherently non-deterministic** | A signed/notarized `.dmg` can **never** be bit-for-bit reproducible |

**Verdict: partial → infeasible.** Installer-level bit-for-bit reproducibility is research-grade
with the current bundler and is **impossible for a signed macOS `.dmg`** by construction.

## Achievability tiers (honest framing)

- **Tier 1 — reproducible binary + frontend (achievable now):** verify the SHA-256 of the
  `desktop-app` executable and the `dist/` assets, independent of the installer wrapper. This is
  where the trust actually lives — it is the code that runs.
- **Tier 2 — reproducible installer wrappers (hard, partial):** `.deb`/`.rpm`/AppImage made
  reproducible via `SOURCE_DATE_EPOCH` + `strip-nondeterminism` post-processing. Feasible with
  effort; depends on bundler behavior we must test empirically.
- **Tier 3 — signed `.dmg` bit-for-bit (infeasible):** out of scope; signing/notarization
  precludes it. The right answer for macOS is to reproduce the **unsigned `.app` payload**, not
  the `.dmg`.

## Recommended D4 scope

Target **Tier 1 as the D4 deliverable**, with Tier 2 documented as a follow-up:

1. **Determinism config (code):**
   - Add `[profile.release] trim-paths = true` to the workspace `Cargo.toml`.
   - Set `SOURCE_DATE_EPOCH` (from the release commit timestamp) and `RUSTFLAGS`
     `--remap-path-prefix` in the release workflow.
   - Pin the Node version for builds (`.nvmrc` / `engines`) to match CI.
2. **Reproducibility verification job (CI/docs):**
   - Build twice in clean environments (or have an independent rebuild) and compare the SHA-256
     of the **binary** and **`dist/`**, not the installer.
   - Document the exact rebuild recipe so an external party can reproduce and compare, using
     `diffoscope` to explain any diff.
3. **Publish per-artifact digests:** extend the existing `SHA256SUMS` manifest (from D3) to also
   list the inner binary digest, so verification ties reproducibility to the signed manifest.
4. **Document Tier 2/3 limits** explicitly so NF-2's status is honest: binary reproducible now;
   installer reproducibility tracked; signed `.dmg` declared out of scope.

## Known hard limits (state these in any NF-2 claim)

- Signed/notarized macOS `.dmg` cannot be bit-for-bit reproducible — reproduce the unsigned
  `.app` payload instead.
- Installer wrapper reproducibility depends on `tauri-bundler` honoring `SOURCE_DATE_EPOCH`;
  this must be verified empirically (a bundler upgrade or post-processing may be required).
- Reproducibility is only meaningful relative to a **recorded build environment** (OS image,
  system libs); cross-distro rebuilds may differ at the binary level due to system `cc`/glibc.

## Open questions

- Does the bundled AppImage/`appimagetool` version honor `SOURCE_DATE_EPOCH`? (empirical test)
- Is `strip-nondeterminism` acceptable as a post-build step, or do we want bundler-native fixes?
- Should the reproducibility check be a blocking CI gate or a periodic/independent audit?

## Next step

Pick up D4 implementation from this research: start with Tier 1 (config + verification recipe),
land it behind the D3 manifest, then spike Tier 2 on `.deb`/AppImage to measure how far the
bundler gets us before deciding on post-processing.
