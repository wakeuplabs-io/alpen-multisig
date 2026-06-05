# Desktop App Executable — Delivery Plan (SSOT)

This document is the **single source of truth** for building, packaging, signing, and
distributing the desktop application executable. It consolidates the requirements scattered
across the PRD, proposal, story map, ADRs, and assessments into one incremental plan.

It is intentionally **non-technical**: each deliverable describes the user-facing value and the
requirements it closes, not the implementation. Technical design is produced per deliverable when
it is picked up.

> **Status: Concluded — 2026-06-05.** Every deliverable is resolved (Done, Partial with handed-off
> follow-ups, or Out of scope). The build → package → reproducibility → signing → cross-platform
> pipeline is implemented in `.github/workflows/release.yml` and exercised end-to-end via CI on all
> three platforms. See the [Closure summary](#closure-summary) for the final per-deliverable outcome
> and the remaining items handed off (the Windows installer and the upstream escalation to Alpen).

## Purpose & scope

Covers everything needed to put a trustworthy, installable binary in a signer's hands:
build, packaging, reproducibility, signing, cryptographic verification, and cross-platform support.
It does **not** cover application features (signing flows, wallet, update lifecycle) — those live in
the story map and feature specs.

## Requirements traceability

| Ref | Requirement | Source |
|-----|-------------|--------|
| PRD §1.1 / NF-1 | Run on latest LTS of Debian Linux, macOS, Windows (8 GB RAM, 2c4t, 1 TB SSD, 20 Mbps) | `0-prd/03-prd-update.md`; `3-stories/non-functional-items.md` |
| PRD §1.2 / NF-2 | Builds must be reproducible (reproducible-builds.org) | `0-prd/03-prd-update.md`; proposal §Deliverables |
| PRD §1.3 / NF-3 | Binary cryptographically signed by **multiple** Alpen Labs employees; published verification instructions | `0-prd/03-prd-update.md`; proposal §Deliverables |
| PRD §1.4 / NF-4 | Install/launch via single command or double-click; deps at most one extra step | `0-prd/03-prd-update.md` |

## Starting point (baseline, before this plan)

Historical baseline that motivated this plan — what existed at the outset (now superseded by the
[Closure summary](#closure-summary)):

- CI (`.github/workflows/ci.yml`) runs lint/build/test for Rust and a **Vite** frontend build only.
- CI does **not** run `tauri build`; no distributable binary is produced anywhere.
- No release workflow, no artifact publication, no checksums.
- `tauri.conf.json` declares `bundle.targets: "all"` but has **no** signing configuration.
- No code signing on any platform; no reproducibility verification.
- A manual Linux PGP MVP is sketched in [`release-signing-mvp.md`](./release-signing-mvp.md) but not automated.

Two adversarial assessment rounds (`assessment/2026-05-13-*`, `assessment/2026-05-14-*`) flagged
the missing release/signing pipeline as a **BLOCKER** before any external release.

## Incremental deliverables

Ordered by dependency and value. Each is independently shippable and demo-able; later items build on
earlier ones. Status is tracked here as the plan progresses.

### D1 · Runnable Linux artifact from local build
- **Value:** A maintainer can produce an installable Linux package (`.deb`/AppImage) from source and
  launch it with a double-click or one command.
- **Closes:** PRD §1.1 (Linux), §1.4, NF-1 (Linux), NF-4 — partial.
- **Status:** Done. `npm run tauri build` produces `.deb`, `.rpm`, and AppImage; the AppImage
  launches with a double-click or one command. Build/install steps documented in
  [`desktop-build-linux.md`](./desktop-build-linux.md). Bundle icons added (the missing
  `bundle.icon` config was blocking the AppImage build).

### D2 · CI produces the Tauri bundle
- **Value:** Every release candidate is built automatically; the binary that ships is the one CI
  builds and tests. Closes the assessment CRITICAL "CI never runs `tauri build`".
- **Closes:** Build automation for release artifacts.
- **Status:** Done. `.github/workflows/release.yml` triggers on `v*` tags and
  `workflow_dispatch`; builds the Tauri bundle on `ubuntu-latest`, uploads `.deb`, `.rpm`, and
  AppImage as workflow artifacts, and creates a GitHub Release with those files when triggered by
  a tag. The release workflow is intentionally separate from `ci.yml` (PR validation) to keep
  the CI feedback loop fast, per ADR-004.

### D2.1 · CI produces the macOS bundle
- **Value:** Testers on macOS can download a native `.dmg` artifact produced by CI without building
  from source.
- **Closes:** PRD §1.1 (macOS), NF-1 (macOS), NF-4 — partial.
- **Status:** Done. `.github/workflows/release.yml` now runs a `build-macos` job on
  `macos-latest` alongside `build-linux`; it builds the Tauri bundle and uploads the `.dmg` as a
  workflow artifact. A dedicated `release` job downloads both platforms' artifacts and attaches the
  `.deb`, `.rpm`, AppImage, and `.dmg` to the GitHub Release on tag builds.

### D3 · Signed release + published verification instructions
- **Value:** A signer can download a binary, verify a detached signature over the published
  `SHA256SUMS` manifest, and trust it came from the project. Single named-employee signing as the
  first trust anchor, using the manifest-and-keyring model (Option A) so D7 is purely additive.
- **Closes:** PRD §1.3, NF-3 — partial (single signer). Builds on
  [`release-signing-mvp.md`](./release-signing-mvp.md).
- **Status:** Done. `release.yml` generates `SHA256SUMS` over all platform artifacts and
  publishes a detached signature `SHA256SUMS.<signer>.asc` when a signing key is configured
  (graceful degradation: checksums always ship; signature ships once `PGP_PRIVATE_KEY` is set).
  Authorized public keys live in [`release-keys/`](../../release-keys/); user verification guide in
  [`verifying-releases.md`](./verifying-releases.md). An Alpen Labs employee generated a personal
  key, committed its public half to `release-keys/`, and set the `PGP_PRIVATE_KEY`/`PGP_PASSPHRASE`
  secrets + `PGP_SIGNER_ID` variable; signed releases verified.

### D4 · Reproducible build verification
- **Value:** An independent party can rebuild from the same source and confirm a bit-for-bit
  identical artifact, with documented steps.
- **Closes:** PRD §1.2, NF-2.
- **Status:** Done (Tier 1). Determinism config landed: `[profile.release] trim-paths = true`
  (root `Cargo.toml`), `SOURCE_DATE_EPOCH` set from the release commit in `release.yml`, and Node
  pinned via `.nvmrc` + `engines`. The release workflow publishes `REPRODUCIBLE-DIGESTS.txt`
  (binary + frontend SHA-256 per platform), folded into the signed `SHA256SUMS` so the signature
  commits to it. An independent party reproduces and compares with
  `scripts/verify-reproducible-build.sh`; the recipe and honest tier limits are documented in
  [`reproducible-builds.md`](./reproducible-builds.md). Tier 2 (installer wrappers) and Tier 3
  (signed `.dmg`, infeasible by construction) remain out of scope and are tracked as follow-ups.
  Research and evidence: [`reproducible-builds-research.md`](./reproducible-builds-research.md).

### D5 · Cross-platform builds (Windows)
- **Value:** Signers on Windows get a native, installable artifact equivalent to Linux and macOS.
- **Closes:** PRD §1.1 (full), NF-1 (full), NF-4 (partial — runnable binary, no installer yet).
- **Status:** Partial — runnable `.exe`, installer pending. A native `windows-latest` build is
  **blocked** by three upstream Windows incompatibilities in the Alpen dependencies (reserved
  `aux` directory in `asm`; CRLF breaking the SSZ parser; and a non-portable path-separator bug
  in `ssz-gen`'s codegen). The first two were worked around (PR #242); the third has no local
  workaround. Per maintainer direction, `build-windows` was **pivoted to cross-compiling on
  `ubuntu-latest`** with `cargo-xwin` (target `x86_64-pc-windows-msvc`). This compiles the full
  app — including the native USB/HID C deps — and publishes a runnable
  `desktop-app-x86_64-windows.exe` (~7.6 MB). Building on Linux sidesteps all three blockers.
  **Gap:** Tauri cannot bundle a Windows installer from Linux (`--bundles` is host-gated), so the
  bare `.exe` ships with `--no-bundle`; it double-clicks to launch but needs the WebView2 runtime
  present and has no Start-menu shortcut/uninstaller, and there is no `.msi`. Full analysis, the
  suggested upstream fix for Alpen, and the installer follow-up options are in
  [`windows-build-incompatibilities.md`](./windows-build-incompatibilities.md).
- **Follow-ups:** (1) Produce a real Windows installer — hand-written NSIS via `makensis` on
  Linux around the cross-compiled `.exe` (+ WebView2 bootstrap), or wait on the upstream fix.
  (2) Escalate blockers 1 + 3 to Alpen so a native Windows build becomes possible — written up
  for them in [`windows-portability-upstream-issues.md`](./windows-portability-upstream-issues.md).

### D6 · Platform code signing (Apple Developer ID / Windows Authenticode)
- **Value:** macOS and Windows recognize the binary as signed/notarized — no OS security warnings,
  native trust on each platform.
- **Closes:** PRD §1.3, NF-3 — extends to all platforms (still single authority).
- **Status:** Done (documentation). Platform code signing requires enrollment in Apple Developer
  Program (macOS) and an Authenticode certificate from a trusted CA (Windows). Both are outside
  the scope of this phase and require Alpen Labs to initiate. Requirements and process documented
  in [`platform-code-signing-requirements.md`](./platform-code-signing-requirements.md); the release
  pipeline will integrate the certificates once provided.

### D7 · Multi-employee signing ceremony
- **Value:** A release is approved and signed by multiple Alpen Labs employees; users can verify the
  multi-party signature, satisfying the "approved by multiple employees" requirement.
- **Closes:** PRD §1.3, NF-3 (full).
- **Status:** Done (documentation). Multi-employee signing requirements and process documented in
  [`multi-employee-signing-requirements.md`](./multi-employee-signing-requirements.md). The release
  infrastructure already supports multiple signature files (`SHA256SUMS.<signer>.asc`) without
  collision. Employees must generate a personal PGP key, commit the public half to `release-keys/`,
  and provide the private key + signer ID to the development team for GitHub Secrets configuration.

### D8 · Auto-update with signature verification
- **Value:** The app can update itself, verifying signatures before applying any update.
- **Closes:** NFR-SUPPLY-CHAIN (updater verification).
- **Status:** Out of scope — not required by PRD. The PRD specifies no requirement for the
  application to update itself automatically. Manual download from GitHub Releases is the
  supported distribution method. D8 is therefore removed from this plan.

## Closure summary

Final outcome of each deliverable. This plan is **concluded**: no further work is scheduled under
it; the two open items below are explicitly handed off.

| Deliverable | Outcome |
|-------------|---------|
| D1 · Runnable Linux artifact | **Done** — `.deb`/`.rpm`/AppImage from source. |
| D2 · CI produces the Tauri bundle | **Done** — automated build on every tag/dispatch. |
| D2.1 · CI produces the macOS bundle | **Done** — `.dmg` artifact. |
| D3 · Signed release + verification | **Done** — `SHA256SUMS` + detached PGP signature; user guide published. |
| D4 · Reproducible build | **Done (Tier 1)** — binary + frontend bit-for-bit; documented recipe. |
| D5 · Cross-platform builds (Windows) | **Partial** — runnable cross-compiled `desktop-app-x86_64-windows.exe`; native build blocked by upstream; installer + escalation handed off (below). |
| D6 · Platform code signing | **Done (documentation)** — requirements published; awaits Alpen-provided certificates. |
| D7 · Multi-employee signing | **Done (documentation + infra)** — pipeline supports multiple signatures; awaits employee keys. |
| D8 · Auto-update | **Out of scope** — not required by PRD; manual download from GitHub Releases. |

### Handed-off / external items

These are intentionally **not** part of this plan's active scope; they depend on follow-up work or
on Alpen Labs:

- **D5 — Windows installer:** ship a real installer (hand-written NSIS via `makensis` on Linux around
  the cross-compiled `.exe`, plus WebView2 bootstrap), or wait on the upstream fix. Today's Windows
  artifact is a runnable bare `.exe` (double-click launch; requires WebView2 present).
- **D5 — Upstream escalation:** the two genuine upstream Windows portability bugs (reserved `aux`
  directory in `asm`; path-separator bug in `ssz-gen`) are written up for Alpen in
  [`windows-portability-upstream-issues.md`](./windows-portability-upstream-issues.md). A native
  Windows build becomes possible once these are fixed.
- **D6 — Certificates:** Apple Developer ID (macOS) and Authenticode (Windows) must be obtained by
  Alpen Labs before platform code signing can be integrated.
- **D7 — Employee keys:** the multi-employee signing ceremony requires each employee to provide a
  PGP key before it can be exercised.

## Sequencing notes

- D1 → D2 → D2.1 → D3 form the minimum trustworthy Linux + macOS release path and should land first.
- D4 (reproducibility) can proceed once D2 exists.
- D5 → D6 → D7 extend trust to all platforms; D7 (multi-employee) is the gate for the first
  external release per NF-3.

## Out of scope

- **HWI subprocess bundling (NF-16 / NF-17)** — under evaluation; not yet confirmed whether the
  shipped binary will bundle HWI. Excluded from this plan until the decision is made; if confirmed,
  it returns as a deliverable (bundle HWI on all 3 platforms, Windows highest-risk per discovery).
- **Auto-update (D8)** — not required by PRD; manual download from GitHub Releases is the supported
  distribution method.
- Application features (signing, wallet, update lifecycle) — see story map and feature specs.
- Security audit — out of scope per proposal; recommended as a separate engagement before production.
- Backend deployment/hosting — see `operations/runbook.md` and platform specs.

## Related documents

- [`desktop-build-linux.md`](./desktop-build-linux.md) — Linux local build steps (D1).
- [`release-signing-mvp.md`](./release-signing-mvp.md) — Linux PGP MVP detail (feeds D3).
- [`verifying-releases.md`](./verifying-releases.md) — user-facing release verification guide (D3).
- [`reproducible-builds-research.md`](./reproducible-builds-research.md) — reproducibility research and plan (D4).
- [`reproducible-builds.md`](./reproducible-builds.md) — how to independently reproduce and verify a release (D4).
- [`platform-code-signing-requirements.md`](./platform-code-signing-requirements.md) — Apple Developer ID and Authenticode requirements for Alpen Labs (D6).
- [`multi-employee-signing-requirements.md`](./multi-employee-signing-requirements.md) — employee PGP key generation and signing ceremony process (D7).
- [`../3-stories/non-functional-items.md`](../3-stories/non-functional-items.md) — NF-1…NF-4 (and NF-16/NF-17, HWI bundling, currently out of scope).
- [`../architecture/adrs/004-ci-pipeline-strategy.md`](../architecture/adrs/004-ci-pipeline-strategy.md) — why release builds are a separate workflow.
