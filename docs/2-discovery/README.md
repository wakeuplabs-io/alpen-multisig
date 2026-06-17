# Phase 1 — Discovery

This folder holds the research, POC findings, and technical investigations produced during **Phase 1 (Protocol Research & Architecture)** of the [WakeUp Labs proposal](../1-proposal/01-alpen-multisig-proposal.md). The consolidated output of this phase lives in two places:

- **Deliverable (client)** — [`docs/external/research-assessment.md`](../external/research-assessment.md).
- **Internal index** — [`docs/deliverable/research.md`](../deliverable/research.md) and [`docs/deliverable/crate-inventory.md`](../deliverable/crate-inventory.md).
- **Architecture** — [`docs/architecture/overview.md`](../architecture/overview.md) and the ADRs under [`docs/architecture/adrs/`](../architecture/adrs/).

Everything under this folder is **raw source material**: POC plans, POC findings, functional analyses, and targeted investigations. It exists so reviewers can trace any claim in the deliverable back to its evidence.

---

## Reading guide

The docs are numbered in roughly chronological order. For topic-driven reading, follow the groupings below.

### POC arc (chronological)

The discovery plan was organised around five focused POCs. Each one reduced a specific uncertainty before committing to an architectural decision.

| # | Doc | What it covers |
|---|---|---|
| 1 | [`02-discovery.md`](./02-discovery.md) | Original phase-1 discovery iteration and POC plan |
| 2 | [`03-poc1-findings.md`](./03-poc1-findings.md) | POC-1 — Admin subprotocol integration (SPS-50/51/65 topology) |
| 3 | [`04-poc2-findings.md`](./04-poc2-findings.md) | POC-2 — Tauri + React + Rust IPC stack validation |
| 4 | [`05-poc3-findings.md`](./05-poc3-findings.md) | POC-3 — Signing library (SPS-65 sighash, threshold verification) |
| 5 | [`14-poc4-plan.md`](./14-poc4-plan.md) | POC-4 plan — mini coordination flow (propose → sign → quorum) |
| 6 | [`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) | POC-5 — Trezor HW wallet integration via `trezor-client` |

### Protocol & crate coverage

| Doc | What it covers |
|---|---|
| [`01-conceptual-overview.md`](./01-conceptual-overview.md) | Conceptual background — Strata, Alpen, bridge mechanics, governance model |
| [`09-functional-analysis.md`](./09-functional-analysis.md) | Functional decomposition — entities, update types, user flows, data model |
| [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md) | Upstream crate surface vs. PRD requirements — full coverage-gap map |
| [`10-asm-bitcoin-state-model.md`](./10-asm-bitcoin-state-model.md) | ASM on-chain state model and Bitcoin transaction lifecycle |
| [`11-asm-repo-migration.md`](./11-asm-repo-migration.md) | Migration from `alpenlabs/alpen` to `alpenlabs/asm` (2026-04-17), Borsh → SSZ |
| [`13-authority-verification-findings.md`](./13-authority-verification-findings.md) | Research on proving that a given key belongs to an authority (challenge-response) |

### Hardware wallet arc

| Doc | What it covers |
|---|---|
| [`06-hardware-wallet-architecture.md`](./06-hardware-wallet-architecture.md) | Architecture decisions — JS SDK vs Rust-native, HWI vs direct transports |
| [`07-hardware-wallet-library-analysis.md`](./07-hardware-wallet-library-analysis.md) | Rust HW library inventory — `hwi-rs`, `trezor-client`, `ledger-transport-hid` |
| [`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) | POC-5 execution — Trezor via HID transport, synthetic PSBT workaround |

### Upstream readiness & build infrastructure

| Doc | What it covers |
|---|---|
| [`12-upstream-readiness-findings.md`](./12-upstream-readiness-findings.md) | Executive summary of upstream protocol maturity and delivery risk |
| [`15-nightly-dependency-finding.md`](./15-nightly-dependency-finding.md) | Why the workspace is forced onto nightly Rust (`generic_const_exprs` via SSZ) |

### Reference material

| Doc | What it covers |
|---|---|
| [`05-snapshot-reference.md`](./05-snapshot-reference.md) | Snapshot/SafeSnap/EIP-712 analogy — onboarding mental model for Ethereum engineers |

---

## Document status

| Status | Meaning |
|---|---|
| **Complete** | Findings are final for Phase 1; tracked evolutions go into specs or ADRs, not here |
| **Superseded** | Original conclusions were revised by a later POC or ADR; post-discovery notes explain what changed — kept as historical record |
| **Reference** | Standalone educational material; not part of the Phase 1 deliverable chain |

Several POC findings (`03`, `04`, `05-poc3`) carry **post-discovery notes** pointing to the decisions that refined or replaced their original conclusions. Follow those pointers rather than trusting the body of the doc in isolation.

---

## What Phase 1 produced

- **Crate integration assessment** — coverage gaps classified by authority and update type, re-validated after the 2026-04-17 `alpenlabs/asm` migration. See [`docs/external/research-assessment.md`](../external/research-assessment.md) §1, [`crate-inventory.md`](../deliverable/crate-inventory.md), and [`08-alpen-crate-prd-coverage.md`](./08-alpen-crate-prd-coverage.md).
- **Hardware wallet compatibility matrix** — SPS-65 sighash vs. BIP-137 gap identified; Rust-native integration path chosen over WebView JS SDKs. See [`docs/external/research-assessment.md`](../external/research-assessment.md) §2, [`docs/external/hardware-wallet-matrix.md`](../external/hardware-wallet-matrix.md), and the HW wallet arc above.
- **Architecture document** — layered architecture (ADR-005), Alpen crate dependency strategy (ADR-001), application-layer strategy (ADR-002, ADR-003), CI pipeline (ADR-004). See [`docs/architecture/overview.md`](../architecture/overview.md).
- **Upstream readiness assessment** — executive-level maturity findings informing scope commitments. See [`12-upstream-readiness-findings.md`](./12-upstream-readiness-findings.md).
- **HWI bundling recommendation** — deferred; the production path is Rust-native HW integration (Trezor via `trezor-client`, Ledger via `hwi-rs`/PSBT). See [`06-hardware-wallet-architecture.md`](./06-hardware-wallet-architecture.md).

Phase 2 work tracks under [`docs/3-stories/`](../3-stories/) (scope) and [`docs/specs/`](../specs/) (per-feature specs).
