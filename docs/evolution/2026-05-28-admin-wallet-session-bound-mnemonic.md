# Evolution Archive: admin-wallet-session-bound-mnemonic

**Date**: 2026-05-28
**Feature branch**: feature/admin-wallet-session-bound-mnemonic
**Spec**: [docs/specs/admin-wallet-session-bound-mnemonic.md](../specs/admin-wallet-session-bound-mnemonic.md)
**Program**: Admin Wallet — Phase 3.7

---

## Feature Summary

Phase 3.7 binds the `WalletService` lifecycle to the user's login session. When a
user logs in with the "Palabras" dev mnemonic, the Admin Wallet is now derived from
*that same mnemonic* rather than from the independent `ADMIN_WALLET_REGTEST_MNEMONIC`
environment variable. This closes the PRD §3.2 gap where the Admin Wallet
(`m/86'/0'/73'/n/n`) and Admin ID (`m/84'/0'/73'/0/0`) were sourced independently and
any mismatch silently showed the wrong wallet.

The Tauri managed wallet state changed from a fixed-at-startup `Arc<WalletService>` to
a session-scoped slot (`WalletSession` wrapping `Arc<RwLock<Option<Arc<WalletService>>>>`)
that is populated at login and cleared at logout. The mnemonic never leaves the Rust
process. The `ADMIN_WALLET_REGTEST_MNEMONIC` env var remains in `.env`: CI/headless fallback for
wallet IPC when no session is active, and still required for the SPS-50 commit/reveal internal key
via `broadcast_env.rs`. Use the same mnemonic for login and in `.env` on regtest. The
hardware-wallet login path intentionally leaves the slot empty (`Disabled`) — Phase 3.8 fills it
with a watch-only wallet.

---

## Steps Completed

12 steps across 5 phases (TDD, 5-phase DES cycle per step).

### Phase 01 — WalletService cancellation signal and shutdown
1. `01-01` Add cancellation signal + idempotent `shutdown()` to `WalletService`
2. `01-02` Rewrite `spawn_background_sync` to `tokio::select!` on the cancellation signal
3. `01-03` Add `SyncStatusDto::disabled_default()` constructor

### Phase 02 — WalletSession newtype (session slot)
4. `02-01` Create `wallet_session.rs` with `WalletSession` newtype (`empty`/`current`/`clear`)
5. `02-02` Implement `init_from_mnemonic` (builds wallet, no RPC, shuts down prior service)
6. `02-03` Implement `current_or_fallback` with env-var fallback (only reader of the env var)

### Phase 03 — Migrate wallet IPC commands + add `wallet_session_init`
7. `03-01` Add `WalletSessionInitInput` DTO and dev-gated `wallet_session_init` command
8. `03-02` Migrate six wallet commands from `Arc<WalletService>` to `WalletSession` state
9. `03-03` Extend `auth_logout` to clear the wallet session slot

### Phase 04 — Simplify main.rs composition root
10. `04-01` Replace managed `Arc<WalletService>` with `WalletSession::empty()`; drop env read

### Phase 05 — Frontend wiring
11. `05-01` Add `walletSessionInit` bridge to `api/admin-wallet.ts`
12. `05-02` Wire `walletSessionInit` in `connectSession` for the mnemonic adapter

---

## Key Decisions

**Session slot as a newtype**: `WalletSession` wraps
`Arc<std::sync::RwLock<Option<Arc<WalletService>>>>` rather than exposing the raw lock.
This keeps poison-recovery (`into_inner`) and shutdown-on-replace logic in one place and
gives the IPC layer a small, intentional API (`current`, `current_or_fallback`, `clear`,
`init_from_mnemonic`).

**Background-sync cancellation via signal, not task abort**: the sync loop observes a
cancellation signal through `tokio::select!` and exits on the next iteration. Re-init and
`clear()` call `shutdown()` on the prior service first, guaranteeing exactly one background
loop per live wallet and a clean teardown at logout.

**`current_or_fallback` is the single env-var reader**: only this function reads
`ADMIN_WALLET_REGTEST_MNEMONIC`, and only when the session slot is empty. A live session
always wins (session A with `env=B` returns wallet A). The empty-slot-with-no-env case
returns `Disabled` rather than panicking.

**`wallet_session_init` is dev-gated**: the command is registered only under
`attach_with_dev_signing` (guarded by `ensure_dev_mnemonic_signing_allowed()`), never in
the production handler set, and contacts no RPC on init.

**HW login leaves the slot empty**: non-mnemonic adapters skip `walletSessionInit`, so the
panel shows `Disabled` until Phase 3.8 supplies a watch-only wallet. This was an explicit
extension point, not an oversight.

---

## Files Modified

### Production
- `desktop-app/src-tauri/src/application/wallet_service.rs` — cancellation signal, `shutdown()`, `spawn_background_sync` select loop, `SyncStatusDto::disabled_default()`
- `desktop-app/src-tauri/src/application/wallet_session.rs` — new `WalletSession` newtype, `init_from_mnemonic`, `current_or_fallback`
- `desktop-app/src-tauri/src/application/mod.rs` — expose `wallet_session`
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` — `WalletSessionInitInput` DTO, `wallet_session_init`, migrate six commands to `WalletSession` state
- `desktop-app/src-tauri/src/commands/authentication.rs` — `auth_logout` clears the session slot
- `desktop-app/src-tauri/src/commands/invoke.rs` — register `wallet_session_init` under dev-signing handler
- `desktop-app/src-tauri/src/main.rs` — managed state is now `WalletSession::empty()`; `build_wallet_service` removed; no env read
- `desktop-app/src/api/admin-wallet.ts` — `walletSessionInit` bridge
- `desktop-app/src/contexts/session-provider.tsx` — call `walletSessionInit` for mnemonic logins
- `desktop-app/src/wallet/mnemonic-adapter.ts` — expose mnemonic to `connectSession` wiring

### Docs
- `docs/specs/admin-wallet-session-bound-mnemonic.md`
- `docs/specs/admin-wallet-implementation-plan.md` (Phase 3.7 marked complete)
- `docs/evolution/2026-05-28-admin-wallet-session-bound-mnemonic.md` (this file)

### Machine artifacts (DES)
- `docs/feature/admin-wallet-session-bound-mnemonic/deliver/roadmap.json`
- `docs/feature/admin-wallet-session-bound-mnemonic/deliver/execution-log.json`
- `docs/feature/admin-wallet-session-bound-mnemonic/deliver/.develop-progress.json`
- `docs/feature/admin-wallet-session-bound-mnemonic/deliver/mutation-report.md`

---

## Quality Gates Passed

| Gate | Result |
|------|--------|
| Deliver integrity verification (12/12 steps complete DES traces) | PASS |
| L1–L6 refactoring pass | PASS |
| Adversarial review | PASS |
| Mutation testing (Phase 3.7 scope, 11/11 viable killed = 100%) | PASS |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `npm run format:check` | PASS |
| `npm run lint` | PASS |
| `npm run build` | PASS |

Manual smoke (requires `npm run tauri dev` + regtest stack): logging in with mnemonic A
while `ADMIN_WALLET_REGTEST_MNEMONIC=B` shows wallet A; logout returns the panel to
`Disabled`.

---

## Lessons Learned

**Cancellation signal beats task handle for clean teardown**: observing a signal inside
the loop (rather than holding and aborting a `JoinHandle`) keeps the shutdown path
idempotent and makes "exactly one loop per live wallet" easy to assert in tests.

**One reader for wallet IPC fallback**: funnelling `ADMIN_WALLET_REGTEST_MNEMONIC` reads for
wallet IPC through `current_or_fallback` made the precedence rule (live session always wins)
trivial to reason about and to test, and let `main.rs` drop its env dependency entirely.
(`broadcast_env.rs` still reads the env var for the commit/reveal internal key.)

**Mutation testing caught real coverage gaps**: the initial per-feature run surfaced
untested `parse_network` arms (testnet/mainnet) and a missing `spawn_background_sync`
activation assertion. Three targeted tests closed the gaps to 11/11 viable mutants killed.
