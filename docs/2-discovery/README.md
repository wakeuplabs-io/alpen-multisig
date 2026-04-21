# Phase 1 — Discovery & Research Index

This folder collects every artefact produced during Phase 1 (discovery and protocol research) of the Alpen Multisig project. The consolidated narrative — assessments, risks, POC conclusions, final recommendations — lives in [`docs/deliverable/research.md`](../deliverable/research.md). Everything here is the raw material behind that deliverable.

The docs are grouped by topic below rather than by filename number; numbers reflect creation order only.

## Reading order

If you are new to this project, read in this order:

1. [`01-conceptual-overview.md`](./01-conceptual-overview.md) — plain-language overview of Bitcoin / Strata / Alpen / ASM and how the multisig fits in.
2. [`10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md) — how the ASM derives governance state from Bitcoin L1 (commit-reveal, OP_RETURN, witness envelope).
3. [`03-poc1-findings.md`](./03-poc1-findings.md) — admin subprotocol end-to-end: topology, action lifecycle, crate-level integration points.
4. [`06-hardware-wallet-architecture.md`](./06-hardware-wallet-architecture.md) — why HW signing must live in Rust (not the WebView) and how the layers split.
5. [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) — which PRD update types / authority roles exist upstream today and which are blocked on Alpen Labs.

## Index by topic

### Protocol & ASM

| Doc | Summary | Status |
|-----|---------|--------|
| [`01-conceptual-overview.md`](./01-conceptual-overview.md) | Protocol primer: Bitcoin, Strata, Alpen, ASM, roles, and the multisig application | Complete |
| [`10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md) | ASM state model; Bitcoin L1 as settlement; commit-reveal tx construction | Complete |
| [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) | Record of the 2026-03-25 upstream split (`alpenlabs/alpen` → `alpenlabs/asm`) and Borsh → SSZ migration | Complete |

### POC findings

| Doc | Summary | Status |
|-----|---------|--------|
| [`03-poc1-findings.md`](./03-poc1-findings.md) | POC-1 — Admin subprotocol integration (topology, flow, crate APIs, feasibility) | Done |
| [`04-poc2-findings.md`](./04-poc2-findings.md) | POC-2 — Tauri + React + IPC architecture; session token isolation. HW-wallet conclusions here were superseded by POC-5. | Done (HW section superseded) |
| [`05-poc3-findings.md`](./05-poc3-findings.md) | POC-3 — Signing library (`compute_sighash`, `sign_sighash`, `verify_threshold`) using Alpen crates | Done |
| [`15-poc5-trezor-findings.md`](./15-poc5-trezor-findings.md) | POC-5 — Trezor HID driver, synthetic PSBT workaround, SPS-65 vs BIP-137 reality check | Done |

> **POC-4.** The POC-4 *plan* is archived at [`archive/06-poc4-plan.md`](./archive/06-poc4-plan.md). POC-4 findings live in the execution specs: [`docs/specs/poc4-e2e-propose-sign-flow.md`](../specs/poc4-e2e-propose-sign-flow.md), [`poc4-step1-desktop-proposal-flow.md`](../specs/poc4-step1-desktop-proposal-flow.md), [`poc4-step2-orchestrator-application-layer.md`](../specs/poc4-step2-orchestrator-application-layer.md), [`poc4-domain-strata-admin-multisig-update.md`](../specs/poc4-domain-strata-admin-multisig-update.md).

### Hardware wallets

| Doc | Summary | Status |
|-----|---------|--------|
| [`06-hardware-wallet-architecture.md`](./06-hardware-wallet-architecture.md) | Where HW signing lives (Rust-native) and why; ADR-005 rationale | Complete |
| [`07-hardware-wallet-library-analysis.md`](./07-hardware-wallet-library-analysis.md) | Library choices (`hwi-rs`, `trezor-client`), SPS-65 vs BIP-137 incompatibility | Complete |
| [`15-poc5-trezor-findings.md`](./15-poc5-trezor-findings.md) | POC-5 implementation notes (see above) | Done |

### Upstream crate coverage & open dependencies

| Doc | Summary | Status |
|-----|---------|--------|
| [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) | Mapping of PRD authorities / update types vs upstream `Role` / `AdminTxType` / `UpdateAction` | Complete |
| [`12-upstream-readiness-findings.md`](./12-upstream-readiness-findings.md) | Executive findings on upstream protocol readiness (questions + pending implementations for Alpen Labs) | Complete |
| [`13-authority-verification-findings.md`](./13-authority-verification-findings.md) | Audit of on-chain authority configuration and verification for each role | Complete |

### Cross-cutting risks & tooling

| Doc | Summary | Status |
|-----|---------|--------|
| [`14-nightly-dependency-finding.md`](./14-nightly-dependency-finding.md) | Why the whole workspace is forced onto nightly Rust (SSZ → `generic_const_exprs`) and mitigation options | Complete |

## Archive

Documents that were authoritative during discovery but are now superseded (content absorbed into `docs/3-stories/`, `docs/architecture/`, or `docs/specs/`, or simply overtaken by later POCs):

| Doc | Reason |
|-----|--------|
| [`archive/02-discovery.md`](./archive/02-discovery.md) | Original POC scoping doc from the start of discovery. Its POCs 1–4 were executed; findings live in 03/04/05 and the `poc4-*` specs. |
| [`archive/05-snapshot-reference.md`](./archive/05-snapshot-reference.md) | Educational Snapshot ↔ Alpen/Strata governance analogy. Absorbed into the story-map and architecture overview. |
| [`archive/06-poc4-plan.md`](./archive/06-poc4-plan.md) | POC-4 plan. Plan executed; canonical POC-4 material is in `docs/specs/poc4-*`. |
| [`archive/09-functional-analysis.md`](./archive/09-functional-analysis.md) | Phase-1 functional breakdown. Absorbed into `docs/3-stories/story-map.md`, `docs/3-stories/non-functional-items.md`, and `docs/architecture/overview.md`. |

## Deliverable

The consolidated Phase 1 deliverable — phase-closing assessment, risks, and HWI bundling recommendation — is in [`docs/deliverable/research.md`](../deliverable/research.md).
