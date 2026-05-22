| WakeUp Labs — Project Note | May 2026 |
|---|---|

# Alpen Strata Multisig App — PRD Update Impact Assessment

This document assesses how the PRD update ([`docs/0-prd/03-prd-update.md`](../0-prd/03-prd-update.md)) affects the WakeUp Labs proposal ([`01-alpen-multisig-proposal.md`](./01-alpen-multisig-proposal.md)).

The full delta against the original PRD is captured in [`docs/2-discovery/18-prd-update-diff.md`](../2-discovery/18-prd-update-diff.md); this note re-reads those deltas through the lens of **what changes for the contracted scope, deliverables, phases, and timeline**.

**Status:** Draft for client alignment — pending Alpen sign-off on the open questions in §6 before the proposal is amended.

---

## 1. Executive Summary

The PRD update is **net additive in scope** for WakeUp Labs. The technical approach (Tauri + Axum + Alpen admin crate + HWI signing) is unchanged. The architecture chosen in Phase 1 — three layers, offchain-coordination-only backend, HW-wallet-only signing — remains aligned with the new requirements.

The three areas that **expand the scope** of the proposal are:

1. **Two-key signer model (Admin ID + Admin Wallet).** Signer key derivation is no longer a single BIP-86 path; it now branches on the selected multisig (BIP-86 P2TR for Payout, BIP-84 P2WPKH for the four admin multisigs) and adds a full BIP-86 HD wallet for BTC custody.
2. **Full Admin Wallet UI.** The original proposal called out "Admin Wallet management (balance view, address generation, UTXO visibility, and fee sourcing)". The update specifies a complete BTC-wallet surface: balance net-of-unconfirmed, per-address balances, unconfirmed-tx list with **fee bump**, Receive (QR + one-time-use address rotation), Send BTC with fully-specified validation rules and exact error wording.
3. **`block_payout` import + Admin-Wallet-funded fees.** New "Import raw" path on Pending payouts; fee inputs and change for `block_payout` transactions are now explicitly sourced from the Admin Wallet.

Three smaller items are **minor adjustments** rather than new scope: the onboarding flow re-orders to "multisig-first", two update-type names are renamed in the proposal forms, and the protocol-research scope shrinks slightly because the dropped appendices (backend design notes, SPS-50/51/65 copies) are no longer carried inside the PRD.

**Recommended posture:** Proceed with the proposal as-is for Phase 1; absorb the deltas into Phase 2 design before committing UI surface to Phase 4; flag a likely **+0.5 to +1.5 week** timeline impact concentrated in Phases 3 and 4 (see §5).

---

## 2. Delta map — proposal → PRD update

The table below cross-references each proposal claim against the new PRD requirements and classifies the impact.

| # | Proposal claim | PRD-update requirement | Impact |
|---|---|---|---|
| 1 | "Authentication is based on the Admin ID derived from the user's connected hardware wallet … ephemeral session key model" | Section 3.2: Admin ID derivation now **depends on the selected multisig** (P2TR `m/86'/0'/73'/0/0` for Payout Admin; P2WPKH `m/84'/0'/73'/0/0` for the four other authorities). | **Scope change** — backend auth must verify two address types (P2TR vs P2WPKH) and two signature formats. Onboarding flow re-orders. |
| 2 | "The derivation path `m/86'/0'/73'/0/n` will be used for all signer key derivation. The user selects from the first 20 addresses on this path" | The 20-address picker is gone. Admin ID is a single derived address; the Admin Wallet uses a full HD tree `m/86'/0'/73'/n/n`. | **Replaces** the picker UX with a two-key auto-derive flow. No saved work loss (no picker built yet). |
| 3 | "Admin Wallet management (balance view, address generation, UTXO visibility, and fee sourcing for administrative transactions)" | Section 4: full wallet UI — Balance net-of-unconfirmed, per-address balances, **unconfirmed tx list with fee-bump**, Receive (text+QR, **one-time-use rotation**, on-device verify), Send BTC with **explicit validation rules and error strings**. | **Scope expansion** — the wallet surface is now a complete BTC wallet, not just an info pane. New work: fee-bump (RBF/CPFP), QR rendering, address rotation, full send-validation. |
| 4 | "Complete UI for all multisig roles: pending/approved/past update views, proposal creation forms, signature copy/paste flows, raw transaction broadcast, and fee rate controls." | Section 5: unchanged in shape; the "Send" inside Pending Update now delegates to the **wallet-send UX pattern** (Section 4). | **Refactor** — shared "send" component used by Wallet Send, Pending Update Send, `block_payout` Send. Reduces duplication; aligns with the new wallet UI. |
| 5 | "All fifteen or more message types correctly constructed and signable" + proposal-forms list | Two renames: **Operator update → Bridge Operator update**, **Sequencer update → Sequencer key update**. | **Cosmetic** — UI label + DTO field rename; confirm with Alpen that the on-chain action type is unchanged (`OperatorSetUpdate`, `SequencerUpdate`). |
| 6 | "Payout Administrator flow: `block_payout` transaction creation (both manual and automatic modes), signing, quorum tracking, broadcast, standardness validation, and historical view." | Section 6: adds **import** of raw Pending `block_payout`; fee inputs **must** come from Admin Wallet; change to first unused change address in Admin Wallet; standardness-limit critical error. | **Scope addition** — import endpoint + UI; explicit Admin-Wallet UTXO selection for fee inputs; tightened standardness error path. |
| 7 | "Manual `block_payout` construction UI: user-specified inputs, fee rate control (0.1 s/vB increments, up to 10,000 s/vB), Admin Wallet fee sourcing, change-address routing, and Bitcoin Core standardness limit enforcement" | Section 6.4: matches the proposal claim precisely. | **No change** — the proposal already anticipated this surface. |
| 8 | "Automatic `block_payout` construction UI: 'Block payouts' button … accounting for all required signatures and the fee/change structure." | Section 6.5: matches; clarifies the input count shown to the user is over `block_payout` inputs (not Admin Wallet fee inputs). | **No change** — already aligned. |
| 9 | "Hardware wallet support will cover all HWI-compatible devices that support Taproot inputs, message signing, and on-device display." | Section 3.2: same HWI compatibility requirement; both BIP-84 and BIP-86 paths now required on the same device. | **Verify** — re-check the HW compatibility matrix from POC-5 covers BIP-84 message signing on each device, not only BIP-86. |
| 10 | "SPS-50/51/65 specification research … Alpen admin crate integration assessment" (Phase 1) | The PRD update document does **not** carry the SPS-50/51/65 copies and backend design notes that were embedded in `01-multisig-ui.md`. The specs remain authoritative in their own sources. | **No change** — Phase 1 already used the upstream sources; the deleted appendices were duplicates. |

Legend: **Scope change/addition** = adds engineering work; **Refactor** = restructure existing work; **Cosmetic** = label-level; **Verify** = confirm an existing assumption still holds; **No change** = already aligned.

---

## 3. Impact by phase

### 3.1 Phase 1 — Protocol Research & Architecture (1.5 weeks)

- **Already covered.** Crate integration assessment (`docs/2-discovery/08-alpen-crate-prd-coverage.md`), HW wallet compatibility (`06-hardware-wallet-architecture.md`, `07-hardware-wallet-library-analysis.md`), authority verification (`13-authority-verification-findings.md`), POC-5 Trezor (`16-poc5-trezor-findings.md`).
- **Net-new Phase 1 work introduced by the update:**
  - Confirm both `m/86'/0'/73'/0/0` (P2TR) and `m/84'/0'/73'/0/0` (P2WPKH) derivations are supported by every HW device in the matrix (POC-5 only validated BIP-86 paths on Trezor).
  - Decide the wire format the backend accepts for the Admin ID — raw pubkey vs. P2TR/P2WPKH address — and align with the canonical signer-set representation in ASM state. This is an extension of the work already started in `13-authority-verification-findings.md`.
  - Verify the rename "Bridge Operator update" / "Sequencer key update" is purely a UI label change against the upstream `OperatorSetUpdate` / `SequencerUpdate` action types.
- **Net effort:** ~0.5–1 day of additional research; **no timeline impact** (absorbable inside the 1.5-week envelope).

### 3.2 Phase 2 — Product Design & UX Validation (1 week)

The UX surface grows materially. The phase deliverables are unchanged in name (flows, wireframes, clickable prototype, frontend reference) but the **content** expands:

- **New flow:** Onboarding — multisig-first selection → HW wallet connect → Admin ID + Admin Wallet auto-derive → nonce sign-in.
- **New screen set (Section 4 of the PRD):** Wallet — Balance, Addresses, Transactions (with fee-bump action), Receive (text + QR, on-device verify, one-time-use rotation), Send BTC (with all validation states).
- **New shared component:** "Send" — reused by Wallet Send, Pending Update Send, manual `block_payout` create, `block_payout` Send. All four flows must converge on the same component and validation rules.
- **New error states to wireframe:**
  - `Destination must be a bitcoin address.`
  - `Destination must be a [mainnet/testnet] bitcoin address.`
  - `Insufficient funds`
  - Standardness-limit exceeded (Payout manual create)
- **New affordance:** Import raw `block_payout` on Pending Payouts.
- **Net effort:** roughly **+2 days** vs. the original 1-week budget. Recommend extending Phase 2 to **1.5 weeks**, or compressing the prototype scope (low-fi for new screens).

### 3.3 Phase 3 — Signing Integration & Backend (2.5 weeks)

This is the phase with the largest scope addition.

- **Backend — auth layer:** verify signatures from **both** P2TR (Schnorr/BIP-340) and P2WPKH (ECDSA + BIP-322-style message signing) Admin IDs depending on the multisig the session is bound to. Today the proposal implicitly assumed a single signature scheme.
- **Backend — new endpoints / extensions:**
  - Import raw Pending `block_payout` (new endpoint or extension of "create pending").
  - Admin Wallet read endpoints (balance, addresses, txs, receive address rotation hint). These can be **client-side only** if the wallet runs purely off the Bitcoin Core / Strata RPC, in which case the backend is unaffected — confirm in Phase 1.
- **Signing-integration layer — new work:**
  - Admin Wallet HD derivation and UTXO selection.
  - Fee-bump (RBF or CPFP) flow for unconfirmed Admin Wallet transactions.
  - One-time-use receive-address rotation (track which addresses have been "seen" with any incoming balance).
  - `block_payout` builder must consume Admin Wallet UTXOs for fee inputs and route change to the Admin Wallet change index, while also accounting for fee-input + change-output bytes when packing `block_payout` inputs under the standardness limit.
- **Test surface:** unit tests for the new wallet logic (UTXO selection, fee-bump, RBF replacement detection), plus per-authority auth-signature variants.
- **Net effort:** **+0.5 to +1 week**. Recommend re-planning Phase 3 to **3–3.5 weeks**, with the wallet logic kept in a dedicated module so it can be parallelised with backend lifecycle work.

### 3.4 Phase 4 — Desktop Application & Frontend (3 weeks)

- **New screen set:** Wallet UI (Balance / Addresses / Transactions / Receive / Send), per §3.2.
- **Re-ordered onboarding flow:** Multisig selection → HW wallet → derive Admin ID + Admin Wallet → nonce sign-in.
- **Shared Send component:** built once, consumed from four flows.
- **QR rendering** for the Receive screen.
- **Address-rotation polling/event** on the Receive screen (define "received" — first-seen-unconfirmed vs. first-confirmation, see §6).
- **Fee-bump UX** for unconfirmed wallet transactions.
- **Per-error UI states** with the exact strings from the PRD.
- **Net effort:** **+0.5 to +1 week** depending on how tight the shared Send abstraction lands. Recommend re-planning Phase 4 to **3.5–4 weeks**.

### 3.5 Phase 5 — Integration, Testing & Hardening (2 weeks)

- **Larger integration matrix:**
  - Wallet flows on testnet (send, receive, fee-bump, rotation).
  - Both Admin ID signature schemes (P2TR and P2WPKH) on each HW device in the device matrix.
  - `block_payout` import → sign → broadcast end-to-end.
- **Net effort:** **+2–3 days**, mostly testing time. Recommend an additional 0.5 week buffer if the device matrix is extended.

### 3.6 Phase totals — original vs. revised

| Phase | Original (proposal) | Revised (with PRD update) |
|---|---|---|
| 1 — Research & Architecture | 1.5 weeks | 1.5 weeks (no change) |
| 2 — UI/UX Design | 1 week | 1.5 weeks |
| 3 — Signing Integration & Backend | 2.5 weeks | 3–3.5 weeks |
| 4 — Desktop App & Frontend | 3 weeks | 3.5–4 weeks |
| 5 — Integration & Hardening | 2 weeks | 2–2.5 weeks |
| **Total (with parallelism preserved)** | **7.5 weeks (*)** | **~8.5–9 weeks (*)** |

`(*)` Same disclaimer as the original proposal: range depends on Alpen-side access and turnaround. The original 8–11 week range remains a reasonable outer envelope; the PRD update shifts the **likely** delivery point upward inside that range.

---

## 4. Deliverable deltas

Rewriting the proposal's deliverables list to reflect the update, **net-new bullets in bold**:

- Tauri desktop application binary, reproducible builds, installable via single command or double-click on all three target platforms. *(unchanged)*
- Multi-employee signed release artifacts with documented cryptographic verification instructions for end users. *(unchanged)*
- Hardware wallet integration supporting all HWI-compatible devices with Taproot + message signing + on-device display, covering **both `m/86'/0'/73'/0/0` (P2TR Admin ID, Payout Admin) and `m/84'/0'/73'/0/0` (P2WPKH Admin ID, other multisigs), plus the full BIP-86 HD wallet at `m/86'/0'/73'/n/n` (Admin Wallet)**.
- Offchain coordination backend with full update lifecycle state machine, ephemeral session-key authentication **supporting both P2TR (Schnorr) and P2WPKH (ECDSA / BIP-322) Admin ID signatures**, and quorum tracking across all five multisig types.
- Signing integration layer consuming the existing Alpen admin subprotocol crate for all proposal and update types, covering all fifteen or more message types correctly constructed and signable. **Action labels in the UI updated to "Bridge Operator update" and "Sequencer key update".**
- Complete UI for all multisig roles: pending/approved/past update views, proposal creation forms, signature copy/paste flows, raw transaction broadcast, and fee-rate controls.
- **Full Admin Wallet UI: Balance (net of unconfirmed), per-address balance view, unconfirmed-transaction list with fee-bump action, Receive screen (text + QR, on-device verify, one-time-use address rotation), Send BTC screen with the full validation surface (destination type/network checks, "Insufficient funds", manual fee rate up to 10,000 s/vB with 0.1 s/vB increments, "Max" button, default fee rate from connected Bitcoin Core).**
- **Shared "Send" UX component reused by Wallet Send, Pending Update Send, and the Payout Administrator `block_payout` Send / manual create flows.**
- Payout Administrator flow: `block_payout` transaction creation (both manual and automatic modes), signing, quorum tracking, broadcast, standardness validation, and historical view. **Includes a new "Import raw `block_payout`" affordance on Pending Payouts.**
- Manual `block_payout` construction UI: user-specified inputs, fee rate control (0.1 s/vB increments, up to 10,000 s/vB), Admin Wallet fee sourcing, change-address routing, and Bitcoin Core standardness-limit enforcement with critical error messaging.
- Automatic `block_payout` construction UI: "Block payouts" button triggering greedy input selection that maximizes included `block_payout` inputs within standardness limits, accounting for all required signatures **and the fee input(s) and change output**.
- Automated integration test suite covering all update types on testnet **plus end-to-end Admin Wallet flows (send, receive, fee-bump, rotation) and both Admin ID signature schemes on the HW-wallet device matrix**.
- Technical documentation covering architecture, API reference, build and release process, and end-user setup guide.

---

## 5. Risk and assumption updates

| Area | Original assumption | Update |
|---|---|---|
| HW-wallet support matrix | Devices tested for Taproot inputs + message signing on `m/86'/...` | Same devices must also expose BIP-84 message signing on `m/84'/0'/73'/0/0`. Re-validate against POC-5 outputs. |
| Backend auth signature scheme | Implicitly single (Schnorr/P2TR) | Two schemes: P2TR for Payout, P2WPKH for the other four. Backend must accept and verify both. |
| Admin Wallet scope | "Balance view, address generation, UTXO visibility, fee sourcing" | A complete BTC wallet — additional functionality includes fee-bump (RBF/CPFP), one-time-use rotation, and full send validation. |
| `block_payout` import | Not in scope | New import affordance + corresponding ingest path. |
| Onboarding flow | Address picker on connect → multisig select | Multisig select first → derive Admin ID per multisig type → nonce sign-in. |
| Timeline | 7.5 weeks (parallelised), 8–11 weeks outer | 8.5–9 weeks (parallelised), 8–11 weeks outer **unchanged** — additional work absorbs slack inside the original range. |
| Out-of-scope | No security audit | **Unchanged** — recommend audit of the expanded auth surface (two signature schemes) and the new wallet-send path. |

---

## 6. Open questions for the team

These must be resolved before amending the proposal. They mirror the open questions in [`docs/2-discovery/18-prd-update-diff.md`](../2-discovery/18-prd-update-diff.md) §8 with proposal-specific framing:

1. **Admin ID representation on the backend** — does the canonical signer set list addresses (P2TR / P2WPKH) or raw pubkeys? Drives the verification path and the message-signing format the HW wallet uses (BIP-322 vs. legacy Bitcoin message signing vs. raw Schnorr over a sighash).
2. **Seed sharing between Admin ID and Admin Wallet** — both derive from the same hardware-wallet seed under account `73'`. Confirm intent and confirm that the Admin Wallet's UTXOs are isolated from any pre-existing wallets on the device.
3. **"Bridge Operator update" / "Sequencer key update"** — UI rename only, or also a semantic change in the on-chain action? Affects whether the signing layer's DTOs need to change.
4. **`block_payout` import format** — raw bitcoin transaction, or a structured envelope (action + signatures + metadata)? Affects backend ingest endpoint design.
5. **"Received" semantics for address rotation** — first-seen-unconfirmed (mempool) or first-confirmation. Affects Receive-screen polling cadence and the user's perception of address reuse.
6. **Fee-bump strategy for Admin Wallet** — RBF (BIP-125) only, CPFP only, or both? Drives wallet-builder complexity and UI affordances.
7. **Backend involvement in Admin Wallet** — does the backend track Admin Wallet UTXOs/addresses, or is the wallet purely client-side against Bitcoin Core / Strata RPC? Reasonable default is client-side (preserves the "backend = coordination only" rule from `AGENTS.md`).
8. **Audit scope adjustment** — given the expanded auth surface and the new wallet-send path, recommend Alpen Labs widen the third-party audit RFP to include both.

---

## 7. Recommended next actions

1. **Confirm the eight open questions in §6 with Alpen Labs** before signing off on a revised timeline.
2. **Re-run the HW-wallet compatibility matrix** for BIP-84 message signing on each in-scope device (extension of POC-5).
3. **Spike the Admin Wallet module** in parallel with Phase 1: a small standalone PoC that derives `m/86'/0'/73'/n/n`, lists UTXOs from a Bitcoin Core regtest node, and builds a fee-bumpable transaction. Output reduces Phase 3 unknowns.
4. **Update the architecture overview** ([`docs/architecture/overview.md`](../architecture/overview.md)) to reflect:
   - Two-key signer model
   - Two auth signature schemes on the backend
   - Admin Wallet as a first-class module
5. **Amend the proposal** (`01-alpen-multisig-proposal.md`) once §6 is resolved, using the revised deliverables in §4 and revised phase totals in §3.6 as the source.

---

## 8. References

- Proposal under review: [`01-alpen-multisig-proposal.md`](./01-alpen-multisig-proposal.md)
- PRD update: [`docs/0-prd/03-prd-update.md`](../0-prd/03-prd-update.md)
- PRD diff (canonical delta): [`docs/2-discovery/18-prd-update-diff.md`](../2-discovery/18-prd-update-diff.md)
- Architecture overview: [`docs/architecture/overview.md`](../architecture/overview.md)
- Crate coverage: [`docs/2-discovery/08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md)
- HW wallet architecture: [`docs/2-discovery/06-hardware-wallet-architecture.md`](../2-discovery/06-hardware-wallet-architecture.md)
- HW wallet library analysis: [`docs/2-discovery/07-hardware-wallet-library-analysis.md`](../2-discovery/07-hardware-wallet-library-analysis.md)
- POC-5 Trezor findings: [`docs/2-discovery/16-poc5-trezor-findings.md`](../2-discovery/16-poc5-trezor-findings.md)
- Authority verification: [`docs/2-discovery/13-authority-verification-findings.md`](../2-discovery/13-authority-verification-findings.md)
