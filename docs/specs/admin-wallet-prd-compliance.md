# Admin Wallet program — PRD compliance matrix

**PRD source:** [`docs/0-prd/03-prd-update.md`](../0-prd/03-prd-update.md)  
**Program plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md)  
**Last updated:** 2026-06-03 (after Release 1 / R1.6, PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212))

This matrix is the **single place** to record PASS / FAIL / N/A for PRD requirements touched by the Admin Wallet program. Phase ✅ markers in the implementation plan mean **engineering slices shipped**, not automatic PRD PASS for whole sections.

## Status legend

| Status | Meaning |
|--------|---------|
| **PASS** | Requirement met for the program scope (see Notes). |
| **FAIL** | Not implemented; no accepted product interpretation. |
| **PARTIAL** | Some bullets met; others open (listed in Notes). |
| **N/A** | Out of Admin Wallet program scope (see plan §1 / §8) or different product line (e.g. Payout §6). |
| **DEFER** | Planned in a numbered phase (4–9) or outside this program. |

## Program scope vs full PRD

| Topic | Admin Wallet plan | Full PRD |
|-------|-------------------|----------|
| Multisigs | Strata Administrator, Alpen Administrator only | Five multisigs (§3.1) |
| Payout (§6) | Excluded | Required for Payout signers |
| HWI | Excluded; direct Trezor/Ledger adapters | HWI device list (§3.2) |
| Connect UX | Canonical paths (R1.4), no address picker | List of addresses to choose (§3.2.1) — **documented deviation** |
| Dev mnemonic login | Regtest/testnet only (`MnemonicPsbtSigner`) | Production: HW for custody (§3.2, §4.3.5) |

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
| 2.1 | Trusted or custom RPC URL | **PASS** | `node_config_store.rs`, Node Config UI | Trusted + custom modes |
| 2.2 | Default = local node; prompt if missing | **PASS** | `ConnectionMode::Local` default | Aligns with PRD today |
| 2.3 | Strata node → BTC/Strata access without extra setup | **DEFER** | — | Broader than Admin Wallet slice |
| 2.2 (end state) | No local node as product assumption | **DEFER** | Phase 9 | Plan end state ≠ current default |

### PRD §3.2 — Connect HW, Admin ID, Admin Wallet

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 3.2.1 | HW via HWI feature set | **FAIL** | Plan §8 excludes HWI | Direct adapters Phase 7; not HWI parity |
| 3.2.1 | User picks from address list | **FAIL** | R1.4 canonical paths | Intentional UX change; see [`admin-wallet-canonical-connect-paths.md`](./admin-wallet-canonical-connect-paths.md) |
| 3.2.1.2 | Admin ID `m/84'/0'/73'/0/0` (non-Payout) | **PASS** | `trezor-adapter.ts`, `ledger-adapter.ts` | Ledger testnet uses `m/84'/1'/73'/0/0` (documented app convention) |
| 3.2.1.3 | Admin Wallet `m/86'/0'/73'/n/n` | **PASS** | BDK descriptors, session init | External `0/*`, change `1/*` |
| 3.2.3 | Nonce signed with Admin ID | **PASS** | Orchestrator auth flow | — |
| 3.2.4 | Readable messages on HW screen | **PARTIAL** | Message signing on connect | Governance PSBT preview not full §3.2.4 audit |
| 3.2 (prod) | Custody from HW only | **FAIL** | `MnemonicPsbtSigner`, Palabras login | Regtest/testnet dev path; mainnet blocked for mnemonic signer (R1.1) |

### PRD §4.1–4.2 — Admin ID in UI (after login)

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 4.1 | See Admin ID, copy to clipboard | **FAIL** | — | Phase 7; today only truncated signer in `SessionChip` |
| 4.2 | View Admin ID on HW to verify | **FAIL** | — | Phase 7 |

### PRD §4.3 — Admin Wallet management

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 4.3.1 | Wallet total net + unconfirmed net visible | **PASS** | R1.5, `WalletBalance`, `do_sync` mempool | UX convention above |
| 4.3.2 | Each funded address + per-address net | **PASS** | R1.6, `compose-addresses-with-balance.ts` | External indices with balance > 0 only; change with funds not listed (Phase 2 policy) |
| 4.3.3 | Unconfirmed tx list + fee bump | **FAIL** | — | Phase 6 |
| 4.3.4.1 | First unused receive address (text) | **PASS** | R1.3, `ReceiveAddressRow` | — |
| 4.3.4.1 | Receive address in QR | **FAIL** | — | Phase 7 |
| 4.3.4.1 | Copy via text or QR click | **PARTIAL** | `CopyButton` on text | QR not shipped |
| 4.3.4.2 | Verify receive address on HW | **FAIL** | — | Phase 7 / 8 |
| 4.3.4.3 | Rotate after credit (one-time use) | **PASS** | R1.3, `next_receive_address` | BDK “used” on observe-in-tx |
| 4.3.5 | Send BTC form + validations | **FAIL** | — | Phase 5 (mnemonic regtest), Phase 8 (HW) |
| 4.3.5.3 | Manual fee rate (0.1 s/vB, default next-block) | **FAIL** | — | Phase 5 Send (reuse Phase 4 control) |
| 4.3.5.5.1 | Confirm → HW signs spend | **FAIL** | — | Phase 8 for product path |

### PRD §5.3 — Governance broadcast (Alpen / Strata admins)

| ID | Requirement (summary) | Status | Evidence / phase | Notes |
|----|------------------------|--------|------------------|-------|
| 5.3.3.2.3 | Quorum “Send” UX like wallet Send | **PARTIAL** | Broadcast screens exist | Wallet Send §4.3.5 not built; Phase 8 shared UX |
| US-H4 | Manual sat/vB on governance broadcast (0.1 steps, max 10 000; default from node) | **FAIL** | — | **Phase 4** (priority); [`02-prd-update-impact.md`](../1-proposal/02-prd-update-impact.md) |
| 5.3 (fees) | Pending-update Send fee via wallet-send pattern (§4.3.5.3) | **DEFER** | Phase 5 / Phase 9 shared Send | — |
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
| §4.3.4 | R1.3 (rotation only) | **PARTIAL** (QR + HW verify **FAIL**) |
| Wallet panel UI | R1.7 (planned) | **PLANNED** |
| §4.3.3, §4.3.5 | Not in Release 1 | **FAIL** (Phases 6, 5, 8) |
| US-H4 broadcast fee | Not in Release 1 | **FAIL** (**Phase 4** priority) |

**Release 1:** R1.0–R1.6 are done; **R1.7** (wallet panel UI polish) is the remaining Release 1 slice. It does **not** mean PRD §4.3 or §4 as a whole is PASS. **Suggested order:** R1.7 → Phase 4 (US-H4 broadcast fee).

---

## Historical doc drift (resolved in plan)

The implementation plan previously stated post-Foundation reveal key at `m/86'/0'/73'/2/0`. **Current behavior:** per-broadcast ephemeral reveal key (R1.0). See §5 baseline table in the updated plan.

Legacy feature roadmaps under `docs/feature/admin-wallet-*` may still mention `COMMIT_FUNDING` or `ADMIN_WALLET_REGTEST_MNEMONIC`; those env vars were removed in Phase 3.6 / 3.7c. Treat this matrix + implementation plan as authoritative for compliance status.

---

## Release 1 — R1.7 (planned — wallet UI only)

| ID | Requirement (summary) | Status | Notes |
|----|------------------------|--------|-------|
| R1.7 | Wallet slide-over UI polish (Alta parity: balance, receive, addresses, sync) | **PLANNED** | Extends R1.2–R1.6; no new PRD MUST until spec; see implementation plan |

---

## How to update

1. When a phase ships, update the row(s) and add PR / commit in Evidence.
2. Do not mark a whole PRD section PASS unless every MUST in that section is PASS or explicitly N/A for this program.
3. Link new slice specs from the implementation plan §2 traceability table with accurate PRD subsection IDs (not §4.1–4.2 for wallet read path).
