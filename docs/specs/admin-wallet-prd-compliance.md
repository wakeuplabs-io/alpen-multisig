# Admin Wallet program — PRD compliance matrix

**PRD source:** [`docs/0-prd/06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) (current snapshot; rows below still carry the §-IDs of `03-prd-update.md`, which the 06 renumbering does not move for this program)  
**Program plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md)  
**Last updated:** 2026-08-14 (G7 — the Admin ID is a bitcoin address again, per PRD 06 §3.b.ii.2; spec [`admin-id-as-bitcoin-address.md`](./admin-id-as-bitcoin-address.md). Certificate → G8, device QA and the §4.1/§4.2 flip → G9)

This matrix is the **single place** to record PASS / FAIL / N/A for PRD requirements touched by the Admin Wallet program. Phase ✅ markers in the implementation plan mean **engineering slices shipped**, not automatic PRD PASS for whole sections.

## Status legend

| Status | Meaning |
|--------|---------|
| **PASS** | Requirement met for the program scope (see Notes). |
| **FAIL** | Not implemented; no accepted product interpretation. |
| **PARTIAL** | Some bullets met; others open (listed in Notes). |
| **N/A** | Out of Admin Wallet program scope (see plan §1 / §8) or different product line (e.g. Payout §6). |
| **DEFER** | Planned in a numbered phase (4–10), Release 2, or outside this program. |

## Program scope vs full PRD

| Topic | Admin Wallet plan | Full PRD |
|-------|-------------------|----------|
| Multisigs | Strata Administrator, Alpen Administrator only | Five multisigs (§3.1) |
| Payout (§6) | Excluded | Required for Payout signers |
| HWI | Excluded; direct Trezor/Ledger adapters | HWI device list (§3.2) |
| Connect UX | Canonical paths (R1.4), no address picker | List of addresses to choose (§3.2.1) — **documented deviation** |
| Dev mnemonic login | Regtest/testnet only (`MnemonicPsbtSigner`) | Production: HW for custody (§3.2, §4.3.5) |
| Wallet sync backend | **Electrum** (R2) | PRD §2 implies viable remote access |

## Wallet balance UX convention (§4.3.1 / §4.3.2)

PRD text uses *"net of unconfirmed"* for displayed balances. R1.5 / R1.6 ([`admin-wallet-balance-ux.md`](./admin-wallet-balance-ux.md), [`admin-wallet-addresses-ux.md`](./admin-wallet-addresses-ux.md)) **PASS** by showing:

- **Hero:** confirmed sats only.
- **Sub-line (when non-zero):** signed `±N sats unconfirmed`.

That pair is the agreed encoding of “total net” + “unconfirmed net visible separately.” It is **not** a single combined hero number.

---

## Requirement matrix

### PRD §2 — Node / RPC (product-wide)

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 2.1 | Trusted or custom RPC URL | **PASS** | `node_config_store.rs`, Node Config UI | Trusted + custom modes (chain RPC today) |
| 2.1 (Electrum) | Trusted or custom Electrum URL | **PASS** | `node_config_store.rs` `electrum_url()`, Node Config UI | R2.3 — Local / Trusted / Custom |
| 2.2 | Default = local node; prompt if missing | **PASS** | `ConnectionMode::Local` default | Aligns with PRD today |
| 2.3 | Strata node → BTC/Strata access without extra setup | **DEFER** | — | Broader than Admin Wallet slice |
| 2.2 (end state) | No local node as product assumption | **PARTIAL** | R2.3 ✅ + Phase 10 | R2.3 done (Electrum URL in Node Config); Phase 10: remote chain RPC presets |
| Wallet sync viable on remote/testnet | Production-viable wallet indexation | **PASS** | `bdk_electrum` sync (R2.2) | Electrum-backed; production-viable on remote testnet/mainnet |

### PRD §3.2 — Connect HW, Admin ID, Admin Wallet

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 3.2.1 | HW via HWI feature set | **FAIL** | Plan §8 excludes HWI | Direct adapters Phase 8; not HWI parity |
| 3.2.1 | User picks from address list | **FAIL** | R1.4 canonical paths | Intentional UX change; see [`admin-wallet-canonical-connect-paths.md`](./admin-wallet-canonical-connect-paths.md) |
| 3.2.1.2 | Admin ID `m/84'/0'/73'/0/0` (non-Payout) | **PASS** | `trezor-adapter.ts`, `ledger-adapter.ts` | Ledger testnet uses `m/84'/1'/73'/0/0` (documented app convention) |
| 3.b.ii.1 (PRD 06) | Admin ID P2TR `m/86'/0'/73'/0/0` for **Payout Administrator** | **DEFER** | — | Not implemented: the path is BIP-84 for every authority. Deferred with the rest of the Payout Administrator scope; destination is the `block_payouts` program |
| 3.2.1.3 | Admin Wallet `m/86'/0'/73'/n/n` | **PASS** | BDK descriptors, session init | External `0/*`, change `1/*` |
| 3.2.3 | Nonce signed with Admin ID | **PASS** | Orchestrator auth flow | — |
| 3.2.4 | Readable messages on HW screen | **PARTIAL** | Message signing on connect | Governance PSBT preview not full §3.2.4 audit |
| 3.2 (prod) | Custody from HW only | **FAIL** | `MnemonicPsbtSigner`, Palabras login | Regtest/testnet dev path; mainnet blocked for mnemonic signer (R1.1) |

### PRD §4.1–4.2 — Admin ID in UI (after login)

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 4.1 | See Admin ID, copy to clipboard | **PASS** | G7 — `AdminIdRow`, `ConnectAdminIdCard`; [`admin-id-as-bitcoin-address.md`](./admin-id-as-bitcoin-address.md) | Admin ID = **P2WPKH bitcoin address** (PRD 06 §3.b.ii.2). The compressed-public-key rendering (#408, PR #444) is reverted: the PRD never stopped saying "address" and the subprotocol maintainer ruled on 2026-08-07. Shown in full + copy at the top of the wallet panel and on the multisig-selection step (#410), once per surface (#413); auth-only caption; **still no QR on the Admin ID** — it must never receive funds, which an address makes more dangerous, not less |
| 4.2 | View Admin ID on HW to verify | **PARTIAL** | G7 — `VerifyOnDeviceButton` fed the Admin ID itself (`AdminIdRow`) | The indirection is gone: with the Admin ID being the address, the device renders the Admin ID **itself**, and the app checks the returned string matches (mismatch = security alarm). This is what answers #409 — no supported signer can display a raw compressed public key, but all of them display an address. Still **PARTIAL** pending device QA on Trezor and Ledger/Speculos → **G9**, which also measures whether a Ledger renders the message text or its SHA-256 hash (§3.2.4 / #402) |

### PRD §4.3 — Admin Wallet management

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 4.3.1 | Wallet total net + unconfirmed net visible | **PASS** | R1.5, `WalletBalance`, `do_sync` mempool | UX convention above; sync backend → R2 |
| 4.3.2 | Each funded address + per-address net | **PASS** | R1.6, `compose-addresses-with-balance.ts` | External indices with balance > 0 only; change with funds not listed (Phase 2 policy) |
| 4.3.3 | Unconfirmed tx list + fee bump | **PARTIAL** | Phase 5 — `wallet_transactions.rs`, `admin_wallet_list_unconfirmed_txs` / `admin_wallet_bump_fee` IPC, `UnconfirmedTxsList` panel section; [`admin-wallet-transactions-fee-bump.md`](./admin-wallet-transactions-fee-bump.md); E2E spec: `desktop-app/e2e-webdriver/test/specs/fee-bump.e2e.js` | Unconfirmed **sent** txs listed with fee/rate. Plain sends bump via **RBF** (BDK `build_fee_bump`); governance commits with a pending pre-signed reveal bump via **CPFP** — a child on the reveal's change lifts the commit+reveal package rate (RBF would invalidate the reveal, R1.0.1). Both sign via session `PsbtSigner`, Electrum-first broadcast. Watch-only sessions see the list; Bump disabled. **Preconditions for PASS:** (1) F-001 persistence fix merged ✅, (2) WebDriver E2E for fee-bump flow (spec created; run `cd desktop-app/e2e-webdriver && npm run test:e2e:fee-bump`), (3) HW signing path for bump (Phase 8). Watch-only cannot bump with HW signing; Trezor Admin Wallet PSBT signing not implemented. |
| 4.3.4.1 | First unused receive address (text) | **PASS** | R1.3, `ReceiveAddressRow` | — |
| 4.3.4.1 | Receive address in QR | **PASS** | Phase 7 — `ReceiveAddressRow` + `QrCode` (`qrcode.react`); [`admin-wallet-admin-id-and-receive-qr.md`](./admin-wallet-admin-id-and-receive-qr.md) | Bare-address payload (not BIP-21); pinned by `build-receive-qr-value` test |
| 4.3.4.1 | Copy via text or QR click | **PASS** | Phase 7 — shared `useClipboardCopy`; address text + QR + icon all copy | — |
| 4.3.4.2 | Verify receive address on HW | **FAIL** | — | Phase 8 (HW device adapters) |
| 4.3.4.3 | Rotate after credit (one-time use) | **PASS** | R1.3, `next_receive_address` | BDK “used” on observe-in-tx |
| 4.3.5 | Send BTC form + validations | **PASS** | Phase 6 — PRs [#289](https://github.com/wakeuplabs-io/alpen-multisig/pull/289), [#292](https://github.com/wakeuplabs-io/alpen-multisig/pull/292), [#293](https://github.com/wakeuplabs-io/alpen-multisig/pull/293), [#294](https://github.com/wakeuplabs-io/alpen-multisig/pull/294); specs [`admin-wallet-send-btc.md`](./admin-wallet-send-btc.md) + [`admin-wallet-send-btc-implementation.md`](./admin-wallet-send-btc-implementation.md) | **Regtest/testnet dev-mnemonic path.** HW on-device confirm → Phase 8; mainnet → Phase 10 |
| 4.3.5.1 | Destination validation (standard types, network mismatch copy) | **PASS** | P6.2 (PR #292) — `admin_wallet_validate_send_address`, `format-send-error.ts` | Exact PRD copy byte-tested; backend-authoritative parse; debounced inline |
| 4.3.5.2 | Amount valid / Max / "Insufficient funds" | **PASS** | P6.3 (PR #293) — `estimate_send` dry-run, Max = drain build | Boundary surfaced pre-Confirm; Max recomputes with fee rate |
| 4.3.5.3 | Manual fee rate (0.1 s/vB, default next-block) | **PASS** | P6.3 (PR #293) — `SendFeeRateControl`, presets default **Fast** (1 block) | 0.1 step, max 10 000, bounds from DTO; custom entry |
| 4.3.5.4 | Change → first unused change index | **PASS** | P6.1 (PR #289) — BDK first-unused internal; pinned by `send_change_goes_to_first_unused_internal_index` | Max sends are drain builds (no change) |
| 4.3.5.5 | Confirm disabled until all fields valid | **PASS** | P6.3/P6.4 — `canConfirmSend` predicate (destination validated ∧ amount ∧ fee ∧ estimate ∧ ¬submitting) | Truth-table tested; architecture Rule 8 guards the wiring |
| 4.3.5.5.1 | Confirm → sign → broadcast → txid; reject → no-op | **PARTIAL** | P6.1/P6.4 — `PsbtSigner` port; `SignFailed` before any broadcast; form retained on failure | Dev-mnemonic path PASS (simulated HW); real on-device confirm/reject → Phase 8 |

### PRD §5.3 — Governance broadcast (Alpen / Strata admins)

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 5.3.3.2.3 | Quorum “Send” UX like wallet Send | **PARTIAL** | Broadcast screens exist | Wallet Send §4.3.5 not built; Phase 9 shared UX |
| US-H4 | Manual sat/vB on governance broadcast (0.1 steps, max 10 000; default **Medium** preset) | **PASS** | Phase 4, `FeeRateSelector`, `fee_rates_estimate` IPC | M1+M2+M3 complete; presets (Slow/Medium/Fast) + Custom; Electrum-first broadcast with node fallback. Wallet Send §4.3.5.3 uses **Fast** default — see [`admin-wallet-send-btc-implementation.md`](./admin-wallet-send-btc-implementation.md) D4 |
| 5.3 (fees) | Pending-update Send fee via wallet-send pattern (§4.3.5.3) | **DEFER** | Phase 6 / Phase 9 shared Send | — |
| Broadcast commit | Funded from Admin Wallet | **PASS** | Phase 3.6+, `WalletService` | — |
| Broadcast commit sign | HW or regtest mnemonic PSBT | **PASS** | R1.1 `PsbtSigner` | Reveal: ephemeral in-app (SPS-50), not HW — protocol constraint |
| Broadcast reveal | Signed and broadcast | **PASS** | R1.0, R1.0.1 | Ephemeral key; change → Admin Wallet |

### PRD §6 — Payout Administrator

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| §6 (all) | Payout flows | **N/A** | Plan §8; separate UI may exist | Admin Wallet program excludes Payout |
| 6.x fees | Fees from Admin Wallet | **N/A** | — | Applies when Payout program starts; path collision noted in plan §7 |

### SPS-50 / governance protocol (referenced by plan)

| Topic | Status | Notes |
|-------|--------|-------|
| Commit funding from Admin Wallet | **PASS** | US-H7, Phase 1+ |
| Reveal internal key | **PASS** | Ephemeral per broadcast (R1.0); **not** `m/86'/0'/73'/2/0` (retired 3.5/3.7b) |
| Pre-sign commit + reveal | **PASS** | R1.0.1 |

---

## Release 1 vs PRD §4.3

| PRD subsection | Release 1 slice | Matrix status |
|----------------|-----------------|---------------|
| §4.3.1 | R1.5 | **PASS** |
| §4.3.2 | R1.6 | **PASS** |
| §4.3.4 | R1.3 (rotation) + Phase 7 (QR + click-to-copy) | **PARTIAL** (QR ✅ Phase 7; HW verify **FAIL** → Phase 8) |
| Wallet panel UI | R1.7 | **PASS** |
| §4.3.3 | Phase 5 | **PARTIAL** (F-001 persistence ✅; E2E spec created; HW bump Phase 8) |
| §4.3.5 | Phase 6 (PRs #289/#292/#293/[#294](https://github.com/wakeuplabs-io/alpen-multisig/pull/294)) | **PASS** (regtest / dev mnemonic) ✅ — HW on-device Phase 8 |
| US-H4 broadcast fee | Phase 4 | **PASS** ✅ |

**Release 1:** R1.0–R1.7 done. **Release 2:** R2.1–R2.3 done ✅. **Phase 4:** done ✅. **Phase 5:** done ✅. **Phase 6:** done ✅ (Send — regtest/dev mnemonic). **Phase 7:** done ✅ (Admin ID display/copy + receive QR & click-to-copy). **Next:** Phase 8 (HW direct adapters — Send-on-HW + verify-on-device, incl. §4.2 / §4.3.4.2).

---

## Release 2 — Electrum wallet sync ✅

| ID | Requirement (summary) | Status | Evidence / Notes |
|----|------------------------|--------|------------------|
| R2 | Electrum-backed wallet sync; production-viable indexation | **PASS** | PR [#261](https://github.com/wakeuplabs-io/alpen-multisig/pull/261), [#262](https://github.com/wakeuplabs-io/alpen-multisig/pull/262), [#263](https://github.com/wakeuplabs-io/alpen-multisig/pull/263) |
| R2.1 | electrs infra — Docker, dev/staging/CI, smoke vs local `bitcoind` | **PASS** | `staging/docker-compose.yml`, `staging/docker-compose.local.yml` |
| R2.2 | `WalletService` sync via `bdk_electrum`; broadcast/fees unchanged | **PASS** | `wallet_service.rs` uses `bdk_electrum::BdkElectrumClient` |
| R2.3 | Electrum URL in Node Config (Local / Trusted / Custom) | **PASS** | `node_config_store.rs` `electrum_url()`, `custom_electrum_url` |

---

## Historical doc drift (resolved in plan)

The implementation plan previously stated post-Foundation reveal key at `m/86'/0'/73'/2/0`. **Current behavior:** per-broadcast ephemeral reveal key (R1.0). See §5 baseline table in the updated plan.

The plan previously excluded Electrum and deferred indexer backends to a separate program. **Current plan:** Electrum is **in scope** as Release 2.

Legacy feature roadmaps under `docs/archive/features/admin-wallet-*` may still mention `COMMIT_FUNDING` or `ADMIN_WALLET_REGTEST_MNEMONIC`; those env vars were removed in Phase 3.6 / 3.7c. Treat this matrix + implementation plan as authoritative for compliance status.

---

## How to update

1. When a phase ships, update the row(s) and add PR / commit in Evidence.
2. Do not mark a whole PRD section PASS unless every MUST in that section is PASS or explicitly N/A for this program.
3. Link new slice specs from the implementation plan §2 traceability table with accurate PRD subsection IDs (not §4.1–4.2 for wallet read path).
