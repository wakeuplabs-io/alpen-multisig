# Spec: Admin Wallet — Send BTC (happy path) — Phase 6

**PRD:** [`03-prd-update.md`](../0-prd/03-prd-update.md) §4.3.5 — *"Send BTC"* (Send-to, Amount, Fee rate, Change routing, Confirm gate).
**Plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) Phase 6 (Send BTC happy path).
**Compliance:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) §4.3.5 — **FAIL → PASS (regtest / dev mnemonic)** on completion of this phase; HW path stays **FAIL** until Phase 8.
**Status:** Planned — roadmap (this document). Detailed slice specs + TDD are written when each increment is picked up.

---

## 1. Objective

Let the signer **spend** from the Admin Wallet: enter a destination, an amount, and a fee rate; confirm; and broadcast a signed Bitcoin transaction — on **regtest with the dev mnemonic login**. This closes PRD §4.3.5 for the regtest/testnet dev-mnemonic path and establishes the Send pipeline that Phase 8 (HW-on-device signing) and Phase 9 (shared Send chrome) extend.

This is the first phase that **moves wallet funds on user instruction**. Every other wallet surface so far has been read-only (balance, addresses, receive) or protocol-driven (governance commit/reveal). Send is signer-initiated value transfer, so the bar for validation clarity and an explicit confirm gate is higher.

**Why now:** the prerequisites already shipped —

- **Fee control** (`FeeRate`, `FeeEstimationService`, `FeeRateSelector`) from Phase 4.
- **Unified signing** (`PsbtSigner` port: mnemonic software signer = simulated HW; Ledger on-device) from R1.1.
- **Electrum-first broadcast** with node-RPC fallback (`TxBroadcaster`) from Phase 4 M3.
- **Change/receive index discipline** (`next_unused_address`) from R1.3.
- **Electrum wallet sync** with mempool visibility from R2.

Send is the composition of these into one user-facing flow; it introduces **no new protocol or custody primitive**.

---

## 2. Scope

**In scope (this phase)**

- A **Send** flow reachable from the Admin Wallet slide-over: destination address, amount, fee rate, Confirm.
- Backend send path on `WalletService`: build a PSBT (recipient + amount, change to the first unused **internal** index), sign via the session `PsbtSigner` (mnemonic software signer on regtest/testnet), broadcast Electrum-first with node-RPC fallback, return the txid.
- Destination validation: standard address types accepted; network-mismatch and non-address rejected with the exact PRD copy (§4.3.5.1).
- Amount validation: `amount ≤ balance − fee`, **Max** button, "Insufficient funds" (§4.3.5.2).
- Fee rate: reuse the Phase 4 `FeeRateSelector` — manual sat/vB, **0.1** step, max **10 000**, default = node **next-block** rate (§4.3.5.3).
- Change routing to the first unused change-index address `…/1/*` (§4.3.5.4).
- A **Confirm** gate disabled until all fields are valid; on confirm → sign → broadcast → show txid; on signer rejection/failure nothing is broadcast and the form is retained for retry or back-out (§4.3.5.5 / §4.3.5.5.1, with the mnemonic signer standing in for the hardware-confirm step).

**Out of scope (deferred — thin touchpoints only if an increment naturally needs them)**

- **Hardware-wallet on-device confirm/sign** for Send → **Phase 8**. This phase exercises the *exact same `PsbtSigner` port*; only the implementor differs, so Phase 8 is an implementor swap, not a flow change. Watch-only / HW sessions show Send **disabled** ("Hardware wallet required to sign", the 3.8 pattern) — visible but inoperable.
- **Mainnet** Send → **Phase 10** (`MnemonicPsbtSigner` is rejected on mainnet by `allowed_on(network)`, R1.1).
- **Shared Send chrome** unifying wallet Send with governance broadcast (Alta S9/S11, PRD §5.3.2.3) → **Phase 9**. Phase 6 builds a focused wallet-Send surface; Phase 9 extracts the shared component.
- **Receive QR / verify-on-device** (§4.3.4 remainder) → Phase 7 / 8 — unrelated to Send.
- Coin-control / manual UTXO selection, RBF opt-in on the send form (post-broadcast bump already exists via Phase 5), multi-recipient sends, address book, fiat conversion — none are PRD §4.3.5 requirements.

---

## 3. PRD traceability per increment

Each increment is small and shippable on regtest with the dev mnemonic; together they cover every MUST in PRD §4.3.5. Order de-risks the pipeline first (P6.1), then layers validation, then closes the confirm-gate contract.

| Increment | Delivers | PRD §4.3.5 coverage |
|---|---|---|
| **P6.1 — Send pipeline walking skeleton** | End-to-end build → sign → broadcast with change routing and a minimal Confirm; returns a txid on regtest | §4.3.5.4 (change → first unused change index); §4.3.5.5 + §4.3.5.5.1 (Confirm → sign → broadcast → txid; mnemonic signer as simulated HW); §4.3.5.3 *(thin — reuses default fee, full fee contract in P6.3)* |
| **P6.2 — Destination validation** | Standard-address acceptance; network and non-address rejection with exact PRD copy; Confirm blocked while invalid | §4.3.5.1 (full) |
| **P6.3 — Amount + fee contract + Max** | `amount ≤ balance − fee`, "Insufficient funds", **Max** button; fee default = node next-block, 0.1 step, max 10 000; recompute on fee change | §4.3.5.2 (full); §4.3.5.3 (full) |
| **P6.4 — Confirm gate + result / reject-retry surfaces** | Confirm enabled only when all fields valid; success (txid) surface; signer-reject / broadcast-failure leaves form intact for retry or back-out | §4.3.5.5 (full); §4.3.5.5.1 (full for the dev-mnemonic path; HW on-device → Phase 8) |

**End state:** PRD §4.3.5 → **PASS (regtest / dev mnemonic)** in the compliance matrix; §4.3.5.5.1 HW on-device confirm stays **FAIL** (Phase 8); mainnet stays gated (Phase 10).

---

## 4. User journey (intent)

1. From the Admin Wallet slide-over the signer opens **Send** (a section/screen alongside Balance, Addresses, Receive, Pending transactions).
2. The signer enters a **destination address**. Invalid or wrong-network input shows the PRD error inline and keeps Confirm disabled.
3. The signer enters an **amount** (or clicks **Max**). Over-balance input shows "Insufficient funds".
4. The signer reviews/sets the **fee rate** (defaulted to the node next-block rate). Changing it recomputes the Max/insufficient-funds boundary.
5. With all fields valid, **Confirm** enables. The signer confirms; the transaction is built, signed by the session signer, and broadcast.
6. On success the signer sees the **transaction ID**. On rejection/failure nothing is broadcast and the form stays as-is for retry or back-out.

For a **watch-only / HW session**, Send is visible but disabled with "Hardware wallet required to sign" — the on-device flow lands in Phase 8.

---

## 5. Architecture fit (intent, not a technical breakdown)

- **Pipeline:** Send composes existing pieces — PSBT build (BDK, from the session wallet descriptor) → `PsbtSigner` (R1.1) → `TxBroadcaster` Electrum-first/node-fallback (Phase 4 M3) → txid. The amount/fee/change semantics mirror the commit-funding path (`build_signed_commit`) but with a user-supplied recipient and change drained to `next_unused_address(Internal)`.
- **Secrets stay in Rust.** React submits destination/amount/fee over IPC and renders validation, the confirm gate, and the result. No key material crosses IPC (unchanged custody surface).
- **Session model:** Send is gated by the session signer exactly like broadcast — `signer present?` else `ReadOnly`; `signer.allowed_on(network)?` else `SignerNotAllowedOnNetwork` (mainnet mnemonic blocked). No new session state.
- **Fee reuse:** the Send form mounts the existing `fee-selection/` `FeeRateSelector`; "next block" default comes from `FeeEstimationService`. No new fee infrastructure.
- **Validation authority:** the backend is authoritative (address parse against the wallet network, fee/amount feasibility from real UTXOs); the frontend mirrors the rules for inline, pre-submit feedback. The two must not diverge in error semantics.
- **Panel placement:** Send slots into the existing slide-over content model next to the Phase 5 "Pending transactions" section; the shared-component extraction (governance broadcast ↔ wallet Send) is intentionally **deferred to Phase 9** so Phase 6 stays a focused vertical slice.

**Primary code areas (orienting, not prescriptive):** `application/wallet_service.rs` (send build/sign/broadcast), `commands/admin_wallet.rs` (Send IPC + capability guard), `domain/admin-wallet/` (Send form, validation hooks, result surface), `domain/fee-selection/` (reused selector).

---

## 6. Risks / notes

- **First fund-moving surface:** validation correctness and the disabled-by-default Confirm gate are safety-critical. Backend and frontend validation must agree; the backend rejects independently of the UI.
- **Max vs fee coupling:** the maximum spendable depends on the selected fee rate and the resulting tx vsize; Max and the "Insufficient funds" boundary must recompute when the fee changes (handled in P6.3).
- **Change-index discipline:** change MUST go to the first unused internal index (§4.3.5.4); reuse the R1.3 gap-aware derivation rather than a fresh window scan.
- **Reject semantics:** for the dev-mnemonic signer there is no physical reject; the flow simulates "nothing broadcast, form retained" so the Phase 8 HW reject path drops in without UI change.
- **Standard vs consensus-valid address types:** §4.3.5.1 lists P2PK/P2PKH/P2SH/P2WPKH/P2WSH/P2TR; rely on the Bitcoin address parser for network/standardness rather than hand-rolled checks.
- **No protocol change:** Send touches no SPS-50/65 envelope and no commit/reveal semantics; it is wallet-local value transfer only.

---

## 7. Done when (phase)

- On regtest with the dev mnemonic login, a signer can send BTC to a valid destination, set a fee rate, confirm, and receive a txid; change lands on the first unused internal index.
- Every PRD §4.3.5 MUST for the regtest/dev-mnemonic path is satisfied (validation copy, Max, fee bounds, Confirm gate, txid/reject behavior).
- Watch-only / HW sessions see Send disabled with the "Hardware wallet required to sign" message (no broadcast possible).
- §4.3.5 marked **PASS (regtest / dev mnemonic)** in the compliance matrix; HW on-device confirm and mainnet remain explicitly deferred.
- `cargo test --workspace` and frontend CI green; manual regtest playbook: fund the Admin Wallet, send to a regtest address, verify the txid and change output on-chain.
