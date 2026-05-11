# PRD Update — Diff vs. `01-multisig-ui.md`

Comparison between the original PRD ([`docs/0-prd/01-multisig-ui.md`](../0-prd/01-multisig-ui.md)) and the PRD update ([`docs/0-prd/03-prd-update.md`](../0-prd/03-prd-update.md)).

**Status:** Reference — tracks the deltas the team needs to absorb. Not a re-write of the PRD itself; the canonical source remains `docs/0-prd/`.

---

## TL;DR

The update is **structurally a re-organisation plus three substantive functional additions**:

1. **New top-level structure** — flat requirement list is grouped into 6 numbered top-level sections.
2. **Flow re-ordering** — the user now selects the multisig **before** connecting a hardware wallet (was: connect address → pick multisig).
3. **New "Admin ID" + "Admin Wallet" model** — replaces the single signing address with a two-key model whose derivation depends on the selected multisig.
4. **New full wallet UI** (Section 4) — Admin ID display + Admin Wallet management (balance, addresses, txs, receive, send BTC with detailed validation).
5. **Payout Administrator updates** — explicit fee-rate UX, fee inputs sourced from Admin Wallet, change to first unused change address, standardness-limit error, raw tx **import** added.
6. **Minor renames** — "Operator update" → "Bridge Operator update", "Sequencer update" → "Sequencer key update".
7. **Scope trim** — the update document is PRD-only; backend design notes (Strata Multisig Backend — Design Guidelines) and the SPS-50/51/65 external copies that were embedded in `01-multisig-ui.md` are **not** carried into `03-prd-update.md`. The protocol specs remain authoritative in their own sources.

---

## 1. Structural changes

### 1.1 Sectioning

| `01-multisig-ui.md` | `03-prd-update.md` |
|---|---|
| Single flat list `1.` with sub-items `1.1 … 1.20` | Six top-level groups `1.` through `6.` |
| Backend notes, SPS-50, SPS-51, SPS-65 appended as "External copy" sections | Not included (PRD-only document) |

The new top-level grouping:

| New section | Topic | Origin in old PRD |
|---|---|---|
| 1 | Application install/run (OS, reproducible builds) | `1.1 – 1.4` |
| 2 | RPC connection (local node / `stratabtc.org`) | `1.5` |
| 3 | Multisig selection → HW wallet → Admin ID / Admin Wallet → nonce sign-in | `1.6 – 1.10` (reordered, expanded) |
| **4** | **Wallet UI — Admin ID display + Admin Wallet management** | **NEW** |
| 5 | Admin-update lifecycle (Approved/Pending/Past/Propose) | `1.11 – 1.15` |
| 6 | Payout Administrator (`block_payout` lifecycle) | `1.16 – 1.20` |

### 1.2 Flow re-ordering (Section 3)

**Old flow:** connect HW wallet → pick address (first 20 on `m/86'/0'/73'/0/n`) → select multisig → sign nonce.

**New flow:** **select multisig first** → connect HW wallet → derive **Admin ID** (key path depends on the multisig chosen) + **Admin Wallet** → sign nonce with Admin ID.

Implication: address derivation can no longer be done before the user picks an authority — the path depends on the selected multisig.

---

## 2. New: "Admin ID" and "Admin Wallet" model (Section 3.2)

The original PRD used a single concept ("connected address") with a single derivation path. The update splits this into two distinct keys with different roles.

### 2.1 Admin ID — authentication & message signing

The Admin ID is the user's identity for the backend session and the signer of admin-subprotocol messages or `block_payout` transactions. **Derivation depends on the selected multisig**:

| Multisig selected | Address type | Derivation | BIP |
|---|---|---|---|
| **Payout Administrator** | **P2TR** | `m/86'/0'/73'/0/0` | BIP-86 |
| All other multisigs (Alpen Admin, Strata Admin, Sequencer Mgr, Security Council) | **P2WPKH** | `m/84'/0'/73'/0/0` | BIP-84 |

Key constraints carried by the update:

- The Admin ID is used **for authenticating with the backend** and **signing admin subprotocol update-related messages**.
- For non-Payout multisigs the Admin ID **MUST NOT be used to sign any bitcoin transactions** (explicit prohibition).
- For the Payout Administrator the Admin ID **is** used to sign all Payout Administrator transactions.

### 2.2 Admin Wallet — BTC custody for the signer

A separate wallet for actually holding/spending BTC:

| Property | Value |
|---|---|
| Derivation | `m/86'/0'/73'/n/n` (BIP-86) |
| Account | hardened `73'` |
| Indexes | `change` and `address` indexes both use `n` |

In the **old PRD** the only derivation mentioned was `m/86'/0'/73'/0/n` (first 20 addresses). The new derivation `m/86'/0'/73'/n/n` is a full HD wallet (change + address indexes), which is what enables the new wallet-management UI in Section 4.

### 2.3 Nonce sign-in

Old: "sign a nonce with the private key of your connected address."
New: "sign a nonce with the private key of your **Admin ID**" (and only that Admin ID is checked against the canonical signer set).

---

## 3. New: Section 4 — Wallet UI

This section did not exist in the original PRD. It introduces a wallet-management screen with the following sub-requirements:

| Sub-req | Requirement |
|---|---|
| 4.1 | Display the Admin ID; copy to clipboard |
| 4.2 | "View on hardware-wallet screen" button to verify the Admin ID on-device |
| 4.3.1 | **Balance:** total BTC balance net of unconfirmed; plus the net of unconfirmed receives/sends |
| 4.3.2 | **Addresses:** list each address that holds a balance with its current (net-of-unconfirmed) balance |
| 4.3.3 | **Transactions:** list each unconfirmed outgoing tx with a **fee-bump** action |
| 4.3.4 | **Receive:** show first unused receive address (text + QR); click to copy; verify on device; auto-rotate to a new address once funds arrive ("one-time use") |
| 4.3.5 | **Send BTC:** full send flow with explicit validation rules (see below) |

### 3.1 Send BTC — explicit error/validation rules

The update enumerates each user-error path with the exact UI text expected:

| Condition | Behaviour |
|---|---|
| Destination is not a standard / consensus-valid output type | Critical error: `Destination must be a bitcoin address.` |
| Destination is on the wrong network (e.g. testnet vs mainnet) | Critical error: `Destination must be a [mainnet/testnet] bitcoin address.` |
| `send amount + mining fee > wallet balance` | Critical error: `Insufficient funds` |
| Valid amount definition | `amount ≤ wallet balance − (fee rate s/vB × tx size vB)` |
| "Max" button | Auto-fills the maximum valid amount |
| Fee rate | Manual `s/vB`, increments of `0.1 s/vB`, max `10,000 s/vB`. Default = "next block" rate from the connected Bitcoin Core node |
| Change output | Goes to the first unused address in the **change index** of the Admin Wallet |
| Confirm button | Disabled until all fields validated; click triggers HW-wallet signing; on confirm → broadcast + show txid; on reject → no-op |

None of this validation surface existed in the original PRD.

---

## 4. Section 5 — Admin updates: deltas

The lifecycle structure (Approved → Pending → Past → Propose) is unchanged, but the **propose-update list has been renamed** and the **"Send" button UX has been delegated to the new wallet flow**.

### 4.1 Propose-update name changes

| Multisig | Old PRD (`01-multisig-ui.md`) | New PRD (`03-prd-update.md`) |
|---|---|---|
| Strata Administrator | **Operator update** | **Bridge Operator update** |
| Strata Sequencer Manager | **Sequencer update** | **Sequencer key update** |

All other update types (Alpen verification key, Alpen Admin Signer, Safe Harbor address, Strata verification key, Strata Admin Signer, Security Council Signer, "Soft" bridge update, "Hard" bridge update, Strata Sequencer Manager Signer update, Defcon 1, Defcon 3) are unchanged.

### 4.2 Pending-update "Send" button UX

| Aspect | Old PRD | New PRD |
|---|---|---|
| Where fee-rate UX lives | Inline in `13.2.3.1`: "manually set the sat/vB fee rate in increments of 0.1 sat/vB using an amount entry field" | Delegated: "UI/UX similar to the 'send' screen in the 'wallet' section" |

The new PRD pushes fee-rate UX into the shared wallet send pattern (Section 4) rather than re-specifying it per flow. Functionally this is a tightening, not a loosening — the wallet send flow has stricter, fully-specified validation.

### 4.3 Approval flow

Old PRD `5.3.2.2`:
> create an approval transaction for a given "Pending" update, paste in the quorum of signatures required to approve the update, and broadcast …

New PRD `5.3.2.2`:
> same wording, plus: "This flow SHOULD have a UI/UX similar to the 'send' screen in the 'wallet' section."

Same "delegate to wallet send UX" pattern as the Send button.

---

## 5. Section 6 — Payout Administrator: deltas

### 5.1 Pending `block_payout` — import added

| Old PRD | New PRD |
|---|---|
| "The user MUST be able to **export** a raw copy of any 'Pending' `block_payout` transaction." | "The user MUST be able to **import and export** a raw copy of any 'Pending' `block_payout` transaction." |

This adds an **import** path — a signer can paste a `block_payout` they received out-of-band and continue the coordination flow.

### 5.2 Spend-signature paste — wording

| Old PRD | New PRD |
|---|---|
| "paste in the **quorum of** signatures required to approve a given `block_payout`" | "paste in the **signatures** required to approve a given `block_payout`" |

The word "quorum of" was removed. The new wording is slightly more permissive (a signer can paste any signatures they have, the protocol still enforces the threshold).

### 5.3 Send button — fee UX

| Old PRD `17.3.3.1` | New PRD `6.2.3.3.1` |
|---|---|
| "Send button … specify the sat/vB fee rate in increments of 0.1 sat/vB using an amount entry field then broadcast" | "Send button … broadcasts the transaction to be confirmed on bitcoin" (fee-rate UX delegated; details now in `6.4`) |

### 5.4 Manually-created Pending `block_payout` (NEW detail in 6.4)

The original PRD (`19`) was a single sentence:

> The user MUST be able to manually create a "Pending" `block_payout` transaction by providing `block_payout` inputs for the transaction then adding their signature to the transaction.

The update adds four new sub-requirements:

| New sub-req | Requirement |
|---|---|
| Fee rate | `s/vB`, increments of `0.1`, max `10,000 s/vB` |
| Fee input source | **MUST come from the user's connected Admin Wallet** |
| Change destination | First unused change address **in the Admin Wallet** |
| Standardness limit | Critical error if the transaction exceeds Bitcoin Core standardness limits in the latest release |

This is the same fee-rate UX as Section 4 (wallet send) — the Admin Wallet is the funding source for mining fees on `block_payout` transactions.

### 5.5 "Block payouts" button — input accounting

| Old PRD `20.1` | New PRD `6.5.1` |
|---|---|
| "as many unspent `block_payout` inputs as will fit … accounting … for the signatures that need to be added to spend the inputs" | "as many unspent `block_payout` inputs as will fit … accounting … for the signatures that need to be added to spend the inputs **and the fee input(s) and change output**" |

The size calculation must now include the Admin Wallet fee inputs and change output.

| Old PRD `20.2` | New PRD `6.5.2` |
|---|---|
| "see how many **inputs** are included" | "see how many **`block_payout` inputs** are included" |

Clarifies the count is over `block_payout` inputs specifically (not the Admin Wallet fee inputs).

---

## 6. What was dropped from the document

These appendices were present in `01-multisig-ui.md` and **are not** in `03-prd-update.md`:

- `[External copy] Strata Multisig Backend — Design Guidelines & Architectural Notes` (Scope, Operational Assumptions, Authority Isolation, Auth/Session Model, Proposal Semantics, Safe-multisig deviation, Code Sketch, Storage)
- `[External copy] SPS-50: L1 transaction header and interpretation`
- `[External copy] SPS-51: Generic simple envelope format`
- `SPS-65: Strata administration subprotocol (Transaction processing subsection)`

These remain authoritative in their original sources. The update document is intentionally PRD-only.

---

## 7. Implementation-impact summary

| Area | Impact | Where the work lives |
|---|---|---|
| Login flow | Re-order screens: multisig selection moves before HW wallet connection | `desktop-app/src/screens/` |
| Wallet abstraction | New "Admin ID" key (P2TR or P2WPKH depending on authority) + new "Admin Wallet" (full BIP-86 HD wallet, `m/86'/0'/73'/n/n`) | `desktop-app/src/wallet/`, HW-wallet adapters |
| Wallet UI (Section 4) | New screen: balance, addresses, txs (with fee-bump), receive (with QR + rotate), send (with full validation) | `desktop-app/src/screens/wallet/` (new) |
| Fee-rate UX | Shared "send" component with explicit validation reused by: Wallet Send, Pending Update Send, `block_payout` create, `block_payout` Send | shared component |
| Update-type names | Rename **Operator update** → **Bridge Operator update**, **Sequencer update** → **Sequencer key update** in proposal UI + any DTOs | both frontend and backend |
| `block_payout` import | New "Import raw" affordance on Payout pending list | Payout screens, backend ingest endpoint |
| `block_payout` fee handling | Fee inputs always sourced from Admin Wallet; change to Admin Wallet change index; standardness-limit error | Payout flow, transaction builder |
| Backend coordination | No surface-level changes from the dropped backend notes — the deltas above are UI/UX; protocol/validity rules remain onchain (see workspace rule: "Backend is coordination only") | `orchestrator-be/` |

---

## 8. Open questions for the team

These are points where the update wording is ambiguous enough that we should confirm before implementing:

1. **Admin ID network parity** — the BIP-84 path for non-Payout multisigs (`m/84'/0'/73'/0/0`) returns a P2WPKH address. Does the canonical signer set list these as P2WPKH addresses, or as raw public keys? The PRD says the address is used "for authenticating with the multisig app backend" — needs alignment with the backend auth design (`docs/2-discovery/13-authority-verification-findings.md`).
2. **Admin Wallet seed sharing** — Admin ID and Admin Wallet derive from the **same** hardware wallet seed (account `73'`). Confirm that this is intentional, and that the Admin Wallet's UTXOs are isolated from any pre-existing wallet on the same device.
3. **"Bridge Operator update" semantics** — purely a rename, or also a semantic change in what the action carries? The crate-side action type is `OperatorSetUpdate` (see `docs/2-discovery/09-functional-analysis.md`); confirm with Alpen that the rename is UI-only.
4. **`block_payout` import format** — is the imported "raw copy" a serialised bitcoin transaction, or a structured envelope that includes existing signatures + metadata? The Pending Payout coordination requires the latter to be useful.
5. **Fee rate cap (10,000 s/vB)** — this cap is now repeated in three places (wallet send, manual payout, default send for pending updates). Confirm this is a hard UI cap rather than guidance.
6. **Address rotation on Receive** — "after the user has received BTC in a given address, the app MUST automatically rotate" — define "received": first-seen unconfirmed vs. first-confirmation. Affects Receive-screen polling cadence.

---

## 9. References

- Original PRD: [`docs/0-prd/01-multisig-ui.md`](../0-prd/01-multisig-ui.md)
- PRD update: [`docs/0-prd/03-prd-update.md`](../0-prd/03-prd-update.md)
- Functional analysis (entities, update types, flows): [`./09-functional-analysis.md`](./09-functional-analysis.md)
- Authority verification research: [`./13-authority-verification-findings.md`](./13-authority-verification-findings.md)
- HW wallet architecture: [`./06-hardware-wallet-architecture.md`](./06-hardware-wallet-architecture.md)
