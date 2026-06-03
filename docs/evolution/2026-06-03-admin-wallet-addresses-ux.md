# Evolution: Admin Wallet — Addresses UX (R1.6)

**Date:** 2026-06-03
**Branch:** `feature/admin-wallet-addresses-ux`
**Spec:** [`docs/specs/admin-wallet-addresses-ux.md`](../specs/admin-wallet-addresses-ux.md)
**Predecessor:** R1.5 Balance UX ([`2026-06-03-admin-wallet-balance-ux.md`](2026-06-03-admin-wallet-balance-ux.md))

## Summary

R1.6 closes PRD §4.3.2 in the Admin Wallet slide-over: each funded external address shows **confirmed** sats as the
primary BTC amount and, when applicable, a muted signed unconfirmed sub-line (`±N sats unconfirmed`) matching R1.5 copy.
The wallet panel header defaults to **Admin Wallet**, shows session/signer context as subtitle, and displays a
**Watch-only** badge when `canSign === false`.

## Business Context

PRD §4.3.2 requires signers to see each address that holds a balance with its current balance net of unconfirmed
activity, with unconfirmed effects visible. Phase 2 already exposed `UtxoDto.confirmations`; R1.6 splits aggregation in
`composeAddressesWithBalance` and renders the breakdown in the addresses accordion.

## Deliverable

| Item | Status |
|------|--------|
| Spec `admin-wallet-addresses-ux.md` | Done |
| `groupUtxoBalancesByDerivation` + `composeAddressesWithBalance` split | Done |
| `AddressRow` — confirmed hero + unconfirmed sub-line + copy | Done |
| Accordion header `Addresses with balance · N` | Done |
| `WalletPanelHeader` — Admin Wallet title + watch-only badge | Done |
| Panel wiring (dashboard + broadcast) + `useAdminWalletCapability` | Done |
| Architecture Rule 6 (address row wiring guard) | Done |
| PRD §4.3.2 | **PASS** (frontend-only; relies on R1.5 mempool sync for regtest unconfirmed UTXOs) |
| Release 1 | **Closed** (§4.3.1–§4.3.2 complete) |
| Rust + frontend CI | Green |

## Key Decisions

- **Hero = confirmed per address, sub-line = signed unconfirmed net** — mirrors R1.5 wallet-level semantics.
- **Hide sub-line when `unconfirmedSats === 0`.**
- **Reuse `formatUnconfirmedBalanceLine`** — no duplicate formatter.
- **No backend/IPC changes** — R1.5 mempool sync is sufficient for regtest pending UTXOs.
- **Per-row `CopyButton`** — optional polish included using shared component.

## Files Changed

**Frontend (production):**

- `desktop-app/src/domain/admin-wallet/model/group-utxo-balances-by-derivation.ts` (new)
- `desktop-app/src/domain/admin-wallet/model/compose-addresses-with-balance.ts`
- `desktop-app/src/domain/admin-wallet/components/address-row.tsx`
- `desktop-app/src/domain/admin-wallet/components/addresses-with-balance-list.tsx`
- `desktop-app/src/domain/admin-wallet/components/wallet-panel-header.tsx`
- `desktop-app/src/screens/proposals-dashboard-screen.tsx`
- `desktop-app/src/screens/broadcast-proposal-screen.tsx`

**Tests:**

- `compose-addresses-with-balance.test.ts` (extended)
- `group-utxo-balances-by-derivation.test.ts` (new)
- `address-row-contract.test.ts` (new)
- `use-addresses-with-balance.test.ts` (type contract)
- `architecture.test.ts` — Rule 6

**Documentation:**

- `docs/specs/admin-wallet-addresses-ux.md`
- `docs/specs/admin-wallet-implementation-plan.md` (R1.6 ✅; Release 1 closed)
- `docs/evolution/2026-06-03-admin-wallet-addresses-ux.md` (this file)

## Known Limitations

- **Negative per-address unconfirmed** depends on BDK UTXO listing; no separate outgoing-spend model in the frontend.
- **Internal/change addresses** remain excluded from the with-balance table (unchanged policy).

## Links

- Implementation plan: [`admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md)
- Spec: [`admin-wallet-addresses-ux.md`](../specs/admin-wallet-addresses-ux.md)
- Next increment: **Phase 4** — Send BTC happy path
