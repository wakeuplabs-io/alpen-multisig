# Spec: Session-bound Admin Wallet (mnemonic login)

> Phase 3.7 of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
> Predecessors merged: Phase 3.5 (retire operator key), Phase 3.6 (Admin Wallet–only commit funding).
> Successor: Phase 3.8 (watch-only Admin Wallet for HW login) builds on the session slot defined here.

## Objective

Bind the `WalletService` lifecycle to the user's login session so that when the user logs in with
the dev mnemonic ("Palabras" path), the Admin Wallet is derived from **that same mnemonic** — not from
a process-wide `ADMIN_WALLET_REGTEST_MNEMONIC` env var fixed at startup.

This closes the PRD §3.2 gap: a single source (today a hardware wallet; in dev, a mnemonic) is the
origin of both the Admin ID (`m/84'/0'/73'/0/0`, P2WPKH) and the Admin Wallet (`m/86'/0'/73'/n/n`,
Taproot). Today these are sourced independently and any mismatch silently shows the wrong wallet.

**Why now:** Phase 4 (Send) and Phase 7 (HW signing) must be built against a session-scoped wallet.
Establishing the session slot now means HW login (Phase 3.8) and HW signing (Phase 7) fill the same
extension point instead of forcing a second refactor later.

## Scope

### In scope

- Replace the single fixed `Arc<WalletService>` managed state with a **session slot** that can be
  filled at login and cleared at logout.
- Mnemonic login: after successful authentication, derive and register the session `WalletService`
  from the same mnemonic used to log in. Mnemonic transits IPC once, then is owned by Rust.
- Logout: drop the session wallet; the panel returns to its `Disabled` state. The background sync
  task for the old session must stop (no stale syncing, no double loop on re-login).
- All wallet IPC commands and the commit-funding path migrate to the session-scoped state and handle
  the "no session" state gracefully (return `Disabled`, never panic), including the brief race window
  between authentication and session init.
- `ADMIN_WALLET_REGTEST_MNEMONIC` demoted to **CI/headless fallback** for wallet IPC when no session is active (integration tests that call wallet IPC without a UI login). **Still required in `.env`** for the SPS-50 commit/reveal internal key via `broadcast_env.rs` (Phase 3.5+). Documented as such; no longer the source of the wallet panel when logged in via Palabras.
- `ALLOW_DEV_MNEMONIC_SIGNING` guard remains and is still required as the explicit regtest opt-in;
  the secret-carrying session-init command is registered in dev-signing builds only.
- Frontend: wire session init on mnemonic login; ensure the panel shows `Disabled` after logout.

### Not in scope

- Hardware-wallet session init / watch-only wallet (Phase 3.8). HW login leaves the slot empty
  (`Disabled`) in this phase.
- Send / signing from the session wallet (Phase 4+).
- Full removal of `ADMIN_WALLET_REGTEST_MNEMONIC` (Phase 9).
- Refactoring the legacy per-call mnemonic commands (`list_mnemonic_addresses`,
  `sign_with_mnemonic_path`) to stop receiving the mnemonic per call — tracked separately; this phase
  only narrows the wallet's mnemonic to a single init-time transit.
- Any change to commit/reveal protocol semantics.

## Technical Design

### Architecture overview

Replace the startup-fixed `Arc<WalletService>` with a managed **`WalletSession`** newtype wrapping a
session slot. The slot is empty at startup. Mnemonic login fills it via a new dev-gated IPC command;
logout clears it. Wallet IPC commands snapshot the inner `Arc<WalletService>` under a short lock and
return `Disabled` when the slot is empty. The background-sync task is owned by the `WalletService` it
syncs and stops when that service is shut down on session replacement/logout. The env var is consulted
only by a lazy fallback inside the slot accessor, used solely when no session is active — so an active
session always wins.

```text
React (session-provider.tsx)
  authenticate() ──ok──► wallet_session_init({ mnemonic, network })   [mnemonic adapter only]
  disconnect()   ──────► auth_logout  ──► WalletSession::clear()

Tauri managed state:  WalletSession { Arc<RwLock<Option<Arc<WalletService>>>> }
  ├─ current()              → live session wallet, or None
  ├─ current_or_fallback()  → session wallet  ▸  else env fallback (CI)  ▸  else Disabled
  ├─ init_from_mnemonic()   → build + store (shuts down any prior service)
  └─ clear()                → shutdown bg task + take()
```

### Decision summary (full trade-offs in the design appendix below)

| # | Decision | Choice |
|---|---|---|
| 1 | Slot container & locking | `WalletSession` newtype over `Arc<std::sync::RwLock<Option<Arc<WalletService>>>>` — `std` RwLock keeps the non-async `admin_wallet_sync_status` command unchanged; lock held only for a cheap `Arc` clone, never across `.await` |
| 2 | Mnemonic transit | New dedicated IPC command `wallet_session_init({ mnemonic, … })`, called by the FE right after auth, mnemonic adapter only; auth stays decoupled from wallet; Phase 3.8 reuses the slot with an xpub |
| 3 | Background-sync teardown | Per-`WalletService` shutdown signal cancelled by the slot before drop/replace (`tokio::select!` on the signal) — prompt exit, no leak, no double loop |
| 4 | No-session state | Reuse `AdminWalletError::Disabled` (no new variant); add `SyncStatusDto::disabled_default()` for the sync command |
| 5 | Env-var fallback | Lazy, inside the slot accessor, only when slot is `None`; active session always wins by construction |

### Production functions / commands

**New**

- `application/wallet_session.rs` — `WalletSession` newtype:
  - `pub fn empty() -> Self`
  - `pub fn current(&self) -> Option<Arc<WalletService>>`
  - `pub fn current_or_fallback(&self) -> Result<Arc<WalletService>, AdminWalletError>`
  - `pub async fn init_from_mnemonic(&self, mnemonic: &str, passphrase: Option<&str>, network: Option<&str>) -> Result<(), AdminWalletError>`
  - `pub fn clear(&self)` (cancels the prior service's bg task, then `take()`s the slot)
  - private `network_from_env()` (mirrors `main.rs` network parsing)
- `commands/admin_wallet.rs` — `WalletSessionInitInput` DTO + `#[tauri::command] async fn wallet_session_init(input, wallet_session) -> Result<(), String>`; gated by `dev_secrets::ensure_dev_mnemonic_signing_allowed()`. Registered in `attach_with_dev_signing` **only** (never `attach_production`).
- `application/wallet_service.rs` — `pub fn shutdown(&self)` (idempotent) + a cancellation signal field.
- `SyncStatusDto::disabled_default()` — all-`None`/`is_syncing:false` with a typed `Disabled` `last_error`.
- Frontend `api/admin-wallet.ts` — `walletSessionInit({ mnemonic, network }): Promise<Result<void>>` typed bridge call.

**Changed**

- `main.rs` — delete `build_wallet_service` and the dummy fallback; `.manage(WalletSession::empty())`.
- `application/wallet_service.rs::spawn_background_sync` — loop uses `tokio::select!` on the
  cancellation signal so it exits promptly on shutdown.
- `commands/admin_wallet.rs` — the six existing commands change their state param from
  `tauri::State<'_, Arc<WalletService>>` to `tauri::State<'_, WalletSession>`. Async commands resolve
  via `current_or_fallback()`; the non-async `admin_wallet_sync_status` uses `current()` +
  `disabled_default()`. The `admin_wallet_info(svc: &WalletService)` helper keeps its signature
  (callers snapshot the `Arc` and deref).
- `commands/invoke.rs` — register `wallet_session_init` in `attach_with_dev_signing`.
- `commands/authentication.rs` / `application/authentication.rs` — `auth_logout` also clears the
  wallet slot (atomic logout): `auth_logout` takes `tauri::State<'_, WalletSession>` and calls
  `wallet_session.clear()` after `authentication::logout()`.
- Frontend `contexts/session-provider.tsx::connectSession` — after `authenticate(...)` succeeds, call
  `walletSessionInit` when `adapter.vendor === 'mnemonic'`. HW / other vendors skip → slot stays empty
  → panel `Disabled`. The mnemonic value is read from the same `WalletAdapterOptions.mnemonic` the FE
  already passes to `createWalletAdapter` (no new location for the secret); expose a minimal accessor
  on the mnemonic adapter if needed rather than widening any other surface.

### IPC contract — `wallet_session_init`

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSessionInitInput {
    pub mnemonic: String,
    pub passphrase: Option<String>, // forward-compat; current load_admin_wallet uses ""
    pub network: Option<String>,    // "regtest" default; mirrors main.rs parsing
}
// Returns Ok(()); FE then triggers a normal sync / get_admin_wallet_info to populate the panel.
// Errors (serialized AdminWalletError): InvalidMnemonic | Descriptor | WalletCreation, plus the
// dev-gate string error. MUST NOT contact RPC: building the wallet is a pure local op so that login
// latency is never coupled to node availability (offline-survivable). The existing sync path remains
// the empirical RPC probe.
```

### Background-sync teardown

`spawn_background_sync` currently clones `Arc<Self>` into a detached task. On logout the slot drops its
`Arc`, but the task's clone keeps the service (and its BDK wallet + RPC creds) alive and syncing
forever, and a second login spawns a second loop. Fix: each `WalletService` owns a cancellation signal;
the loop is `tokio::select! { _ = <cancelled> => break, _ = sleep(SYNC_INTERVAL) => {…} }`. The slot
calls `svc.shutdown()` inside `clear()`/`init_from_mnemonic()` **before** dropping/replacing the `Arc`.

> **Dependency note:** `tokio-util` (`CancellationToken`) is **not** currently a workspace dependency.
> Adding it is governed by [ADR-001](../architecture/adrs/001-alpen-crate-dependencies.md). To avoid a
> new dependency, prefer an equivalent built on the already-present `tokio` (`tokio::sync::Notify` or
> an `Arc<AtomicBool>` + `Notify` checked in the `select!`). Implementation chooses; document the
> outcome. Do not add `tokio-util` without recording the rationale.

### Env-var fallback semantics

`current_or_fallback()` returns the live session wallet if present (branch 1, active session always
wins). Only when the slot is `None` does it consult `ADMIN_WALLET_REGTEST_MNEMONIC` (branch 2), build a
wallet, and cache it in the slot. No env → `Disabled`. This makes "login with mnemonic A while env=B
shows A, not B" true by construction (branch 1 returns before branch 2 is evaluated). The
`check_enabled()` guard (`BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1`) stays inside
`WalletService::sync()`/`fund_commit()` unchanged, so even a built fallback wallet is inert outside
regtest-dev. `main.rs` no longer reads the env var; `wallet_session.rs` reads it for wallet IPC
fallback only. `broadcast_env.rs` still reads it for the commit/reveal internal key at
`m/86'/0'/73'/2/0` (Phase 3.5+). Use the same mnemonic for login and in `.env` on regtest.

> Documented caveat: in a headless/CI context where env is set and no UI session exists, the fallback
> re-materializes on the next IPC after a `clear()`. This is the intended CI behavior; a real UI logout
> clears a *session* wallet and the FE stops issuing wallet IPC, so the panel goes `Disabled`.

### Production code vs. test helpers

- **Production functions**: `WalletSession` (`empty`/`current`/`current_or_fallback`/`init_from_mnemonic`/`clear`),
  `wallet_session_init` Tauri command, `WalletService::shutdown`, `SyncStatusDto::disabled_default`,
  the migrated six commands, FE `walletSessionInit`.
- **Test helpers** (must live in `#[cfg(test)]` / test modules, never registered as Tauri commands):
  test mnemonics (the existing `abandon … about`), builders that construct a `WalletSession` with a
  pre-seeded service, env-var set/clear guards (reuse the existing `ENV_LOCK` serialization pattern).
  The legacy `list_mnemonic_addresses` / `sign_with_mnemonic_path` remain dev-signing-only and are not
  affected by this phase.

## Test Cases

Tests target production functions only.

**Happy path**

- `wallet_session_init` with a valid mnemonic → `Ok(())`; `current()` is `Some`; subsequent
  `get_balance` / `list_addresses` derive from that mnemonic (external address index 0 equals
  `load_admin_wallet(mnemonic)`'s external addr 0).
- After init, `get_admin_wallet_info` returns an address derived from the session mnemonic, not env.

**Edge cases**

- `wallet_session_init` does **not** contact RPC: succeeds with a dead/unset `BITCOIN_RPC_URL`.
- `init_from_mnemonic` while a session already exists → prior service `shutdown()` called before the
  new one is stored; only one background loop running afterward.
- `list_addresses` page-window bounds unchanged against a session wallet (existing tests stay green).

**Expected errors**

- `wallet_session_init` with an invalid mnemonic → serialized `InvalidMnemonic`; slot stays `None`
  (no partial state).
- `wallet_session_init` without `ALLOW_DEV_MNEMONIC_SIGNING` → dev-gate error; and the command is not
  registered in `attach_production` (handler-set assertion).
- Slot-lock poison → mapped to `Disabled` / recovered via `into_inner`, never panics.

**Race / no-session state**

- Any async wallet IPC with empty slot and no env → returns `Disabled`; `admin_wallet_sync_status`
  returns `disabled_default()` (no panic, `is_syncing=false`).
- HW login simulation (`vendor != 'mnemonic'`, no init) → slot stays `None` → `Disabled`.
- `auth_logout` clears the slot → next `current()` is `None` → `Disabled`; background task cancelled
  (assert the cancellation signal is observed / task exits).
- Re-login after logout builds a fresh service with a fresh live background task (no stale loop).

**Offline fallback / env semantics**

- No session + env set → fallback built and cached; second call reuses the same `Arc` (pointer-eq).
- **Active session A while env=B → returns A** (regression-critical for the stated invariant).
- No session + env unset → `Disabled`.
- Env mnemonic set but `BITCOIN_NETWORK != regtest` → `sync()` returns `Disabled` via `check_enabled()`.

**Authority isolation**

- Not applicable to wallet derivation directly; covered by the existing auth/role tests. Session init
  occurs only after a successful role authentication, unchanged here.

## Module structure

Each file has one reason to change:

- `application/wallet_session.rs` — **owns the session slot lifecycle**: hold/replace/clear the
  `Arc<WalletService>`, env-fallback policy for wallet IPC, slot locking. The only file that knows about
  `RwLock<Option<…>>`. Reads `ADMIN_WALLET_REGTEST_MNEMONIC` for wallet IPC fallback only
  (`broadcast_env.rs` still reads it for the commit/reveal internal key).
- `application/wallet_service.rs` — **owns one wallet's behavior and its background-task lifecycle**
  (now including the cancellation signal + `shutdown`). Unchanged otherwise.
- `commands/admin_wallet.rs` — **thin IPC boundary**: snapshot from the slot, delegate, serialize
  errors. `wallet_session_init` is the only write entry point.
- `commands/invoke.rs` — **handler-registration composition root**; enforces the dev-gate / production
  command split (secret-carrying command absent from production builds by construction).
- `main.rs` — **managed-state composition root**: `.manage(WalletSession::empty())` only.
- `infrastructure/admin_wallet/wallet.rs` — unchanged; `load_admin_wallet` stays the pure derivation
  function the slot depends on (dependency direction: slot → derivation, not the reverse).
- `infrastructure/dev_secrets.rs` — unchanged; reused gate.
- Frontend `api/admin-wallet.ts` — transport only for `wallet_session_init`.
  `contexts/session-provider.tsx` — orchestration (decides when to init/clear); no new secret storage.

Dependency direction: `WalletSession` (slot policy) depends on `WalletService` (behavior) and
`load_admin_wallet` (derivation). IPC command modules depend on `WalletSession`. Nothing in the
infrastructure layer depends on the command or session layer.

---

## Design appendix — decision trade-offs

### D1. Slot container & locking

- **Newtype vs raw type alias** — a `WalletSession` newtype hosts `init`/`clear`/fallback in one place
  and keeps `tauri::State<WalletSession>` signatures clean; a raw `Arc<RwLock<Option<…>>>` leaks
  locking discipline into every command.
- **`Option<Arc<WalletService>>` vs `Option<WalletService>`** — storing an `Arc` lets a command clone
  it cheaply, drop the lock, then run the long async `sync()`/`fund_commit()` without holding the slot
  lock across `.await`. Storing the value directly would serialize all wallet IPC and risk deadlock
  with the bg task.
- **`std::sync::RwLock` vs `tokio::sync::RwLock`** — `admin_wallet_sync_status` is a **non-async**
  Tauri command; a tokio async-lock would force making it `async` (FE contract change) or risk
  blocking a runtime worker via `blocking_read()`. `std::RwLock` lets the sync command read normally;
  the lock is only ever held for a microsecond `Arc` clone with no `.await` underneath, so the
  "no std lock across await" rule holds by construction. The slot lock is independent of the
  `tokio::Mutex`/`RwLock` *inside* `WalletService`, which stay as-is.
- **RwLock vs Mutex** — many concurrent readers snapshot the `Arc`; writes (init/clear) are rare.

### D2. Mnemonic transit

| Option | Security exposure | Scope | Coupling | Phase 3.8 (HW) fit |
|---|---|---|---|---|
| **A: dedicated `wallet_session_init`** | Mnemonic crosses IPC once, then Rust-owned; never logged/echoed. Narrower than today's per-sign mnemonic commands. | Small | Auth stays decoupled from wallet | Excellent — same command with `{xpub}` (no secret) |
| B: move mnemonic ownership into Rust at connect-time | Best | Large (rewrites adapter + per-call commands) | Couples a bigger refactor into 3.7 | Slot reusable, refactor out of scope |
| C: extend `auth_complete` with optional mnemonic | Mnemonic flows through the auth verifier path (leak risk in auth errors/logs) | Medium | Bad — couples auth to wallet | Poor |

**Chosen: A.** Minimal change satisfying "wallet derived from the login mnemonic," keeps concerns
separate, forward-compatible with HW.

### D3. Background-sync teardown alternatives

- *Generation counter* — task checks an `AtomicU64` each tick; lingers up to 30s, still pins the `Arc`.
- *Stored JoinHandle aborted on logout* — works, but splits ownership (slot tracks handles separately).
- *Cancellation signal owned by the service (chosen)* — immediate exit, releases the `Arc`, signal
  lives with the thing it controls. Implemented without a new dependency where feasible (see note).

### D4. No-session signal

Reuse `Disabled` rather than a new `NoSession` variant: from the signer's perspective the states are
identical ("wallet not available right now"), and the FE already renders a `Disabled` card and maps the
error code. A new variant would add a parallel FE state for no UX benefit.

### D5. Env fallback placement

Moving env consultation from `main.rs` (eager, process-wide) into the accessor (lazy, only when no
session) is what decouples the wallet from the env var. Normal login never touches env; the regression
invariant (A over B) holds by construction; CI/headless keeps a working wallet on first IPC.
