# Governance Broadcast — Fee Rate Selection

## 1. Purpose

Define the user-facing behavior for fee rate selection when broadcasting governance transactions (commit funding for approved proposals). This document is purely functional — no implementation details.

## 2. Scope

- **Where:** Governance broadcast screen (approved proposals that reached quorum).
- **Who:** Strata Administrator and Alpen Administrator signers.
- **What:** The signer selects a fee rate before broadcasting the commit transaction.

Later phases will reuse this pattern for wallet Send (Phase 6) and shared Send UX (Phase 9).

## 3. Fee Rate Presets

The signer MUST be presented with three speed presets plus a custom option:

| Preset | Meaning | Source |
|--------|---------|--------|
| **Fast** | Highest priority, fastest confirmation | Node fee estimate for next block + security margin |
| **Medium** | Balanced fee and confirmation time (default) | Node fee estimate for ~6 blocks + security margin |
| **Slow** | Lowest fee, confirmation may take longer | Node fee estimate for ~12 blocks + security margin |
| **Custom** | Manual entry | Signer-defined, starting from the Medium default |

The **Medium** preset MUST be selected by default.

> **Wallet Send uses a different default:** Admin Wallet Send (PRD §4.3.5.3) overrides the
> initial preset to **Fast** (next-block estimate). Governance broadcast and fee-bump on
> governance rows keep **Medium** as the default. See
> [`admin-wallet-send-btc-implementation.md`](./admin-wallet-send-btc-implementation.md) decision D4.

### 3.1 Fee Estimate Source

Fee estimates MUST come from the connected Bitcoin node via `estimatesmartfee` using the following confirmation targets:

| Preset | Confirmation target |
|--------|-------------------|
| Fast | Next block (1) |
| Medium | ~6 blocks |
| Slow | ~12 blocks |

The node may be:
- A local full node (default connection method per PRD §2).
- A trusted remote RPC endpoint (PRD §2.2).

If the node is unavailable or returns an error, the Electrum server MAY be used as a fallback for fee estimation.

### 3.2 Security Margin

Each preset MUST apply a security margin on top of the raw node estimate to increase the likelihood of timely inclusion during mempool congestion. The margin is a percentage added to the estimated rate:

| Preset | Suggested margin |
|--------|-----------------|
| Fast | +20% |
| Medium | +10% |
| Slow | +5% |

These values are recommendations and may be adjusted based on observed mempool behavior.

## 4. Custom Fee Rate

When the signer selects **Custom**, they MUST be able to manually specify the fee rate:

- **Unit:** sat/vB (satoshis per virtual byte).
- **Increment:** 0.1 sat/vB.
- **Minimum:** The node's minimum relay fee (typically 1 sat/vB).
- **Maximum:** 10,000 sat/vB.
- **Default starting value:** The Medium preset value at the time Custom is selected.

The custom entry MUST show the estimated total network fee in sats based on the transaction's virtual size.

## 5. Replace-by-Fee (RBF)

All transactions broadcast from the application MUST signal RBF (BIP-125) by default. This allows the signer to bump the fee of an unconfirmed transaction later if needed.

RBF signaling is non-negotiable and not user-configurable at this time. Fee bump UX is covered by Phase 6 (Transactions + fee-bump).

## 6. Broadcast Path

The transaction MUST be broadcast via the Electrum server. If the Electrum server is unavailable or rejects the transaction, the Bitcoin node RPC MUST be used as a fallback.

If both Electrum and the node RPC are unavailable, the application MUST offer the option to copy the raw signed transaction hex to the clipboard so the signer can broadcast through any other Bitcoin RPC.

## 7. UI Reference

The fee rate selection UI follows the Alta handoff design pattern:

- **Segmented control** with three presets: Slow / Medium / Fast.
- **Settings button** to expand custom fee rate entry.
- **Description line** below the control explaining the selected preset or showing the custom rate.
- **Estimated fee display** showing total network fee in sats.

Reference: `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/components.jsx` — `FeeRateInput` component.

## 8. Constraints Summary

| Constraint | Value |
|-----------|-------|
| Fee estimate source | Bitcoin node (`estimatesmartfee`), Electrum fallback |
| UTXO source | Electrum (wallet sync), node fallback |
| Broadcast source | Electrum, Bitcoin node RPC fallback, manual hex copy |
| Presets | Slow / Medium / Fast |
| Default preset | Medium |
| Custom increment | 0.1 sat/vB |
| Custom minimum | Node's minimum relay fee (typically 1 sat/vB) |
| Custom maximum | 10,000 sat/vB |
| RBF | Always enabled (BIP-125) |
| Security margin | Fast +20%, Medium +10%, Slow +5% (recommended) |

## 9. PRD Traceability

| PRD | Section | Requirement |
|-----|---------|-------------|
| `01-multisig-ui.md` | §5 (line 85) | Manual sat/vB fee rate in increments of 0.1 for governance broadcast |
| `01-multisig-ui.md` | §6.3.3 (line 115) | Manual sat/vB fee rate for block_payout broadcast |
| `03-prd-update.md` | §4.3.5.3 (line 83) | Manual fee rate 0.1 s/vB increments, max 10,000, default next-block |
| `03-prd-update.md` | §5.3.3.1 (line 106) | Governance broadcast UX similar to wallet Send |
| `05-prd-payout-admin` | §6.4 (line 140) | Fee rate for manual block_payout creation |
