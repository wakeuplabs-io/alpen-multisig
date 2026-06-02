# Evolution: Admin Wallet — Canonical connect paths (R1.4)

**Date:** 2026-06-02
**Branch:** `feature/admin-wallet-canonical-connect-paths`
**Spec:** [`docs/specs/admin-wallet-canonical-connect-paths.md`](../specs/admin-wallet-canonical-connect-paths.md)
**Predecessor:** R1.3 Receive rotation ([`2026-06-02-admin-wallet-receive-rotation.md`](2026-06-02-admin-wallet-receive-rotation.md))

## Summary

R1.4 removes connect-time derivation picking from the desktop wallet connect flow. The app now derives one canonical Admin ID during connect and advances directly to authority selection; users no longer choose from a 20-address BIP-84 list. This aligns sign-in with PRD §3.2's single canonical identity model and keeps the Admin Wallet session on the canonical BIP-86 account path established by earlier Admin Wallet phases.

## Business Context

The previous picker was POC-era scaffolding that let a signer authenticate with arbitrary indexes under `m/84'/.../0/n`. That was useful for manual demos, but it conflicted with the protocol-facing identity: Strata/Alpen administrators authenticate with the canonical Admin ID at index 0. Removing the picker reduces signer confusion and prevents accidental "not a member" failures caused by picking a non-canonical address.

## Deliverable

Single increment delivered via the SDD workflow (spec → branch → red/green TDD → refactor → verification → docs).

| Item | Status |
|------|--------|
| Spec `admin-wallet-canonical-connect-paths.md` | Done |
| Connect state machine collapsed from `connect → picking → selected` to `connect → selected` | Done |
| Frontend address picker UI removed | Done |
| `WalletAdapter.listAddresses` and mutable connect-time path selection removed | Done |
| Trezor/Ledger address-list IPC commands and infra functions removed | Done |
| Mnemonic connect reduced from a 20-address derivation window to canonical `count: 1` | Done |
| WebDriver login helper updated for canonical-only connect | Done |
| Row-1 co-sign e2e script/spec retired | Done |
| Architecture overview and implementation plan updated | Done |

## Key Decisions

- **Canonical entry is built from `adapter.connect()`.** The existing no-list fallback already produced a single entry from `WalletAccountInfo`; R1.4 makes that the only path and uses index 0 for display state.
- **Verify-on-device stays.** The connect-time `verify_address_on_device` command remains because it now verifies the canonical Admin ID path. Receive-address verification is still Phase 7.
- **Mnemonic connect keeps `list_mnemonic_addresses` but asks for one address.** The Tauri command remains the dev mnemonic derivation primitive; the connect flow no longer requests or displays a 20-address window.
- **Ledger testnet/regtest coin-type behavior stays unchanged.** Ledger continues to use its existing testnet app conventions for Admin ID and Admin Wallet xpubs.
- **Row-1 co-sign e2e is retired, not rewritten.** It simulated a second signer by selecting row #1 of the same mnemonic, which is no longer a valid product flow.

## Files Changed

**Frontend (production):**

- `desktop-app/src/domain/connect-wallet/hooks/use-hw-wallet-connect.ts`
- `desktop-app/src/domain/connect-wallet/model/hw-wallet-connect.types.ts`
- `desktop-app/src/domain/connect-wallet/components/hw-wallet-connect.tsx`
- `desktop-app/src/domain/connect-wallet/components/authority-selection-phase.tsx`
- `desktop-app/src/domain/connect-wallet/components/authenticate-session-phase.tsx`
- `desktop-app/src/domain/connect-wallet/components/connect-phase.tsx`
- `desktop-app/src/domain/connect-wallet/components/selected-phase.tsx`
- `desktop-app/src/domain/connect-wallet/components/picking-phase.tsx` (removed)
- `desktop-app/src/wallet/types.ts`
- `desktop-app/src/wallet/trezor-adapter.ts`
- `desktop-app/src/wallet/ledger-adapter.ts`
- `desktop-app/src/wallet/mnemonic-adapter.ts`

**Backend (production):**

- `desktop-app/src-tauri/src/commands/hw_wallet.rs`
- `desktop-app/src-tauri/src/commands/invoke.rs`
- `desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs`
- `desktop-app/src-tauri/src/infrastructure/hw_wallet/ledger.rs`

**Tests and E2E:**

- `desktop-app/src/domain/connect-wallet/canonical-connect-paths.test.ts` (new)
- `desktop-app/package.json`
- `desktop-app/e2e-webdriver/test/helpers/login-mnemonic.mjs`
- `desktop-app/e2e-webdriver/test/specs/proposal-co-sign-row1.e2e.js` (removed)
- `desktop-app/e2e-webdriver/package.json`
- `desktop-app/e2e-webdriver/README.md`
- `desktop-app/e2e-webdriver/test/specs/proposal-broadcast-quorum.e2e.js`

**Documentation:**

- `docs/specs/admin-wallet-implementation-plan.md`
- `docs/architecture/overview.md`
- `docs/evolution/2026-06-02-admin-wallet-canonical-connect-paths.md`

## Known Limitations (post-R1.4)

- **Multi-signer WebDriver flow needs a new fixture strategy.** The removed row-1 spec used one mnemonic with a non-canonical BIP-84 index as a second signer. A product-faithful replacement should use two distinct mnemonics whose canonical Admin IDs are both present in the regtest authority signer set.
- **Historical POC/spec docs still mention address selection.** Those documents describe earlier POC scope and are intentionally left unchanged.

## Links

- Implementation plan: [`admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md)
- Spec: [`admin-wallet-canonical-connect-paths.md`](../specs/admin-wallet-canonical-connect-paths.md)
- R1.3 predecessor: [`2026-06-02-admin-wallet-receive-rotation.md`](2026-06-02-admin-wallet-receive-rotation.md)
