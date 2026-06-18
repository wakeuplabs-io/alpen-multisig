## Wave: DELIVER / [REF] Implementation Summary

Phase 1 of the Admin Wallet program (US-H7) is complete. The desktop app can now fund governance commit transactions from a BIP-86 Taproot Admin Wallet on regtest using BDK + Bitcoin Core RPC, while the existing `sendtoaddress` legacy path remains the default for CI/E2E. The reveal path, orchestrator coordination, and commit/reveal protocol are unchanged.

## Wave: DELIVER / [REF] Files Modified

**Production (Rust)**
- `Cargo.toml` (root) — added `bdk_wallet = "1"` and `bdk_bitcoind_rpc = "0.18"` to workspace deps
- `desktop-app/src-tauri/Cargo.toml` — wired BDK deps from workspace
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/mod.rs` (new) — re-exports wallet functions
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/wallet.rs` (new) — `load_admin_wallet`, `get_external_address`, `AdminWalletError`; BIP-86 derivation at `m/86'/0'/73'` (account 73', regtest coin type 0')
- `desktop-app/src-tauri/src/infrastructure/mod.rs` — added `pub mod admin_wallet`
- `desktop-app/src-tauri/src/application/commit_funding.rs` (new) — `CommitFunding` trait, `BitcoindSendToAddress`, `BdkAdminWalletMnemonic`, `select_commit_funding`, `CommitFundingError`
- `desktop-app/src-tauri/src/application/mod.rs` — added `pub mod commit_funding`
- `desktop-app/src-tauri/src/application/proposals.rs` — `broadcast_commit_then_reveal` now accepts `&dyn CommitFunding` and delegates commit funding; reveal path unchanged
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` (new) — `get_admin_wallet_info` Tauri IPC command
- `desktop-app/src-tauri/src/commands/mod.rs` — added `pub(crate) mod admin_wallet`
- `desktop-app/src-tauri/src/commands/invoke.rs` — registered `get_admin_wallet_info` in invoke handlers
- `desktop-app/src-tauri/src/commands/proposals.rs` — wires `select_commit_funding` and passes to `broadcast_commit_then_reveal`

**Production (TypeScript/React)**
- `desktop-app/src/api/admin-wallet.ts` (new) — typed Tauri IPC adapter for `get_admin_wallet_info`
- `desktop-app/src/domain/broadcast-proposal/hooks/use-admin-wallet-info.ts` (new) — fetches admin wallet info on mount; returns null when not in admin_wallet mode
- `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx` — optional `adminWalletInfo` prop; renders "Funding Source" section with address + balance when non-null
- `desktop-app/src/screens/broadcast-proposal-screen.tsx` — wires `useAdminWalletInfo` and passes to `BroadcastDetailsCard`

**Artefacts**
- `docs/archive/features/admin-wallet-regtest-commit/deliver/roadmap.json` — 6-step delivery plan
- `docs/archive/features/admin-wallet-regtest-commit/deliver/execution-log.json` — DES audit log (all 6 steps PASS)
- `Cargo.lock` — updated

## Wave: DELIVER / [REF] Scenarios Green Count

No DISTILL `.feature` files (wave not run). Test counts from `cargo test` and `npm run build`:
- Rust: **74 / 74** tests pass (69 lib + 5 binary)
- TypeScript: build exits 0, lint exits 0
- Clippy: 0 warnings

## Wave: DELIVER / [REF] DoD Check

| Criterion | Status |
|---|---|
| `COMMIT_FUNDING=admin_wallet` on regtest funds commit from `m/86'/0'/73'/0/0` | ✅ Implemented |
| Change goes to first unused `m/86'/0'/73'/1/*` | ✅ BDK change handling |
| Reveal and orchestrator PATCH unchanged | ✅ Verified |
| Default `COMMIT_FUNDING` unset → legacy bitcoind path | ✅ Regression tests pass |
| `COMMIT_FUNDING=admin_wallet` on non-regtest → clear error | ✅ Guard in `select_commit_funding` |
| Minimal UI: funding mode, address, balance before confirm | ✅ `BroadcastDetailsCard` updated |
| `cargo test -p desktop-app` passes | ✅ 74/74 |
| `npm run build` passes | ✅ |
| `cargo clippy -- -D warnings` passes | ✅ |
| DES integrity: all 6 steps traced | ✅ `des-verify-integrity` exit 0 |

## Wave: DELIVER / [REF] Quality Gates

| Gate | Result |
|---|---|
| DES integrity (Phase 6) | ✅ All 6 steps complete |
| Rust tests | ✅ 74/74 |
| TypeScript build | ✅ |
| Clippy -D warnings | ✅ |
| Refactoring (Phase 3) | ✅ No new warnings post-implementation |
| Adversarial review | ⏭ Skipped (fast-track, user preference for agility) |
| Mutation testing | ⏭ Skipped (Rust stack — mutmut is Python-only) |

## Wave: DELIVER / [REF] Pre-requisites

- US-H6 commit/reveal flow in `broadcast_commit_then_reveal` (proposals.rs)
- `HttpBitcoinRpcClient` in `infrastructure/bitcoin_rpc.rs`
- `dev_secrets.rs` env guard pattern for `ALLOW_DEV_MNEMONIC_SIGNING`
- Existing Tauri IPC command registration in `commands/invoke.rs`
