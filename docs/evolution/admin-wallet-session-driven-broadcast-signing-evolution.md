# Evolution: Admin Wallet Session-Driven Broadcast Signing (R1.1)

**Feature ID:** admin-wallet-session-driven-broadcast-signing
**Status:** Complete
**Date:** 2026-06-01

## Summary

Implemented R1.1 of the Admin Wallet — session-driven broadcast signing with a unified `PsbtSigner` port supporting both mnemonic (simulated HW) and real hardware wallet (Trezor/Ledger) signing paths.

## What Changed

### Backend (Rust)
- **New:** `PsbtSigner` trait (driven port) in `application/psbt_signer.rs`
- **New:** `MnemonicPsbtSigner` (software signer, regtest/testnet only)
- **New:** `HwPsbtSigner` (hardware signer, any network) with `FakeHwDevice` test double
- **Modified:** `WalletService` — split `build_psbt`/`sign`, holds `Option<Arc<dyn PsbtSigner>>`
- **Modified:** `WalletSession` — attaches correct signer per login type
- **Modified:** `commands/proposals.rs` — structured `{code, message}` error contract (DDD-8)
- **Removed:** `ALLOW_DEV_MNEMONIC_SIGNING` env flag (replaced by per-signer network capability)
- **Renamed:** `BdkAdminWalletMnemonic` → `AdminWalletCommitFunding`

### Frontend (TypeScript/React)
- **New:** `BroadcastDevicePrompt` component ("Confirm on your device")
- **New:** `deriveBroadcastError` — parses structured JSON errors, maps to recovery actions
- **New:** `BroadcastError` view-model with `code`, `message`, `recovery`
- **Modified:** `useBroadcastProposal` — recovery-gated `canResubmit`, `awaiting-device` phase
- **Modified:** `useAdminWalletCapability` — surfaces `signerKind` + `canSignReason`
- **Modified:** `BroadcastDetailsCard` — device prompt mount, kind-specific error copy
- **Modified:** `BroadcastPhaseProgress` — ranks `awaiting-device` as commit-active

## Architecture Decisions (DDD-1 through DDD-9)

All 9 domain-driven decisions from the DESIGN wave were implemented and locked:
- DDD-1: `PsbtSigner` driven port on `WalletService`
- DDD-2: Two implementors behind same port
- DDD-3: Remove `ALLOW_DEV_MNEMONIC_SIGNING`, replace with `allowed_on(network)`
- DDD-4: HW device access via `spawn_blocking`, re-open by fingerprint
- DDD-5: Reveal unchanged — signed by ephemeral envelope key
- DDD-6: Slice R1.1 into (a) port + mnemonic + flag removal, (b) HW signer
- DDD-7: PSBT carries taproot derivation metadata
- DDD-8: Structured broadcast error contract
- DDD-9: Frontend device-UX + error surfacing

## Test Coverage

- **31 roadmap steps** executed with TDD (PREPARE → RED → GREEN → COMMIT)
- Rust workspace tests passing (clippy clean with `-D warnings`, fmt clean)
- TypeScript contract tests run as `tsx` scripts (repo convention): IPC schemas, admin-wallet
  capability DTO parsing/degradation, and `deriveBroadcastError` mapping — all wired into CI
- **E2E:** Mnemonic walking skeleton (regtest, no device); real Ledger PSBT signing verified against Speculos
- **Release checklist:** 6 manual real-device test paths documented

## Quality Gates

| Gate | Status |
|------|--------|
| Roadmap created + approved (3 iterations) | ✅ |
| All steps COMMIT/PASS (5-phase TDD) | ✅ |
| L1-L4 refactoring complete | ✅ |
| Adversarial review passed (1 iteration + fixes) | ✅ |
| Mutation testing | ⚠️ Skipped (timeout) |
| All tests passing | ✅ |
| Clippy + fmt + lint + build | ✅ |

## Post-merge hardening & R1.1 closure

After the initial DELIVER wave, the following landed before R1.1 was closed:

- **Real Ledger on-device signing:** commit-PSBT signing verified end-to-end against Speculos
  (`tests/ledger_speculos_sign_integration.rs`), including master-fingerprint capture at connect and
  signing without `register_wallet`. The Trezor on-device path is stubbed (`HwSigningFailed`) pending Phase 7.
- **Merged latest `develop`** (proposal `actionType`, node-config) — single conflict in `api/ipc-schemas.ts`
  resolved additively.
- **`ALLOW_DEV_MNEMONIC_SIGNING` fully removed as a signing/broadcast gate.** The leftover `broadcast_env.rs`
  "Gate 1" (and the `MnemonicSigningDisabled` variant) were deleted; broadcast capability is now decided solely
  by `WalletService::can_sign()` → `PsbtSigner::allowed_on(network)`. The flag and its references were removed
  from `.env.example`, `render.yaml`, `staging/*`, the e2e-webdriver docs, and the Ledger integration test. The
  env name survives **only** as the dev-only mnemonic/raw-key signing IPC exposure gate (`dev_secrets.rs`, P-040;
  debug builds auto-enable).
- **`BdkAdminWalletMnemonic` → `AdminWalletCommitFunding`** rename completed (the name no longer implies mnemonic
  once a HW signer can be attached).
- **Cleanup / simplification:**
  - Removed a dead, duplicated admin-wallet signing stack that had been added to `orchestrator-be`
    (`psbt_signer`/`wallet_service`/`wallet_session`/`admin_wallet`/`hw_wallet`, ~1.6k LOC) — it was never wired and
    violated "backend is coordination only".
  - Dropped the `vitest` + `@testing-library` toolchain (introduced for a few DOM-render tests CI never ran) and
    kept only the high-value pure-logic tests as `tsx` scripts, wired into CI.
  - Simplified `MnemonicPsbtSigner` (removed an unused `network` field) and removed dead-code allowances.

## Commits

Initial DELIVER wave plus post-merge hardening/cleanup commits on
`feature/admin-wallet-session-driven-broadcast-signing`. See `git log --oneline develop..HEAD` for the full list.
