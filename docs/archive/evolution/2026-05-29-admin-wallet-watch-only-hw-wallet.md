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
| D7 | Account-xpub coin type | Trezor: `m/86'/0'/73'` (`0'`). **Ledger: network-aware — `1'` on regtest/testnet/signet, `0'` on mainnet** (revised during HW validation, see below) | Original spec mandated `0'` for both to match `load_admin_wallet`. Live Ledger testing showed the Ledger testnet app **rejects** coin type `0'` (APDU `6a82`), so the Ledger must follow its own Admin-ID `1'` convention on non-mainnet networks. The xpub network version bytes are normalised at wallet build, so addresses remain consistent for the same key material |
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

Three chained bugs surfaced only during **live hardware validation** (Ledger via the speculos emulator on regtest) — all invisible to the unit suite, which had no real device. They were diagnosed by adding temporary `eprintln!` diagnostics to the IPC commands and reading the speculos APDU log:

1. **Ledger rejects coin type `0'` (APDU `6a82`).** The Ledger testnet Bitcoin app only serves the testnet coin type `1'`; requesting `m/86'/0'/73'` failed at the device, so `get_account_xpub` errored and the session never initialised. Fix: Ledger xpub path is now network-aware (`1'` on regtest/testnet/signet, `0'` on mainnet), mirroring its Admin-ID convention. This **revised decision D7**. Trezor is unaffected (accepts `0'`).
2. **Error serialization masked the failure.** `serialize_wallet_error` used serde's default externally-tagged form (`"Disabled"`), but the frontend `AdminWalletError` union expects `{ type: 'Disabled' }`. So a failed/absent session rendered as a misleading "0 balance, no error card" empty panel (and later as a "set BITCOIN_NETWORK…" card whose advice was wrong — the env was fine, there was simply no session). Fix: `serialize_wallet_error` now emits `{ type, message }`; the frontend union + formatter handle all variants.
3. **IPC payload not wrapped in `{ input }`.** `walletSessionInitWatchOnly` invoked with a flattened payload while the Rust command takes a single `input: WatchOnlyInitInput` parameter (bound by name). Tauri could not deserialise the argument, so the command body never ran and the watch-only session silently failed to initialise. Fix: wrap as `{ input }`, matching the (already-correct) mnemonic path. This was the bug that actually blocked the feature end-to-end.

A latent display issue was also fixed: the receive-address row rendered at 11px with no label, making the address effectively invisible.

## Lessons learned

- The Phase 3.7 design (decoupled session slot, `Option` keypair groundwork) made this phase straightforward — adding a second fill path required no structural changes to `WalletSession`.
- A dedicated pure capability command (`admin_wallet_can_sign`) with no RPC dependency is the right pattern for UI-state queries; piggybacking on sync-triggering commands would couple render latency to node availability.
- User-facing copy must never reference implementation phases or internal milestone names — enforced as a spec constraint (user-facing copy constraint section in the spec).
- **The IPC boundary needs a real-device integration test, not just typed unit tests.** All three blocking bugs lived at the Rust↔JS↔device seam and passed the unit suite. The original `walletSessionInitWatchOnly` test asserted only `typeof === 'function'` (testing theater) and missed the wrong argument shape; it has been strengthened to intercept the Tauri `invoke` and assert the `{ input }` payload wrapping. The WebDriver smoke covers the mnemonic path but not the Ledger watch-only path — a HW (speculos) smoke for this flow would have caught all three.
- **Device assumptions must be validated against the actual app, not the spec.** The spec's D7 (`0'` for both devices) was correct in intent but wrong in practice: the Ledger testnet app cannot derive `0'`. Hardware behaviour overrides spec decisions; design notes that assert device capabilities should be flagged as assumptions until proven on-device.
- **Overloaded error variants hide root causes.** `AdminWalletError::Disabled` means both "env guard failed" and "no session", so a watch-only init failure surfaced as misleading "set BITCOIN_NETWORK…" guidance. Distinct states deserve distinct, accurate error messages.

## Links

- Spec: `docs/specs/admin-wallet-watch-only-hw-wallet.md`
- Implementation plan: `docs/specs/admin-wallet-implementation-plan.md` (Phase 3.8 row)
- Predecessor evolution: `docs/archive/evolution/2026-05-28-admin-wallet-session-bound-mnemonic.md`
- Feature workspace (deliver audit trail): `docs/archive/features/admin-wallet-watch-only-hw-wallet/`
