# Evolution Archive: admin-wallet-session-bound-mnemonic

**Date**: 2026-05-28 / 2026-05-29 (3.7a + 3.7b + 3.7c delivered)
**Feature branch**: feature/admin-wallet-session-bound-mnemonic
**Spec**: [docs/specs/admin-wallet-session-bound-mnemonic.md](../specs/admin-wallet-session-bound-mnemonic.md)
**Program**: Admin Wallet — Phase 3.7

---

## Feature Summary

**3.7a (delivered)** binds the `WalletService` lifecycle to the user's login session. When a
user logs in with the "Palabras" dev mnemonic, the Admin Wallet is derived from
*that same mnemonic* rather than from the independent `ADMIN_WALLET_REGTEST_MNEMONIC`
environment variable for **panel, sync, addresses, and commit funding**.

**3.7b** extends the same policy to the SPS-50 commit/reveal internal key at `m/86'/0'/73'/2/0`:
`SessionState` caches the derived keypair at login; `resolve_commit_reveal_keypair` in
`broadcast_env.rs` uses the session key when present and only reads `ADMIN_WALLET_REGTEST_MNEMONIC`
for CI/headless (no session).

The Tauri managed wallet state changed from a fixed-at-startup `Arc<WalletService>` to
a session-scoped slot (`WalletSession`) populated at login and cleared at logout. After 3.7b,
the slot holds `SessionState { wallet, commit_reveal_keypair }`; the mnemonic is not stored.
`ADMIN_WALLET_REGTEST_MNEMONIC` becomes CI/headless fallback only for all paths. The
hardware-wallet login path intentionally leaves the slot empty (`Disabled`) — Phase 3.8 fills it
with a watch-only wallet.

---

## Steps Completed

16 steps across 6 phases (TDD, 5-phase DES cycle per step).

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

### Phase 06 — Session-bound commit/reveal key (3.7b + 3.7c)
13. `06-01` `SessionState` holds `commit_reveal_keypair` (derived at login, not stored)
14. `06-02` `commit_reveal_keypair_or_fallback` on `WalletSession` — session key wins, env-var is CI/headless fallback only
15. `06-03` Session-aware `load_broadcast_env` in `broadcast_env.rs`; single reader for the env var
16. `06-04` Wire `WalletSession` into prepare/broadcast commands; remove `ADMIN_WALLET_REGTEST_MNEMONIC` from `.env.regtest` (3.7c)

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

**`current_or_fallback` is the single env-var reader (3.7a)**: only this function reads
`ADMIN_WALLET_REGTEST_MNEMONIC` for wallet IPC, and only when the session slot is empty. A live session
always wins (session A with `env=B` returns wallet A). **3.7b** adds `commit_reveal_keypair_or_fallback`
with the same policy for broadcast; `broadcast_env.rs` must not read the env var independently.

**`commit_reveal_keypair_or_fallback` mirrors `current_or_fallback` policy (3.7b)**: the same
"live session always wins" rule applies to the SPS-50 internal key. `broadcast_env.rs` delegates
to this method exclusively; the env var is never read directly from broadcast code.

**3.7c removes `ADMIN_WALLET_REGTEST_MNEMONIC` from `.env.regtest`**: after 3.7b, the env var is
only needed for headless CI (no Tauri session). The regtest dev flow now derives everything from
the mnemonic entered at login, so the variable was removed from the example env file to reduce
confusion.

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
- `desktop-app/src-tauri/src/application/wallet_session.rs` — `SessionState` extended with `commit_reveal_keypair`; `commit_reveal_keypair_or_fallback` added
- `desktop-app/src-tauri/src/application/broadcast_env.rs` — session-aware `load_broadcast_env`; single env-var reader
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` — prepare/broadcast commands wired to `WalletSession`
- `.env.regtest` — `ADMIN_WALLET_REGTEST_MNEMONIC` removed (3.7c)

### Docs
- `docs/specs/admin-wallet-session-bound-mnemonic.md`
- `docs/specs/admin-wallet-implementation-plan.md` (Phase 3.7 marked complete)
- `docs/archive/evolution/2026-05-28-admin-wallet-session-bound-mnemonic.md` (this file)

### Machine artifacts (DES)
- `docs/archive/features/admin-wallet-session-bound-mnemonic/deliver/roadmap.json`
- `docs/archive/features/admin-wallet-session-bound-mnemonic/deliver/execution-log.json`
- `docs/archive/features/admin-wallet-session-bound-mnemonic/deliver/.develop-progress.json`
- `docs/archive/features/admin-wallet-session-bound-mnemonic/deliver/mutation-report.md`

---

## Quality Gates Passed

| Gate | Result |
|------|--------|
| Deliver integrity verification (16/16 steps complete DES traces) | PASS |
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

**One reader per env-var path**: funnelling `ADMIN_WALLET_REGTEST_MNEMONIC` reads through a
single method per domain (`current_or_fallback` for wallet IPC, `commit_reveal_keypair_or_fallback`
for broadcast) made the precedence rule (live session always wins) trivial to reason about and to
test. After 3.7b/c, `main.rs` and `broadcast_env.rs` both dropped their direct env reads entirely.

**Mutation testing caught real coverage gaps**: the initial per-feature run surfaced
untested `parse_network` arms (testnet/mainnet) and a missing `spawn_background_sync`
activation assertion. Three targeted tests closed the gaps to 11/11 viable mutants killed.
