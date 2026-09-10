# Strata Multisig — Story Map

Qualified user stories derived from `0-prd/` (client requirements) and shaped by `2-discovery/` findings. Follows Jeff Patton's Story Mapping: the **backbone** (user activities, left→right) expresses what the product does; **slices** (top→bottom) express release increments. The walking skeleton (Slice 0) is the thinnest end-to-end path proving the system works.

**Scope:** functional user stories only. Non-functional concerns are listed separately in [`non-functional-items.md`](./non-functional-items.md) — they become specs, not user stories.

---

## 1. Actors

| Actor | Description |
|---|---|
| **Signer** | Generic role — any authorized member of a multisig authority. Used when behavior is identical across authorities. |
| **Alpen Admin Signer** | Member of the Alpen Administrator multisig. |
| **Strata Admin Signer** | Member of the Strata Administrator multisig. |
| **Sequencer Manager Signer** | Member of the Strata Sequencer Manager multisig. |
| **Security Council Signer** | Member of the Strata Security Council multisig. |
| **Payout Admin Signer** | Member of the Payout Administrator multisig. |

---

## 2. Backbone (User Activities)

The backbone is the user's journey, left to right. One signer traverses roughly this path per session.

| A. Start app | B. Connect wallet | C. Enter authority | D. Review state | E. Create proposal | F. Sign / Cancel | G. Coordinate sigs | H. Broadcast | I. Payout ops (Payout Admin swimlane) |
|---|---|---|---|---|---|---|---|---|
| Launch, configure RPC | Select HW + address | Select multisig + auth | Pending / Approved / Past | New update of supported type | Approve or cancel | Export / paste sigs | On-chain tx w/ fee control | Pending/past payouts, manual + auto construction |

---

## 3. Slicing (Release Increments)

| Slice | Intent | Scope |
|---|---|---|
| **0 — Walking Skeleton** | Prove one E2E path end-to-end with the authority+action combo fully covered today (Strata Admin signer update). Software signer or basic HW. No in-app broadcast — raw tx export only. | US-A1, B1, C1, C2, D1, E1, F1, H1 |
| **1a — Admin Wallet regtest commit funding** | Walking skeleton: BDK + chain RPC pays governance commit on regtest; legacy bitcoind wallet funding remains default for CI. | US-H7 (extends US-H6) |
| **1 — Real HW + in-app broadcast** | Full HW wallet flow (list 20 addresses, on-device verify), in-app Bitcoin broadcast, manual fee control. | C3, C4, D5, H4, H6, I1, I4 |
| **2 — All authorities & update types** | Expand to remaining 4 authorities and all 12 update types. Depends on upstream Alpen crate support (8 types still missing — see risks). | F2, F3, F4, F6, F7, F8, F9, F10, F11, F12, F13 |
| **3 — Approved state, cancellation, past view** | Post-quorum lifecycle: approved/past views, cancellation flow, optional auto-broadcast on quorum. | D3, D4, F14, G1 (cancel variant), H2, I2, I3 |
| **4 — Payout Administrator** | `block_payout` operations: pending/past views, manual + automatic construction, sign, broadcast. | J1..J9 |
| **5 — Manual fallback & access control** | Full offline fallback (compose tx without backend), denied-access paths, session/wallet disconnect. | C5, C6, C7, G1, H3, H5 |

> Slices are iteration boundaries, not strict phases — details may shift when confronted with implementation reality. Walking skeleton is the non-negotiable first step.
>
> Full phased Admin Wallet program: [`docs/specs/admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md).

---

## 4. Qualified User Stories

Format: `As a <actor>, I want to <goal>, so that <motivation>.` Each story carries a classification, acceptance signals extracted from the PRD, and source references. Acceptance signals are lifted from the PRD — not invented.

### Activity A — Start the app

#### US-A1 · Launch the app
- **Story:** As a Signer, I want to launch the application and land on the wallet connection screen, so that I can begin a governance session.
- **Classification:** Functional
- **Acceptance signals:** App starts from a single command or double-click; dependency install ≤ one additional command/click.
- **Source:** UI PRD §1.4, §1.4.1.
- **Slice:** 0.

#### US-A2 · Configure node or RPC connection
- **Story:** As a Signer, I want the app to connect to my local Strata node by default and offer a trusted RPC fallback when unavailable, so that I can interact with Bitcoin/Strata state regardless of my setup.
- **Classification:** Functional
- **Acceptance signals:** Default attempts local-node connection; when missing, prompts user to start local node or switch to trusted RPC; `stratabtc.org` preset offered; custom URL input accepted.
- **Source:** UI PRD §1.5, §1.5.1–1.5.3.
- **Slice:** 1. *(Walking skeleton can stub with a static RPC endpoint.)*

### Activity B — Connect hardware wallet

#### US-B1 · Connect hardware wallet (minimal)
- **Story:** As a Signer, I want to connect a supported hardware wallet and obtain a signer identity, so that I can authenticate and sign governance actions with private keys that never leave the device.
- **Classification:** Functional
- **Acceptance signals:** Device must be HWI-compatible with Taproot inputs, message signing, on-device display, and SPS-65 compatibility. First 20 addresses on `m/86'/0'/73'/0/n` derivation path are available.
- **Source:** UI PRD §1.6, §1.6.1, §1.6.2.
- **Slice:** 0 (single-address path is enough for skeleton).
- **Discovery note:** Hardware list effectively narrows to Ledger Nano S+/Stax and Trezor Model T / Safe 3. SPS-65 digest handling requires synthetic PSBT binding or BIP-137 — still to be validated against on-chain ASM (see `2-discovery/` §risks).

#### US-B2 · Select a signer address from the first 20
- **Story:** As a Signer, I want to browse and pick one of the first 20 addresses on the derivation path, so that I can use a distinct key per role or per rotation.
- **Classification:** Functional
- **Acceptance signals:** User picks from the first 20 addresses on `m/86'/0'/73'/0/n`.
- **Source:** UI PRD §1.6.2.
- **Slice:** 1.

#### US-B3 · Verify selected address on-device
- **Story:** As a Signer, I want to verify that the address shown in the app matches what my hardware wallet displays, so that I can detect UI or driver tampering before signing anything.
- **Classification:** Functional
- **Acceptance signals:** App triggers on-device address display for the selected address.
- **Source:** UI PRD §1.6.5.
- **Slice:** 1.

#### US-B4 · Copy selected address to clipboard
- **Story:** As a Signer, I want to copy my selected address to the clipboard, so that I can share it with other signers or paste it into external tools.
- **Classification:** Functional
- **Acceptance signals:** Address visible in UI; clipboard copy action available.
- **Source:** UI PRD §1.6.4.
- **Slice:** 1.

### Activity C — Enter a multisig authority (authentication)

#### US-C1 · Select a multisig authority
- **Story:** As a Signer, I want to see the list of multisigs my selected address is a signer on and pick one, so that my subsequent actions are scoped to that authority.
- **Classification:** Functional
- **Acceptance signals:** List restricted to authorities where the address appears in the canonical signer set. Five supported authorities: Alpen Admin, Strata Admin, Sequencer Manager, Security Council, Payout Admin.
- **Source:** UI PRD §1.7, §1.7.1–1.7.5; Backend PRD (canonical signer set per authority).
- **Slice:** 0 (Strata Admin only for skeleton).

#### US-C2 · Authenticate via ephemeral session
- **Story:** As a Signer, I want to prove ownership of my signer key and receive a bounded session for the selected authority, so that I can view and modify proposals securely.
- **Classification:** Functional
- **Acceptance signals:**
  - Client generates an ephemeral keypair at session start.
  - Signer signs a structured auth message binding the ephemeral key to the authority, including nonce and expiry.
  - Backend verifies signature against canonical signer set derived from ASM state; grants access only on valid match.
  - Invalid signature → error; valid signature but address not in signer set → error.
  - Subsequent requests are signed with the ephemeral private key.
- **Source:** UI PRD §1.8, §1.8.1–1.8.3; Backend PRD (ephemeral session model); Proposal §Technical Approach 2.
- **Slice:** 0.

#### US-C3 · Deny non-signer access (negative path)
- **Story:** As an unauthorized user whose address is not in the canonical signer set, I should be denied view of any pending, approved, or past proposals, so that confidentiality is preserved.
- **Classification:** Functional
- **Acceptance signals:** Non-signer request is rejected before any proposal metadata is disclosed. No inference of proposal existence is possible.
- **Source:** Backend PRD (non-signer confidentiality).
- **Slice:** 5.

#### US-C4 · Close the multisig session (return to authority selection)
- **Story:** As a Signer, I want to exit the current multisig and return to the authority selection screen, so that I can switch between multisigs without disconnecting my wallet.
- **Classification:** Functional
- **Acceptance signals:** "Close" action returns user to multisig selection screen.
- **Source:** UI PRD §1.9.
- **Slice:** 5.

#### US-C5 · Disconnect the hardware wallet
- **Story:** As a Signer, I want to disconnect my selected address and return to the wallet connection screen, so that I can end my session safely or hand off the device.
- **Classification:** Functional
- **Acceptance signals:** Disconnect action returns user to wallet connection screen.
- **Source:** UI PRD §1.10.
- **Slice:** 1.

### Activity D — Review proposal state

#### US-D1 · List pending proposals with expiry and signature progress
- **Story:** As a Signer on a non-Payout authority, I want to see all pending proposals with their time remaining until expiry and the count of approval signatures collected, so that I know which proposals need my action and their progress toward quorum.
- **Classification:** Functional
- **Acceptance signals:** Lists all pending updates; each shows time left before 7-day expiry and `collected / required` approval signatures. Visible only to signers of that authority.
- **Source:** UI PRD §1.13, §1.13.1; Backend PRD (status tracking).
- **Slice:** 0.

#### US-D2 · View proposal details and collected signatures
- **Story:** As a Signer, I want to fetch the action payload and the full signature list for a specific proposal, so that I can review what is being proposed before approving.
- **Classification:** Functional
- **Acceptance signals:** Returns action payload; returns collected signatures. Data stays consistent with on-chain ASM state.
- **Source:** Backend PRD (retrieve action + signatures); UI PRD implicit in review flows.
- **Slice:** 1.

#### US-D3 · List approved proposals with cancellation signatures
- **Story:** As an Alpen or Strata Admin Signer, I want to see all approved proposals and the count of cancellation signatures collected for each, so that I can coordinate an emergency cancellation before enactment.
- **Classification:** Functional
- **Acceptance signals:** Lists approved updates; shows cancellation signatures per update. Approved = quorum reached and confirmed on-chain, not yet enacted. Not applicable to Sequencer Manager or to Defcon 1, which execute immediately; Defcon 3 *is* applicable.
- **Source:** UI PRD §1.12, §1.12.2.
- **Slice:** 3.

#### US-D4 · List past proposals (enacted, canceled, expired)
- **Story:** As a Signer on a non-Payout authority, I want to see all past proposals, so that I can audit governance history.
- **Classification:** Functional
- **Acceptance signals:** Lists updates that have been enacted, canceled, or expired.
- **Source:** UI PRD §1.14.
- **Slice:** 3.

### Activity E — Create proposals (one story per supported update type)

> All creation stories share the same shape: the signer composes the update payload for a specific type and submits it — together with their own signature — as a new pending proposal. ActionId is `hash(MultisigAction, SeqNo)`; duplicate `(action, seqno)` submissions are rejected (Backend PRD).

> **Discovery gap:** `2-discovery/08-alpen-crate-prd-coverage.md` shows the upstream Alpen admin crate currently only covers Strata Admin signer update and Sequencer update. The other 11 types depend on upstream Alpen expanding the `Role` enum, `AdminTxType`, and sighash tags. Track this as a dependency for Slice 2.

| ID | Story title | Actor | Authority | Update type | Slice |
|---|---|---|---|---|---|
| US-E1 | Create Strata Admin signer update proposal | Strata Admin Signer | Strata Admin | Strata signer update | 0 |
| US-E2 | Create Sequencer update proposal | Sequencer Manager Signer | Sequencer Manager | Sequencer key change (executes immediately) | 1 |
| US-E3 | Create Alpen verification key update | Alpen Admin Signer | Alpen Admin | Alpen VK update | 2 |
| US-E4 | Create Alpen signer update | Alpen Admin Signer | Alpen Admin | Alpen signer update | 2 |
| US-E5 | Create Safe Harbor address update | Strata Admin Signer | Strata Admin | Safe Harbor address | 2 |
| US-E6 | Create Strata verification key update | Strata Admin Signer | Strata Admin | Strata VK update | 2 |
| US-E7 | Create Security Council signer update | Strata Admin Signer | Strata Admin | Security Council signer update | 2 |
| US-E8 | Create operator update | Strata Admin Signer | Strata Admin | Bridge operator add/remove | 2 |
| ~~US-E9~~ | ~~Create soft bridge update~~ | — | — | **Retired** — "soft bridge update" is no longer a relevant concept and has no counterpart upstream at any revision | — |
| ~~US-E10~~ | ~~Create hard bridge update~~ | — | — | **Retired** — same as US-E9 | — |
| US-E11 | Create Sequencer Manager signer update | Sequencer Manager Signer | Sequencer Manager | Sequencer Manager signer update | 2 |
| US-E12 | Create Defcon 1 transaction | Security Council Signer | Security Council | Defcon 1 emergency action — executes immediately, never queued, no cancel | 2 |
| US-E13 | Create Defcon 3 transaction | Security Council Signer | Security Council | Defcon 3 emergency action — timelocked; reaches Approved and has a cancel window | 2 |
| US-E14 | Cancel a queued Defcon 3 | Security Council Signer | Security Council | Cancel of a queued Defcon 3, signed by the council itself | 2 |

Shared acceptance signals for all US-E*:
- Proposal is persisted with stable `ActionId = hash(MultisigAction, SeqNo)`.
- Creator's signature is stored with the proposal.
- Duplicate `(action, seqno)` submissions are rejected without mutating existing state.
- Multiple distinct proposals may share the same SeqNo.
- **Source:** UI PRD §1.15 family; Backend PRD (ActionId, idempotency, duplicate handling).

### Activity F — Sign / Cancel

#### US-F1 · Approve a pending proposal
- **Story:** As a Signer, I want to produce an approval signature on a pending proposal using my hardware wallet, so that I contribute toward quorum.
- **Classification:** Functional
- **Acceptance signals:** Approval signature produced for any pending update; hardware wallet screen displays a human-readable representation of the message being signed. Signature is appended to the proposal's signature list.
- **Source:** UI PRD §1.13.2, §1.6.6; Backend PRD (append signature).
- **Slice:** 0.
- **Discovery note:** The "message being signed" on the HW screen is constrained by device firmware (BIP-137 text or PSBT fields, not raw SPS-65 digest). Visualization strategy is a design question for Phase 2, not a US detail.

#### US-F2 · Cancel an approved proposal
- **Story:** As an Alpen or Strata Admin Signer, I want to produce a cancellation signature on an approved proposal, so that the authority can block enactment.
- **Classification:** Functional
- **Acceptance signals:** Cancellation signature produced; a fresh cancellation quorum is required. Not applicable to Sequencer Manager or to Defcon 1. A Defcon 3 cancel is signed by the Security Council itself — see US-E14.
- **Source:** UI PRD §1.12.1; Backend PRD (cancellation signature path).
- **Slice:** 3.

### Activity G — Coordinate signatures (offline fallback)

#### US-G1 · Export all collected approval signatures
- **Story:** As a Signer, I want to copy all approval signatures collected so far for a proposal to my clipboard, so that I can hand them to another signer or to an offline broadcaster.
- **Classification:** Functional
- **Acceptance signals:** Copy action exports all available approval signatures for the selected pending update.
- **Source:** UI PRD §1.13.2.1.
- **Slice:** 5.
- **Note:** Redundant with US-H1 for the online coordination path; retained as a helper for the offline/manual fallback scenario (see US-H5).

#### US-G2 · Export all collected cancellation signatures
- **Story:** As a Signer, I want to copy all cancellation signatures collected so far for an approved proposal to my clipboard, so that I can coordinate the cancellation broadcast.
- **Classification:** Functional
- **Acceptance signals:** Copy action exports all available cancellation signatures for the selected approved update.
- **Source:** UI PRD §1.12.1.1.
- **Slice:** 3.

#### US-G3 · Query last confirmed SeqNo for an authority
- **Story:** As a Signer constructing a new proposal without the coordination backend, I want to retrieve the last confirmed SeqNo for my authority directly from on-chain state, so that I can build a valid ActionId.
- **Classification:** Functional
- **Acceptance signals:** Backend caches canonical on-chain SeqNo for quick access; value is derivable directly from ASM state if backend unavailable.
- **Source:** Backend PRD (last confirmed SeqNo).
- **Slice:** 5.

### Activity H — Broadcast on Bitcoin

#### US-H1 · Export raw approval transaction for external broadcast
- **Story:** As a Signer, I want the app to assemble the raw approval transaction for a pending proposal once quorum of approval signatures has been reached, and copy it to my clipboard, so that I can broadcast it manually using external tooling.
- **Classification:** Functional
- **Acceptance signals:**
  - Action available only when the proposal has reached quorum of approval signatures.
  - Raw transaction is assembled automatically using signatures already collected by the backend (no manual signature paste).
  - Raw transaction copied to clipboard in a broadcast-ready encoding.
  - No in-app broadcast, no fee rate control.
- **Source:** UI PRD §1.13.2.2.
- **Slice:** 0.

#### US-H2 · Create and broadcast a cancellation transaction
- **Story:** As a Signer, I want to paste the quorum of cancellation signatures and broadcast the cancellation transaction, so that an approved update is blocked before enactment.
- **Classification:** Functional
- **Acceptance signals:** Cancellation transaction built for any approved update; quorum signatures pasted in; broadcast via app RPC or raw clipboard copy. Canceled updates remain off-chain and visible only to authority signers.
- **Source:** UI PRD §1.12.1.2, §1.12.1.2.3.
- **Slice:** 3.

#### US-H3 · Offer auto-broadcast when my signature completes quorum
- **Story:** As a Signer whose signature causes a pending update to reach quorum, I want to be offered the option to immediately construct, sign, and broadcast the Bitcoin transaction, so that I can finalize the update in one step or decline and let another signer handle it.
- **Classification:** Functional
- **Acceptance signals:** Option presented only to the quorum-completing signer; user can accept or decline.
- **Source:** UI PRD §1.13.2.3.
- **Slice:** 3.

#### US-H4 · Set manual fee rate for broadcast
- **Story:** As a Signer broadcasting any approval, cancellation, or payout transaction, I want to set the sat/vB fee rate manually in 0.1 increments up to 10 000 sat/vB, so that I can react to fee-market conditions.
- **Classification:** Functional
- **Acceptance signals:** "Send" button exposes an amount entry field that accepts 0.1 sat/vB increments up to 10 000 sat/vB.
- **Source:** UI PRD §1.13.2.3.1 (and §1.17.3.3.1 for payouts).
- **Slice:** 1.

#### US-H5 · Compose a transaction manually when the backend is unavailable
- **Story:** As a Signer, I want to aggregate signatures manually and broadcast a valid approval or cancellation transaction directly to Bitcoin without the coordination backend, so that governance is never blocked by backend downtime.
- **Classification:** Functional
- **Acceptance signals:** Signers can aggregate signatures offline; construct a valid approval or cancellation transaction locally; broadcast directly to Bitcoin. No backend dependency for correctness.
- **Source:** Backend PRD (manual fallback).
- **Slice:** 5.

#### US-H6 · Broadcast an approval transaction via app Bitcoin RPC
- **Story:** As a Signer, I want to broadcast the approval transaction directly from the app using its Bitcoin RPC connection, so that I can finalize the update without external tooling.
- **Classification:** Functional
- **Acceptance signals:**
  - Broadcast is triggered from the app; raw transaction is assembled from backend-collected signatures (same as US-H1).
  - Fee rate configurable per US-H4.
- **Source:** UI PRD §1.13.2.2; implemented as commit/reveal per [`docs/specs/proposal-broadcast-commit-reveal.md`](../specs/proposal-broadcast-commit-reveal.md).
- **Slice:** 1.

#### US-H7 · Fund an approved proposal commit from the Admin Wallet (regtest walking skeleton)
- **Story:** As a Strata Administrator or Alpen Administrator Signer on regtest, I want the desktop app to pay the Bitcoin commit transaction for an approved governance proposal from an Admin Wallet Taproot address derived at `m/86'/0'/73'/n/n`, so that we validate Admin Wallet derivation, UTXO selection, and on-chain spend before building the full wallet UI.
- **Classification:** Functional
- **Acceptance signals:**
  - Only for `approved` proposals in the existing in-app commit/reveal broadcast flow (US-H6 / `proposal-broadcast-commit-reveal.md`).
  - Commit funding uses BDK with a Bitcoin Core–compatible RPC endpoint; descriptors use BIP-86 account `73'` (regtest coin `0'`).
  - Minimum paths: external `m/86'/0'/73'/0/0` for funding; change to first unused `m/86'/0'/73'/1/*`.
  - Commit **destination** remains the operator-derived Taproot commit address (protocol unchanged).
  - Reveal: operator key in Tauri process; orchestrator claim + PATCH unchanged.
  - Regtest-only enablement via `BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1`. (Phase 3.6 made the Admin Wallet the sole commit funder; the legacy `COMMIT_FUNDING`/`sendtoaddress` path was removed.)
  - Phase 1 commit signing: regtest dev mnemonic in Tauri (dev flags); no HWI; no Ledger/Trezor required for US-H7.
  - UI: funding mode, Admin Wallet address and available balance before confirm; existing broadcast phase progress and txids on success.
  - Clear errors: insufficient Admin Wallet funds, RPC failure, misconfiguration.
- **Source:** `docs/0-prd/03-prd-update.md` §3.2, §5.3.2.2–5.3.2.3; [`docs/specs/proposal-broadcast-commit-reveal.md`](../specs/proposal-broadcast-commit-reveal.md); walking skeleton for PRD §4.
- **Slice:** **1a — Admin Wallet regtest commit funding**
- **Depends on:** US-C1, US-C2, US-H6, regtest stack (`bitcoind` in dev only).
- **Out of scope:** Payout; P2TR Admin ID; US-H4 fee UI; full WalletPanel; PRD §4.3.5 Send; US-H2 cancel; HWI; HW-signed commit; mainnet/testnet enablement in US-H7 (regtest only).
- **Discovery note:** US-H7 adds the first Admin Wallet spend via BDK + chain RPC, layered on the in-app commit/reveal (US-H6). See §6 *Addendum (broadcast drift)* for the Slice 0 reconciliation. Technical spec: [`docs/specs/admin-wallet-regtest-commit-funding.md`](../specs/admin-wallet-regtest-commit-funding.md).

### Activity I — Payout operations (Payout Admin swimlane)

#### US-I1 · List pending block_payout transactions
- **Story:** As a Payout Admin Signer, I want to see all pending `block_payout` transactions with expiry countdown, txid, and signature progress, so that I can coordinate payouts under review.
- **Classification:** Functional
- **Acceptance signals:** Lists all pending payouts; each shows time left before 7-day expiry, transaction ID, and `collected / required` approval signatures. Visible only to Payout Admin signers.
- **Source:** UI PRD §1.17, §1.17.1.
- **Slice:** 4.

#### US-I2 · List past block_payout transactions
- **Story:** As a Payout Admin Signer, I want to see all past `block_payout` transactions with confirmation status, block timestamp, and txid, so that I can audit payout history.
- **Classification:** Functional
- **Acceptance signals:** Lists past payouts; shows confirmation status (Unconfirmed / Confirmed), block timestamp, and txid.
- **Source:** UI PRD §1.18.
- **Slice:** 4.

#### US-I3 · Export raw pending block_payout transaction
- **Story:** As a Payout Admin Signer, I want to export a raw copy of any pending `block_payout` transaction, so that I can review or broadcast it using external tooling.
- **Classification:** Functional
- **Acceptance signals:** Export action produces a raw transaction artifact for the selected pending payout.
- **Source:** UI PRD §1.17.2.
- **Slice:** 4.

#### US-I4 · Sign a pending block_payout
- **Story:** As a Payout Admin Signer, I want to produce a spend signature for a pending `block_payout` transaction, so that it can reach quorum.
- **Classification:** Functional
- **Acceptance signals:** Spend signature produced for any pending payout.
- **Source:** UI PRD §1.17.3.
- **Slice:** 4.

#### US-I5 · Export collected spend signatures for a payout
- **Story:** As a Payout Admin Signer, I want to copy all spend signatures collected so far for a pending `block_payout`, so that I can aggregate or broadcast them offline.
- **Classification:** Functional
- **Acceptance signals:** Copy action exports all available spend signatures for the selected pending payout.
- **Source:** UI PRD §1.17.3.1.
- **Slice:** 4.

#### US-I6 · Broadcast a signed block_payout
- **Story:** As a Payout Admin Signer, I want to paste the quorum of spend signatures and broadcast the signed `block_payout`, so that funds are distributed on-chain.
- **Classification:** Functional
- **Acceptance signals:** Broadcast via app RPC **or** copy raw transaction to clipboard for external broadcast.
- **Source:** UI PRD §1.17.3.2.
- **Slice:** 4.

#### US-I7 · Offer auto-broadcast when my payout signature completes quorum
- **Story:** As a Payout Admin Signer whose signature completes quorum, I want to be offered the option to immediately broadcast the payout transaction, so that I can finalize in one step.
- **Classification:** Functional
- **Acceptance signals:** Option presented only to the quorum-completing signer; user can accept or decline.
- **Source:** UI PRD §1.17.3.3.
- **Slice:** 4.

#### US-I8 · Manually construct a block_payout transaction
- **Story:** As a Payout Admin Signer, I want to manually create a pending `block_payout` by providing the desired inputs and adding my signature, so that I can craft atypical payouts when the automatic flow does not fit.
- **Classification:** Functional
- **Acceptance signals:** User can provide `block_payout` inputs and attach their own signature to create a new pending payout.
- **Source:** UI PRD §1.19.
- **Slice:** 4.

#### US-I9 · Automatically construct a block_payout transaction
- **Story:** As a Payout Admin Signer, I want to click a "Block Payouts" button that automatically builds a `block_payout` transaction including as many unspent `block_payout` inputs as fit within Bitcoin standardness, so that I can consolidate routine payouts with one action.
- **Classification:** Functional
- **Acceptance signals:**
  - Automatic selection maximizes unspent `block_payout` inputs within standardness limits, accounting for signatures and fee/change.
  - User sees how many inputs are included.
  - If clicked before the most recent pending payout is confirmed, the new transaction must be identical to the most-recent pending transaction (idempotent replay).
- **Source:** UI PRD §1.20, §1.20.1–1.20.4.
- **Slice:** 4.

---

## 5. Dependencies & Risks That Affect Slicing

Surfaced from `2-discovery/`:

- **~~Alpen crate gap (blocks Slice 2)~~ — resolved.** When this was written, 8 of 13 update types had no representation in the upstream admin subprotocol crate. The pin bump to ASM `v0.1-alpha.11` closed that: every update type still in scope now exists upstream, and the two that do not (soft/hard bridge update) were withdrawn rather than added. `2-discovery/08-alpen-crate-prd-coverage.md` is the superseded snapshot; see [`specs/security-council.md`](../specs/security-council.md) and [ADR-007](../architecture/adrs/007-asm-pin-for-security-council.md).
- **HW wallet SPS-65 digest gap (affects all signing stories):** No consumer device natively signs a raw SPS-65 digest. POC-5 validated a synthetic PSBT binding on Trezor, but not yet against real on-chain ASM. This is a design constraint that could reshape US-F1/US-F2/US-I4 acceptance criteria.
- **Strata node RPC surface (affects Slice 1+):** No documented/verified RPC for reading current ASM state (signer sets, last_seqno, queued updates, confirmation_depth). Required for Activity D, authority access control, and fee/standardness validation.
- **Payout Admin architecture unknown (affects Slice 4):** Payout is not part of SPS-65 — it is a Bitcoin-native UTXO spend from a bridge multisig script. Script templates and spending conditions are not documented. Slice 4 may need its own mini-discovery.
- **~~Security Council and Alpen Admin role definitions~~ — resolved.** When this was written only Strata Admin and Sequencer Manager existed upstream. As of ASM `v0.1-alpha.11` the Security Council role and all four of its update types are defined and proven against a regtest ASM; Alpen Admin is implemented. Payout Administrator remains the one authority with no upstream role. See [`specs/security-council.md`](../specs/security-council.md).

---

## 6. Walking Skeleton — Acceptance of "it works"

Slice 0 is complete when a Strata Admin Signer can:
1. Launch the app (US-A1).
2. Connect a hardware wallet (or software key for dev mode) and obtain a signer identity (US-B1).
3. Select Strata Admin and authenticate (US-C1, US-C2).
4. See a pending proposal (US-D1).
5. Create a Strata Admin signer update proposal with their own signature (US-E1).
6. Approve a pending proposal by signing it (US-F1).
7. Once quorum is reached, export the raw approval transaction from the app and broadcast it externally (US-H1).

No in-app broadcast, no fee control, no other authorities, no cancellation, no payout — all deferred to later slices.

**Addendum (broadcast drift):** In-app commit/reveal broadcast (US-H6, [`proposal-broadcast-commit-reveal.md`](../specs/proposal-broadcast-commit-reveal.md)) is implemented on desktop. Slice 0 step 7 (export-only US-H1) remains valid as manual fallback but is not the only path. Admin Wallet commit funding (US-H7) extends US-H6 on regtest via Slice 1a.

---

## 7. Out of Scope (for this map)

These items belong elsewhere and are not stories in this map:

- **Non-functional concerns** (reproducible builds, signed binaries, cross-platform packaging, persistence, HA, session bounds, coordination-only invariants): see [`non-functional-items.md`](./non-functional-items.md).
- **Audit** (explicitly out of the proposal's scope — `1-proposal/` §Out of Scope).
- **Technical design details** (tech stack, API contracts, data model, device-specific signing binding): belong to architecture + specs after this map is validated.
- **Implementation ordering within a slice:** this map says *what* ships together, not *in what code order* — that is the job of per-story specs and the handoff phase.

---

## Traceability

Each US cites its PRD source. The full extraction grids behind this map (47 raw UI seeds + 9 backend seeds + discovery shaping factors) are available on request — not committed here to keep the map minimal.

**Implementation audits:** PRD §3–4 lifecycle gap analysis lives in [`docs/assessment/audits/proposal_status_lifecycle_audit.md`](../assessment/audits/proposal_status_lifecycle_audit.md); delivery tracking in [`proposal-lifecycle-expiry-and-status-completion.md`](../specs/proposal-lifecycle-expiry-and-status-completion.md) and [US-EXP](../assessment/deferred-backlog.md) in the deferred backlog.
