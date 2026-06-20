# RCA Context — fix-broadcast-mining-bitcoind-wallet

## Bug
Broadcast proposal flow fails on regtest with:
`bitcoin rpc \`getnewaddress\` error: Requested wallet does not exist or is not loaded`.

## Root cause
A regtest-only chain-mining step is embedded in the **production** broadcast function.
`application/proposals.rs:296-302` (Step 8) calls `btc_rpc.mine_blocks(1)` → internally
`mine_blocks()` calls `get_new_address()` → RPC `getnewaddress` against the wallet-scoped
URL `/wallet/asm-runner`. When bitcoind has no `asm-runner` wallet loaded, it fails.

This is the only residual bitcoind Core-wallet dependency in a broadcast path that otherwise
already uses BDK/Admin Wallet (commit funding, reveal change address, commit signing all via BDK).

## Classification
Long-standing architectural defect (incomplete migration), introduced in PR #125 (ebf79fc,
2026-05-13). Phase 3.6 (PR #187) migrated funding to BDK but left the mining step behind.
Last touched in PR #198 (66e5fea). NOT a recent regression.

## Design intent (requirements — treat deviations as defects)
1. The bitcoind/Core wallet must NOT be used for anything in production code. Use BDK/Admin Wallet.
2. Block mining (mine_blocks / generatetoaddress / getnewaddress) must NOT be in the production
   broadcast path — it is a dev/regtest concern only (faucet `regtest-dev-api` / e2e harness).

## Fix (minimal, subtractive, compiler-checked)
1. Remove Step 8 mining from `application/proposals.rs:296-302`.
2. Remove `mine_blocks` + `get_new_address` from `BitcoinRpcClient` trait and `HttpBitcoinRpcClient` impl
   (`infrastructure/bitcoin_rpc.rs:20-24, 156-168`).
3. Remove the `/wallet/{wallet}` URL branch (`bitcoin_rpc.rs:42-44`); make the prod client node-level
   only, drop the `wallet_name` constructor param.
4. Remove `btc_wallet_name` from `BroadcastEnv` (`broadcast_env.rs:38, 67-86`) and the 3 call sites in
   `commands/proposals.rs:327, 664, 708` (pass nothing / node-level).
5. Fix `commands/asm_state.rs:89-97` — `getblockcount` is node-level; build the client without the wallet.
6. Remove `BITCOIN_WALLET_NAME=asm-runner` from `desktop-app/.env`.
7. Flip the test invariant: `application/proposals.rs:1455-1486` currently asserts `mine_blocks` is
   called exactly once; it must assert `mine_blocks` is NEVER called (or remove it from `MockBtcRpc`).

## Regtest confirmation note
On regtest standalone, confirmation now relies on external mining (faucet `regtest-dev-api` / e2e
harness — already how e2e operates). DECISION (user-confirmed): delegate mining to the dev-only
faucet/harness. Do NOT add mining back into the broadcast path or as an app command.

## Leave untouched (legit dev/test-only)
`regtest-dev-api/`, `desktop-app/src-tauri/tests/admin_wallet_integration.rs`,
`desktop-app/e2e-webdriver/.../fund-admin-wallet.mjs`, `e2e-tests/`, `asm/`.

## Regression test (primary deliverable)
A test proving the production broadcast path does NOT invoke any bitcoind wallet-scoped RPC
(no `mine_blocks` / `getnewaddress`) on regtest. Must fail against current code, pass after fix.
