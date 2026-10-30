# block-payout-spike

Isolated Cargo workspace that proves a Payout Administrator **K-of-M set can
software-sign and (on regtest) broadcast** an `AdminBurnTx` (`block_payout`)
**without a hardware wallet**.

This crate is **not** a member of the parent `alpen-multisig` workspace. It
path-depends on `strata-bridge` and therefore pins `bitcoin 0.32.9`. The parent
workspace stays on `0.32.6`. See [`GO_NO_GO.md`](GO_NO_GO.md).

## Commands

Run from this directory so rustup picks `nightly-2026-09-01` (strata-bridge's
toolchain, not alpen-multisig's `nightly-2026-01-01`):

```bash
cd alpen-multisig/block-payout-spike

# K-of-M software signing (no bitcoind)
cargo test --test k_of_m_software

# Broadcast on regtest (spawns bitcoind; downloads Core 30.2 if not on PATH)
cargo test --test broadcast_regtest -- --ignored --nocapture
```

## Out of scope

Desktop app, orchestrator, hardware wallets, false-claim proof verification,
and any change to `strata-bridge` protocol code.
