# Evolution: Admin Wallet — Canonical connect paths (R1.4)

**Date:** 2026-06-02
**Status:** Merged to `develop` — [PR #206](https://github.com/wakeuplabs-io/alpen-multisig/pull/206) (`9bf5c3f`, 2026-06-02). Feature branch deleted after merge.
**Spec:** [`docs/specs/admin-wallet-canonical-connect-paths.md`](../specs/admin-wallet-canonical-connect-paths.md)
**Predecessor:** R1.3 Receive rotation ([`2026-06-02-admin-wallet-receive-rotation.md`](2026-06-02-admin-wallet-receive-rotation.md))

## Summary

R1.4 removes connect-time derivation picking from the desktop wallet connect flow. The app now derives one canonical Admin ID during connect and advances directly to authority selection; users no longer choose from a 20-address BIP-84 list. This aligns sign-in with PRD §3.2's single canonical identity model and keeps the Admin Wallet session on the canonical BIP-86 account path established by earlier Admin Wallet phases.

A follow-up in the same PR restores the three-step proposal WebDriver flow using a **second demo mnemonic** (`DEMO_MNEMONIC_COSIGN`, last word `absent`) at the same canonical path `m/84'/0'/73'/0/0`, registered as `strata_administrator.keys[1]` in `scripts/asm-params.example.json`.

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
| Row-1 co-sign e2e (`proposal-co-sign-row1`) retired | Done |
| Multi-signer e2e restored (`proposal-co-sign-mnemonic` + `DEMO_MNEMONIC_COSIGN` in asm-params) | Done |
| Rust + WebDriver proposal flow verified (add-signer → co-sign-mnemonic → broadcast-quorum) | Done |
| Architecture overview and implementation plan updated | Done |
| Release 1 marked complete in implementation plan | Done (this closeout doc) |

## Key Decisions

- **Canonical entry is built from `adapter.connect()`.** The existing no-list fallback already produced a single entry from `WalletAccountInfo`; R1.4 makes that the only path and uses index 0 for display state.
- **Verify-on-device stays.** The connect-time `verify_address_on_device` command remains because it now verifies the canonical Admin ID path. Receive-address verification is still Phase 7.
- **Mnemonic connect keeps `list_mnemonic_addresses` but asks for one address.** The Tauri command remains the dev mnemonic derivation primitive; the connect flow no longer requests or displays a 20-address window.
- **Ledger testnet/regtest coin-type behavior stays unchanged.** Ledger continues to use its existing testnet app conventions for Admin ID and Admin Wallet xpubs.
- **Second signer for e2e uses a distinct mnemonic, not a derivation index.** `DEMO_MNEMONIC_COSIGN` (`… absent`) derives `029b8c2b…` at `m/84'/0'/73'/0/0` and replaces the old `keys[1]` (`037f6704…`, index 1 of the primary mnemonic).

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
- `desktop-app/src/wallet/types.ts`, adapters, `demo-mnemonic.ts`

**Backend (production):**

- `desktop-app/src-tauri/src/commands/hw_wallet.rs`, `invoke.rs`
- `desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs`, `ledger.rs`

**Tests and E2E:**

- `desktop-app/src/domain/connect-wallet/canonical-connect-paths.test.ts` (new)
- `desktop-app/e2e-webdriver/test/helpers/login-mnemonic.mjs` (`DEMO_MNEMONIC_COSIGN`)
- `desktop-app/e2e-webdriver/test/specs/proposal-co-sign-mnemonic.e2e.js` (replaces `proposal-co-sign-row1`)
- `desktop-app/e2e-webdriver/package.json`, `README.md`
- `e2e-tests` fixtures (`DEMO_COSIGN_MNEMONIC`, `keys[1]` pubkey)

**Regtest fixtures:**

- `scripts/asm-params.example.json`, `staging/asm-params.template.json` (`keys[1]` → cosign pubkey)

**Documentation:**

- `docs/specs/admin-wallet-implementation-plan.md`
- `docs/architecture/overview.md`
- `docs/evolution/2026-06-02-admin-wallet-canonical-connect-paths.md`

## Verification

- Local: `from-scratch` + stack + `test:e2e:proposal-add-signer` → `proposal-co-sign-mnemonic` → `proposal-broadcast-quorum` + `mine-blocks.sh` — all passed (2026-06-02).
- CI: [run 26833109948](https://github.com/wakeuplabs-io/alpen-multisig/actions/runs/26833109948) green after rustfmt/prettier fix.

## Known Limitations (post-closeout)

- **Historical POC/spec docs** may still mention address selection or `co-sign-row1`; those describe earlier scope and are left unchanged unless explicitly refreshed.
- `list_mnemonic_addresses` still derives a window when called with `count > 1`; only connect uses `count: 1`.

## Links

- Implementation plan: [`admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md) (Release 1 complete; next: **Release 2** Electrum sync)
- Spec: [`admin-wallet-canonical-connect-paths.md`](../specs/admin-wallet-canonical-connect-paths.md)
- R1.3 predecessor: [`2026-06-02-admin-wallet-receive-rotation.md`](2026-06-02-admin-wallet-receive-rotation.md)
