# Spec: Admin Wallet Core Read Path (Phase 2)

> **⚠️ Guard condition updated in Phase 3.6** ([`admin-wallet-commit-funding-only.md`](./admin-wallet-commit-funding-only.md)).
> References below to `COMMIT_FUNDING=admin_wallet` as an enablement condition are obsolete: the
> `COMMIT_FUNDING` env var was removed. `AdminWalletError::Disabled` / `WalletService::check_enabled()`
> now gate on `BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1` only. The read-path design is otherwise unchanged.

## Objective

Expose BDK-backed **read APIs** for the Admin Wallet over Tauri IPC so that the desktop app — and the Phase 3 WalletPanel that will consume them — can observe balance, UTXOs, derived addresses, and chain-sync status without ever leaving the Rust process for secrets. This is the **data backbone** of the Admin Wallet program: no Send, no signing, no UI shell beyond a small extension of Phase 1's broadcast card.

This spec is Phase 2 of the Admin Wallet program. It introduces a `WalletService` over the descriptors and BDK wallet established in Phase 1, and adds typed IPC commands plus thin React hooks that Phase 3 will reuse.

**Related:** [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md) · [Phase 1 spec (US-H7)](./admin-wallet-regtest-commit-funding.md) · [Feature delta](../archive/features/admin-wallet-core-read-path/feature-delta.md)

## Scope

### Included

- `WalletService` (new) in `desktop-app/src-tauri/src/application/wallet_service.rs` — orchestrates BDK sync and exposes pure read APIs over Phase 1's `infrastructure/admin_wallet/wallet.rs`.
- Typed Tauri IPC commands (extending `commands/admin_wallet.rs`):
  - `admin_wallet_get_balance`
  - `admin_wallet_list_utxos`
  - `admin_wallet_list_addresses`
  - `admin_wallet_sync`
  - `admin_wallet_sync_status`
- Background chain-sync task: 30s cadence while at least one read IPC has been called in the past 5 minutes; idles otherwise.
- React hooks under `desktop-app/src/domain/admin-wallet/hooks/`:
  - `useAdminWalletBalance`
  - `useAdminWalletUtxos`
  - `useAdminWalletAddresses(keychain, page)`
  - `useAdminWalletSync()`
- Minimal UI extension of Phase 1's `BroadcastDetailsCard` to render `UTXOs: <N>` and `Last sync: <relative>` when `COMMIT_FUNDING=admin_wallet`.
- Error taxonomy for the read path: `RpcUnreachable`, `RpcAuthFailed`, `DescriptorParseError`, `SyncIncomplete`, `RegtestGuardViolation`, `Disabled`.
- Tests: unit for derivation/index invariants and DTO shaping; integration against the Phase 1 regtest harness; frontend hook tests with mocked IPC.

### Not included

- Send / PSBT build / sign / broadcast — Phase 4.
- Fee-bump / RBF — Phase 5.
- Receive rotation policy and Admin ID display — Phase 6.
- Hardware wallet adapters (Trezor/Ledger) — Phase 7.
- Governance broadcast UX refactor / shared Send chrome — Phase 8.
- Remote testnet/mainnet RPC hardening, TLS, network presets — Phase 9.
- Full WalletPanel (Balance/Addresses/Transactions/Receive/Send tabs) — Phase 3.
- BDK state persistence across restarts.
- New environment variables (Phase 2 reuses Phase 1's set exactly).
- Changes to `application/commit_funding.rs` external API or `application/proposals.rs::broadcast_commit_then_reveal`.
- SPS-50/51/65 validation or protocol changes — never in this program.

## Requirements Alignment

- **Authorities:** Strata Administrator and Alpen Administrator only (current program scope).
- **Two-key model unchanged** from Phase 1: Admin ID (`m/84'/0'/73'/0/0`, P2WPKH) authenticates and signs SPS-65 messages; **Admin Wallet** (`m/86'/0'/73'/n/n`, P2TR) is the BTC custody layer.
- **Backend remains coordination-only** per `proposal-broadcast-commit-reveal.md` — Phase 2 changes nothing about commit/reveal or orchestrator state.
- **Secrets stay in Rust.** The mnemonic and any derived xprv never cross the IPC boundary; the React layer sees only addresses, sats, outpoints, and status fields.
- **Signer safety carry-over:** all read APIs are side-effect-free; only `admin_wallet_sync` issues outbound RPC traffic, and it is rate-collapsed (see §Technical Design).

## State Model

Phase 2 does **not** introduce or modify protocol state. It introduces an in-process **wallet sync state**:

| State field | Source | Lifetime |
|---|---|---|
| `tip_height` | BDK chain client | Refreshed every sync |
| `last_synced_block` | BDK wallet local chain | Refreshed every sync |
| `last_synced_at` | Rust `chrono::Utc::now()` after a successful sync | Refreshed every sync |
| `is_syncing` | `WalletService` in-flight flag | Toggled while a sync future is pending |
| `last_error` | Most recent typed sync failure, cleared on next success | Reset on success |

Persistence: **none** in Phase 2. The state is rebuilt by re-syncing from chain on app launch (same posture as Phase 1).

## Product Flow

### Entry

Unchanged from Phase 1. The user lands on `/proposals/:actionId/broadcast` for an `approved` proposal with `COMMIT_FUNDING=admin_wallet`.

### Phase 2 surface (broadcast card)

- The existing `BroadcastDetailsCard` already shows funding mode, Admin Wallet address, and balance from Phase 1's `get_admin_wallet_info`.
- Phase 2 adds two fields to the same card when funding mode is `admin_wallet`:
  - `UTXOs: <N>` — count from `admin_wallet_list_utxos`.
  - `Last sync: <relative time>` — derived from `admin_wallet_sync_status.last_synced_at`, refreshed at least every 15s while the screen is mounted.
- On sync error, the relative-time field is replaced by `Sync error: <typed message>` and the stale timestamp is hidden.

### Background sync

Triggered by the first Phase 2 read IPC after app start. While at least one read IPC has been invoked within the past 5 minutes, the loop runs every 30s. Otherwise it idles to avoid background RPC pressure. The cadence and idle window are Rust constants in Phase 2 (no env knob — see Phase 9 for production hardening).

### Confirm + broadcast

Unchanged from Phase 1. Phase 2's APIs are observational only.

## Technical Design

### Module layout

```
desktop-app/src-tauri/src/
├── application/
│   ├── commit_funding.rs           (Phase 1 — unchanged externally)
│   ├── proposals.rs                (Phase 1 — unchanged)
│   └── wallet_service.rs           (NEW — Phase 2)
├── infrastructure/
│   └── admin_wallet/
│       ├── mod.rs                  (extended re-exports)
│       └── wallet.rs               (extended with read helpers)
└── commands/
    └── admin_wallet.rs             (extended with 5 new IPC commands)
```

```
desktop-app/src/
├── api/
│   └── admin-wallet.ts             (extended adapter — 5 new functions)
└── domain/
    ├── admin-wallet/               (NEW)
    │   └── hooks/
    │       ├── use-admin-wallet-balance.ts
    │       ├── use-admin-wallet-utxos.ts
    │       ├── use-admin-wallet-addresses.ts
    │       └── use-admin-wallet-sync.ts
    └── broadcast-proposal/
        └── components/
            └── broadcast-details-card.tsx  (extended with utxoCount + lastSyncedAt)
```

### `WalletService`

Single owner of the BDK `Wallet` for read concerns. Holds a `tokio::sync::Mutex<Wallet>` and an `Arc<RwLock<SyncState>>`. Spawns the background sync task on first use.

```rust
pub struct WalletService {
    wallet: Arc<Mutex<bdk_wallet::Wallet>>,
    rpc: Arc<bdk_bitcoind_rpc::Emitter<...>>,
    sync_state: Arc<RwLock<SyncState>>,
    sync_in_flight: Arc<AtomicBool>,        // collapses concurrent sync calls
    last_read_at: Arc<RwLock<Option<Instant>>>, // gates the background loop
}

impl WalletService {
    pub async fn get_balance(&self) -> Result<BalanceDto, AdminWalletError>;
    pub async fn list_utxos(&self) -> Result<Vec<UtxoDto>, AdminWalletError>;
    pub async fn list_addresses(
        &self,
        keychain: Keychain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Vec<AddressDto>, AdminWalletError>;
    pub async fn sync(&self) -> Result<SyncStatusDto, AdminWalletError>;
    pub fn sync_status(&self) -> SyncStatusDto;
}
```

Read methods acquire the wallet mutex, **copy** the required data into the DTO, and release. They never await network I/O while holding the lock. `sync()` holds the lock for the full chain-emit cycle; concurrent callers collapse via `sync_in_flight`.

### IPC contracts

All DTOs derive `serde::{Serialize, Deserialize}` and `specta::Type` (or equivalent) for typed TS bindings.

```rust
// Returned by admin_wallet_get_balance
pub struct BalanceDto {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
    pub total_sats: u64,
}

// Returned by admin_wallet_list_utxos
pub struct UtxoDto {
    pub outpoint: OutPointDto,        // { txid: String, vout: u32 }
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub keychain: KeychainDto,         // "external" | "internal"
    pub derivation_index: u32,
    pub confirmations: u32,            // derived from BDK chain state, NOT gettxout
}

// Returned by admin_wallet_list_addresses
pub struct AddressDto {
    pub index: u32,
    pub address: String,
    pub is_used: bool,                 // BDK heuristic: any tx in derivation index
}

// Returned by admin_wallet_sync and admin_wallet_sync_status
pub struct SyncStatusDto {
    pub tip_height: Option<u32>,
    pub last_synced_block: Option<u32>,
    pub last_synced_at: Option<String>,  // ISO-8601
    pub is_syncing: bool,
    pub last_error: Option<TypedError>,
}

pub enum AdminWalletError {
    RpcUnreachable { message: String },
    RpcAuthFailed { message: String },
    DescriptorParseError { message: String },
    SyncIncomplete { message: String },
    RegtestGuardViolation { message: String },
    Disabled,                            // mode/env not active
}
```

`AdminWalletError::Disabled` is returned by every Phase 2 IPC command when `COMMIT_FUNDING != admin_wallet` OR `BITCOIN_NETWORK != regtest` OR `ALLOW_DEV_MNEMONIC_SIGNING != 1`. This guarantees the React layer can render a single coherent "wallet disabled" state regardless of which command was invoked.

### Address listing window

Per the plan and US-B2, Phase 2 exposes a **fixed window of 20 addresses per keychain**, paged. BDK's internal `gap_limit` keeps its default for sync correctness — the 20-window is a UI presentation cap, not a derivation cap.

- `page_size` defaults to and is capped at `20`.
- `page_index = 0` returns indices `0..=19` of the requested keychain.
- Out-of-bound page returns `[]` (not an error).

### Confirmations source

Computed from BDK chain state: `confirmations = tip_height.saturating_sub(utxo_height).saturating_add(1)` when the UTXO is confirmed, else `0`. **RPC `gettxout` is not used** — keeping BDK as single source of truth avoids divergent counts between the cached chain and live RPC.

### Concurrency model

- One `tokio::sync::Mutex<Wallet>` per `WalletService` instance.
- Read commands: acquire lock → snapshot DTO → release. Always returns the last successful state, even while a sync is in flight.
- `admin_wallet_sync`: checks `sync_in_flight`. If `true`, awaits the in-flight future. If `false`, sets the flag, acquires the wallet mutex, performs `bdk_bitcoind_rpc::Emitter` cycle, updates `SyncState`, releases.
- Acceptable at regtest scale and single-user desktop usage. Revisit in Phase 9 when remote-RPC latency may justify a snapshot-based read path.

### Background sync loop

```text
loop {
    sleep(SYNC_INTERVAL);                       // const = 30s
    let last_read = *last_read_at.read().await;
    if last_read.map_or(false, |t| t.elapsed() < SYNC_IDLE_WINDOW) {   // const = 5 min
        let _ = self.sync().await;              // errors recorded in SyncState
    }
}
```

The loop is spawned on first IPC invocation, not on app start, to avoid unnecessary chain access for users who never open the broadcast screen.

### Env and guards

Phase 2 introduces **no new env vars**. Reused exactly from Phase 1:

| Variable | Role in Phase 2 |
|---|---|
| `BITCOIN_RPC_URL` | Chain RPC base URL (BDK `Emitter`) |
| `BITCOIN_RPC_USER` / `BITCOIN_RPC_PASS` | RPC auth |
| `BITCOIN_NETWORK` | Must be `regtest`; otherwise IPC returns `Disabled` |
| `ADMIN_WALLET_REGTEST_MNEMONIC` | Descriptor secret for the BDK wallet |
| `ALLOW_DEV_MNEMONIC_SIGNING` | Existing dev-secret guard |
| `COMMIT_FUNDING` | Must equal `admin_wallet` for Phase 2 reads to be enabled |

### Tauri wiring

Register the 5 new commands in `commands/invoke.rs` alongside Phase 1's `get_admin_wallet_info`. Construct one `WalletService` in Tauri `setup` and place it in managed state; commands resolve it via `tauri::State<Arc<WalletService>>`.

### React hooks

All hooks return `{ data, isLoading, error, refresh }` with `data` typed against the IPC schema. `useAdminWalletSync` additionally exposes `triggerSync()` and is the **only** hook that calls `admin_wallet_sync` — others subscribe to sync-status changes for cache invalidation.

Hooks live under `desktop-app/src/domain/admin-wallet/hooks/` mirroring the Phase 1 `domain/broadcast-proposal/hooks/` layout. The IPC adapter (`api/admin-wallet.ts`) is extended with one function per new command.

### Phase 1 surface extension

`BroadcastDetailsCard` gets two new optional props:

```ts
interface BroadcastDetailsCardProps {
  // ...existing props
  utxoCount?: number;
  lastSyncedAt?: string | null;
  syncError?: AdminWalletErrorDto | null;
}
```

`broadcast-proposal-screen.tsx` is extended to consume `useAdminWalletUtxos` and `useAdminWalletSync` when `useAdminWalletInfo` returns non-null, and to pass the new props through. When funding mode is `bitcoind` (or unset), both new props are `undefined` and the card renders byte-identical to Phase 1.

## API Contract (orchestrator)

Unchanged from Phase 1. Phase 2 adds no orchestrator endpoints.

## Test Plan

| Layer | What to verify |
|---|---|
| Unit (Rust) | DTO shaping: BDK `LocalUtxo` → `UtxoDto` confirmation arithmetic on tip / spent / unconfirmed cases |
| Unit (Rust) | Address windowing: `list_addresses(external, 0, 20)` returns indices `0..=19`; `list_addresses(internal, 1, 20)` returns indices `20..=39`; out-of-bound page returns empty |
| Unit (Rust) | `AdminWalletError::Disabled` returned when any guard fails (`COMMIT_FUNDING != admin_wallet` / `BITCOIN_NETWORK != regtest` / `ALLOW_DEV_MNEMONIC_SIGNING != 1`) — once per IPC |
| Unit (Rust) | Concurrent `sync()` calls collapse into one in-flight future (assert single `Emitter` cycle observed) |
| Integration (regtest) | Reusing Phase 1 harness: fund `m/86'/0'/73'/0/0` with `sendtoaddress`, mine 1 block → `admin_wallet_get_balance` returns confirmed sats; `admin_wallet_list_utxos` lists 1 UTXO with `confirmations == 1`; `admin_wallet_sync_status.last_synced_block == tip_height` |
| Integration (regtest) | Stop bitcoind → `admin_wallet_sync` returns `AdminWalletError::RpcUnreachable`; reads still return the prior snapshot |
| Regression (Phase 1) | With `COMMIT_FUNDING` unset, broadcast card renders byte-identical to pre-Phase-2; existing Phase 1 tests pass without change |
| Frontend (TS) | Hook unit tests with mocked IPC: `Disabled` error surfaced; `triggerSync` exposed only by `useAdminWalletSync`; `useAdminWalletAddresses` defaults `page_size=20`, caps at 20 |
| Frontend (TS) | `BroadcastDetailsCard` renders `UTXOs: N` and `Last sync: …` only when `utxoCount` and `lastSyncedAt` are non-`undefined` |

## Manual Fallback

Phase 2 introduces no manual fallback path — it adds read-only observability. The Phase 1 manual hex export from the existing broadcast flow remains available; nothing in Phase 2 changes it.

## Open Questions Resolved (from §Phase 2 plan)

| # | Question | Resolution |
|---|---|---|
| 1 | Sync trigger model | Both: background 30s loop (gated by 5-min idle window) + explicit `admin_wallet_sync` IPC |
| 2 | Address listing window | Fixed 20 per keychain, paged; US-B2 alignment |
| 3 | UTXO confirmations | BDK chain state (`tip - utxo_height + 1`); no RPC `gettxout` |
| 4 | Concurrency | `tokio::sync::Mutex<Wallet>`, short-snapshot reads, `sync_in_flight` collapses concurrent syncs |
| 5 | Env vars | Reuse Phase 1 exactly; no new env in Phase 2 |
| 6 | Module naming | Introduce `application/wallet_service.rs` as the read-API owner; `commit_funding.rs` unchanged externally; Phase 4 Send will extend `WalletService` and `CommitFunding` may delegate to it later |

## Amendment (R1.5 — balance UX)

`WalletService::do_sync` originally synced **blocks only** (`Emitter::next_block` + `apply_block_connected_to`). That left `BalanceDto.unconfirmed_sats` at zero and prevented mempool-driven receive rotation until a block was mined.

**R1.5 addition:** after the block loop, `do_sync` calls `Emitter::mempool()` and `wallet.apply_unconfirmed_txs(...)`. IPC contracts and DTO shapes are unchanged; `admin_wallet_get_balance` and `admin_wallet_sync` behavior now reflect mempool credits/spends before confirmation.

See [`admin-wallet-balance-ux.md`](./admin-wallet-balance-ux.md) and [`2026-06-03-admin-wallet-balance-ux.md`](../evolution/2026-06-03-admin-wallet-balance-ux.md).

## Amendment (R1.6 — addresses UX)

The Phase 2 read path already returns `UtxoDto.confirmations` on each UTXO. R1.6 does **not** change IPC or DTOs; the
desktop app splits external UTXOs per derivation index into `confirmedSats` and `unconfirmedSats` in
`groupUtxoBalancesByDerivation` / `composeAddressesWithBalance` and renders per-address unconfirmed sub-lines in the
wallet panel (PRD §4.3.2). Mempool visibility for unconfirmed UTXOs on regtest depends on the R1.5 `do_sync` amendment
above.

See [`admin-wallet-addresses-ux.md`](./admin-wallet-addresses-ux.md) and [`2026-06-03-admin-wallet-addresses-ux.md`](../evolution/2026-06-03-admin-wallet-addresses-ux.md).

## Planned change (Release 2 — Electrum sync)

**Release 2** replaces Core RPC block-scan sync (`bdk_bitcoind_rpc::Emitter` in `WalletService::do_sync`) with **Electrum** (`bdk_electrum`) for wallet indexation. Delivered in slices **R2.1** (electrs infra) → **R2.2** (sync migration) → **R2.3** (Node Config URL). IPC contracts and wallet panel UX (R1.2–R1.7) remain; broadcast and fees stay on chain RPC. See [`admin-wallet-electrum-sync.md`](./admin-wallet-electrum-sync.md).

## Links

- Program phases: [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md)
- Phase 1 spec (precursor): [`admin-wallet-regtest-commit-funding.md`](./admin-wallet-regtest-commit-funding.md)
- Phase 2 requirements & user stories (DISCUSS): [`../feature/admin-wallet-core-read-path/feature-delta.md`](../archive/features/admin-wallet-core-read-path/feature-delta.md)
- Protocol broadcast (unchanged): [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md)
- PRD references: `docs/0-prd/03-prd-update.md` §4.1–4.2
- Story map: [`../3-stories/story-map.md`](../3-stories/story-map.md)
