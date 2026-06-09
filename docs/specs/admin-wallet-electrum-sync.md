# Admin Wallet — Electrum sync (Release 2)

> **Release 2** of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
> Plan-level summary only — delivery slicing and technical design are deferred to a follow-up spec revision.

## Objective

Replace the current wallet **indexation** path (block-by-block sync via `bdk_bitcoind_rpc`) with **Electrum** (`bdk_electrum`) so balance, UTXOs, addresses, and receive rotation remain viable on remote testnet/mainnet. Production testing showed Core RPC sync latency is unacceptable; this release is a **hard prerequisite** for Phases 4–10 outside local regtest.

## Scope

### Included

- Electrum-backed wallet sync for the Admin Wallet read path (balance, UTXOs, addresses, receive rotation).
- Configurable Electrum server URL (trusted presets and custom) alongside the existing chain RPC used for broadcast and fee operations.
- Regtest, testnet, and mainnet.
- Parity with current IPC contracts and wallet panel UX (R1.2–R1.7).

### Not included

- Any indexer backend other than Electrum.
- Changing commit/reveal protocol, `PsbtSigner`, or broadcast signing flows.
- Send (Phase 5), transaction list / RBF (Phase 6), Admin ID UI (Phase 7), or shared Send UX (Phase 9).
- Detailed delivery slices — to be defined when implementation starts.

## Architecture (plan level)

Wallet **sync** uses Electrum. **Broadcast**, `submitpackage`, and fee estimation continue to use a Bitcoin Core–compatible JSON-RPC client (`BITCOIN_RPC_URL`). The app therefore configures two chain-access endpoints, not one.

## Done when

- On testnet or mainnet (or regtest with a local Electrum indexer), wallet sync completes in production-viable time and the panel shows correct balance, receive address, and addresses-with-balance.
- Governance broadcast and signing behavior unchanged from Release 1.
- `cargo test --workspace` and frontend CI green.

## Related

- [Implementation plan](./admin-wallet-implementation-plan.md) — §2 traceability, Release 2
- [PRD compliance matrix](./admin-wallet-prd-compliance.md)
- [Core read path spec](./admin-wallet-core-read-path.md) — current sync baseline (pre-R2)
- [Sync progress indicator spec](./admin-wallet-sync-progress.md) — **deferred**; primary mitigation is R2, not block-scan progress UI
