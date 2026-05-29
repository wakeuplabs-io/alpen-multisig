# Spec: Watch-only Admin Wallet (hardware-wallet login)

> Phase 3.8 of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
> Predecessor: Phase 3.7 ([session-bound mnemonic wallet](./admin-wallet-session-bound-mnemonic.md)) — established the
> `WalletSession` slot this phase fills with an xpub instead of a mnemonic.
> Successor: Phase 7 (HW PSBT signing) replaces the watch-only descriptor with a signing path at the same slot.

## Status

| Slice | Scope | Delivery |
|-------|-------|----------|
| **3.8** | Watch-only `WalletService` from a HW account xpub at HW login; read path visible, signing surfaced as unavailable | Planned |

## Objective

After Phase 3.7, a user who logs in with a **mnemonic** gets a fully functional session Admin Wallet, but a user who
logs in with a **hardware wallet** (Trezor/Ledger) leaves the `WalletSession` slot empty — the panel shows the
`Disabled` card. That is a regression against PRD §3.2 intent: the HW *is* the source of the Admin Wallet
(`m/86'/0'/73'/n/n`), so balance and addresses should be visible even before signing is wired.

Phase 3.8 closes that gap **without** building any signing infrastructure:

- At HW login, fetch the BIP-86 **account xpub** at `m/86'/0'/73'` from the device and build a **watch-only** BDK
  wallet (public descriptors, no private key material) registered in the same session slot.
- Balance, UTXOs, addresses, and sync work identically to a mnemonic session.
- Every signing operation (governance commit funding, reveal, future Send) is **surfaced as unavailable** with a
  clear "Hardware wallet required to sign" message — visible but inoperable, never a panic and never a silent failure.

**Why now:** establishing the watch-only branch on the existing slot means Phase 7 only has to swap the read-only
descriptor for a PSBT signer at the same extension point — no second refactor of auth/session wiring.

## Scope

### In scope

- **HW account-xpub export.** New device operations that return the **full** BIP-86 account xpub at `m/86'/0'/73'`
  (the current `connect`/`get_*_info` operations only return the BIP-84 Admin ID leaf key, truncated for display).
  - Trezor: `get_public_key` with `InputScriptType::SPENDTAPROOT` at the account path; return the full `Xpub`.
  - Ledger: `get_extended_pubkey` at the account path (hardened prefix only, which the device already supports).
- **Watch-only wallet construction.** `load_watch_only_admin_wallet(account_xpub, network)` builds
  `tr(<xpub>/0/*)` (external) and `tr(<xpub>/1/*)` (internal) descriptors with **no** private keys.
- **Capability on `WalletService`.** A `can_sign` flag distinguishes a signing wallet (mnemonic) from a watch-only
  wallet (xpub). Read methods are unchanged for both; signing methods short-circuit to a typed read-only error when
  `can_sign` is false.
- **Watch-only session init.** `WalletSession::init_from_xpub` stores a watch-only `SessionState` with **no**
  commit/reveal keypair. A new IPC command `wallet_session_init_watch_only` accepts a plain xpub (no secret) and is
  registered in **both** production and dev-signing handler sets (unlike the dev-gated `wallet_session_init`).
- **Sign-path gating.** Governance commit funding (`WalletService::fund_commit`) returns a read-only error for a
  watch-only session; the broadcast key resolution distinguishes "no session" from "watch-only session present".
- **Frontend wiring.** HW login fetches the account xpub via the adapter and calls `walletSessionInitWatchOnly`;
  the panel shows balance/addresses; the governance broadcast/commit action surfaces "Hardware wallet required to
  sign" based on a session capability flag. Mnemonic login is untouched. Logout clears the slot (already handled).

### Not in scope

- **PSBT construction or any HW signing** (Phase 7). This phase only reads public data and proactively disables
  signing affordances.
- **The Send form.** Send functionality does not exist yet and is out of scope here. The signing-disabled UX in
  this phase targets the governance broadcast/commit path that exists today. (The existing Send placeholder copy is
  reworded to drop its plan/phase reference — see the user-facing copy constraint below — but no Send behavior is
  added.)
- **Decoupling the read path from the regtest dev guard.** `WalletService::check_enabled()` currently gates `sync`
  on `BITCOIN_NETWORK=regtest && ALLOW_DEV_MNEMONIC_SIGNING=1`; this phase keeps that guard (watch-only is
  demoable on regtest, consistent with the current state). Read-on-testnet/mainnet is Phase 9 hardening — see
  decision **D6**.
- **Master-fingerprint / key-origin metadata in descriptors.** Read-only balance/addresses do not need origin
  info; Phase 7 adds `[fp/86h/0h/73h]` origin when PSBT signing requires it — see decision **D3**.
- **Changing the mnemonic path, the commit/reveal protocol, or the Admin ID (`m/84'/0'/73'`) auth flow.**

## Technical Design

### Architecture overview

The Phase 3.7 slot already supports "wallet present but no commit/reveal key" semantics through
`commit_reveal_keypair() -> Option<…>`. Phase 3.8 makes the **wallet itself** capability-aware and adds a second
way to fill the slot — from an xpub instead of a mnemonic.

```text
React (session-provider.tsx) — connectSession(), after auth succeeds, branches on adapter.vendor:
  'mnemonic'        ──► walletSessionInit({ mnemonic })                 [3.7, dev-gated, secret]
  'trezor'|'ledger' ──► xpub = adapter.getAccountXpub()                 [3.8, new device call]
                        walletSessionInitWatchOnly({ xpub, network })   [3.8, no secret]
  other / mock      ──► (skip) → slot stays empty → Disabled

Tauri managed state:  WalletSession { Arc<RwLock<Option<SessionState>>> }
  SessionState { wallet: Arc<WalletService>, commit_reveal_keypair: Option<UntweakedKeypair> }
                                                              ▲ was non-optional in 3.7; now Option
  WalletService { …, can_sign: bool }                        ▲ new capability flag
    ├─ read path  (get_balance/list_utxos/list_addresses/sync)  → unchanged for both kinds
    └─ fund_commit (signing)                                    → Err(ReadOnly) when !can_sign

proposals_prepare_broadcast / proposals_broadcast
  └─ load_broadcast_env(wallet_session):
       current().is_none()              → WalletSessionRequired   (logged out)
       current() present, key == None   → ReadOnly                (watch-only HW session — Phase 7)
       key == Some(kp)                  → use kp                  (mnemonic session)
```

### Decision summary (full trade-offs in the appendix)

| # | Decision | Choice |
|---|----------|--------|
| D1 | How HW fills the slot | Dedicated `wallet_session_init_watch_only` IPC command mirroring `wallet_session_init` — **not** extending `auth_complete`. Reconciles the plan's wording with Phase 3.7 decision **D2** (the slot is the established, auth-decoupled extension point). |
| D2 | Watch-only vs signing capability | Add `can_sign: bool` to `WalletService` + `new_watch_only` constructor; `new` stays signing (backward-compatible with all existing call sites/tests). |
| D3 | Descriptor shape | Plain `tr(xpub/0/*)` / `tr(xpub/1/*)`, no key origin. Sufficient for read-only; Phase 7 adds origin for PSBT. |
| D4 | Commit/reveal key for watch-only | `SessionState.commit_reveal_keypair` becomes `Option`; watch-only stores `None`. No private key is ever derived from an xpub. |
| D5 | Sign-disabled error | New `AdminWalletError::ReadOnly` + `BroadcastEnvError::ReadOnly`, distinct from `Disabled` (env) and `WalletSessionRequired` (logged out), so the FE shows "Hardware wallet required to sign". |
| D6 | Read-path enablement | Keep the existing regtest `check_enabled()` guard for sync this phase; do **not** broaden to testnet/mainnet (Phase 9). |
| D7 | Account-xpub coin type | Use `m/86'/0'/73'` (coin type `0'`) for **both** devices to match `load_admin_wallet`. The Ledger Admin-ID `1'` coin-type convention must **not** carry over here, or watch-only addresses would diverge from the mnemonic wallet. |
| D8 | `canSign` exposure to FE | Dedicated pure command `admin_wallet_can_sign() -> bool` (no RPC/sync), read from the session slot. |

### Production functions / commands

**New — backend**

- `infrastructure/admin_wallet/wallet.rs` (or new `watch_only.rs`):
  - `pub fn load_watch_only_admin_wallet(account_xpub: &str, network: Network) -> Result<bdk_wallet::Wallet, AdminWalletError>`
    — parses the xpub, builds `tr(xpub/0/*)`/`tr(xpub/1/*)`, `create_wallet_no_persist`. Pure, no RPC.
  - `AdminWalletError::ReadOnly` variant (`#[error("admin wallet is watch-only; hardware wallet required to sign")]`).
- `application/wallet_service.rs`:
  - `pub fn new_watch_only(wallet: bdk_wallet::Wallet) -> Self` — identical to `new` but `can_sign = false`.
  - `pub fn can_sign(&self) -> bool`.
  - `fund_commit` gains an early `if !self.can_sign() { return Err(AdminWalletError::ReadOnly) }` guard (before sync).
  - `error_code` maps `ReadOnly` → `"ReadOnly"`.
- `application/wallet_session.rs`:
  - `SessionState.commit_reveal_keypair: Option<UntweakedKeypair>` (was non-optional).
  - `build_session_from_xpub(account_xpub: &str, network) -> Result<SessionState, AdminWalletError>` — watch-only
    wallet, `commit_reveal_keypair: None`.
  - `pub async fn init_from_xpub(&self, account_xpub: &str, network: Option<&str>) -> Result<(), AdminWalletError>`
    — same shutdown-prior-then-store discipline as `init_from_mnemonic`.
  - `pub fn can_sign(&self) -> bool` — `self.read_slot().map(|s| s.wallet.can_sign()).unwrap_or(false)`.
- `infrastructure/hw_wallet/trezor.rs`: `pub fn get_account_xpub(path: &str) -> Result<String, String>`
  (`SPENDTAPROOT`, returns the full xpub string).
- `infrastructure/hw_wallet/ledger.rs`: `pub fn get_account_xpub(path: &str) -> Result<String, String>`
  (`get_extended_pubkey` at the account path).
- `commands/hw_wallet.rs`: `get_trezor_admin_wallet_xpub` / `get_ledger_admin_wallet_xpub` Tauri commands returning
  `{ xpub, derivationPath }` (both run via `spawn_blocking` like the existing HW commands).
- `commands/admin_wallet.rs`:
  - `WatchOnlyInitInput { xpub: String, network: Option<String> }` DTO.
  - `#[tauri::command] async fn wallet_session_init_watch_only(input, wallet_session) -> Result<(), String>` — no
    dev gate (carries no secret).
  - `#[tauri::command] async fn admin_wallet_can_sign(wallet_session) -> Result<bool, String>` — pure, no RPC.
- `commands/invoke.rs`: register `wallet_session_init_watch_only`, `admin_wallet_can_sign`,
  `get_trezor_admin_wallet_xpub`, `get_ledger_admin_wallet_xpub` in **both** `attach_production` and
  `attach_with_dev_signing` (no secret material).

**New — frontend**

- `api/admin-wallet.ts`:
  - `walletSessionInitWatchOnly({ xpub, network? }): Promise<ApiResult<null>>`.
  - `getAdminWalletCanSign(): Promise<ApiResult<boolean>>`.
  - extend `AdminWalletError` union with `{ type: 'ReadOnly' }`.
- `wallet/types.ts`: optional `getAccountXpub?(): Promise<string>` on `WalletAdapter`.
- `wallet/trezor-adapter.ts` / `wallet/ledger-adapter.ts`: implement `getAccountXpub()` calling the new commands.
- `domain/admin-wallet/hooks/use-admin-wallet-capability.ts`: `useAdminWalletCapability()` → `{ canSign }`.

**Changed**

- `infrastructure/broadcast_env.rs::resolve_commit_reveal_keypair`: when a session is present but
  `commit_reveal_keypair()` is `None`, return `BroadcastEnvError::ReadOnly` (new); keep `WalletSessionRequired` only
  for the no-session case. Add `BroadcastEnvError::ReadOnly`.
- `contexts/session-provider.tsx::connectSession`: branch on `adapter.vendor` (mnemonic vs trezor/ledger) and call
  the matching init. HW failure is non-fatal to login (same posture as the 3.7 mnemonic branch) — the panel falls
  back to `Disabled`.
- `domain/admin-wallet` broadcast/commit affordance: when `canSign === false`, render the action disabled with
  "Hardware wallet required to sign". The Phase 4 `SendPlaceholder` is unchanged.

### IPC contract — `wallet_session_init_watch_only`

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchOnlyInitInput {
    pub xpub: String,            // BIP-86 account xpub at m/86'/0'/73'
    pub network: Option<String>, // "regtest" default; mirrors wallet_session_init parsing
}
// Returns Ok(()). MUST NOT contact RPC — building a watch-only wallet is a pure local op, so HW login
// latency is never coupled to node availability. The existing sync path remains the RPC probe.
// Errors (serialized AdminWalletError): Descriptor | WalletCreation (e.g. malformed xpub, wrong network).
```

### User-facing copy constraint

No user-facing string (panel messages, button labels, disabled-state hints, placeholders) may reference the
implementation plan, a phase number, or internal milestone naming. Copy describes **state and action**, not roadmap.

| Surface | Copy |
|---------|------|
| Watch-only signing affordance | "Hardware wallet required to sign" |
| Send placeholder (reworded) | "Send is not available yet." (drop the prior "Phase 4" reference) |

This also applies to any new strings introduced by this phase. Internal identifiers, code comments, and this spec
may reference phases; the running UI may not.

### Network consistency (D7)

`load_admin_wallet` derives the Admin Wallet at `m/86h/0h/73h` (coin type `0'`). The watch-only path **must** fetch
the account xpub at the same `m/86'/0'/73'` so a HW initialized from a seed yields the same external/internal
addresses as the mnemonic wallet for that seed. The xpub's own network version bytes must match the session
`network`; mismatches surface as `AdminWalletError::Descriptor`/`WalletCreation` at `init_from_xpub` time.

### Production code vs. test helpers

- **Production functions**: `load_watch_only_admin_wallet`, `WalletService::{new_watch_only, can_sign}`,
  `WalletSession::{init_from_xpub, can_sign}`, `build_session_from_xpub`, `wallet_session_init_watch_only`,
  `admin_wallet_can_sign`, the HW `get_account_xpub` ops + their commands, the FE bridges/adapters/hook, the
  `ReadOnly` error variants, and the `fund_commit`/`resolve_commit_reveal_keypair` gates.
- **Test helpers** (`#[cfg(test)]` / test modules only, never registered as commands): a pinned test **account
  xpub** derived from the existing `abandon … about` mnemonic at `m/86'/0'/73'` (so watch-only and mnemonic
  derivations can be asserted equal); builders constructing a watch-only `WalletSession` via `init_from_xpub`;
  the existing `ENV_TEST_LOCK` for guard env only. HW device calls are not unit-tested (no device in CI); they are
  covered by the WebDriver smoke playbook.

## Test Cases

Tests target production functions only.

**Happy path**

- `load_watch_only_admin_wallet(test_account_xpub, Regtest)` → `Ok`; external address index 0 **equals**
  `load_admin_wallet(test_mnemonic, Regtest)` external index 0 (watch-only ≡ mnemonic for the same seed) — the
  regression anchor for D7.
- `init_from_xpub` → `current()` is `Some`; `get_balance`/`list_addresses`/`list_utxos` succeed against the
  watch-only wallet; `list_addresses` window bounds match the mnemonic-wallet behavior.
- `wallet_session_init_watch_only` does **not** contact RPC: succeeds with a dead/unset `BITCOIN_RPC_URL`.
- `admin_wallet_can_sign` → `false` after a watch-only init; `true` after a mnemonic init; `false` with no session.

**Capability / sign-path gating**

- `WalletService::new_watch_only(...).can_sign()` is `false`; `WalletService::new(...).can_sign()` is `true`.
- `fund_commit` on a watch-only service → `Err(AdminWalletError::ReadOnly)` **without** contacting RPC (guard runs
  before sync).
- `resolve_commit_reveal_keypair` with a watch-only session present → `BroadcastEnvError::ReadOnly`;
  with no session → `BroadcastEnvError::WalletSessionRequired`; with a mnemonic session → the session keypair
  (regression: mnemonic broadcast still works after the `Option` change).

**Expected errors**

- `init_from_xpub` with a malformed xpub → `Descriptor`/`WalletCreation`; slot stays `None` (no partial state).
- `init_from_xpub` with an xpub whose network ≠ the requested network → typed error; slot stays `None`.

**Edge cases / lifecycle**

- `init_from_xpub` while a session already exists → prior service `shutdown()` is called before storing the new
  one; exactly one background loop afterward.
- `auth_logout` after a watch-only init → `current()` is `None`; background task cancelled (cancel signal observed).
- Re-login mnemonic → watch-only → mnemonic round-trips correctly (`can_sign` flips each time; no stale loop).

**Frontend**

- `connectSession` with `vendor === 'trezor'` calls `getAccountXpub` then `walletSessionInitWatchOnly`; with
  `vendor === 'mnemonic'` the 3.7 path is unchanged; with `vendor === 'mock'` neither init is called.
- `useAdminWalletCapability` exposes `canSign === false` for a watch-only session; the broadcast/commit affordance
  renders disabled with "Hardware wallet required to sign".
- HW init failure is non-fatal: login still completes and the panel shows `Disabled`.

**Authority isolation**

- Not applicable to wallet derivation. Session init runs only after a successful role authentication (unchanged).

## Module structure

Each file keeps one reason to change:

- `infrastructure/admin_wallet/wallet.rs` — **pure wallet derivation** (mnemonic *and* xpub variants); no session
  or RPC knowledge. `load_watch_only_admin_wallet` sits beside `load_admin_wallet` (same dependency direction:
  session → derivation).
- `infrastructure/hw_wallet/{trezor,ledger}.rs` — **device transport**; new `get_account_xpub` is one more
  device read alongside the existing connect/list/sign operations.
- `application/wallet_service.rs` — **one wallet's behavior + capability**; `can_sign` is intrinsic to the service,
  not derived from env.
- `application/wallet_session.rs` — **slot lifecycle**; now fills from either a mnemonic or an xpub. The only owner
  of "what kind of session is live".
- `infrastructure/broadcast_env.rs` — **non-secret broadcast config + key resolution policy**; gains the
  watch-only (`ReadOnly`) branch; never reads the device or derives keys from an xpub.
- `commands/{admin_wallet,hw_wallet}.rs` — **thin IPC boundary**; snapshot/delegate/serialize.
- `commands/invoke.rs` — **handler-registration composition root**; the watch-only init and capability/xpub reads
  are present in production (no secret), while the mnemonic init remains dev-gated.
- Frontend `api/admin-wallet.ts` (transport), `wallet/*-adapter.ts` (device access), `contexts/session-provider.tsx`
  (orchestration: which init to run), `domain/admin-wallet/hooks` (capability state). No new secret storage anywhere.

Dependency direction: `WalletSession` → (`WalletService`, `load_watch_only_admin_wallet`); IPC commands →
`WalletSession`; nothing in `infrastructure` depends on `commands`/`application` session layers.

---

## Design appendix — decision trade-offs

### D1. How the HW fills the slot — dedicated command vs `auth_complete`

The plan text says "extend `auth_complete` (HW path)". Phase 3.7 deliberately chose (its decision **D2**) **not** to
couple wallet init to the auth verifier, because mnemonics/secrets flowing through the auth path widen the leak
surface, and a dedicated session-init command is the forward-compatible extension point. Phase 3.8 follows that
established pattern: a `wallet_session_init_watch_only` command mirrors `wallet_session_init`. The only difference
from the mnemonic command is that it **carries no secret**, so it is registered in production builds too. This keeps
auth and wallet concerns separate and makes the mnemonic/xpub branches symmetric in `connectSession`.

### D2. Capability flag vs descriptor introspection

BDK does not cheaply expose "does this wallet hold private keys". Rather than parse descriptors at every signing
call, the capability is recorded once at construction (`new` = signing, `new_watch_only` = read-only). This makes
`can_sign()` O(1), keeps all existing `WalletService::new(wallet)` call sites and tests compiling unchanged, and
makes the read-only invariant explicit and testable.

### D3. Descriptor shape — plain xpub vs key origin

For read-only balance/address derivation, `tr(xpub/0/*)` is sufficient and minimal. Embedding key origin
(`tr([fp/86h/0h/73h]xpub/0/*)`) is only required when BDK must build a PSBT that a hardware device can match to its
own key — i.e. Phase 7. Capturing the master fingerprint now would add an extra device round-trip for no Phase 3.8
benefit. Chosen: plain xpub now; Phase 7 adds origin when it adds PSBT signing.

### D4. Commit/reveal key for a watch-only session

A watch-only session has no private material, so it cannot hold a commit/reveal keypair. `SessionState`'s
`commit_reveal_keypair` becomes `Option`; the mnemonic path stores `Some`, the xpub path stores `None`. This is the
smallest change that keeps the invariant "a watch-only session can never produce a signing key" true by
construction, and it composes with the already-`Option`-returning `commit_reveal_keypair()` accessor.

### D5. A distinct `ReadOnly` error vs reusing `Disabled` / `WalletSessionRequired`

Three states are genuinely different to the signer and must read differently in the UI:

| State | Meaning | Error |
|-------|---------|-------|
| Env not enabled | Wrong environment (not regtest / dev signing off) | `Disabled` |
| No active session | Logged out | `WalletSessionRequired` |
| Watch-only session | HW logged in, signing not yet wired | `ReadOnly` → "Hardware wallet required to sign" |

Collapsing `ReadOnly` into `Disabled` would tell a HW user to change env vars (wrong); collapsing into
`WalletSessionRequired` would tell them to log in (they already are). A distinct variant is warranted.

### D6. Read-path enablement vs the regtest dev guard

`WalletService::sync()`/`fund_commit()` call `check_enabled()`, which requires `BITCOIN_NETWORK=regtest` **and**
`ALLOW_DEV_MNEMONIC_SIGNING=1`. A watch-only read path arguably should not depend on a *signing* flag, but
rederiving that guard touches every wallet IPC path and the broader network story (testnet/mainnet presets) owned by
Phase 9. To keep Phase 3.8 tightly scoped, the watch-only wallet syncs under the existing regtest guard (demoable on
regtest, where `ALLOW_DEV_MNEMONIC_SIGNING=1` is already set). Decoupling read-enablement from sign-enablement is
explicitly deferred and recorded here so the assumption is not lost.

### D7. Account-xpub coin type — `0'` for both devices

`load_admin_wallet` uses `m/86h/0h/73h`. The Ledger Admin-ID code path uses coin type `1'` (`m/84'/1'/73'`), a
testnet convention for the *auth* key. If that convention leaked into the Admin Wallet xpub fetch, the watch-only
wallet would derive different addresses than the mnemonic wallet for the same seed, breaking the read-equivalence
the panel promises. Both devices therefore fetch the Admin Wallet account xpub at `m/86'/0'/73'` (coin type `0'`),
asserted by the watch-only ≡ mnemonic test (D7 anchor).

### D8. `canSign` exposure — dedicated pure command vs piggyback on sync

`get_admin_wallet_info` triggers a sync (RPC); a capability check must not. `admin_wallet_can_sign` reads the slot
synchronously with no RPC, so the FE can render the correct affordance immediately after login, before/independent
of any sync. It also avoids growing `SyncStatusDto` with a non-sync concept.
