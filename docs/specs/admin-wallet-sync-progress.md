# Spec: Admin Wallet sync progress indicator

## Objective

When the Admin Wallet syncs against the Bitcoin node, it scans the chain block-by-block via
BDK's `Emitter` (`do_sync()` in `desktop-app/src-tauri/src/application/wallet_service.rs`,
`Emitter::new(&rpc, checkpoint, 0)` looping over `emitter.next_block()`). Against a remote
regtest node with ~1300 blocks and ~400 ms/RPC-call latency, a full sync takes several minutes.
During that time the UI shows only a static "Refreshing…" with no progress, so the signer
cannot tell whether the wallet is working or hung.

Surface **live sync progress** (processed / total blocks and percent) so the user knows the sync
is advancing. The progress state must appear **only after the in-flight sync has been running for
more than 3 seconds**, so fast syncs against a local node do not flash a progress indicator
(avoid flicker).

## Scope

**Included**

- Backend (`WalletService`): track blocks processed and the target block total during `do_sync()`,
  track when the current sync started, and expose progress through the existing
  `sync_status()` / `SyncStatusDto` path — but only when a sync is in flight **and** has been
  running for more than the 3-second threshold.
- Frontend: extend the `SyncStatusDto` TS type with the progress field, add a pure label helper,
  poll sync status while a sync is in flight, and render a meaningful progress label in `SyncChip`.

**NOT included**

- Changing the sync algorithm (no parallel block fetch, no checkpoint seeding from a known height).
- Any change to broadcast or signing flows.
- A DOM-rendering test harness (vitest / @testing-library) — frontend logic is tested as pure
  functions per the existing `tsx` + `node:assert` convention.

## Technical Design

### Backend — `desktop-app/src-tauri/src/application/wallet_service.rs`

**New constant**

```rust
/// A sync must run longer than this before progress is surfaced to the UI.
/// Below this threshold, fast (local-node) syncs complete without ever showing a progress
/// indicator, avoiding a flicker on every refresh.
const SYNC_PROGRESS_THRESHOLD_MS: u64 = 3_000;
```

**New DTO** (next to the existing `BalanceDto` / `SyncStatusDto`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressDto {
    pub processed_blocks: u32,
    pub total_blocks: u32,
    pub percent: u8,
}
```

**`SyncStatusDto` gains one field:**

```rust
pub sync_progress: Option<SyncProgressDto>, // None when idle or under the 3s threshold
```

`disabled_default()` and the lock-contended fallback branch in `sync_status()` set
`sync_progress: None`.

**New `WalletService` fields (lock-free, `Arc<Atomic*>`):**

```rust
sync_blocks_processed: Arc<AtomicU32>, // blocks applied in the current sync (reset at start)
sync_blocks_total:     Arc<AtomicU32>, // target block count for the current sync (set by do_sync)
sync_started_at_ms:    Arc<AtomicU64>, // UNIX-epoch millis when current sync started; 0 = idle
```

These are atomics specifically so `sync_status()` can read them **without taking any lock the
sync loop already holds**. The sync loop holds `Mutex<bdk_wallet::Wallet>` while applying blocks;
it must never block on a progress counter. Reads use `Ordering::Relaxed`.

**New pure helper (free function, unit-testable, no I/O):**

```rust
/// Percent complete, clamped to 0..=100. Returns 100 when `total == 0` (nothing to scan).
fn percent_complete(processed: u32, total: u32) -> u8;
```

**New private helper:**

```rust
/// Current wall-clock time as UNIX-epoch milliseconds (used for the elapsed-since-start check).
fn now_unix_ms() -> u64;
```

**Lifecycle (in `sync()` and `do_sync()`):**

- `sync()` — after winning the `sync_in_flight` `compare_exchange`:
  - `sync_started_at_ms = now_unix_ms()`, `sync_blocks_processed = 0`, `sync_blocks_total = 0`.
  - run `do_sync()`.
  - on completion (Ok **or** Err): `sync_in_flight = false` **and** `sync_started_at_ms = 0`
    (clearing the start time makes progress disappear immediately when the sync ends).
- `do_sync()`:
  - after creating the RPC client, call `rpc.get_block_count()` (via `RpcApi`) to learn the target
    tip; `total = target.saturating_sub(checkpoint.height() as u64)`, stored (saturating) into
    `sync_blocks_total`.
  - inside the block loop, after each successful `apply_block_connected_to`, increment
    `sync_blocks_processed` by 1 (`fetch_add(1, Relaxed)`).
  - `get_block_count()` failure maps to the existing `rpc_error_from_message(...)` (typed
    `AdminWalletError`); no panic / no `.unwrap()`.

**`sync_status()` — progress gate:**

```rust
let is_syncing = self.sync_in_flight.load(Relaxed);
let started    = self.sync_started_at_ms.load(Relaxed);
let elapsed    = now_unix_ms().saturating_sub(started);
let sync_progress = if is_syncing && started != 0 && elapsed > SYNC_PROGRESS_THRESHOLD_MS {
    let processed = self.sync_blocks_processed.load(Relaxed);
    let total     = self.sync_blocks_total.load(Relaxed);
    Some(SyncProgressDto { processed_blocks: processed, total_blocks: total,
                           percent: percent_complete(processed, total) })
} else {
    None
};
```

### Frontend

**`desktop-app/src/api/admin-wallet.ts`** — extend the transport DTO:

```ts
export type SyncProgressDto = {
    processedBlocks: number
    totalBlocks: number
    percent: number
}

export type SyncStatusDto = {
    tipHeight: number | null
    lastSyncedBlock: number | null
    lastSyncedAt: string | null
    isSyncing: boolean
    lastError: { code: string; message: string } | null
    syncProgress: SyncProgressDto | null // present only mid-sync after the >3s threshold
}
```

**`desktop-app/src/domain/admin-wallet/model/format-sync-progress-label.ts`** — new pure helper
(single responsibility: format a `SyncProgressDto` into a human label):

```ts
// "Syncing 234 / 1,277 blocks (18%)" — thousands grouping via toLocaleString('en-US').
export function formatSyncProgressLabel(progress: SyncProgressDto): string
```

**`desktop-app/src/domain/admin-wallet/model/types.ts`** — re-export `SyncProgressDto` alongside
the existing `SyncStatusDto` re-export.

**`desktop-app/src/domain/admin-wallet/model/__fixtures__/make-sync-status.ts`** — add
`syncProgress: null` to the default fixture.

**`desktop-app/src/domain/admin-wallet/components/sync-chip.tsx`** — label selection:

- When `isRefreshing && syncStatus?.syncProgress != null` → label = `formatSyncProgressLabel(...)`,
  rendered in a span with `data-testid="e2e-wallet-sync-progress"`.
- Otherwise → existing behavior unchanged (error message / relative time / "Never synced",
  button text "Refreshing…" / "Refresh").
- All existing `data-testid` attributes preserved.

**`desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-sync.ts`** — live polling:

- Add `isSyncing` boolean state. `triggerSync` sets it `true` at the start and `false` in a
  `finally` block.
- Add a `useEffect` keyed on `isSyncing`: while `true`, poll `getAdminWalletSyncStatus()` every
  `SYNC_POLL_INTERVAL_MS = 1000` and `setSyncStatus(...)`; the cleanup clears the interval.
- This is required because `triggerSync()` `await`s the entire `admin_wallet_sync` call (which only
  returns when the full scan finishes); without polling the UI never observes intermediate progress.
- **Constraint:** this effect is internal to `useAdminWalletSync` and keyed on its own `isSyncing`
  state. It must NOT be wired into the on-open `syncAndRefresh` effect deps in
  `use-wallet-panel-data.ts` (those deps must stay referentially stable callbacks only, or the
  panel re-fires its open effect and loops — see prior PRs #220/#222/#223).

### Production code vs. test helpers

- **Production functions:**
  - Rust: `percent_complete`, `now_unix_ms`, the modified `sync()` / `do_sync()` / `sync_status()`,
    `SyncProgressDto`, extended `SyncStatusDto`.
  - TS: `formatSyncProgressLabel`, extended `SyncStatusDto` / new `SyncProgressDto` types, polling in
    `useAdminWalletSync`, label branch in `SyncChip`.
- **Test helpers:** `make-sync-status.ts` fixture (already test-only, under `__fixtures__/`). No new
  Tauri command is registered; the progress flows through the existing `admin_wallet_sync_status`
  command unchanged. Backend threshold tests drive the private atomics directly from the same-file
  `#[cfg(test)] mod tests` (no production-exposed setter).

## Test Cases

### Backend (Rust, `cargo test`, no RPC required)

1. `percent_complete(234, 1277) == 18` — truncating integer arithmetic.
2. `percent_complete(0, 0) == 100` — divide-by-zero guard (nothing to scan).
3. `percent_complete(1277, 1277) == 100` — full.
4. `percent_complete(200, 100) == 100` — clamp on over-count.
5. `percent_complete(50, 100) == 50` — midpoint.
6. **Progress reported only after >3s:** with `sync_in_flight = true`,
   `sync_started_at_ms = now - 4000`, `processed = 234`, `total = 1277` →
   `sync_status().sync_progress == Some(SyncProgressDto { 234, 1277, 18 })`.
7. **Hidden under threshold:** same atomics but `sync_started_at_ms = now - 1000` →
   `sync_progress == None`.
8. **Hidden when not syncing:** `sync_in_flight = false` (regardless of counters) →
   `sync_progress == None`.
9. **DTO shape:** `SyncStatusDto::disabled_default().sync_progress == None`; `SyncProgressDto`
   serializes to camelCase (`processedBlocks` / `totalBlocks` / `percent`).

### Frontend (TS, `tsx` + `node:assert`)

10. `formatSyncProgressLabel({ processedBlocks: 234, totalBlocks: 1277, percent: 18 })` ===
    `"Syncing 234 / 1,277 blocks (18%)"` — verifies thousands grouping and template.
11. `formatSyncProgressLabel({ processedBlocks: 0, totalBlocks: 0, percent: 100 })` produces a
    well-formed string (no `NaN`, no `undefined`).
12. **DTO shape (TS):** `makeSyncStatus({ syncProgress: { processedBlocks: 1, totalBlocks: 2,
    percent: 50 } })` type-checks and round-trips; default `makeSyncStatus().syncProgress === null`.

> Live-polling behavior (the `useAdminWalletSync` interval) and the `SyncChip` DOM render are not
> unit-tested here — no DOM/timer test harness exists in the project (consistent with the
> `BroadcastDetailsCard` BLOCKED_BY_DEPENDENCY note). The pure label helper (10–11) plus the backend
> threshold tests (6–8) cover the testable logic; rendering is verified manually / via WebDriver.

### CI registration

- Backend tests run under the existing `cargo test --workspace`.
- Add an npm script `test:model-format-sync-progress-label` and register it in
  `.github/workflows/ci.yml` next to the other admin-wallet contract tests.

## Module structure

- `application/wallet_service.rs` — **single responsibility:** the Admin Wallet's BDK service
  (sync lifecycle, reads, signing). Gains the progress atomics, `SyncProgressDto`, `percent_complete`,
  and `now_unix_ms`. This file already holds the sibling DTOs (`BalanceDto`, `SyncStatusDto`); the new
  DTO and helper stay co-located for cohesion. (It is already >200 production lines; extracting all
  DTOs into a `wallet_dtos.rs` module is a possible Phase 6 refactor but is out of scope as a
  behavior change.)
- `domain/admin-wallet/model/format-sync-progress-label.ts` — **single responsibility:** format a
  sync-progress DTO into a display string. Pure, no React, no I/O.
- `domain/admin-wallet/hooks/use-admin-wallet-sync.ts` — **single responsibility:** own sync state
  and side effects (trigger + status polling) for the Admin Wallet.
- `domain/admin-wallet/components/sync-chip.tsx` — **single responsibility:** presentational sync
  status chip; selects a label and emits a refresh intent. No business logic beyond label selection.

**Dependency direction:** the presentational `SyncChip` and the hook depend on the pure
`formatSyncProgressLabel` helper and on the transport types in `api/admin-wallet.ts` (re-exported via
`model/types.ts`), not the reverse. The pure helper depends only on the `SyncProgressDto` type.
