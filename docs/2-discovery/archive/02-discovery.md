# Discovery — Iteration 1: Alpen Strata Multisig Admin Flow

> **Archived (2026-04-17).** This is the original POC scoping doc from the very start of discovery. Its proposed POCs 1–4 were executed and the findings live in [`03-poc1-findings.md`](../03-poc1-findings.md), [`04-poc2-findings.md`](../04-poc2-findings.md), [`05-poc3-findings.md`](../05-poc3-findings.md), and the [`docs/specs/poc4-*`](../../specs/) specs. The consolidated index is in [`docs/2-discovery/README.md`](../README.md).

## Overview

This document captures the findings from the first discovery iteration focused on understanding the admin/proposal/signing flow between Strata, Alpen, and BTC. The goal was to assess technical feasibility, end-to-end integration, and identify which areas to validate first through targeted POCs.

## Key Risks & Challenges

| # | Risk Area | Description |
|---|-----------|-------------|
| 1 | **Cross-layer connectivity** | How Alpen, Strata, and BTC actually connect — what calls what, and through which interfaces. |
| 2 | **End-to-end protocol integration** | Understanding which parts of the flow live on-chain vs. off-chain, and how a complete action travels through the system. |
| 3 | **Wallet compatibility** | Hardware wallet support constraints — especially around Taproot inputs, message signing, and on-device display (HWI device matrix). |
| 4 | **Admin subprotocol contracts** | The role and behavior of subprotocols (SPS-50, SPS-51, SPS-65) and the admin crate API surface. |
| 5 | **Signing model** | Validating the authorization and signature scheme on the BTC/UTXO side — multisig threshold logic, key derivation, and transaction construction. |
| 6 | **Wallet-to-UI connection** | Exposing a reasonable frontend experience for wallet connection, address selection, and signing. |

## Initial Assumptions

- There is an **EVM-like ↔ Strata ↔ BTC** relationship across layers.
- The functional flow resembles a **proposal → approval → execution** pattern (conceptually similar to Snapshot in terms of UX mental model, though not necessarily in implementation).
- The existing Alpen admin subprotocol crate (`crates/asm/subprotocols/admin`) provides the core types and logic for constructing protocol update messages.

## Open Questions

### 1. System Topology — Where does each part live?

- What runs on BTC?
- What runs on Strata?
- What runs on Alpen / the protocol layer?

### 2. Roles & Permissions Model

- Can a single address hold multiple roles, or only one?
- Can all roles perform the same actions?
- What is the operational impact of each role?

### 3. Proposal Generation

- Does the backend generate the proposal?
- If so, based on what inputs, rules, or state?

### 4. Payout Flow

- What does the payout operation do within the flow?
- Is it part of final execution, settlement, or a separate mechanism?

### 5. Backend / Indexing / Status Tracking

- Is a backend or indexer required to surface state, history, and lifecycle information?
- Or can the necessary state be resolved directly from on-chain / protocol data?

## Proposed POCs

To reduce uncertainty, we proposed four focused POCs. **POC 1 is prioritized** as it directly aligns with Phase 1 (Protocol Research & Architecture) of the [project proposal](../../1-proposal/01-alpen-multisig-proposal.md).

### POC 1 — Admin / Subprotocol Integration (Prioritized)

**Objective:** Understand how the Alpen Administration Subprotocol works at a high level — its topology, how the layers connect, and how an admin action flows through the system end-to-end.

**Alignment with Phase 1:** This POC maps directly to the first phase of the proposal — internalizing SPS-50/51/65 specs, reviewing the admin crate, identifying integration points, and producing the integration assessment deliverable.

**Expected outcomes:**
- A clear, documented understanding of the admin subprotocol topology and end-to-end flow.
- Identification of the key integration points (RPC, crate API) for the multisig application.
- Identify gaps or blockers that would change the architectural approach.

### POC 2 — Wallet Integration UI

- Test wallet connection from a frontend.
- Validate compatibility and UX friction with HWI-compatible hardware wallets.

### POC 3 — Signing Test UI

- Minimal screen to test the signing/approval flow.
- Understand what the user signs and how the wallet responds.

### POC 4 — Mini Backend (E2E)

- Lightweight backend to construct requests/proposals, track status, and connect the flow end-to-end.
- Validates whether an indexer / service layer is truly needed.

## Key Takeaway

The main gap today is **not** building a UI — it is correctly understanding the architecture of the admin flow and the signing model between Strata, Alpen, and BTC. The best path forward is short, targeted technical POCs that reduce uncertainty before committing to a final architecture.

---

## POC 1 — Admin / Subprotocol Integration (Detail)

### Objective

Understand how the Alpen Administration Subprotocol works at a high level — its topology, how the layers connect, and how an admin action flows through the system end-to-end.

The primary input for this POC is the [Alpen Administration Subprotocol specification](https://www.notion.so/317901ba000f80bf8d96eb) and the existing [admin crate](https://github.com/alpenlabs/alpen/tree/main/crates/asm/subprotocols/admin) implementation.

### Scope

- Read and understand the admin subprotocol spec and the admin crate source code (`authority.rs`, `handler.rs`, `queued_update.rs`, `state.rs`, `subprotocol.rs`).
- If possible, run a minimal interaction against a Strata testnet node to observe the flow in practice.
- Document findings on how the system works.

### Questions This POC Should Answer

1. **System topology** — What lives on BTC, what lives on Strata, and how do the layers connect?
2. **Admin action flow** — How does an admin action travel through the system from creation to execution?
3. **Integration points** — What are the main interfaces (RPC, crate API) we need to integrate with?
4. **Feasibility** — Are there any blockers or major unknowns that would change our architectural approach?

### Success Criteria

- A clear, documented understanding of the admin subprotocol topology and end-to-end flow.
- Identification of the key integration points for the multisig application.
