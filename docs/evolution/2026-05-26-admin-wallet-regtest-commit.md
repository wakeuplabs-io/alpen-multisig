# Evolution: admin-wallet-regtest-commit

**Date:** 2026-05-26
**Feature ID:** admin-wallet-regtest-commit
**Wave:** DELIVER
**Status:** COMPLETE

---

## Feature Summary

Phase 1 of the Admin Wallet program (US-H7). The desktop app can now fund governance commit transactions from a BIP-86 Taproot Admin Wallet on regtest using BDK + Bitcoin Core RPC. The existing `sendtoaddress` legacy path remains the default for CI/E2E. The reveal path, orchestrator coordination, and commit/reveal protocol are unchanged.

The feature is activated via the `COMMIT_FUNDING=admin_wallet` environment variable, guarded to regtest only. Unset or other values fall back to the existing `BitcoindSendToAddress` path.

---

## Business Context

Alpen Multisig governance workflows require committing transactions to the Bitcoin chain. Previously the only supported funding method was delegating to a Bitcoin Core RPC `sendtoaddress` call, which requires a funded node wallet. The Admin Wallet feature allows regtest development and integration testing without relying on a pre-funded node wallet, using a deterministic BIP-86 Taproot HD wallet derived from a dev mnemonic. This unblocks local developer workflows and future regtest E2E automation.

---

## Steps Completed

| Step ID | Phase | Description | Result |
|---------|-------|-------------|--------|
| 01-01 | Backend core | Add BDK workspace dependencies to Cargo.toml | PASS |
| 01-02 | Backend core | Create admin_wallet infrastructure module with BDK wallet load and sync | PASS |
| 01-03 | Backend core | Create CommitFunding trait with BitcoindSendToAddress and BdkAdminWalletMnemonic variants | PASS |
| 02-01 | Integration | Inject CommitFunding into broadcast_commit_then_reveal in proposals.rs | PASS |
| 02-02 | Integration | Add get_admin_wallet_info Tauri IPC command and register it in lib.rs | PASS |
| 03-01 | Frontend | Add funding mode label and Admin Wallet info to the broadcast screen | PASS |

Total: 6 steps, all PASS (2 RED_UNIT steps SKIPPED as NOT_APPLICABLE — no business logic / no unit test infrastructure available).

---

## Key Decisions

### CommitFunding abstraction via trait
`CommitFunding` was introduced as a Rust trait with two variants (`BitcoindSendToAddress`, `BdkAdminWalletMnemonic`) rather than an `if/else` in `proposals.rs`. This cleanly separates concerns and makes future funding variants (e.g., PSBT-based co-signing) addable without modifying the core broadcast flow.

### BIP-86 Taproot at account 73' (regtest, coin type 0')
The derivation path `m/86'/0'/73'` was chosen to match the established Admin ID derivation account convention (`73'`) used elsewhere in the project. Coin type `0'` is used for regtest (Bitcoin mainnet coin type, standard for regtest BDK wallets). External address: `m/86'/0'/73'/0/*`, change: `m/86'/0'/73'/1/*`.

### Regtest guard on BdkAdminWalletMnemonic
`select_commit_funding` returns a clear error immediately if `COMMIT_FUNDING=admin_wallet` is set on any network other than regtest. This prevents accidental use of a dev mnemonic on mainnet or signet.

### Adversarial review skipped (fast-track)
The adversarial review gate was skipped at user's request for delivery agility. Mutation testing was skipped because the project uses the Rust stack and mutmut is Python-only.

### RED_UNIT skipped for two steps
- Step 01-01: dependency addition has no business logic to unit test.
- Step 03-01: no React Testing Library / Vitest infrastructure in project — hook unit tests not feasible without first establishing that test infrastructure.

---

## Lessons Learned

### Trait-based injection for multi-strategy features
Using a Rust trait for `CommitFunding` proved clean and testable. The variant selection (`select_commit_funding`) is independently testable from the broadcast flow. Recommended pattern for future multi-strategy concerns (e.g., reveal funding, fee estimation strategies).

### Frontend unit test gap
The project has no React unit test infrastructure (no Vitest, no React Testing Library). When new hooks are added they can only be validated via build success and manual smoke tests. Establishing a minimal Vitest setup would unlock hook-level unit tests and improve confidence for future frontend slices.

### Mutation testing gap on Rust
Mutation testing was marked NOT_APPLICABLE due to mutmut being Python-only. The project has no Rust-native mutation testing tool configured. `cargo-mutants` is a viable option and should be evaluated for future iterations where mutation coverage is required.

### BDK sync on regtest
BDK `bdk_bitcoind_rpc` RPC sync requires a running Bitcoin Core node with the wallet loaded. Error handling in `AdminWalletError` surfaces RPC connectivity and insufficient balance issues as structured errors — important for clear UX on the broadcast screen.

---

## Issues Encountered

No blocking issues were encountered during delivery. All steps completed in a single session (2026-05-26).

Minor note: Cargo.lock required updating due to new BDK workspace dependencies. This was expected and included in the step 01-01 commit.

---

## Production Files Modified

**Rust (desktop-app/src-tauri)**
- `Cargo.toml` (root) — `bdk_wallet = "1"`, `bdk_bitcoind_rpc = "0.18"` workspace deps
- `desktop-app/src-tauri/Cargo.toml` — wired from workspace
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/mod.rs` (new)
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/wallet.rs` (new)
- `desktop-app/src-tauri/src/infrastructure/mod.rs`
- `desktop-app/src-tauri/src/application/commit_funding.rs` (new)
- `desktop-app/src-tauri/src/application/mod.rs`
- `desktop-app/src-tauri/src/application/proposals.rs`
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` (new)
- `desktop-app/src-tauri/src/commands/mod.rs`
- `desktop-app/src-tauri/src/commands/invoke.rs`
- `desktop-app/src-tauri/src/commands/proposals.rs`

**TypeScript/React (desktop-app/src)**
- `desktop-app/src/api/admin-wallet.ts` (new)
- `desktop-app/src/domain/broadcast-proposal/hooks/use-admin-wallet-info.ts` (new)
- `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx`
- `desktop-app/src/screens/broadcast-proposal-screen.tsx`

---

## Quality Gates at Completion

| Gate | Result |
|------|--------|
| DES integrity (all 6 steps traced) | PASS |
| Rust tests (`cargo test -p desktop-app`) | PASS — 74/74 |
| TypeScript build (`npm run build`) | PASS |
| Clippy (`-D warnings`) | PASS — 0 warnings |
| Adversarial review | SKIPPED (fast-track) |
| Mutation testing | SKIPPED (no Rust tool configured) |

---

## Migrated Permanent Artifacts

This feature ran DELIVER wave only (no DISCUSS, DESIGN, or DISTILL waves). No lasting architecture, UX, or scenario artifacts were produced. No migration is required.

The workspace `docs/feature/admin-wallet-regtest-commit/` contains only session artifacts:
- `deliver/execution-log.json`
- `deliver/roadmap.json`
- `deliver/.develop-progress.json`
- `feature-delta.md`
