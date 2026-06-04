# Desktop App Executable — Delivery Plan (SSOT)

This document is the **single source of truth** for building, packaging, signing, and
distributing the desktop application executable. It consolidates the requirements scattered
across the PRD, proposal, story map, ADRs, and assessments into one incremental plan.

It is intentionally **non-technical**: each deliverable describes the user-facing value and the
requirements it closes, not the implementation. Technical design is produced per deliverable when
it is picked up.

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
| NFR-SUPPLY-CHAIN | Full release pipeline: reproducible, signed, supply-chain hardened; updater verifies signatures | `assessment/deferred-backlog.md` (P-011 full) |

## Current state

Starting point for this plan — what exists today:

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
- **Closes:** NFR-SUPPLY-CHAIN — partial (build automation).
- **Status:** Done. `.github/workflows/release.yml` triggers on `v*` tags and
  `workflow_dispatch`; builds the Tauri bundle on `ubuntu-latest`, uploads `.deb`, `.rpm`, and
  AppImage as workflow artifacts, and creates a GitHub Release with those files when triggered by
  a tag. The release workflow is intentionally separate from `ci.yml` (PR validation) to keep
  the CI feedback loop fast, per ADR-004.

### D3 · Signed Linux release + published verification instructions
- **Value:** A signer can download the Linux binary, verify a detached signature, and trust it came
  from the project. Single-key signing as the first trust anchor.
- **Closes:** PRD §1.3, NF-3 — partial (single signer, one OS). Builds on
  [`release-signing-mvp.md`](./release-signing-mvp.md).
- **Status:** Not started.

### D4 · Reproducible build verification
- **Value:** An independent party can rebuild from the same source and confirm a bit-for-bit
  identical artifact, with documented steps.
- **Closes:** PRD §1.2, NF-2.
- **Status:** Not started.

### D5 · Cross-platform builds (macOS + Windows)
- **Value:** Signers on macOS and Windows get a native, installable artifact equivalent to Linux.
- **Closes:** PRD §1.1 (full), NF-1 (full), NF-4 (full).
- **Status:** Not started.

### D6 · Platform code signing (Apple Developer ID / Windows Authenticode)
- **Value:** macOS and Windows recognize the binary as signed/notarized — no OS security warnings,
  native trust on each platform.
- **Closes:** PRD §1.3, NF-3 — extends to all platforms (still single authority).
- **Status:** Not started.

### D7 · Multi-employee signing ceremony
- **Value:** A release is approved and signed by multiple Alpen Labs employees; users can verify the
  multi-party signature, satisfying the "approved by multiple employees" requirement.
- **Closes:** PRD §1.3, NF-3 (full).
- **Status:** Not started.

### D8 · Auto-update with signature verification
- **Value:** The app can update itself, verifying signatures before applying any update.
- **Closes:** NFR-SUPPLY-CHAIN (updater verification).
- **Status:** Not started — lowest priority; deferred-backlog item.

## Sequencing notes

- D1 → D2 → D3 form the minimum trustworthy Linux release path and should land first.
- D4 (reproducibility) can proceed in parallel once D2 exists.
- D5 → D6 → D7 extend trust to all platforms; D7 (multi-employee) is the gate for the first
  external release per NF-3.
- D8 is optional/deferred and may be dropped from the first external release.

## Out of scope

- **HWI subprocess bundling (NF-16 / NF-17)** — under evaluation; not yet confirmed whether the
  shipped binary will bundle HWI. Excluded from this plan until the decision is made; if confirmed,
  it returns as a deliverable (bundle HWI on all 3 platforms, Windows highest-risk per discovery).
- Application features (signing, wallet, update lifecycle) — see story map and feature specs.
- Security audit — out of scope per proposal; recommended as a separate engagement before production.
- Backend deployment/hosting — see `operations/runbook.md` and platform specs.

## Related documents

- [`desktop-build-linux.md`](./desktop-build-linux.md) — Linux local build steps (D1).
- [`release-signing-mvp.md`](./release-signing-mvp.md) — Linux PGP MVP detail (feeds D3).
- [`../3-stories/non-functional-items.md`](../3-stories/non-functional-items.md) — NF-1…NF-4 (and NF-16/NF-17, HWI bundling, currently out of scope).
- [`../architecture/adrs/004-ci-pipeline-strategy.md`](../architecture/adrs/004-ci-pipeline-strategy.md) — why release builds are a separate workflow.
- [`../assessment/deferred-backlog.md`](../assessment/deferred-backlog.md) — NFR-SUPPLY-CHAIN (P-011 full).
