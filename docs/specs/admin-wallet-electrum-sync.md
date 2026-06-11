# Admin Wallet — Electrum sync (Release 2)

> **Release 2** of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
> **Status: ✅ Complete** — R2.1 (PR [#261](https://github.com/wakeuplabs-io/alpen-multisig/pull/261)), R2.2 (PR [#262](https://github.com/wakeuplabs-io/alpen-multisig/pull/262)), R2.3 (PR [#263](https://github.com/wakeuplabs-io/alpen-multisig/pull/263)).

## Objective

Replace the current wallet **indexation** path (block-by-block sync via `bdk_bitcoind_rpc`) with **Electrum** (`bdk_electrum`) so balance, UTXOs, addresses, and receive rotation remain viable on remote testnet/mainnet. Production testing showed Core RPC sync latency is unacceptable; this release is a **hard prerequisite** for Phases 4–10 outside local regtest.

## Architecture (plan level)

Wallet **sync** uses an **Electrum-protocol indexer** (regtest: **electrs** against local `bitcoind`). **Broadcast**, `submitpackage`, and fee estimation continue to use a Bitcoin Core–compatible JSON-RPC client (`BITCOIN_RPC_URL`). The app configures **two chain-access endpoints**, not one.

## Delivery slices

Release 2 ships in **three slices**, in order. Technical design for each slice is expanded at implementation kickoff.

| Slice | Name | Summary |
|-------|------|---------|
| **R2.1** | Electrum indexer infra | electrs (or equivalent) in Docker; wired into local dev, staging, CI, and `scripts/` alongside existing regtest `bitcoind`; indexer synced to the local Bitcoin node; smoke/verification that indexation works before app changes |
| **R2.2** | Admin Wallet sync migration | Replace `WalletService` wallet sync with `bdk_electrum` in **one slice** — balance, UTXOs, addresses, receive rotation; **no** broadcast or fee-path changes; fixed Electrum URL (constant or env) to prove end-to-end on regtest |
| **R2.3** | Electrum URL in Node Config | Expose Electrum URL in application configuration the same way as BTC RPC and Strata today (`NodeConfig`: Local / Trusted / Custom); retire the R2.2 fixed URL |

**Suggested order:** R2.1 → R2.2 → R2.3 → Phase 4.

### R2.1 — Electrum indexer infra

**Goal:** A regtest Electrum-protocol indexer runs everywhere the team develops and tests, backed by the same local `bitcoind` used today for broadcast and ASM.

**In scope**

- Docker service (electrs) in `staging/docker-compose*.yml`, depending on the existing `bitcoin` service.
- Integration with local dev and CI (canonical recipe documented; align with or extend `scripts/bitcoind-asm-runner.sh` where needed).
- Health/smoke checks: indexer reachable; after fund + (optional) mine, a known address or scripthash is visible via the Electrum protocol.

**Out of scope**

- Desktop app or `WalletService` code changes.
- Testnet/mainnet public Electrum presets (Phase 10 / R2.3 Trusted mode).

**Done when:** `docker compose up` (and CI) brings up `bitcoind` + electrs; smoke script passes; team can point a manual Electrum client or future R2.2 build at the local URL.

---

### R2.2 — Admin Wallet sync migration

**Goal:** Wallet read path syncs via Electrum instead of Core RPC block-scan; broadcast and fees unchanged.

**In scope**

- `bdk_electrum` in the workspace; `WalletService::do_sync` (and related read-path sync) uses Electrum.
- Retire `bdk_bitcoind_rpc::Emitter` for wallet sync only — `HttpBitcoinRpcClient` / `BITCOIN_RPC_URL` stay for broadcast, `submitpackage`, and fee estimate.
- Fixed Electrum URL (env var or compile-time constant) pointing at R2.1 local electrs.
- Update wallet integration tests and manual regtest checklist: panel balance, receive rotation, addresses-with-balance; governance broadcast still works.
- Preserve Release 1 behavior for unconfirmed/mempool visibility where Electrum allows (R1.5 / R1.3 semantics).

**Out of scope**

- Node Config UI or persisted Electrum URL (R2.3).
- Dual-path toggle (Emitter vs Electrum) — big-bang swap once R2.1 is green.
- Send, tx list, Admin ID UI.

**Done when:** On regtest with R2.1 infra, wallet panel read path is correct after Electrum sync in production-viable time; broadcast/fee flows unchanged; `cargo test --workspace` and frontend CI green.

---

### R2.3 — Electrum URL in Node Config

**Goal:** Signers configure the Electrum indexer URL like BTC RPC and Strata — Local default, Trusted preset, Custom URL.

**In scope**

- Extend `NodeConfig` (Rust + IPC + `node-config.json` persistence) with Electrum URL fields.
- Extend Node Config UI (`NodeConfigModal` / connect flow) and Local/Trusted/Custom presets.
- Remove R2.2 fixed URL; `WalletService` reads Electrum URL from `NodeConfig`.
- `.env.example` / runbooks updated for local electrs URL.

**Out of scope**

- Remote testnet/mainnet Trusted presets hardening (Phase 10).
- Broadcast RPC config changes beyond documenting the dual-endpoint model.

**Done when:** User can set Electrum URL via Node Config; wallet sync uses it on regtest; Local mode defaults to the R2.1 local electrs URL.

---

## Release 2 — done when

All of R2.1–R2.3 are complete:

- Wallet panel read path syncs in production-viable time on regtest (and is ready for testnet/mainnet presets in Phase 10).
- Balance, receive rotation, and addresses-with-balance remain correct.
- Governance broadcast and signing behavior unchanged from Release 1.
- `cargo test --workspace` and frontend CI green.

## Scope (program level)

### Included

- Electrum-backed wallet sync for the Admin Wallet read path.
- Configurable Electrum server URL (R2.3) alongside existing chain RPC for broadcast and fees.
- Regtest, testnet, and mainnet (testnet/mainnet presets mature in Phase 10).

### Not included

- Any indexer backend other than Electrum protocol (electrs in dev/CI).
- Changing commit/reveal protocol, `PsbtSigner`, or broadcast signing flows.
- Send (Phase 5), transaction list / RBF (Phase 6), Admin ID UI (Phase 7), shared Send UX (Phase 9).
- [`admin-wallet-sync-progress.md`](./admin-wallet-sync-progress.md) block-scan progress UI — **deferred** unless still needed after R2.2.

## Related

- [Implementation plan](./admin-wallet-implementation-plan.md) — §2 traceability, Release 2
- [PRD compliance matrix](./admin-wallet-prd-compliance.md)
- [Core read path spec](./admin-wallet-core-read-path.md) — current sync baseline (pre-R2)
- Staging stack: [`staging/docker-compose.yml`](../staging/docker-compose.yml), [`staging/docker-compose.local.yml`](../staging/docker-compose.local.yml)
- Local bitcoind: [`scripts/bitcoind-asm-runner.sh`](../scripts/bitcoind-asm-runner.sh)
