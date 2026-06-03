# Evolution: Admin Wallet — Balance UX (R1.5)

**Date:** 2026-06-03
**Branch:** `feature/admin-wallet-balance-ux`
**Commits:** `078e2ed` (spec), `47e72d0` (frontend), finalize commit (mempool sync + docs)
**Spec:** [`docs/specs/admin-wallet-balance-ux.md`](../specs/admin-wallet-balance-ux.md)
**PR:** [#211](https://github.com/wakeuplabs-io/alpen-multisig/pull/211)
**Predecessor:** R1.4 Canonical connect paths ([`2026-06-02-admin-wallet-canonical-connect-paths.md`](2026-06-02-admin-wallet-canonical-connect-paths.md))

## Summary

R1.5 closes PRD §4.3.1 in the Admin Wallet slide-over: the hero balance stays on **confirmed** sats; when the wallet has pending activity, a muted tertiary line shows the signed net unconfirmed amount (`+N sats unconfirmed` / `−N sats unconfirmed`). During delivery we found that `WalletService::do_sync` only applied **blocks**, so `BalanceDto.unconfirmedSats` stayed zero and receive rotation did not observe mempool credits until a block was mined. R1.5 therefore includes a small **mempool sync** step (`Emitter::mempool` + `apply_unconfirmed_txs`) so regtest testing with `sendtoaddress` (no `generatetoaddress`) works as documented for R1.3 and R1.5.

## Business Context

PRD §4.3.1 requires the signer to see total balance net of unconfirmed movements and the net unconfirmed amount separately. Phase 2 already exposed `confirmedSats` and `unconfirmedSats` over IPC; R1.2 only rendered confirmed in the UI. R1.5 surfaces unconfirmed in the panel without changing DTO shape or IPC command names.

## Deliverable

| Item | Status |
|------|--------|
| Spec `admin-wallet-balance-ux.md` | Done |
| `WalletBalance` — `confirmedSats` + `unconfirmedSats`, tertiary line | Done |
| `formatUnconfirmedBalanceLine` + unit tests | Done |
| Panel wiring (dashboard + broadcast) | Done |
| Architecture Rule 5 (balance wiring guard) | Done |
| `WalletService::do_sync` — mempool apply after block sync | Done |
| Manual regtest: `sendtoaddress` without mine → unconfirmed line + receive rotation | Verified |
| Rust + frontend CI | Green |

## Key Decisions

- **Hero = confirmed, line = signed unconfirmed net.** Matches R1.2 conservative hero; PRD satisfied via confirmed hero plus separate pending line (not `totalSats` as hero).
- **Hide line when `unconfirmedSats === 0`.** No `+0 sats unconfirmed` noise when the wallet is fully confirmed.
- **Reuse `formatSignedSats` via `formatUnconfirmedBalanceLine`.** Single formatter for copy; component stays presentational.
- **Header unchanged.** Session-first panel title kept; watch-only badge deferred to R1.6.
- **Mempool sync in `do_sync` (scope amendment).** Original spec said frontend-only; manual regtest proved block-only sync left `unconfirmedSats` at zero. After block loop, sync now calls `emitter.mempool()` and `wallet.apply_unconfirmed_txs`. No new IPC; same `BalanceDto` contract.

## Files Changed

**Frontend (production):**

- `desktop-app/src/domain/admin-wallet/model/format-unconfirmed-balance-line.ts` (new)
- `desktop-app/src/domain/admin-wallet/components/wallet-balance.tsx`
- `desktop-app/src/domain/admin-wallet/components/wallet-panel-content.tsx`
- `desktop-app/src/screens/proposals-dashboard-screen.tsx`
- `desktop-app/src/screens/broadcast-proposal-screen.tsx`

**Backend (production):**

- `desktop-app/src-tauri/src/application/wallet_service.rs` — mempool apply in `do_sync`

**Tests:**

- `desktop-app/src/domain/admin-wallet/model/__tests__/format-unconfirmed-balance-line.test.ts` (new)
- `desktop-app/src/domain/admin-wallet/architecture.test.ts` — Rule 5
- `desktop-app/package.json` — `test:model-format-unconfirmed-balance-line`

**Documentation:**

- `docs/specs/admin-wallet-balance-ux.md`
- `docs/specs/admin-wallet-implementation-plan.md` (R1.5 ✅; next → R1.6)
- `docs/specs/admin-wallet-core-read-path.md` (sync amendment note)
- `docs/evolution/2026-06-03-admin-wallet-balance-ux.md` (this file)

## Known Limitations (post-R1.5)

- **Per-address unconfirmed** → R1.6 (`admin-wallet-addresses-ux.md`).
- **Negative unconfirmed net** depends on BDK `trusted_pending` + `untrusted_pending`; outgoing mempool spends are not separately modeled beyond that net.
- **R1.6 not included:** addresses table still shows single `balanceSats` per row until R1.6.

## Links

- Implementation plan: [`admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md) (R1.5 ✅; next: R1.6)
- Spec: [`admin-wallet-balance-ux.md`](../specs/admin-wallet-balance-ux.md)
- PR: https://github.com/wakeuplabs-io/alpen-multisig/pull/211
