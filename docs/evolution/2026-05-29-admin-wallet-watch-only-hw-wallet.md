# Evolution: Watch-only Admin Wallet (HW login) — Phase 3.8

**Date:** 2026-05-29
**Feature ID:** admin-wallet-watch-only-hw-wallet
**Branch:** feature/admin-wallet-watch-only-hw-wallet
**Predecessor:** Phase 3.7 — session-bound mnemonic wallet ([evolution](2026-05-28-admin-wallet-session-bound-mnemonic.md))
**Successor:** Phase 7 — HW PSBT signing

## Summary

Phase 3.8 of the Admin Wallet implementation plan. After Phase 3.7, a user who logged in with a hardware wallet (Trezor/Ledger) left the `WalletSession` slot empty and saw the `Disabled` card — a regression against PRD §3.2 intent. This phase closes that gap by fetching the BIP-86 account xpub at `m/86'/0'/73'` from the device at login and building a watch-only BDK wallet registered in the same session slot. Balance, UTXOs, and addresses are visible immediately. All signing operations surface a clear "Hardware wallet required to sign" message rather than silently failing or panicking. The mnemonic login path is entirely unchanged.

## Business context

PRD §3.2 specifies that the hardware wallet is the source of the Admin Wallet (`m/86'/0'/73'/n/n`). Leaving HW users with a `Disabled` wallet panel degrades operator trust and blocks them from verifying their balance before any governance action. Watch-only visibility at login — with honest signing-unavailable feedback — gives HW users the same read confidence as mnemonic users, while deferring Phase 7 signing infrastructure to a focused, later phase.

## Key decisions

| # | Decision | Choice | Rationale |
|---|----------|--------|-----------|
| D1 | How HW fills the slot | Dedicated `wallet_session_init_watch_only` IPC command, not extending `auth_complete` | Follows Phase 3.7 D2: auth and wallet init are decoupled; no secret crosses the auth boundary; registered in both production and dev-signing handlers |
| D2 | Watch-only vs signing capability | `can_sign: bool` on `WalletService` + `new_watch_only` constructor | O(1) capability check; all existing `WalletService::new` call sites and tests compile unchanged; invariant is explicit and testable |
| D3 | Descriptor shape | Plain `tr(xpub/0/*)` / `tr(xpub/1/*)`, no key origin metadata | Sufficient for read-only balance/addresses; Phase 7 adds `[fp/86h/0h/73h]` origin when PSBT signing requires it |
| D4 | Commit/reveal key for watch-only | `SessionState.commit_reveal_keypair` becomes `Option<UntweakedKeypair>` | Watch-only stores `None`; no private key can ever be derived from an xpub; invariant held by construction |
| D5 | Sign-disabled error | New `AdminWalletError::ReadOnly` + `BroadcastEnvError::ReadOnly`, distinct from `Disabled` and `WalletSessionRequired` | Three genuinely different states with different remedies; collapsing them would show wrong guidance to HW users |
| D6 | Read-path enablement | Keep existing regtest `check_enabled()` guard for sync | Keeps Phase 3.8 tightly scoped; decoupling read-enablement from sign-enablement is deferred to Phase 9 |
| D7 | Account-xpub coin type | `m/86'/0'/73'` (coin type `0'`) for both Trezor and Ledger | Matches `load_admin_wallet`; using Ledger's `1'` coin-type convention from the Admin-ID path would produce different addresses than the mnemonic wallet for the same seed |
| D8 | `canSign` exposure to frontend | Dedicated pure command `admin_wallet_can_sign() -> bool`, no RPC | Capability check must not trigger a sync; FE can render the correct affordance immediately after login, before/independent of any sync |

## Steps completed

15 steps across 3 phases:

**Phase 1 — Backend core (steps 01–05)**
- Step 01: `load_watch_only_admin_wallet` in `infrastructure/admin_wallet/wallet.rs` — builds `tr(xpub/0/*)` / `tr(xpub/1/*)` descriptors with no private keys; pure, no RPC
- Step 02: `WalletService::new_watch_only` constructor + `can_sign()` predicate; `fund_commit` early `ReadOnly` guard
- Step 03: `AdminWalletError::ReadOnly` variant + `error_code` mapping
- Step 04: `SessionState.commit_reveal_keypair` changed to `Option<UntweakedKeypair>`; `build_session_from_xpub`; `WalletSession::init_from_xpub` with shutdown-prior discipline; `WalletSession::can_sign`
- Step 05: `BroadcastEnvError::ReadOnly` + `resolve_commit_reveal_keypair` branching (no session → `WalletSessionRequired`; watch-only → `ReadOnly`; mnemonic → keypair)

**Phase 2 — Backend IPC (steps 06–10)**
- Step 06: Trezor `get_account_xpub` — `get_public_key` with `SPENDTAPROOT` at `m/86'/0'/73'`; returns full xpub string
- Step 07: Ledger `get_account_xpub` — `get_extended_pubkey` at account path
- Step 08: `get_trezor_admin_wallet_xpub` / `get_ledger_admin_wallet_xpub` Tauri commands returning `{ xpub, derivationPath }` via `spawn_blocking`
- Step 09: `WatchOnlyInitInput` DTO; `wallet_session_init_watch_only` command (no dev gate); `admin_wallet_can_sign` pure command
- Step 10: Registration of all four new commands in both `attach_production` and `attach_with_dev_signing` in `commands/invoke.rs`

**Phase 3 — Frontend (steps 11–15)**
- Step 11: `walletSessionInitWatchOnly` and `getAdminWalletCanSign` in `api/admin-wallet.ts`; `AdminWalletError` union extended with `{ type: 'ReadOnly' }`
- Step 12: `getAccountXpub?()` optional method on `WalletAdapter` type; implemented in `trezor-adapter.ts` and `ledger-adapter.ts`
- Step 13: `useAdminWalletCapability()` hook returning `{ canSign }`
- Step 14: `contexts/session-provider.tsx::connectSession` branching on `adapter.vendor` — mnemonic uses 3.7 path; trezor/ledger fetches xpub and calls `walletSessionInitWatchOnly`; mock/other skips init; HW init failure is non-fatal to login
- Step 15: Governance broadcast/commit affordance gated on `canSign === false` → renders disabled with "Hardware wallet required to sign"; `SendPlaceholder` copy cleaned of phase references

## What shipped

**Backend**
- `load_watch_only_admin_wallet(account_xpub, network)` — pure watch-only BDK wallet construction from xpub
- `WalletService::can_sign` capability flag + `new_watch_only` constructor
- `AdminWalletError::ReadOnly` + `BroadcastEnvError::ReadOnly` error variants
- `SessionState.commit_reveal_keypair` changed from `UntweakedKeypair` to `Option<UntweakedKeypair>`
- `WalletSession::init_from_xpub` + `WalletSession::can_sign`
- `build_session_from_xpub` — watch-only session builder
- Three-way `resolve_commit_reveal_keypair` branching in `broadcast_env.rs`
- `get_account_xpub` on both `trezor.rs` and `ledger.rs`
- `get_trezor_admin_wallet_xpub` and `get_ledger_admin_wallet_xpub` Tauri commands
- `wallet_session_init_watch_only` IPC command (no dev gate)
- `admin_wallet_can_sign` IPC command (pure, no RPC)

**Frontend**
- `walletSessionInitWatchOnly` and `getAdminWalletCanSign` in `api/admin-wallet.ts`
- `getAccountXpub?()` on `WalletAdapter` + implementations in both device adapters
- `useAdminWalletCapability()` hook
- `session-provider.tsx` vendor-based branching for session init
- Broadcast/commit UI gating on `canSign` with "Hardware wallet required to sign" copy

## Issues encountered

None blocking. The main design clarification was the coin type for the Ledger xpub path (D7): the Ledger Admin-ID convention uses `1'` (testnet), but the Admin Wallet must use `0'` to match `load_admin_wallet` and produce consistent addresses.

## Lessons learned

- The Phase 3.7 design (decoupled session slot, `Option` keypair groundwork) made this phase straightforward — adding a second fill path required no structural changes to `WalletSession`.
- A dedicated pure capability command (`admin_wallet_can_sign`) with no RPC dependency is the right pattern for UI-state queries; piggybacking on sync-triggering commands would couple render latency to node availability.
- User-facing copy must never reference implementation phases or internal milestone names — enforced as a spec constraint (user-facing copy constraint section in the spec).

## Links

- Spec: `docs/specs/admin-wallet-watch-only-hw-wallet.md`
- Implementation plan: `docs/specs/admin-wallet-implementation-plan.md` (Phase 3.8 row)
- Predecessor evolution: `docs/evolution/2026-05-28-admin-wallet-session-bound-mnemonic.md`
- Feature workspace (deliver audit trail): `docs/feature/admin-wallet-watch-only-hw-wallet/`
