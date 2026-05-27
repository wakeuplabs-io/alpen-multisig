## Wave: DISCUSS / [REF] Persona ID

**Signer (Strata Administrator or Alpen Administrator)** — operates the desktop app on regtest, already authenticated for an authority, currently viewing the broadcast screen of an `approved` proposal (US-H6/H7). In Phase 2, the signer's experience is unchanged except that the broadcast surface shows richer Admin Wallet state (UTXO count + last sync timestamp) sourced from the new read APIs.

Secondary consumer: **Phase 3 UI developer** — needs typed React hooks and IPC contracts to build the WalletPanel without re-inventing data plumbing. Not an end user; treated as an internal stakeholder so Phase 3 design has a stable surface to consume.

## Wave: DISCUSS / [REF] JTBD One-liner

`job_id: infrastructure-only`
**infrastructure_rationale:** Phase 2 produces the data backbone (BDK read APIs over IPC + thin React hooks) for the Admin Wallet. It has no end-user behavior of its own — by design, Phase 3 owns the WalletPanel that renders this data. The only user-visible diff in Phase 2 is a richer Phase 1 broadcast card (UTXO count + last sync), which exists solely to satisfy the slice composition gate (no slice may ship 100% `@infrastructure`). This rationale is recorded explicitly per the DISCUSS infrastructure-only escape valve.

## Wave: DISCUSS / [REF] Locked Decisions

| ID | Decision | Verdict | Source |
|---|---|---|---|
| D1 | Module ownership: introduce `application/wallet_service.rs` (read APIs over BDK). `infrastructure/admin_wallet/wallet.rs` (Phase 1) extended with pure read helpers. `application/commit_funding.rs` unchanged externally; may delegate to `WalletService` in Phase 4 | ✅ | Plan §Phase 2 "WalletService read APIs"; predecessor module layout |
| D2 | Sync model: both — background loop (default 30s, configurable, suspended when no read API has been called in the last N seconds) + explicit `admin_wallet_sync` IPC for pull-to-refresh | ✅ | Phase 1 already syncs on demand; Phase 3 will need staleness signaling |
| D3 | Address listing: fixed window of 20 indices per keychain (external + internal), paged (`page_index`, `page_size=20`). BDK `gap_limit` default stays for sync. Aligned with US-B2 ("first 20 addresses") | ✅ | Story map US-B2; PRD §4.1–4.2 |
| D4 | UTXO confirmations: derived from BDK chain state (`tip_height - utxo_height + 1`), not from RPC `gettxout`. Single source of truth = BDK | ✅ | Avoid divergent counts between Rust sync state and RPC view |
| D5 | Concurrency: single `Wallet` behind `tokio::sync::Mutex`. Read commands take a short lock, copy fields into DTOs, release. Sync command takes the lock for the full sync duration. Acceptable at regtest scale; revisit in Phase 9 | ✅ | Phase 1 already serializes wallet access; consistent posture |
| D6 | Env vars: reuse Phase 1 exactly (`BITCOIN_RPC_URL`, `BITCOIN_RPC_USER`, `BITCOIN_RPC_PASS`, `BITCOIN_NETWORK`, `ADMIN_WALLET_REGTEST_MNEMONIC`, `ALLOW_DEV_MNEMONIC_SIGNING`). No new env in Phase 2 | ✅ | Spec scope; no remote-RPC hardening yet (Phase 9) |
| D7 | Demonstrability: extend Phase 1 `BroadcastDetailsCard` with `utxoCount` and `lastSyncedAt` derived from new IPC commands. Full WalletPanel deferred to Phase 3 | ✅ | Slice composition gate (≥1 user-visible value story per slice) |
| D8 | Persistence: no new persistence in Phase 2. BDK state in-memory, re-synced on app start (same posture as Phase 1). Persistence decision deferred to a later phase if start-up sync latency becomes a UX problem | ✅ | Plan §Phase 2 "call out the decision but don't expand it" |
| D9 | Regtest-only enablement: same guard as Phase 1 (`BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1`). Phase 9 owns testnet/mainnet | ✅ | Plan §Phase 9 |
| D10 | IPC naming convention: prefix all new commands with `admin_wallet_*` (snake_case Rust side, camelCase TS adapter). Hooks live under `desktop-app/src/domain/admin-wallet/hooks/` mirroring Phase 1 layout | ✅ | Consistency with Phase 1 |

## Wave: DISCUSS / [REF] User Stories with Elevator Pitches

> All stories use `job_id: infrastructure-only` (see JTBD one-liner). Each story below either targets the extended Phase 1 broadcast surface (user-visible) or is explicitly labelled `@infrastructure`. The slice contains at least one user-visible value story (D7).

### US-RP1 · Show Admin Wallet UTXO count and last-sync timestamp on broadcast screen

**Story.** As a Signer broadcasting an `approved` proposal on regtest with `COMMIT_FUNDING=admin_wallet`, I want the broadcast details card to also display the Admin Wallet UTXO count and the timestamp of the last successful chain sync, so that I can confirm the wallet has fresh state and the funding source has spendable coins before clicking Confirm.

#### Elevator Pitch
Before: signer sees only `funding mode | address | balance` on the broadcast card; cannot tell whether shown balance reflects current chain state or how many coins back it.
After: run `npm run tauri dev` with `COMMIT_FUNDING=admin_wallet` + funded regtest wallet → opens `/proposals/:actionId/broadcast` → sees `Funding Source` block now lists `UTXOs: 3` and `Last sync: 12s ago`.
Decision enabled: signer decides whether to click `Confirm broadcast` now or trigger a manual `Refresh` before committing on-chain.

**Acceptance criteria.**
- AC-1: When `COMMIT_FUNDING=admin_wallet`, `BroadcastDetailsCard` renders `UTXOs: <N>` where `<N>` matches the count returned by `admin_wallet_list_utxos`.
- AC-2: `Last sync: <relative time>` is computed from `admin_wallet_sync_status.last_synced_at`. Updates at least every 15s while the screen is mounted.
- AC-3: When `COMMIT_FUNDING` is unset/`bitcoind`, the card renders unchanged (Phase 1 regression).
- AC-4: If the most recent sync failed, the card surfaces `Sync error: <typed message>` and does NOT show a stale "Last sync" value.

### US-RP2 · Expose Admin Wallet balance via IPC `@infrastructure`

**Story.** As a Tauri client (consumed by US-RP1 today and Phase 3 WalletPanel later), I want an IPC command `admin_wallet_get_balance` returning confirmed and unconfirmed sats, so that the UI never has to read BDK state directly.

**infrastructure_rationale:** No standalone user-visible output; consumed by US-RP1's card and Phase 3.

**Acceptance criteria.**
- AC-1: `admin_wallet_get_balance` returns `{ confirmed_sats: u64, unconfirmed_sats: u64, total_sats: u64 }`.
- AC-2: Values reflect the last successful sync. On a never-synced wallet, all fields are `0` and `admin_wallet_sync_status.last_synced_at` is `null`.
- AC-3: When `COMMIT_FUNDING != admin_wallet` OR `BITCOIN_NETWORK != regtest` + `ALLOW_DEV_MNEMONIC_SIGNING != 1`, the command returns a typed `AdminWalletError::Disabled` (no panic, no leaked descriptor).

### US-RP3 · Expose Admin Wallet UTXO list via IPC `@infrastructure`

**Story.** As a Tauri client, I want `admin_wallet_list_utxos` returning the full UTXO set with provenance, so that Phase 3 can render per-coin detail and US-RP1 can compute the count.

**infrastructure_rationale:** Consumed by US-RP1 (count) and Phase 3 (detail rendering).

**Acceptance criteria.**
- AC-1: Returns `Vec<AdminWalletUtxo>` where each item has `outpoint: { txid, vout }`, `value_sats`, `script_pubkey_hex`, `keychain: "external" | "internal"`, `derivation_index: u32`, `confirmations: u32`.
- AC-2: `confirmations` is computed from BDK chain state (D4), NOT from `gettxout` RPC.
- AC-3: Empty wallet returns `[]`, not error.
- AC-4: Same disabled-mode behavior as US-RP2.AC-3.

### US-RP4 · Expose Admin Wallet address list (paged, 20-window) via IPC `@infrastructure`

**Story.** As a Tauri client, I want `admin_wallet_list_addresses` returning a paged window of derived addresses per keychain, so that Phase 3 can render Receive (external) and Change (internal) tables without paging logic on the JS side.

**infrastructure_rationale:** Consumed by Phase 3 WalletPanel.

**Acceptance criteria.**
- AC-1: Request shape `{ keychain: "external" | "internal", page_index: u32, page_size: u32 (default 20, max 20) }`.
- AC-2: Returns `Vec<AdminWalletAddress>` with `index: u32`, `address: String`, `is_used: bool` (BDK heuristic: any tx in derivation index).
- AC-3: First call with defaults returns indices `0..=19` of the requested keychain (US-B2 alignment).
- AC-4: Out-of-bound page returns `[]`, not error.
- AC-5: Same disabled-mode behavior as US-RP2.AC-3.

### US-RP5 · Trigger and inspect Admin Wallet chain sync via IPC `@infrastructure`

**Story.** As a Tauri client, I want `admin_wallet_sync` (fire-and-await) and `admin_wallet_sync_status` (snapshot), plus a background sync loop, so that staleness and freshness are explicit and pull-to-refresh works.

**infrastructure_rationale:** Consumed by US-RP1 (timestamp display) and Phase 3 (loading indicators / refresh button).

**Acceptance criteria.**
- AC-1: `admin_wallet_sync_status` returns `{ tip_height: u32 | null, last_synced_block: u32 | null, last_synced_at: ISO8601 | null, is_syncing: bool, last_error: { kind, message } | null }`.
- AC-2: `admin_wallet_sync` performs a chain sync and resolves only after success or typed error. Errors map to: `RpcUnreachable`, `RpcAuthFailed`, `DescriptorParseError`, `SyncIncomplete`, `RegtestGuardViolation`.
- AC-3: Background loop runs at 30s cadence while at least one read IPC has been called in the past 5 minutes. Suspends otherwise. Cadence and idle window read from a single Rust constant (no env var in Phase 2 — see D6).
- AC-4: Two concurrent calls to `admin_wallet_sync` collapse into one in-flight sync (no double-RPC storm).
- AC-5: While `is_syncing == true`, read IPC commands return the last successful snapshot, NOT an error (reads do not block on the sync mutex).

### US-RP6 · Consume Admin Wallet read APIs from typed React hooks `@infrastructure`

**Story.** As a Phase 3 UI developer, I want React hooks `useAdminWalletBalance`, `useAdminWalletUtxos`, `useAdminWalletAddresses(keychain, page)`, `useAdminWalletSync()`, so that the future WalletPanel and the US-RP1 card share one adapter layer.

**infrastructure_rationale:** Hook layer is internal infrastructure consumed by US-RP1 today and Phase 3 later.

**Acceptance criteria.**
- AC-1: Each hook returns `{ data, isLoading, error, refresh }` with `data` typed against the IPC schema and `error` of `AdminWalletError` union.
- AC-2: Hooks live under `desktop-app/src/domain/admin-wallet/hooks/` (mirrors Phase 1 `domain/broadcast-proposal/hooks/`).
- AC-3: `useAdminWalletSync` exposes `syncStatus`, `triggerSync()`, and is the only hook that calls `admin_wallet_sync`. The other hooks subscribe to status changes for cache invalidation; they do not trigger sync themselves.
- AC-4: Unit tests in `desktop-app/src/domain/admin-wallet/hooks/__tests__/` use a mocked Tauri IPC and cover the disabled-mode (`AdminWalletError::Disabled`) path.

## Wave: DISCUSS / [REF] Definition of Done

| # | Criterion | Status target |
|---|---|---|
| 1 | All 6 US acceptance criteria pass on regtest with funded Admin Wallet | Manual smoke + integration tests |
| 2 | New IPC commands typed end-to-end (Rust ↔ TS) with no `any`/`unknown` leaks | `tsc --noEmit` clean |
| 3 | `cargo fmt --check` passes | Green |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings` passes | Green |
| 5 | `cargo test --workspace` passes (unit + integration; integration reuses Phase 1 regtest harness) | Green |
| 6 | `cd desktop-app && npm run format:check && npm run lint && npm run build` passes | Green |
| 7 | Phase 1 broadcast flow regression: with `COMMIT_FUNDING` unset, behavior and UI byte-identical to pre-Phase-2 | Regression test |
| 8 | Disabled-mode errors (`AdminWalletError::Disabled`) verified for every new IPC command | Integration test |
| 9 | Spec doc `docs/specs/admin-wallet-core-read-path.md` created and linked from `docs/specs/admin-wallet-implementation-plan.md` §Phase 2 | PR review |

## Wave: DISCUSS / [REF] Out-of-Scope

Phase boundaries from `docs/specs/admin-wallet-implementation-plan.md` are authoritative. Explicitly deferred in Phase 2:

- Send / PSBT build / sign / broadcast — Phase 4.
- Fee-bump / RBF — Phase 5.
- Receive rotation policy and Admin ID display — Phase 6.
- Hardware wallet adapters (Trezor/Ledger) — Phase 7.
- Governance broadcast UX refactor / shared Send chrome — Phase 8.
- Remote testnet/mainnet RPC hardening, TLS, network presets — Phase 9.
- Full WalletPanel (Balance/Addresses/Transactions/Receive/Send tabs) — Phase 3.
- BDK state persistence across restarts — deferred; revisit if start-up sync latency hurts UX.
- New env vars — none in Phase 2 (D6).
- Changes to `commit_funding.rs` external API or `broadcast_commit_then_reveal` — none in Phase 2 (D1).
- SPS-50/51/65 validation or protocol changes — never in this program.

## Wave: DISCUSS / [REF] WS Strategy

**Strategy: B — Brownfield extension (Phase 1 already established the walking skeleton for the Admin Wallet program).**

Phase 2 ships as a **precursor commit** layered on Phase 1, not as a standalone walking skeleton. The slice composition gate is honored via US-RP1's user-visible extension of the Phase 1 broadcast card. No new descriptors, no new env, no new external dependencies; only new in-process APIs over BDK state that Phase 1 already loads.

## Wave: DISCUSS / [REF] Driving Ports

Inbound surfaces introduced or extended by Phase 2:

| Port | Kind | Direction | Owner |
|---|---|---|---|
| `admin_wallet_get_balance` | Tauri IPC command | TS → Rust | `desktop-app/src-tauri/src/commands/admin_wallet.rs` (extend) |
| `admin_wallet_list_utxos` | Tauri IPC command | TS → Rust | same |
| `admin_wallet_list_addresses` | Tauri IPC command | TS → Rust | same |
| `admin_wallet_sync` | Tauri IPC command | TS → Rust | same |
| `admin_wallet_sync_status` | Tauri IPC command | TS → Rust | same |
| `useAdminWalletBalance` / `useAdminWalletUtxos` / `useAdminWalletAddresses` / `useAdminWalletSync` | React hook | React → IPC adapter | `desktop-app/src/domain/admin-wallet/hooks/` |
| `BroadcastDetailsCard` extension | React component prop | UI surface | `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx` |
| Background sync task | Internal tokio task | Self-driven | `WalletService` startup hook in Tauri `setup` |

Downstream (driven) — unchanged from Phase 1: chain RPC (`HttpBitcoinRpcClient` → `bdk_bitcoind_rpc`), BDK `Wallet` over Phase 1 descriptors.

## Wave: DISCUSS / [REF] Pre-requisites

- Phase 1 (US-H7 / `admin-wallet-regtest-commit-funding`) **merged** — provides:
  - `infrastructure/admin_wallet/wallet.rs` (`load_admin_wallet`, descriptors, `AdminWalletError`)
  - `application/commit_funding.rs` (BDK + dev-mnemonic guards)
  - `commands/admin_wallet.rs` and `get_admin_wallet_info` IPC command (extend)
  - `BroadcastDetailsCard` with `adminWalletInfo` prop (extend)
  - Workspace deps `bdk_wallet = "1"`, `bdk_bitcoind_rpc = "0.18"`
- `scripts/bitcoind-asm-runner.sh` regtest harness available for integration tests.
- `dev_secrets.rs` env guard pattern (`ALLOW_DEV_MNEMONIC_SIGNING`).
- Existing Tauri IPC registration in `commands/invoke.rs`.
- `tokio` runtime already wired (`#[tokio::main]` in `main.rs`).

## Wave: DISCUSS / [REF] Wave Decisions Summary

**Feature type:** Backend (Rust IPC + thin React hooks). One user-visible touch-point on Phase 1's broadcast card.

**Primary needs:** Phase 3 UI needs a stable, typed read surface (balance, UTXOs, addresses, sync status) backed by BDK chain sync on regtest, with explicit staleness and error taxonomy.

**Key constraints established:**
- Regtest-only; no remote-RPC hardening (D9 / Phase 9).
- Secrets stay in Rust; React never sees mnemonic or xprv.
- Single source of truth for confirmations = BDK chain state (D4).
- Single `Wallet` mutex; reads take short snapshot locks (D5).
- No new env vars (D6); no new persistence (D8).

**Upstream changes:** None — Phase 1 contract preserved end-to-end.

**Handoff to:** `nw-solution-architect` (DESIGN) for module/data-model design; `nw-platform-architect` (DEVOPS) gets only KPI / observability needs (sync latency on regtest, background-loop telemetry) when those become a concern — not in Phase 2.
