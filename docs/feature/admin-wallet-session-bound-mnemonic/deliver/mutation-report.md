# Mutation Testing Report — admin-wallet-session-bound-mnemonic

**Strategy:** per-feature
**Files tested:** wallet_session.rs, wallet_service.rs (new functions: disabled_default, shutdown, check_enabled, spawn_background_sync)
**Tool:** cargo-mutants 27.0.0
**Date:** 2026-05-28

## Results

**Final (after gap-closing commit `49fc130`):** kill rate 11/11 viable mutants killed (100%) — Gate (≥80%): **PASS**.
Initial run was 8/11 (72.7% — FAIL); three surviving mutants were real test gaps and were closed by adding
two `parse_network` network-variant tests (testnet, mainnet) and one `spawn_background_sync` activation assertion.

Initial run detail below for the record.

Kill rate: 8/11 viable mutants killed (72.7%)
Gate (≥80%): FAIL (initial run — superseded by 100% after remediation)

### Breakdown by run

| Run | File / scope | Total | Unviable | Viable | Killed | Missed |
|-----|-------------|-------|----------|--------|--------|--------|
| 1 | wallet_session.rs (all 9) | 9 | 4 | 5 | 3 | 2 |
| 2 | wallet_service.rs — disabled_default, shutdown, check_enabled | 6 | 1 | 5 | 5 | 0 |
| 3 | wallet_service.rs — spawn_background_sync | 1 | 0 | 1 | 0 | 1 |
| **Total** | | **16** | **5** | **11** | **8** | **3** |

Note: wallet_service.rs remaining functions (secs_to_iso8601, rpc_error_from_message, sync, do_sync, build_and_sign_tx, get_balance, list_utxos, list_addresses, fund_commit) were not run in this session because the full 64-mutant run exceeded the 5-minute wall-clock budget. Only Phase 3.7 new functions were scoped.

## Surviving mutants

### 1. wallet_session.rs:74 — delete match arm "testnet" in parse_network
**Mutation:** remove the `"testnet" => Network::Testnet` arm from `parse_network`
**Reason survived:** No test calls `init_from_mnemonic` or `current_or_fallback` with `network = Some("testnet")`. All tests use `None` (defaulting to regtest).
**Real gap:** Yes — the testnet branch is untested.

### 2. wallet_session.rs:75 — delete match arm "bitcoin" | "mainnet" in parse_network
**Mutation:** remove the `"bitcoin" | "mainnet" => Network::Bitcoin` arm from `parse_network`
**Reason survived:** No test passes `network = Some("bitcoin")` or `Some("mainnet")`.
**Real gap:** Yes — the mainnet/bitcoin branch is untested.

### 3. wallet_service.rs:340 — replace WalletService::spawn_background_sync with ()
**Mutation:** replace the entire function body with a no-op
**Reason survived:** The test `spawn_background_sync_loop_exits_after_shutdown` calls `svc.spawn_background_sync()` but the observable assertions (no panic, idempotent shutdown) still pass when the body is a no-op — no task is spawned but no assertion checks `bg_task_started.load(true)` nor waits for a sync tick.
**Real gap:** Partial. The loop-exit behavior is tested; the activation (that the loop is actually started on first call) is not directly asserted.

## Analysis and remediation

### Gap 1 & 2: parse_network non-regtest arms

These arms guard mainnet and testnet usage. In the current Phase 3 scope (regtest only), these code paths are unreachable in production. However, deleting them would silently fall through to the `_` arm (Regtest), which is incorrect behavior for future mainnet/testnet deployments.

**Recommended fix:** Add two parametrized tests to `wallet_session.rs`:
```rust
#[tokio::test]
async fn init_from_mnemonic_parses_testnet_network() {
    // ... assert that the loaded wallet is on Testnet
}

#[tokio::test]
async fn init_from_mnemonic_parses_mainnet_network() {
    // ... assert that the loaded wallet is on Bitcoin mainnet
}
```

### Gap 3: spawn_background_sync activation

**Recommended fix:** Assert `bg_task_started` is `true` after calling `spawn_background_sync()`, or verify a sync tick occurs by checking `last_synced_at` changes:
```rust
svc.spawn_background_sync();
assert!(svc.bg_task_started.load(Ordering::SeqCst), "bg task flag must be set");
```

## Conclusion

**Resolved.** The three surviving mutants were real test gaps and have been closed in commit
`49fc130`:

- Two `parse_network` non-regtest variants (testnet, mainnet) — closed by
  `init_from_mnemonic_with_testnet_network_uses_testnet` and
  `init_from_mnemonic_with_mainnet_network_uses_mainnet`.
- One `spawn_background_sync` activation gap — closed by asserting `bg_task_started` is `true`
  after `spawn_background_sync()`.

Final kill rate: **11/11 viable = 100%** for the Phase 3.7 covered scope — gate **PASS**.

The broader `wallet_service.rs` functions (secs_to_iso8601, sync, do_sync, etc.) were not covered in this per-feature run due to the 5-minute wall-clock constraint on regtest BDK compilation. A CI nightly delta run is recommended to cover the remaining 55+ mutants in that file.
